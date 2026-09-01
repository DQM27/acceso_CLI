//! Controlador de teclado de la Surface de Historial (`/historial`, `/h`) —
//! §5.2/DEC-023/024. `historial.rs` es el modelo puro (filtro, resolución de
//! clave:valor); este archivo es el único que traduce teclas, consulta
//! `AppCore::buscar_historial` y decide cuándo abrir/cerrar la Surface.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent};
use rusqlite::Connection;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::application::{AppCore, exportar_historial_seleccion_con_conexion};
use crate::database::queries::ingresos::FiltroHistorial;
use crate::historial::exportacion::ColumnaHistorial as ColumnaExportacion;
use crate::services::error::RegistroIngresoServiceError;

use super::estado::{EdicionColumnas, ObjetivoColumnas};
use super::historial::HistorialState;
use super::{AppState, HistorialExportacionPendiente, NivelFeedback};

/// Abre la Surface con el catálogo de empresas (para resolver `empresa:`) y
/// el rango de fechas por defecto (mes actual) — y aplica esa consulta de
/// una vez, sin filtro todavía (DEC-063): antes hacía falta un Enter más
/// sobre la pantalla vacía sólo para ver "todo" antes de poder filtrar.
/// Si el operador necesita un filtro puntual, Esc vuelve a editar la
/// consulta (DEC-024) — el mecanismo ya existía, sólo faltaba no obligar a
/// pasar por él para ver el resultado sin filtrar.
pub(super) fn abrir_historial(core: &AppCore, app: &mut AppState) {
    let empresas = core.listar_empresas().unwrap_or_default();
    app.historial = Some(HistorialState::nuevo(empresas));
    app.input.reset();
    app.feedback = None;
    app.sugerencias.clear();
    aplicar_consulta(core, app);
}

fn cerrar_historial(core: &AppCore, app: &mut AppState) {
    app.historial = None;
    app.input.reset();
    super::recomputar(core, app);
}

pub(super) fn manejar_historial(
    core: &AppCore,
    app: &mut AppState,
    key: KeyEvent,
    pendiente: &mut HistorialExportacionPendiente,
) {
    // Con el hilo de exportación en vuelo, el teclado no tiene nada que
    // hacer acá — no hay forma de cancelarlo (mismo criterio que
    // `tui/app/historial_jobs.rs`), así que se ignora hasta que
    // `recibir_exportacion_si_lista` lo resuelva.
    let exportando_en_hilo = app.historial.as_ref().is_some_and(|h| h.exportando);
    if exportando_en_hilo {
        return;
    }
    let editando_destino = app
        .historial
        .as_ref()
        .is_some_and(|h| h.exportacion_destino.is_some());
    if editando_destino {
        manejar_exportacion(core, app, key, pendiente);
        return;
    }
    let mostrando_resultado = app
        .historial
        .as_ref()
        .is_some_and(|h| h.resultado.is_some());
    if mostrando_resultado {
        manejar_resultado(core, app, key);
    } else {
        manejar_edicion(core, app, key);
    }
}

fn manejar_edicion(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => cerrar_historial(core, app),
        KeyCode::Enter => aplicar_consulta(core, app),
        _ => {
            app.input.handle_event(&Event::Key(key));
        }
    }
}

fn manejar_resultado(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        // Esc vuelve a editar la MISMA consulta, sin perder lo ya escrito
        // (DEC-024) — no borra `app.input` ni el filtro ya resuelto.
        KeyCode::Esc => volver_a_edicion(app),
        KeyCode::Up => mover_seleccion(app, -1),
        KeyCode::Down => mover_seleccion(app, 1),
        KeyCode::PageDown => paginar(core, app, 1),
        KeyCode::PageUp => paginar(core, app, -1),
        // F4 abre el selector de columnas de la tabla de Historial — mismo
        // mecanismo que las otras dos tablas (§5.2), anidado: Esc lo cierra
        // y vuelve a mostrar los mismos resultados, sin volver a consultar.
        KeyCode::F(4) => {
            app.edicion_columnas = Some(EdicionColumnas {
                objetivo: ObjetivoColumnas::Historial,
                seleccion: 0,
            });
        }
        // F5 abre el destino de exportación — mismo atajo que la TUI
        // clásica, sobre el resultado ya aplicado en pantalla (exporta el
        // filtro completo, no sólo la página visible).
        KeyCode::F(5) => abrir_exportacion(app),
        // Cualquier tecla de edición de texto (escribir, borrar, mover el
        // cursor) vuelve a editar la consulta de una vez, con esa tecla ya
        // aplicada — antes hacía falta un Esc explícito sólo para poder
        // empezar a escribir el siguiente filtro, un paso que no aportaba
        // nada (reportado en runtime real).
        KeyCode::Char(_)
        | KeyCode::Backspace
        | KeyCode::Delete
        | KeyCode::Left
        | KeyCode::Right
        | KeyCode::Home
        | KeyCode::End => {
            volver_a_edicion(app);
            app.input.handle_event(&Event::Key(key));
        }
        _ => {}
    }
}

