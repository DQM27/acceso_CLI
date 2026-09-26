//! Canal privado real de Supabase Realtime (Phoenix Channels) -- reemplaza
//! a `io.github.jan.supabase.realtime` que usaba `NubeRealtime.kt` (ver ese
//! archivo: mismo contrato hacia Kotlin -- `ObservadorRealtimeNube.enCambioRemoto()`
//! sigue siendo lo único que dispara `CambiosNube.solicitar()`, que
//! `SincronizacionPeriodica` ya escucha y debounce -- así que ese archivo
//! es el único que cambió del lado nativo).
//!
//! Nace del laboratorio `benchmarks/realtime-rust`
//! (rama `claude/realtime-rust-spike`, ver su `HANDOFF.md`) -- probado de
//! punta a punta contra `control-acceso-staging` antes de reemplazar el
//! mecanismo real. Nunca toca `Nucleo`/`AppCore` directamente -- recibe el
//! JWT ya vigente desde Kotlin (`ProveedorTokenRealtimeNube`, que delega a
//! `Nucleo::sesion_realtime_nube_con_secreto`, el mismo método que ya usaba
//! `NubeRealtime.kt`), evitando reimplementar la obtención del JWT acá.

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use lattis_realtime_spike::{ConfigCanalPrivado, EventoSupervisorPrivado};
use serde_json::json;

const EVENTO_CAMBIO_NUBE: &str = "cambio_nube";
/// Ver el espejo de escritorio (`desktop/src-tauri/src/realtime_nube.rs`)
/// -- mismo criterio: bien por debajo de las horas reales de `expires_in`,
/// sólo para ejercitar de verdad la renovación proactiva (push in-band) en
/// una sesión larga.
const INTERVALO_RENOVACION_TOKEN: Duration = Duration::from_secs(10 * 60);

/// Lo mínimo que este puente necesita para unirse al canal PRIVADO real --
/// `access_token` vigente, `sitio_id` (para el topic) y `dispositivo_id`
/// (para Presence). Implementado del lado Kotlin/Swift delegando a
/// `Nucleo::sesion_realtime_nube_con_secreto`/`sesion_realtime_nube`.
#[derive(Debug, Clone, uniffi::Record)]
pub struct TokenRealtimeNube {
    pub access_token: String,
    pub sitio_id: String,
    pub dispositivo_id: String,
}

/// `None` si todavía no hay con qué autenticar (sin sesión activa, sin
/// secreto de dispositivo guardado, o la llamada de red falló) -- no es un
/// error del puente, sólo significa que no hay nada que observar todavía.
#[uniffi::export(callback_interface)]
pub trait ProveedorTokenRealtimeNube: Send + Sync {
    fn token_fresco(&self) -> Option<TokenRealtimeNube>;
}

/// Único evento que le importa a Kotlin -- un cambio remoto llegó
/// (`cambio_nube`). A diferencia del espejo de escritorio, acá no hace
/// falta pasar el payload ni filtrar el eco del propio dispositivo:
/// `NubeRealtime.kt` tampoco lo hacía (`SincronizacionPeriodica` ya
/// debounce y corre `sincronizar_con_nube` sin importar quién disparó el
/// aviso), así que esto conserva el mismo comportamiento 1:1.
#[uniffi::export(callback_interface)]
pub trait ObservadorRealtimeNube: Send + Sync {
    fn en_cambio_remoto(&self);
}

/// Handle opaco de la tarea en curso -- `detener()` la para (logout,
/// cambio de sesión). Guarda el `Runtime` propio de esta conexión entero
/// (este crate no corre un runtime async de por sí, `UniFFI` expone
/// funciones síncronas): pararla es simplemente apagarlo, sin esperar a
/// que las tareas terminen solas.
#[derive(uniffi::Object)]
pub struct TareaRealtimeNube {
    runtime: Mutex<Option<tokio::runtime::Runtime>>,
}

#[uniffi::export]
impl TareaRealtimeNube {
    pub fn detener(&self) {
        let runtime = self
            .runtime
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(runtime) = runtime {
            runtime.shutdown_background();
        }
    }
}

fn url_websocket_realtime(base_url: &str, apikey: &str) -> String {
    let base = base_url.replacen("http", "ws", 1);
    format!("{base}/realtime/v1/websocket?apikey={apikey}&vsn=1.0.0")
}

