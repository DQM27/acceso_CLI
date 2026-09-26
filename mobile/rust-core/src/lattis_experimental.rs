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

use lattis_realtime_spike::{ConfigCanalPrivado, EventoSupervisor, EventoSupervisorPrivado};

/// Mismo evento que ya emite la infraestructura real
/// (`private.emitir_cambio_nube_sitio()`) y que
/// `mobile/android/.../NubeRealtime.kt` ya escucha en producción -- este
/// puente observa el mismo canal/evento reales, no uno inventado para el
/// laboratorio (ver el espejo de escritorio, misma constante).
const EVENTO_CAMBIO_NUBE: &str = "cambio_nube";
/// Ver el espejo de escritorio (`lattis_experimental.rs` de
/// `desktop/src-tauri`) -- mismo criterio: bien por debajo de las horas
/// reales de `expires_in`, sólo para ejercitar de verdad la renovación
/// proactiva (push in-band) en un shadow-run largo.
const INTERVALO_RENOVACION_TOKEN: Duration = Duration::from_secs(10 * 60);

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

/// Lo mínimo que este puente necesita para unirse al canal PRIVADO real --
/// `access_token` vigente + `sitio_id` (para armar el topic). Implementado
/// del lado Kotlin/Swift delegando a un método que YA existe y ya se usa
/// en producción (`Nucleo::sesion_realtime_nube`/`_con_secreto`, ver
/// `mobile/rust-core/src/lib.rs`) -- Opción A del HANDOFF: este módulo
/// nunca reimplementa la obtención del JWT ni toca `Nucleo`/`AppCore`
/// directamente, sólo recibe lo que el lado nativo ya sabe conseguir.
#[derive(Debug, Clone, uniffi::Record)]
pub struct TokenLattisExperimental {
    pub access_token: String,
    pub sitio_id: String,
}

/// Ver [`TokenLattisExperimental`]. `None` si todavía no hay con qué
/// autenticar (sin sesión de usuario activa, sin secreto de dispositivo
/// guardado, o la llamada de red falló) -- no es un error del puente, sólo
/// significa que no hay nada que observar todavía.
#[uniffi::export(callback_interface)]
pub trait ProveedorTokenLattisExperimental: Send + Sync {
    fn token_fresco(&self) -> Option<TokenLattisExperimental>;
}

fn aplanar_privado(evento: &EventoSupervisorPrivado) -> (String, String) {
    match evento {
        EventoSupervisorPrivado::UnidoAlCanal => ("unido_al_canal".to_owned(), String::new()),
        EventoSupervisorPrivado::EventoRecibido(payload) => {
            ("evento_recibido".to_owned(), payload.to_string())
        }
        EventoSupervisorPrivado::TokenRenovado => ("token_renovado".to_owned(), String::new()),
        EventoSupervisorPrivado::Desconectado { motivo } => {
            ("desconectado".to_owned(), motivo.clone())
        }
        EventoSupervisorPrivado::Reintentando { intento, espera } => (
            "reintentando".to_owned(),
            format!("intento {intento}, espera {espera:?}"),
        ),
    }
}