/// Mismo efecto que Esc sobre el resultado (DEC-024: la consulta y el filtro
/// ya resuelto no se tocan) — factorizado porque ahora también lo dispara
/// cualquier tecla de edición, no sólo Esc.
fn volver_a_edicion(app: &mut AppState) {
    if let Some(historial) = &mut app.historial {
        historial.resultado = None;
        historial.no_reconocidos.clear();
    }
}

fn abrir_exportacion(app: &mut AppState) {
    let Some(historial) = &mut app.historial else {
        return;
    };
    let sin_resultados = historial
        .resultado
        .as_ref()
        .is_none_or(|resultado| resultado.total == 0);
    if sin_resultados {
        app.mostrar_feedback(
            "No hay movimientos para exportar".to_string(),
            NivelFeedback::Advertencia,
        );
        return;
    }
    historial.exportacion_destino = Some(Input::new(ruta_exportacion_predeterminada()));
}

fn manejar_exportacion(
    core: &AppCore,
    app: &mut AppState,
    key: KeyEvent,
    pendiente: &mut HistorialExportacionPendiente,
) {
    match key.code {
        // Cancela la exportación y vuelve a mostrar el mismo resultado —
        // el filtro y la consulta ya aplicada no se tocan.
        KeyCode::Esc => {
            if let Some(historial) = &mut app.historial {
                historial.exportacion_destino = None;
            }
        }
        KeyCode::Enter => confirmar_exportacion(core, app, pendiente),
        _ => {
            if let Some(historial) = &mut app.historial
                && let Some(destino) = &mut historial.exportacion_destino
            {
                destino.handle_event(&Event::Key(key));
            }
        }
    }
}

/// Dispara la exportación en un hilo aparte en vez de bloquear el bucle —
/// medido (`docs/pendientes.md`): armar el XLSX de 100,000 movimientos
/// tarda ~33 segundos, y esta interfaz no tiene ningún otro mecanismo para
/// seguir respondiendo mientras tanto (un solo hilo, sin runtime async).
/// Mismo patrón que `tui/app/historial_jobs.rs`: hilo con su propia conexión
/// de sólo lectura al archivo, reusando `exportar_historial_seleccion_con_conexion`
/// (extraída del núcleo específicamente para esto). Exporta el filtro
/// completo (no sólo la página en pantalla) con todas las columnas del
/// exportador (`ColumnaExportacion::ALL`) — elegir un subconjunto de
/// columnas para exportar queda deliberadamente fuera de esta primera
/// versión; hoy es todo o nada.
fn confirmar_exportacion(
    core: &AppCore,
    app: &mut AppState,
    pendiente: &mut HistorialExportacionPendiente,
) {
    let Some(historial) = &app.historial else {
        return;
    };
    let Some(destino_input) = &historial.exportacion_destino else {
        return;
    };
    let destino = match normalizar_destino(destino_input.value()) {
        Ok(destino) => destino,
        Err(mensaje) => {
            app.mostrar_feedback(mensaje, NivelFeedback::Error);
            return;
        }
    };
    let mut filtro = historial.filtro.clone();
    filtro.offset = 0;

    if let Some(historial) = &mut app.historial {
        historial.exportacion_destino = None;
        historial.exportando = true;
    }
    *pendiente = Some(exportar_en_hilo(
        core.ruta_base_datos().to_path_buf(),
        filtro,
        destino,
    ));
}

