//! Puente hacia WebView2 (`PrintToPdf`) — crea una ventana oculta, carga el
//! HTML que arma `html.rs`, y pide el PDF.
//!
//! **El completion handler de `PrintToPdf` no es confiable en este embedding**
//! (Tauri + wry + `webview2-com` 0.38.2) — confirmado con logs de diagnóstico
//! contra datos reales, probando tres formas distintas de esperarlo (channel
//! manual anidado en `on_page_load`, `PrintToPdfCompletedHandler::wait_for_async_operation`
//! —la función pensada justo para bombear el mensaje mientras espera—, y
//! esa misma función movida afuera de `on_page_load` a un hilo aparte): en
//! los tres casos el PDF se escribe bien y rápido en disco, pero el aviso
//! de "ya terminé" nunca le llega a Rust. En vez de seguir persiguiendo por
//! qué esa entrega falla puntualmente acá, se dispara `PrintToPdf` sin
//! esperar su callback y se espera a que el archivo aparezca en disco y su
//! tamaño se estabilice — funciona porque el comportamiento real (escribe
//! rápido y bien) ya está confirmado con evidencia repetida.
//!
//! Configuración de impresión por defecto (sin `ICoreWebView2PrintSettings`
//! explícito) — confirmado con datos reales que alcanza para que la cebra
//! salga bien (Chromium respeta el `@page`/fondos del propio HTML).

use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

/// Cuánto esperar a que el PDF aparezca y se termine de escribir antes de
/// darlo por fallido. Generoso a propósito: un historial grande (miles de
/// filas) puede tardar más que uno chico, y no hay forma de saber el
/// tamaño de antemano sin duplicar la consulta.
const TIMEOUT_ESCRITURA: Duration = Duration::from_secs(45);
const INTERVALO_SONDEO: Duration = Duration::from_millis(200);
/// El archivo tiene que mantener el mismo tamaño durante esta cantidad de
/// sondeos seguidos para considerarlo "terminado de escribir" — un sondeo
/// que lo agarra a mitad de escritura vería un tamaño que sigue creciendo.
const SONDEOS_ESTABLE: u32 = 3;

/// Nombre de ventana único por llamada — dos exportaciones a la vez (poco
/// probable en esta app de un solo usuario, pero no imposible si alguien
/// hace doble clic rápido) no deben pisarse la una a la otra.
fn etiqueta_ventana() -> String {
    static CONTADOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let numero = CONTADOR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("exportar-pdf-{numero}")
}

pub async fn generar_pdf(app: &AppHandle, html: String, destino: PathBuf) -> Result<(), String> {
    let html_temporal = std::env::temp_dir().join(format!("{}.html", etiqueta_ventana()));
    std::fs::write(&html_temporal, &html).map_err(|e| format!("escribir HTML temporal: {e}"))?;
    let resultado = generar_pdf_desde_archivo(app, &html_temporal, &destino).await;
    let _ = std::fs::remove_file(&html_temporal);
    resultado
}

async fn generar_pdf_desde_archivo(
    app: &AppHandle,
    html_path: &Path,
    destino: &Path,
) -> Result<(), String> {
    let url = url::Url::from_file_path(html_path)
        .map_err(|_| "no se pudo convertir la ruta del HTML a file:// URL".to_string())?;

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let tx = std::sync::Mutex::new(Some(tx));
    let destino_evento = destino.to_owned();

    let ventana = WebviewWindowBuilder::new(app, etiqueta_ventana(), WebviewUrl::External(url))
        .visible(false)
        .on_page_load(move |webview, payload| {
            if !matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                return;
            }
            // Sólo dispara `PrintToPdf` — no espera su completion handler
            // (ver comentario del módulo). El resultado que manda acá es
            // "¿se pudo lanzar la orden de imprimir sin error?", no "¿ya
            // terminó?" — eso se confirma después, afuera de este
            // callback, sondeando el archivo.
            let resultado = lanzar_print_to_pdf(&webview, &destino_evento);
            if let Some(tx) = tx.lock().unwrap().take() {
                let _ = tx.send(resultado);
            }
        })
        .build()
        .map_err(|e| format!("crear ventana oculta para exportar: {e}"))?;

    let lanzado = rx
        .await
        .map_err(|_| "el canal se cerró sin resultado".to_string());
    let resultado = match lanzado {
        Ok(Ok(())) => esperar_archivo_listo(destino).await,
        Ok(Err(error)) => Err(error),
        Err(error) => Err(error),
    };
    let _ = ventana.close();
    resultado
}