/// Arranca el shadow-run del canal PRIVADO real (JWT de dispositivo +
/// datos reales) en un hilo y runtime propios -- igual criterio que
/// [`iniciar_shadow_run_lattis_experimental`] (vuelve enseguida, no
/// bloquea al llamador FFI). A diferencia de esa función (heartbeat
/// público), acá `proveedor_token` SÍ puede hacer red (vía
/// `sesion_realtime_nube_con_secreto`, bloqueante) -- inofensivo porque
/// corre en el runtime dedicado de este hilo, nunca en el runtime
/// compartido del resto de la app (a diferencia de `desktop/src-tauri`,
/// que sí comparte un único runtime de Tauri y por eso ese espejo separa
/// la renovación en una tarea aparte -- ver el doc-comment de
/// `INTERVALO_RENOVACION_TOKEN` ahí).
///
/// `url_websocket_realtime` -- a diferencia de desktop, acá lo arma quien
/// llama (Kotlin ya conoce `base_url()`/`apikey()` vía
/// `sesion_realtime_nube`, o los puede pedir con
/// `control_acceso::nube::base_url()`/`apikey()` si hiciera falta
/// exponerlos) para no duplicar esa conversión en dos lugares del mismo
/// binario.
#[uniffi::export]
pub fn iniciar_shadow_run_canal_privado_lattis_experimental(
    url_websocket_realtime: String,
    proveedor_token: Box<dyn ProveedorTokenLattisExperimental>,
    observador: Box<dyn ObservadorLattisExperimental>,
) {
    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Runtime::new() else {
            return;
        };
        runtime.block_on(async move {
            lattis_realtime_spike::instalar_crypto_provider_tolerante();

            // Sin sesión/secreto todavía -- nada que observar. Kotlin
            // decide cuándo volver a llamar esta función (por ejemplo,
            // justo después de un login exitoso).
            let Some(inicial) = proveedor_token.token_fresco() else {
                return;
            };
            let topic = format!("realtime:sitio:{}", inicial.sitio_id);

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let supervisor = tokio::spawn(lattis_realtime_spike::supervisar_canal_privado(
                ConfigCanalPrivado {
                    url: url_websocket_realtime,
                    topic,
                    evento_esperado: EVENTO_CAMBIO_NUBE.to_string(),
                    backoff_base: Duration::from_secs(2),
                    backoff_tope: Duration::from_secs(30),
                    intervalo_heartbeat: Duration::from_secs(15),
                    renovar_token_cada: Some(INTERVALO_RENOVACION_TOKEN),
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
                let (tipo, detalle) = aplanar_privado(&evento);
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

    use super::{
        ProveedorTokenLattisExperimental, TokenLattisExperimental,
        iniciar_shadow_run_canal_privado_lattis_experimental,
    };

    struct ProveedorDePrueba {
        access_token: String,
        sitio_id: String,
    }

    impl ProveedorTokenLattisExperimental for ProveedorDePrueba {
        fn token_fresco(&self) -> Option<TokenLattisExperimental> {
            Some(TokenLattisExperimental {
                access_token: self.access_token.clone(),
                sitio_id: self.sitio_id.clone(),
            })
        }
    }

    /// Prueba manual real contra `control-acceso-staging` -- NUNCA
    /// producción (ver el doc-comment del módulo). No corre en un `cargo
    /// test` normal sin las cuatro variables de entorno de abajo puestas a
    /// mano (mismo par base que el test de arriba, más
    /// `REALTIME_DEVICE_JWT`/`REALTIME_SITIO_ID` de un dispositivo
    /// descartable real -- mismo criterio que
    /// `benchmarks/realtime-rust/src/bin/smoke_supervisor_privado.rs`):
    /// sin ellas se salta sola, sin fallar. Con ellas puestas, prueba el
    /// puente COMPLETO de punta a punta -- incluida la parte NUEVA de este
    /// espejo mobile (el `callback_interface`
    /// `ProveedorTokenLattisExperimental` y el hilo/runtime propio) --
    /// contra el WebSocket real, uniéndose al canal PRIVADO real. La
    /// entrega real de un broadcast `cambio_nube` (el mecanismo en sí, no
    /// esta wiring) ya se probó de punta a punta contra este mismo
    /// proyecto desde el espejo de escritorio (ver HANDOFF.md,
    /// 2026-09-26) -- este test alcanza con confirmar que el `phx_join`
    /// privado real se completa a través de la nueva interfaz `UniFFI`.
    #[test]
    fn el_shadow_run_privado_conecta_a_staging_con_jwt_real_y_se_une_al_canal() {
        let Ok(url_base) = std::env::var("REALTIME_WS_URL") else {
            eprintln!(
                "saltando el_shadow_run_privado_conecta_a_staging_con_jwt_real_y_se_une_al_canal: \
                 falta REALTIME_WS_URL (prueba manual, ver doc-comment)"
            );
            return;
        };
        let Ok(apikey) = std::env::var("REALTIME_APIKEY") else {
            eprintln!(
                "saltando el_shadow_run_privado_conecta_a_staging_con_jwt_real_y_se_une_al_canal: \
                 falta REALTIME_APIKEY (prueba manual, ver doc-comment)"
            );
            return;
        };
        let Ok(jwt_dispositivo) = std::env::var("REALTIME_DEVICE_JWT") else {
            eprintln!(
                "saltando el_shadow_run_privado_conecta_a_staging_con_jwt_real_y_se_une_al_canal: \
                 falta REALTIME_DEVICE_JWT (prueba manual, ver doc-comment)"
            );
            return;
        };
        let Ok(sitio_id) = std::env::var("REALTIME_SITIO_ID") else {
            eprintln!(
                "saltando el_shadow_run_privado_conecta_a_staging_con_jwt_real_y_se_une_al_canal: \
                 falta REALTIME_SITIO_ID (prueba manual, ver doc-comment)"
            );
            return;
        };
        let url = format!("{url_base}?apikey={apikey}&vsn=1.0.0");

        let recibidos = Arc::new(Mutex::new(Vec::new()));
        let observador = Box::new(ObservadorDePrueba {
            recibidos: Arc::clone(&recibidos),
        });
        let proveedor = Box::new(ProveedorDePrueba {
            access_token: jwt_dispositivo,
            sitio_id,
        });

        iniciar_shadow_run_canal_privado_lattis_experimental(url, proveedor, observador);

        let limite = Instant::now() + Duration::from_secs(15);
        while Instant::now() < limite {
            if recibidos
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .iter()
                .any(|(tipo, _)| tipo == "unido_al_canal")
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
            eventos.iter().any(|(tipo, _)| tipo == "unido_al_canal"),
            "no se unió al canal privado real en 15s -- ¿el JWT/sitio_id siguen siendo \
             válidos? eventos recibidos: {eventos:?}"
        );
    }
}
