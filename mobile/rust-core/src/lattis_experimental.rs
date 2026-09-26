//! Puente experimental Rust → Kotlin/Swift para el laboratorio Realtime
//! (`benchmarks/realtime-rust`, rama `claude/realtime-rust-spike`).
//!
//! Espejo mobile del mismo experimento ya hecho para `desktop/src-tauri`
//! (ver `lattis_experimental.rs` de ese crate y
//! `benchmarks/realtime-rust/README.md`, "Etapa 4"). Este archivo entero
//! sólo existe con la feature `lattis-realtime-experimental` (apagada por
//! default) -- y, a diferencia de desktop (que tiene un único punto de
//! arranque donde encadenar una llamada detrás de la misma feature), acá
//! la puerta real es que ningún código de `mobile/android`/`mobile/ios`
//! invoca la función de abajo todavía: se expone vía `UniFFI` pero no hay
//! ningún botón ni pantalla que la dispare. Nunca toca el resto de este
//! crate (`Nucleo`/`AppCore` internos) -- sólo observa la conexión a un
//! proyecto Realtime configurable (pensado para `control-acceso-staging`,
//! nunca producción sin que esa decisión esté tomada; ver
//! `benchmarks/realtime-rust/README.md`, "Shadow-run en producción real")
//! y llama al observador que le pasen desde el lado nativo.

use std::time::Duration;

use lattis_realtime_spike::EventoSupervisor;

/// Implementado del lado Kotlin/Swift -- recibe cada evento del
/// supervisor ya aplanado a dos strings, mismo criterio que el DTO
/// `EventoExperimental` del lado desktop (evita exponer por la frontera
/// FFI el enum tal cual del laboratorio).
#[uniffi::export(callback_interface)]
pub trait ObservadorLattisExperimental: Send + Sync {
    fn en_evento(&self, tipo: String, detalle: String);
}

fn aplanar(evento: &EventoSupervisor) -> (String, String) {
    match evento {
        EventoSupervisor::Conectado => ("conectado".to_owned(), String::new()),
        EventoSupervisor::LatidoOk => ("latido_ok".to_owned(), String::new()),
        EventoSupervisor::Desconectado { motivo } => ("desconectado".to_owned(), motivo.clone()),
        EventoSupervisor::Reintentando { intento, espera } => (
            "reintentando".to_owned(),
            format!("intento {intento}, espera {espera:?}"),
        ),
    }
}

/// Arranca el shadow-run experimental en un hilo propio, con su propio
/// runtime de `tokio` (este crate no corre uno de por sí -- `UniFFI` expone
/// funciones síncronas normales) -- vuelve enseguida, no bloquea al
/// llamador FFI. Cada evento del supervisor se manda al `observador` que
/// le pasen desde Kotlin/Swift.
#[uniffi::export]
pub fn iniciar_shadow_run_lattis_experimental(
    url: String,
    observador: Box<dyn ObservadorLattisExperimental>,
) {
    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Runtime::new() else {
            return;
        };
        runtime.block_on(async move {
            lattis_realtime_spike::instalar_crypto_provider_tolerante();

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let supervisor = tokio::spawn(lattis_realtime_spike::supervisar_heartbeat(
                url,
                Duration::from_secs(1),
                Duration::from_secs(30),
                tx,
                || false,
            ));

            while let Some(evento) = rx.recv().await {
                let (tipo, detalle) = aplanar(&evento);
                observador.en_evento(tipo, detalle);
            }

            supervisor.abort();
        });
    });
}
