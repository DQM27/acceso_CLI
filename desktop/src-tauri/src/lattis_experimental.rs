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

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use control_acceso::nube::{self, TokenDispositivo};
use lattis_realtime_spike::{ConfigCanalPrivado, EventoSupervisor, EventoSupervisorPrivado};
use tauri::{Emitter, Manager};
use tokio::sync::mpsc;

use crate::estado::GuiState;

const VARIABLE_URL: &str = "LATTIS_EXPERIMENTAL_WS_URL";
const EVENTO_TAURI: &str = "lattis://experimental";

/// Puerta del canal privado real -- a diferencia de `VARIABLE_URL` (que
/// exige pegar la URL completa a mano), acá alcanza con que la variable
/// EXISTA (cualquier valor) porque la URL/apikey/JWT/topic salen todos de
/// configuración real (`nube::base_url()`/`apikey()`) y del secreto de
/// ESTE dispositivo (`nube::credenciales::cargar_secreto()`) -- nunca de
/// algo que quien arranca la app tenga que pegar. Por eso mismo es más
/// peligrosa que `VARIABLE_URL`: si el dispositivo ya está activado contra
/// **producción** (`control-acceso-nube`, el default de `nube::base_url()`
/// sin `CONTROL_ACCESO_SUPABASE_URL`), encender esta variable conecta el
/// canal privado real contra producción -- exactamente lo que el HANDOFF
/// (paso 7) dice que nunca se activa sin decisión explícita. Sólo
/// encenderla en una máquina con `scripts/activar_sandbox.ps1` corrido
/// (apunta a `control-acceso-staging`) o con
/// `CONTROL_ACCESO_SUPABASE_URL`/`APIKEY` puestas a mano.
const VARIABLE_CANAL_PRIVADO: &str = "LATTIS_EXPERIMENTAL_CANAL_PRIVADO";
const EVENTO_TAURI_PRIVADO: &str = "lattis://experimental-privado";
/// Mismo evento que ya emite la infraestructura real
/// (`private.emitir_cambio_nube_sitio()`, ver
/// `supabase/migrations/*emitir_cambio_nube*.sql`) y que
/// `desktop/src/nubeRealtime.ts` ya escucha en producción -- este puente
/// observa el mismo canal/evento reales, no uno inventado para el
/// laboratorio.
const EVENTO_CAMBIO_NUBE: &str = "cambio_nube";
/// Margen bien por debajo de `expires_in` real (horas) -- sólo importa que
/// sea más frecuente que el vencimiento para que `renovar_token` (push
/// in-band) se ejercite de verdad en un shadow-run largo, no que imite el
/// vencimiento real.
const INTERVALO_RENOVACION_TOKEN: Duration = Duration::from_secs(10 * 60);

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

/// DTO serializable del evento del canal privado -- mismo criterio que
/// `EventoExperimental` (aplanar en vez de derivar `Serialize` en el
/// laboratorio). `EventoRecibido` sólo manda el payload crudo (JSON) tal
/// cual llegó -- este puente es puramente observador, no lo interpreta.
#[derive(Clone, serde::Serialize)]
struct EventoExperimentalPrivado {
    tipo: &'static str,
    detalle: String,
}

impl From<&EventoSupervisorPrivado> for EventoExperimentalPrivado {
    fn from(evento: &EventoSupervisorPrivado) -> Self {
        match evento {
            EventoSupervisorPrivado::UnidoAlCanal => Self {
                tipo: "unido_al_canal",
                detalle: String::new(),
            },
            EventoSupervisorPrivado::EventoRecibido(payload) => Self {
                tipo: "evento_recibido",
                detalle: payload.to_string(),
            },
            EventoSupervisorPrivado::TokenRenovado => Self {
                tipo: "token_renovado",
                detalle: String::new(),
            },
            EventoSupervisorPrivado::Desconectado { motivo } => Self {
                tipo: "desconectado",
                detalle: motivo.clone(),
            },
            EventoSupervisorPrivado::Reintentando { intento, espera } => Self {
                tipo: "reintentando",
                detalle: format!("intento {intento}, espera {espera:?}"),
            },
        }
    }
}

/// Pide un `TokenDispositivo` vigente para ESTE dispositivo, reusando el
/// mismo caché que ya usa el resto de la app (`GuiState::autenticar_con_cache`,
/// ver `comandos::nube::autenticar`) -- Opción A del HANDOFF: nunca
/// reimplementar la llamada a `device-auth` acá. `None` si este
/// dispositivo todavía no se activó (`cargar_secreto` vacío) o si la
/// llamada de red falló -- en ambos casos no hay nada que observar
/// todavía, no es un error del puente.
fn token_fresco<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<TokenDispositivo> {
    let secreto = nube::credenciales::cargar_secreto()?;
    app.state::<GuiState>().autenticar_con_cache(&secreto).ok()
}

