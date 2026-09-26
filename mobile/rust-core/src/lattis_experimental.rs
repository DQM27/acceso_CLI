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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::{ObservadorLattisExperimental, iniciar_shadow_run_lattis_experimental};

    struct ObservadorDePrueba {
        recibidos: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl ObservadorLattisExperimental for ObservadorDePrueba {
        fn en_evento(&self, tipo: String, detalle: String) {
            self.recibidos
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push((tipo, detalle));
        }
    }

    /// Prueba manual real contra `control-acceso-staging` -- NUNCA
    /// producción (ver el doc-comment del módulo). No corre en un `cargo
    /// test` normal sin las variables de entorno de abajo puestas a mano
    /// (`REALTIME_WS_URL`/`REALTIME_APIKEY` de `control-acceso-staging`,
    /// mismo par que usan los `smoke_*` del laboratorio): sin ellas se
    /// salta sola, sin fallar. Con ellas puestas, prueba el puente
    /// COMPLETO de punta a punta: conecta de verdad al WebSocket real y
    /// confirma que `iniciar_shadow_run_lattis_experimental` (la función
    /// expuesta por `UniFFI`) efectivamente llama al `callback_interface`
    /// que implementaría Kotlin/Swift -- la prueba de la "Etapa 4 --
    /// shadow-run" que quedaba pendiente, sin tocar producción.
    #[test]
    fn el_shadow_run_conecta_a_staging_y_llama_al_observador_real() {
        let Ok(url_base) = std::env::var("REALTIME_WS_URL") else {
            eprintln!(
                "saltando el_shadow_run_conecta_a_staging_y_llama_al_observador_real: \
                 falta REALTIME_WS_URL (prueba manual, ver doc-comment)"
            );
            return;
        };
        let Ok(apikey) = std::env::var("REALTIME_APIKEY") else {
            eprintln!(
                "saltando el_shadow_run_conecta_a_staging_y_llama_al_observador_real: \
                 falta REALTIME_APIKEY (prueba manual, ver doc-comment)"
            );
            return;
        };
        let url = format!("{url_base}?apikey={apikey}&vsn=1.0.0");

        let recibidos = Arc::new(Mutex::new(Vec::new()));
        let observador = Box::new(ObservadorDePrueba {
            recibidos: Arc::clone(&recibidos),
        });

        iniciar_shadow_run_lattis_experimental(url, observador);

        let limite = Instant::now() + Duration::from_secs(15);
        while Instant::now() < limite {
            if !recibidos
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        let eventos = recibidos
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert!(
            !eventos.is_empty(),
            "no llegó ningún evento desde control-acceso-staging en 15s -- \
             ¿la URL/apikey siguen siendo válidas?"
        );
        assert!(
            eventos
                .iter()
                .any(|(tipo, _)| tipo == "conectado" || tipo == "latido_ok"),
            "llegaron eventos pero ninguno fue \"conectado\"/\"latido_ok\": {eventos:?}"
        );
    }
}