/// Sondea `destino` hasta que exista y su tamaño se mantenga estable
/// durante [`SONDEOS_ESTABLE`] chequeos seguidos, o hasta
/// [`TIMEOUT_ESCRITURA`]. No bloquea ningún hilo del runtime — usa
/// `tokio::time::sleep`, no `std::thread::sleep`.
async fn esperar_archivo_listo(destino: &Path) -> Result<(), String> {
    let inicio = tokio::time::Instant::now();
    let mut ultimo_tamano: Option<u64> = None;
    let mut sondeos_iguales = 0u32;

    loop {
        if let Ok(metadatos) = std::fs::metadata(destino) {
            let tamano = metadatos.len();
            if tamano > 0 && Some(tamano) == ultimo_tamano {
                sondeos_iguales += 1;
                if sondeos_iguales >= SONDEOS_ESTABLE {
                    return Ok(());
                }
            } else {
                sondeos_iguales = 0;
                ultimo_tamano = Some(tamano);
            }
        }
        if inicio.elapsed() > TIMEOUT_ESCRITURA {
            return Err(
                "se agotó el tiempo de espera generando el PDF (el archivo nunca apareció \
                 completo)"
                    .to_string(),
            );
        }
        tokio::time::sleep(INTERVALO_SONDEO).await;
    }
}

#[cfg(windows)]
fn lanzar_print_to_pdf(webview: &tauri::WebviewWindow, destino: &Path) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_7;
    use windows::core::{HSTRING, Interface};

    let destino = destino.to_owned();
    // `with_webview` corre el closure de forma síncrona antes de devolver
    // el control (ver documentación de Tauri), pero igual exige `'static`
    // — el `Arc` es sólo para poder leer el resultado después de que
    // `with_webview` retorna, no porque haga falta compartirlo de verdad
    // entre hilos.
    let resultado_interno = std::sync::Arc::new(std::sync::Mutex::new(None::<Result<(), String>>));
    let resultado_interno_closure = resultado_interno.clone();
    webview
        .with_webview(move |plataforma| {
            let resultado = (|| -> Result<(), String> {
                let controller = plataforma.controller();
                let core = unsafe { controller.CoreWebView2() }.map_err(|e| e.to_string())?;
                let core7: ICoreWebView2_7 = core.cast().map_err(|e| e.to_string())?;
                let ruta = HSTRING::from(destino.to_string_lossy().to_string());
                // Handler "vacío" — no se espera su resultado (ver
                // comentario del módulo), pero `PrintToPdf` igual lo exige
                // como parámetro.
                let handler = webview2_com::PrintToPdfCompletedHandler::create(Box::new(
                    |_resultado, _mostrar_dialogo| Ok(()),
                ));
                unsafe { core7.PrintToPdf(&ruta, None, &handler) }.map_err(|e| e.to_string())
            })();
            *resultado_interno_closure.lock().unwrap() = Some(resultado);
        })
        .map_err(|e| format!("with_webview falló: {e}"))?;

    resultado_interno
        .lock()
        .unwrap()
        .take()
        .unwrap_or_else(|| Err("with_webview no llegó a ejecutar el closure".to_string()))
}

#[cfg(not(windows))]
fn lanzar_print_to_pdf(_webview: &tauri::WebviewWindow, _destino: &Path) -> Result<(), String> {
    Err("PrintToPdf sólo está implementado en Windows".to_string())
}