/// Arranca el shadow-run del canal PRIVADO real (JWT de dispositivo +
/// datos reales) SI `LATTIS_EXPERIMENTAL_CANAL_PRIVADO` está configurada Y
/// este dispositivo ya se activó contra algún proyecto (`cargar_secreto`
/// no vacío) -- en cualquier otro caso vuelve sin hacer nada, igual que
/// `iniciar_shadow_run_experimental`. A diferencia de esa función
/// (heartbeat público, sin JWT), ésta sí llama a `GuiState` -- pero sólo a
/// `autenticar_con_cache` (que ya usa producción para el sync real), nunca
/// a `AppCore`/`core()` completo.
pub fn iniciar_shadow_run_canal_privado_experimental<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) {
    if std::env::var(VARIABLE_CANAL_PRIVADO).is_err() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        lattis_realtime_spike::instalar_crypto_provider_tolerante();

        let app_para_token_inicial = app.clone();
        let Some(token_inicial) =
            tauri::async_runtime::spawn_blocking(move || token_fresco(&app_para_token_inicial))
                .await
                .unwrap_or(None)
        else {
            // Sin secreto configurado, o `device-auth` no respondió -- no
            // hay con qué autenticar el canal todavía. No reintenta solo:
            // el próximo arranque de la app lo vuelve a intentar.
            return;
        };

        let topic = format!("realtime:sitio:{}", token_inicial.sitio_id);
        let token_cacheado = Arc::new(Mutex::new(token_inicial.access_token));

        let token_para_closure = Arc::clone(&token_cacheado);
        let obtener_token_fresco = move || {
            token_para_closure
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        };

        let app_para_refresco = app.clone();
        let token_para_refresco = Arc::clone(&token_cacheado);
        let refrescador = tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(INTERVALO_RENOVACION_TOKEN).await;
                let app_para_token = app_para_refresco.clone();
                if let Ok(Some(token)) =
                    tauri::async_runtime::spawn_blocking(move || token_fresco(&app_para_token))
                        .await
                {
                    *token_para_refresco.lock().unwrap_or_else(PoisonError::into_inner) =
                        token.access_token;
                }
            }
        });

        let (tx, mut rx) = mpsc::unbounded_channel();
        let supervisor = tauri::async_runtime::spawn(lattis_realtime_spike::supervisar_canal_privado(
            ConfigCanalPrivado {
                url: url_websocket_realtime(),
                topic,
                evento_esperado: EVENTO_CAMBIO_NUBE.to_string(),
                backoff_base: Duration::from_secs(2),
                backoff_tope: Duration::from_secs(30),
                intervalo_heartbeat: Duration::from_secs(15),
                renovar_token_cada: Some(INTERVALO_RENOVACION_TOKEN),
            },
            obtener_token_fresco,
            tx,
            || false,
        ));

        while let Some(evento) = rx.recv().await {
            let _ = app.emit(EVENTO_TAURI_PRIVADO, EventoExperimentalPrivado::from(&evento));
        }

        supervisor.abort();
        refrescador.abort();
    });
}

/// `nube::base_url()` es HTTP(S) (para las llamadas REST/`device-auth`) --
/// el WebSocket de Realtime vive en el mismo host, mismo esquema pero
/// `wss://` y la ruta fija `/realtime/v1/websocket`. `apikey` viaja como
/// query param, igual que en los `smoke_*` del laboratorio y que
/// `supabase-js` arma internamente para `nubeRealtime.ts`.
fn url_websocket_realtime() -> String {
    convertir_a_url_websocket(nube::base_url(), nube::apikey())
}

/// Parte pura de [`url_websocket_realtime`], separada para poder testearla
/// sin depender de `nube::base_url()`/`apikey()` (memoizados en un
/// `OnceLock` global -- una vez fijados por el primer test que los toque
/// en este binario, quedan así para el resto de los tests del mismo
/// proceso).
fn convertir_a_url_websocket(base_url: &str, apikey: &str) -> String {
    let base = base_url.replacen("http", "ws", 1);
    format!("{base}/realtime/v1/websocket?apikey={apikey}&vsn=1.0.0")
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use tauri::Listener;

    use super::{
        EVENTO_TAURI, VARIABLE_URL, convertir_a_url_websocket, iniciar_shadow_run_experimental,
    };

    #[test]
    fn convierte_https_a_wss_y_agrega_la_ruta_real_de_websocket() {
        assert_eq!(
            convertir_a_url_websocket("https://xidaepyaljzkpbsxrqsm.supabase.co", "clave"),
            "wss://xidaepyaljzkpbsxrqsm.supabase.co/realtime/v1/websocket?apikey=clave&vsn=1.0.0"
        );
    }

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