/// Arranca el canal privado real para el dispositivo/sesión actual --
/// `base_url`/`apikey` son los mismos que ya expone `SesionRealtimeNube`
/// (`Nucleo::sesion_realtime_nube_con_secreto`), así que Kotlin no necesita
/// pedirlos aparte. `usuario_cedula`/`usuario_nombre` sólo se usan para
/// Presence (panel "quién está en línea").
#[uniffi::export]
pub fn iniciar_realtime_nube(
    base_url: String,
    apikey: String,
    usuario_cedula: String,
    usuario_nombre: String,
    proveedor_token: Box<dyn ProveedorTokenRealtimeNube>,
    observador: Box<dyn ObservadorRealtimeNube>,
) -> std::sync::Arc<TareaRealtimeNube> {
    let runtime = tokio::runtime::Runtime::new().expect("no se pudo armar el runtime de Realtime");

    runtime.spawn(async move {
        lattis_realtime_spike::instalar_crypto_provider_tolerante();

        let Some(inicial) = proveedor_token.token_fresco() else {
            return;
        };
        let topic = format!("realtime:sitio:{}", inicial.sitio_id);
        let url = url_websocket_realtime(&base_url, &apikey);

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let supervisor = tokio::spawn(lattis_realtime_spike::supervisar_canal_privado(
            ConfigCanalPrivado {
                url,
                topic,
                evento_esperado: EVENTO_CAMBIO_NUBE.to_string(),
                backoff_base: Duration::from_secs(2),
                backoff_tope: Duration::from_secs(30),
                intervalo_heartbeat: Duration::from_secs(15),
                renovar_token_cada: Some(INTERVALO_RENOVACION_TOKEN),
                presencia: Some(json!({
                    "dispositivo_id": inicial.dispositivo_id,
                    "usuario_cedula": usuario_cedula,
                    "usuario_nombre": usuario_nombre,
                })),
            },
            move || {
                proveedor_token
                    .token_fresco()
                    .map(|token| token.access_token)
                    .unwrap_or_default()
            },
            tx,
            || false,
        ));

        while let Some(evento) = rx.recv().await {
            if matches!(evento, EventoSupervisorPrivado::EventoRecibido(_)) {
                observador.en_cambio_remoto();
            }
        }

        supervisor.abort();
    });

    std::sync::Arc::new(TareaRealtimeNube {
        runtime: Mutex::new(Some(runtime)),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{
        ObservadorRealtimeNube, ProveedorTokenRealtimeNube, TokenRealtimeNube,
        iniciar_realtime_nube, url_websocket_realtime,
    };

    #[test]
    fn convierte_https_a_wss_y_agrega_la_ruta_real_de_websocket() {
        assert_eq!(
            url_websocket_realtime("https://xidaepyaljzkpbsxrqsm.supabase.co", "clave"),
            "wss://xidaepyaljzkpbsxrqsm.supabase.co/realtime/v1/websocket?apikey=clave&vsn=1.0.0"
        );
    }

    struct ProveedorDePrueba {
        token: TokenRealtimeNube,
    }
    impl ProveedorTokenRealtimeNube for ProveedorDePrueba {
        fn token_fresco(&self) -> Option<TokenRealtimeNube> {
            Some(self.token.clone())
        }
    }

    struct ObservadorDePrueba {
        cambios: Arc<Mutex<u32>>,
    }
    impl ObservadorRealtimeNube for ObservadorDePrueba {
        fn en_cambio_remoto(&self) {
            *self
                .cambios
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) += 1;
        }
    }

    /// Prueba manual real contra `control-acceso-staging` -- NUNCA
    /// producción. No corre en un `cargo test` normal sin las cuatro
    /// variables de entorno de abajo puestas a mano (mismo criterio que
    /// `benchmarks/realtime-rust/src/bin/smoke_supervisor_privado.rs`):
    /// sin ellas se salta sola, sin fallar.
    #[test]
    fn se_une_al_canal_privado_real_y_llama_al_observador_ante_un_cambio_remoto() {
        use std::time::Duration;

        let Ok(base_url) = std::env::var("REALTIME_BASE_URL") else {
            eprintln!(
                "saltando se_une_al_canal_privado_real_y_llama_al_observador_ante_un_cambio_remoto: \
                 falta REALTIME_BASE_URL (prueba manual, ver doc-comment)"
            );
            return;
        };
        let Ok(apikey) = std::env::var("REALTIME_APIKEY") else {
            eprintln!("falta REALTIME_APIKEY");
            return;
        };
        let Ok(jwt_dispositivo) = std::env::var("REALTIME_DEVICE_JWT") else {
            eprintln!("falta REALTIME_DEVICE_JWT");
            return;
        };
        let Ok(sitio_id) = std::env::var("REALTIME_SITIO_ID") else {
            eprintln!("falta REALTIME_SITIO_ID");
            return;
        };
        let dispositivo_id = std::env::var("REALTIME_DISPOSITIVO_ID").unwrap_or_default();

        let cambios = Arc::new(Mutex::new(0_u32));
        let tarea = iniciar_realtime_nube(
            base_url,
            apikey,
            String::new(),
            String::new(),
            Box::new(ProveedorDePrueba {
                token: TokenRealtimeNube {
                    access_token: jwt_dispositivo,
                    sitio_id,
                    dispositivo_id,
                },
            }),
            Box::new(ObservadorDePrueba {
                cambios: Arc::clone(&cambios),
            }),
        );

        // Espera hasta 15s a que `en_cambio_remoto` se dispare de verdad --
        // requiere un cambio real disparado a mano contra `sitio_id` (ver
        // HANDOFF.md, "probado con un dispositivo descartable") mientras
        // este test corre.
        let limite = std::time::Instant::now() + Duration::from_secs(15);
        while std::time::Instant::now() < limite {
            if *cambios.lock().unwrap_or_else(std::sync::PoisonError::into_inner) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        tarea.detener();
        assert!(
            *cambios.lock().unwrap_or_else(std::sync::PoisonError::into_inner) > 0,
            "no llegó ningún cambio remoto en 15s -- ¿se disparó un cambio real contra este \
             sitio_id mientras corría el test?"
        );
    }
}
