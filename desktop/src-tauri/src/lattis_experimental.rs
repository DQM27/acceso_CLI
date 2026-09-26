//! Puente experimental Rust → frontend para el laboratorio Realtime
//! (`benchmarks/realtime-rust`, rama `claude/realtime-rust-spike`).
//!
//! Este módulo entero sólo existe cuando se compila con la feature
//! `lattis-realtime-experimental` (ver `Cargo.toml`) -- apagada por
//! default, así que en un build normal ni siquiera se compila este
//! archivo. Con la feature encendida, además, no hace nada salvo que se
//! configure la variable de entorno de abajo: la variable, no la
//! feature, es la puerta real. Nunca toca `comandos::nube`/`AppCore` --
//! sólo observa la conexión a un proyecto Realtime configurable (pensado
//! para `control-acceso-staging`, nunca apuntar esto a producción sin que
//! esa decisión esté tomada explícitamente; ver
//! `benchmarks/realtime-rust/README.md`, "Shadow-run en producción real")
//! y emite un evento `Tauri` de puro diagnóstico, en paralelo, sin
//! comparar ni reemplazar nada del sync real todavía.

use std::time::Duration;

use lattis_realtime_spike::EventoSupervisor;
use tauri::Emitter;
use tokio::sync::mpsc;

const VARIABLE_URL: &str = "LATTIS_EXPERIMENTAL_WS_URL";
const EVENTO_TAURI: &str = "lattis://experimental";

/// DTO serializable del evento -- `EventoSupervisor` (del laboratorio) no
/// deriva `Serialize` a propósito (no depende de `serde` sólo para esto),
/// así que se aplana a algo simple para mandarlo por el canal de eventos
/// de `Tauri`.
#[derive(Clone, serde::Serialize)]
struct EventoExperimental {
    tipo: &'static str,
    detalle: String,
}

impl From<&EventoSupervisor> for EventoExperimental {
    fn from(evento: &EventoSupervisor) -> Self {
        match evento {
            EventoSupervisor::Conectado => Self {
                tipo: "conectado",
                detalle: String::new(),
            },
            EventoSupervisor::LatidoOk => Self {
                tipo: "latido_ok",
                detalle: String::new(),
            },
            EventoSupervisor::Desconectado { motivo } => Self {
                tipo: "desconectado",
                detalle: motivo.clone(),
            },
            EventoSupervisor::Reintentando { intento, espera } => Self {
                tipo: "reintentando",
                detalle: format!("intento {intento}, espera {espera:?}"),
            },
        }
    }
}

/// Arranca el shadow-run experimental SI `LATTIS_EXPERIMENTAL_WS_URL` está
/// configurada -- en cualquier otra máquina (todas, hoy) esto vuelve sin
/// hacer nada. Por eso es seguro llamarla siempre desde
/// `configurar_arranque` detrás de la feature: la puerta real es la
/// variable de entorno, no la feature de compilación.
pub fn iniciar_shadow_run_experimental(app: tauri::AppHandle) {
    let Ok(url) = std::env::var(VARIABLE_URL) else {
        return;
    };

    tauri::async_runtime::spawn(async move {
        lattis_realtime_spike::instalar_crypto_provider_tolerante();

        let (tx, mut rx) = mpsc::unbounded_channel();
        let supervisor = tauri::async_runtime::spawn(lattis_realtime_spike::supervisar_heartbeat(
            url,
            Duration::from_secs(1),
            Duration::from_secs(30),
            tx,
            || false,
        ));

        while let Some(evento) = rx.recv().await {
            let _ = app.emit(EVENTO_TAURI, EventoExperimental::from(&evento));
        }

        supervisor.abort();
    });
}