fn exportar_en_hilo(
    ruta_base_datos: PathBuf,
    filtro: FiltroHistorial,
    destino: PathBuf,
) -> mpsc::Receiver<(Result<usize, String>, PathBuf)> {
    let (emisor, receptor) = mpsc::channel();
    std::thread::spawn(move || {
        let resultado: Result<usize, String> = (|| {
            let conexion = Connection::open(&ruta_base_datos).map_err(|error| error.to_string())?;
            conexion
                .busy_timeout(Duration::from_secs(5))
                .map_err(|error| error.to_string())?;
            exportar_historial_seleccion_con_conexion(
                &conexion,
                &filtro,
                None,
                &ColumnaExportacion::ALL,
                &destino,
            )
            .map_err(|error| error.to_string())
        })();
        let _ = emisor.send((resultado, destino));
    });
    receptor
}

/// Revisa sin bloquear si la exportación en curso ya terminó — llamado en
/// cada vuelta del bucle principal (`mod.rs::run`), igual que
/// `login::recibir_autenticacion`/`root::recibir_root_creado`. Devuelve si
/// acaba de resolverse, para que el bucle sepa que hay que redibujar.
pub(super) fn recibir_exportacion_si_lista(
    app: &mut AppState,
    pendiente: &mut HistorialExportacionPendiente,
) -> bool {
    let Some(receptor) = pendiente.as_ref() else {
        return false;
    };
    let Ok((resultado, destino)) = receptor.try_recv() else {
        return false;
    };
    *pendiente = None;
    if let Some(historial) = &mut app.historial {
        historial.exportando = false;
    }
    match resultado {
        Ok(cantidad) => app.mostrar_feedback(
            format!("Exportado — {cantidad} movimientos → {}", destino.display()),
            NivelFeedback::Exito,
        ),
        Err(error) => app.mostrar_feedback(
            format!("No se pudo exportar: {error}"),
            NivelFeedback::Error,
        ),
    }
    true
}

/// Mismo criterio que `ruta_exportacion_predeterminada` de la TUI clásica
/// (reescrito, no importado — DEC-002/DEC-014): nombre con fecha/hora en
/// `Documents` si existe una carpeta de usuario válida, si no sólo el
/// nombre (relativo al directorio de trabajo).
fn ruta_exportacion_predeterminada() -> String {
    let nombre = super::historial::nombre_exportacion_predeterminado();
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .filter(|ruta| ruta.is_absolute())
        .map(|ruta| ruta.join("Documents"))
        .filter(|ruta| ruta.is_dir())
        .map(|ruta| ruta.join(&nombre))
        .unwrap_or_else(|| PathBuf::from(nombre))
        .display()
        .to_string()
}

fn normalizar_destino(valor: &str) -> Result<PathBuf, String> {
    let valor = valor.trim();
    if valor.is_empty() {
        return Err("Ingrese una ruta para el archivo XLSX".to_string());
    }
    let mut destino = PathBuf::from(valor);
    match destino.extension().and_then(|extension| extension.to_str()) {
        None => destino.set_extension("xlsx"),
        Some(extension) if extension.eq_ignore_ascii_case("xlsx") => true,
        Some(_) => return Err("La exportación debe usar la extensión .xlsx".to_string()),
    };
    Ok(destino)
}

fn mover_seleccion(app: &mut AppState, delta: isize) {
    let Some(historial) = &mut app.historial else {
        return;
    };
    let Some(resultado) = &historial.resultado else {
        return;
    };
    if resultado.items.is_empty() {
        return;
    }
    let actual = historial.seleccion as isize;
    let ultimo = resultado.items.len() as isize - 1;
    historial.seleccion = (actual + delta).clamp(0, ultimo) as usize;
}

/// Interpreta el texto tecleado, arma un filtro "de cero" (sin `corte_id`:
/// una consulta nueva, no una página siguiente de la misma) y consulta.
fn aplicar_consulta(core: &AppCore, app: &mut AppState) {
    let Some(historial) = &app.historial else {
        return;
    };
    let (mut filtro, no_reconocidos) = historial.resolver_filtro(app.input.value());
    filtro.offset = 0;
    filtro.corte_id = None;
    match core.buscar_historial(&filtro) {
        Ok(pagina) => {
            if let Some(historial) = &mut app.historial {
                filtro.corte_id = Some(pagina.corte_id);
                historial.filtro = filtro;
                historial.seleccion = 0;
                historial.no_reconocidos = no_reconocidos;
                historial.resultado = Some(pagina);
            }
        }
        Err(error) => app.mostrar_feedback(mensaje_error(&error), NivelFeedback::Error),
    }
}

