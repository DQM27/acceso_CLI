//! Canal privado real de Supabase Realtime (Phoenix Channels) -- reemplaza
//! al cliente `supabase-js` que usaba `desktop/src/nubeRealtime.ts` (ver
//! ese archivo: mismo contrato hacia React, `iniciarRealtimeNube`/
//! `emitirActualizacion`, así que `App.tsx`/`BarraNube.tsx` no cambiaron).
//!
//! Nace del laboratorio `benchmarks/realtime-rust`
//! (rama `claude/realtime-rust-spike`, ver su `HANDOFF.md`) -- probado de
//! punta a punta contra `control-acceso-staging` antes de reemplazar el
//! mecanismo real. Reusa `GuiState::autenticar_con_cache` para el JWT
//! (nunca reimplementa `device-auth`) y el mismo evento real `cambio_nube`
//! que ya emite `private.emitir_cambio_nube_sitio()` en Postgres.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use control_acceso::nube::{self, TokenDispositivo};
use lattis_realtime_spike::{ConfigCanalPrivado, EventoSupervisorPrivado};
use serde_json::json;
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;

use crate::comandos::nube::ejecutar_sincronizacion;
use crate::estado::GuiState;

const EVENTO_ESTADO: &str = "nube://estado_realtime";
const EVENTO_SINCRONIZADO: &str = "nube://sincronizado_realtime";
const EVENTO_CAMBIO_NUBE: &str = "cambio_nube";
/// Bien por debajo de las horas reales de `expires_in` -- sólo para que la
/// renovación proactiva (push in-band, sin reconectar) se ejercite en
/// sesiones largas, no para imitar el vencimiento real.
const INTERVALO_RENOVACION_TOKEN: Duration = Duration::from_secs(10 * 60);
/// Mismo valor que usaba `nubeRealtime.ts` (`sincronizarPorAviso`, JS) --
/// coalesce ráfagas de varios `cambio_nube` seguidos (ej. una importación
/// masiva) en una sola sincronización.
const DEBOUNCE: Duration = Duration::from_millis(600);

/// Tarea en curso, para poder pararla desde `detener_realtime_nube` o antes
/// de arrancar una nueva (cambio de sesión) -- gestionado por Tauri
/// (`.manage(EstadoRealtimeNube::default())` en `lib.rs`).
#[derive(Default)]
pub struct EstadoRealtimeNube {
    tarea: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

fn detener_tarea_previa(estado: &EstadoRealtimeNube) {
    let tarea_previa = estado
        .tarea
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    if let Some(tarea) = tarea_previa {
        tarea.abort();
    }
}

/// Pide un `TokenDispositivo` vigente reusando el mismo caché que ya usa
/// el resto de la app (`comandos::nube::autenticar`, `GuiState::autenticar_con_cache`).
/// `None` si este dispositivo todavía no se activó o la llamada de red
/// falló -- en ese caso `unirse_privado` simplemente rechaza el `phx_join`
/// (token vacío) y el supervisor reintenta solo con backoff, igual que
/// una caída de red cualquiera.
fn token_fresco<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<TokenDispositivo> {
    let secreto = nube::credenciales::cargar_secreto()?;
    app.state::<GuiState>().autenticar_con_cache(&secreto).ok()
}

fn url_websocket_realtime() -> String {
    convertir_a_url_websocket(nube::base_url(), nube::apikey())
}

/// Parte pura de [`url_websocket_realtime`], separada para poder testearla
/// sin depender de `nube::base_url()`/`apikey()` (memoizados en un
/// `OnceLock` global).
fn convertir_a_url_websocket(base_url: &str, apikey: &str) -> String {
    let base = base_url.replacen("http", "ws", 1);
    format!("{base}/realtime/v1/websocket?apikey={apikey}&vsn=1.0.0")
}

/// Arranca (o reinicia, si ya había una tarea corriendo) el canal privado
/// real para el dispositivo/sesión actual. Se llama desde React cuando
/// arranca una sesión (`App.tsx`, mismo punto que antes llamaba a
/// `iniciarRealtimeNube` de `supabase-js`) -- `usuario_cedula`/`usuario_nombre`
/// sólo se usan para Presence (panel "quién está en línea").
pub fn iniciar<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    usuario_cedula: String,
    usuario_nombre: String,
) {
    let estado = app.state::<EstadoRealtimeNube>();
    detener_tarea_previa(&estado);

    let app_tarea = app.clone();
    let handle = tauri::async_runtime::spawn(async move {
        lattis_realtime_spike::instalar_crypto_provider_tolerante();

        let app_para_token_inicial = app_tarea.clone();
        let Some(token_inicial) =
            tauri::async_runtime::spawn_blocking(move || token_fresco(&app_para_token_inicial))
                .await
                .unwrap_or(None)
        else {
            // Sin secreto configurado todavía -- no debería pasar en una
            // sesión ya logueada (hace falta el dispositivo activado para
            // llegar hasta acá), pero si pasa, no hay con qué autenticar:
            // salir tranquilo, sin reintentar solo (quien vuelva a llamar
            // `iniciar` -- ej. el próximo cambio de sesión -- lo reintenta).
            let _ = app_tarea.emit(EVENTO_ESTADO, "CHANNEL_ERROR");
            return;
        };

        let dispositivo_id_propio = token_inicial.dispositivo_id.clone();
        let topic = format!("realtime:sitio:{}", token_inicial.sitio_id);
        let token_cacheado = Arc::new(Mutex::new(token_inicial.access_token));

        let token_para_closure = Arc::clone(&token_cacheado);
        let obtener_token_fresco = move || {
            token_para_closure
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        };

        let app_para_refresco = app_tarea.clone();
        let token_para_refresco = Arc::clone(&token_cacheado);
        let refrescador = tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(INTERVALO_RENOVACION_TOKEN).await;
                let app_para_token = app_para_refresco.clone();
                if let Ok(Some(token)) =
                    tauri::async_runtime::spawn_blocking(move || token_fresco(&app_para_token))
                        .await
                {
                    *token_para_refresco
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner) = token.access_token;
                }
            }
        });

        let (tx, rx) = mpsc::unbounded_channel();
        let supervisor = tauri::async_runtime::spawn(lattis_realtime_spike::supervisar_canal_privado(
            ConfigCanalPrivado {
                url: url_websocket_realtime(),
                topic,
                evento_esperado: EVENTO_CAMBIO_NUBE.to_string(),
                backoff_base: Duration::from_secs(2),
                backoff_tope: Duration::from_secs(30),
                intervalo_heartbeat: Duration::from_secs(15),
                renovar_token_cada: Some(INTERVALO_RENOVACION_TOKEN),
                presencia: Some(json!({
                    "dispositivo_id": dispositivo_id_propio,
                    "usuario_cedula": usuario_cedula,
                    "usuario_nombre": usuario_nombre,
                })),
            },
            obtener_token_fresco,
            tx,
            || false,
        ));

        manejar_eventos(app_tarea, rx, dispositivo_id_propio).await;

        supervisor.abort();
        refrescador.abort();
    });

    *estado.tarea.lock().unwrap_or_else(PoisonError::into_inner) = Some(handle);
}

