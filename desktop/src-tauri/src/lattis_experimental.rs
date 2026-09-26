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
pub fn iniciar_shadow_run_experimental<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use tauri::Listener;

    use super::{EVENTO_TAURI, VARIABLE_URL, iniciar_shadow_run_experimental};

    /// Prueba manual real contra `control-acceso-staging` -- NUNCA
    /// producción (ver el doc-comment del módulo). No corre en un `cargo
    /// test` normal sin las variables de entorno de abajo puestas a mano
    /// (`REALTIME_WS_URL`/`REALTIME_APIKEY` de `control-acceso-staging`,
    /// mismo par que usan los `smoke_*` del laboratorio): sin ellas se
    /// salta sola, sin fallar -- mismo criterio que esos binarios. Con
    /// ellas puestas, prueba el puente COMPLETO de punta a punta: conecta
    /// de verdad al WebSocket real, y confirma que
    /// `iniciar_shadow_run_experimental` (la función que
    /// `configurar_arranque` llamaría en una app real) efectivamente narra
    /// eso como un evento `Tauri` real que un `listen(...)` del lado
    /// frontend recibiría -- la prueba de la "Etapa 4 -- shadow-run" que
    /// quedaba pendiente, sin tocar producción.
    #[test]
    fn el_shadow_run_conecta_a_staging_y_emite_el_evento_tauri_real() {
        let Ok(url_base) = std::env::var("REALTIME_WS_URL") else {
            eprintln!(
                "saltando el_shadow_run_conecta_a_staging_y_emite_el_evento_tauri_real: \
                 falta REALTIME_WS_URL (prueba manual, ver doc-comment)"
            );
            return;
        };
        let Ok(apikey) = std::env::var("REALTIME_APIKEY") else {
            eprintln!(
                "saltando el_shadow_run_conecta_a_staging_y_emite_el_evento_tauri_real: \
                 falta REALTIME_APIKEY (prueba manual, ver doc-comment)"
            );
            return;
        };

        // SAFETY: es una seguridad de datos, no de memoria -- nada más en
        // este binario de tests lee/escribe esta variable en paralelo, y
        // `iniciar_shadow_run_experimental` la lee una sola vez,
        // sincrónicamente, antes de spawnear nada. `std` pide `unsafe`
        // acá desde la edición 2024 porque `set_var` ya no es atómico
        // frente a otros hilos en general, no porque este uso puntual sea
        // riesgoso.
        unsafe {
            std::env::set_var(VARIABLE_URL, format!("{url_base}?apikey={apikey}&vsn=1.0.0"));
        }

        let app = tauri::test::mock_app();
        let recibidos: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let recibidos_listener = Arc::clone(&recibidos);
        app.listen(EVENTO_TAURI, move |evento| {
            recibidos_listener
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(evento.payload().to_owned());
        });

        iniciar_shadow_run_experimental(app.handle().clone());

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
            "no llegó ningún evento {EVENTO_TAURI} desde control-acceso-staging en 15s -- \
             ¿la URL/apikey siguen siendo válidas?"
        );
        assert!(
            eventos
                .iter()
                .any(|carga| carga.contains("conectado") || carga.contains("latido_ok")),
            "llegaron eventos pero ninguno fue \"conectado\"/\"latido_ok\": {eventos:?}"
        );
    }
}