/// `delta` en páginas (`+1`/`-1`). El `corte_id` ya fijado en `aplicar_consulta`
/// se conserva: así ingresos nuevos registrados mientras se navega no corren
/// las páginas ya vistas.
fn paginar(core: &AppCore, app: &mut AppState, delta: isize) {
    let Some(historial) = &app.historial else {
        return;
    };
    let Some(resultado) = &historial.resultado else {
        return;
    };
    let limite = historial.filtro.limite.max(1);
    let nuevo_offset = if delta < 0 {
        historial.filtro.offset.saturating_sub(limite)
    } else if historial.filtro.offset + limite < resultado.total {
        historial.filtro.offset + limite
    } else {
        historial.filtro.offset
    };
    if nuevo_offset == historial.filtro.offset {
        return;
    }
    let mut filtro = historial.filtro.clone();
    filtro.offset = nuevo_offset;
    match core.buscar_historial(&filtro) {
        Ok(pagina) => {
            if let Some(historial) = &mut app.historial {
                historial.filtro = filtro;
                historial.seleccion = 0;
                historial.resultado = Some(pagina);
            }
        }
        Err(error) => app.mostrar_feedback(mensaje_error(&error), NivelFeedback::Error),
    }
}

fn mensaje_error(error: &RegistroIngresoServiceError) -> String {
    match error {
        RegistroIngresoServiceError::RangoFechasInvalido => {
            "El rango de fechas no es válido (desde debe ser antes que hasta)".to_string()
        }
        _ => "No se pudo consultar el historial".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruta_vacia_es_error() {
        assert!(normalizar_destino("").is_err());
        assert!(normalizar_destino("   ").is_err());
    }

    #[test]
    fn sin_extension_agrega_xlsx() {
        let destino = normalizar_destino("historial").unwrap();
        assert_eq!(destino.extension().and_then(|e| e.to_str()), Some("xlsx"));
    }

    #[test]
    fn extension_xlsx_se_conserva_sin_distinguir_mayusculas() {
        let destino = normalizar_destino("historial.XLSX").unwrap();
        assert_eq!(destino.extension().and_then(|e| e.to_str()), Some("XLSX"));
    }

    #[test]
    fn otra_extension_es_error() {
        assert!(normalizar_destino("historial.csv").is_err());
    }

    #[test]
    fn exportar_en_hilo_aparte_termina_con_el_archivo_real_en_disco() {
        use chrono::{TimeZone, Utc};

        use crate::services::usuario_service::CrearRootInicialInput;

        let unico = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directorio = std::env::temp_dir().join(format!(
            "control_acceso_cli_exportar_{}_{unico}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directorio).unwrap();
        let ruta_base_datos = directorio.join("control_acceso.sqlite");
        let core = AppCore::abrir(&ruta_base_datos).unwrap();
        core.crear_root_inicial(CrearRootInicialInput {
            cedula: "ROOT-1".into(),
            nombre: "Ana".into(),
            password: "password1".into(),
        })
        .unwrap();
        let actor = core.autenticar("ROOT-1", "password1").unwrap();

        let mut app = AppState::con_sesion(actor);
        let mut historial = HistorialState::nuevo(Vec::new());
        historial.filtro = FiltroHistorial::nuevo(
            Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap(),
        );
        let destino = directorio.join("export.xlsx");
        historial.exportacion_destino = Some(Input::new(destino.display().to_string()));
        app.historial = Some(historial);

        let mut pendiente: HistorialExportacionPendiente = None;
        confirmar_exportacion(&core, &mut app, &mut pendiente);
        assert!(app.historial.as_ref().unwrap().exportando);
        assert!(pendiente.is_some());

        for _ in 0..200 {
            if recibir_exportacion_si_lista(&mut app, &mut pendiente) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        assert!(!app.historial.as_ref().unwrap().exportando);
        assert!(pendiente.is_none());
        assert!(destino.exists(), "el archivo real debía quedar en disco");

        std::fs::remove_dir_all(&directorio).ok();
    }
}