/// Detiene el canal privado -- se llama al cerrar sesión (`App.tsx`, mismo
/// punto que antes cancelaba `iniciarRealtimeNube`).
pub fn detener<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    detener_tarea_previa(&app.state::<EstadoRealtimeNube>());
}

/// Traduce los eventos del supervisor a lo que espera `nubeRealtime.ts`:
/// estado de conexión (`EVENTO_ESTADO`, para `BarraNube.tsx`) y, cuando
/// llega un `cambio_nube` que NO originó este mismo dispositivo, dispara
/// la sincronización real (`ejecutar_sincronizacion`, la misma función que
/// ya usan el pulso periódico y el botón manual) y emite el resumen
/// (`EVENTO_SINCRONIZADO`). Debounce de [`DEBOUNCE`] para coalescer
/// ráfagas -- mismo criterio que `programarSincronizacion` en la versión
/// JS que esto reemplaza.
async fn manejar_eventos<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    mut eventos: mpsc::UnboundedReceiver<EventoSupervisorPrivado>,
    dispositivo_id_propio: String,
) {
    let mut pendiente_de_sync = false;
    loop {
        let espera = tokio::time::sleep(DEBOUNCE);
        tokio::select! {
            evento = eventos.recv() => {
                match evento {
                    None => return,
                    Some(EventoSupervisorPrivado::UnidoAlCanal) => {
                        let _ = app.emit(EVENTO_ESTADO, "SUBSCRIBED");
                    }
                    Some(EventoSupervisorPrivado::EventoRecibido(payload)) => {
                        let es_propio = payload
                            .get("dispositivo_id")
                            .and_then(serde_json::Value::as_str)
                            == Some(dispositivo_id_propio.as_str());
                        if !es_propio {
                            pendiente_de_sync = true;
                        }
                    }
                    Some(EventoSupervisorPrivado::TokenRenovado) => {}
                    Some(
                        EventoSupervisorPrivado::Desconectado { .. }
                        | EventoSupervisorPrivado::Reintentando { .. },
                    ) => {
                        let _ = app.emit(EVENTO_ESTADO, "CHANNEL_ERROR");
                    }
                }
            }
            () = espera, if pendiente_de_sync => {
                pendiente_de_sync = false;
                let app_para_sync = app.clone();
                let resultado = tauri::async_runtime::spawn_blocking(move || {
                    ejecutar_sincronizacion(&app_para_sync.state::<GuiState>())
                })
                .await;
                if let Ok(Ok(resumen)) = resultado {
                    let _ = app.emit(EVENTO_SINCRONIZADO, resumen);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::convertir_a_url_websocket;

    #[test]
    fn convierte_https_a_wss_y_agrega_la_ruta_real_de_websocket() {
        assert_eq!(
            convertir_a_url_websocket("https://xidaepyaljzkpbsxrqsm.supabase.co", "clave"),
            "wss://xidaepyaljzkpbsxrqsm.supabase.co/realtime/v1/websocket?apikey=clave&vsn=1.0.0"
        );
    }
}
