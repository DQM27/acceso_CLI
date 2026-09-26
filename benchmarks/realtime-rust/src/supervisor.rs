//! Etapa 3: bucle de reconexión con backoff -- el mismo criterio que
//! `iniciarRealtimeNube` en `desktop/src/nubeRealtime.ts`: si la conexión
//! se cae (falla al conectar, o el servidor la cierra en medio de la
//! escucha), esperar con backoff exponencial y reconectar solo, sin que
//! nadie tenga que reiniciar la app. El contador de intentos vuelve a 0 en
//! cuanto una conexión llega a establecerse de verdad.
//!
//! Este módulo sólo supervisa el HEARTBEAT (no un `phx_join` a un canal) --
//! alcanza para probar el mecanismo de reconexión en sí; unirlo a un canal
//! real es una composición directa que no necesita lógica nueva acá.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

use crate::backoff::proxima_espera_con_jitter;
use crate::cliente::{ClienteRealtime, ErrorCliente};

/// Semilla para el jitter del backoff (ver `backoff::proxima_espera_con_jitter`)
/// -- deliberadamente NO determinista (usa el reloj real), porque acá el
/// objetivo es justamente que dos procesos distintos que fallan al mismo
/// tiempo esparzan sus reintentos entre sí. La función pura en sí
/// (`proxima_espera_con_jitter`) ya está probada con semillas fijas en
/// `backoff.rs` -- esto sólo la alimenta con algo que varía en producción.
fn semilla_de_ahora() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventoSupervisor {
    Conectado,
    LatidoOk,
    /// La conexión se perdió (falló al conectar, o se cayó en medio de la
    /// escucha) -- `motivo` es sólo para diagnóstico humano, no se compara
    /// en los tests.
    Desconectado {
        motivo: String,
    },
    Reintentando {
        intento: u32,
        espera: Duration,
    },
}

/// Corre el bucle de conectar → latir → (si se cae) reintentar con backoff,
/// PARA SIEMPRE, salvo que `detener` devuelva `true` -- pensado así para que
/// los tests puedan cortar la corrida determinísticamente ("parar apenas
/// veas 2 `LatidoOk`") en vez de correr con un timeout arbitrario. Un uso
/// real (fuera de este laboratorio) pasaría `|| false` y lo correría en una
/// tarea de fondo indefinida, como ya hace `nubeRealtime.ts`.
pub async fn supervisar_heartbeat(
    url: String,
    backoff_base: Duration,
    backoff_tope: Duration,
    eventos: UnboundedSender<EventoSupervisor>,
    mut detener: impl FnMut() -> bool,
) {
    let mut intentos_seguidos: u32 = 0;

    loop {
        match ClienteRealtime::conectar(&url).await {
            Ok(mut cliente) => {
                intentos_seguidos = 0;
                let _ = eventos.send(EventoSupervisor::Conectado);

                loop {
                    match cliente.latido().await {
                        Ok(true) => {
                            let _ = eventos.send(EventoSupervisor::LatidoOk);
                            if detener() {
                                return;
                            }
                        }
                        Ok(false) => {
                            let _ = eventos.send(EventoSupervisor::Desconectado {
                                motivo: "el servidor no confirmó el heartbeat".to_string(),
                            });
                            break;
                        }
                        Err(error) => {
                            let _ = eventos.send(EventoSupervisor::Desconectado {
                                motivo: error.to_string(),
                            });
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                let _ = eventos.send(EventoSupervisor::Desconectado {
                    motivo: error.to_string(),
                });
            }
        }

        let espera = proxima_espera_con_jitter(
            intentos_seguidos,
            backoff_base,
            backoff_tope,
            semilla_de_ahora(),
        );
        let _ = eventos.send(EventoSupervisor::Reintentando {
            intento: intentos_seguidos,
            espera,
        });
        intentos_seguidos += 1;
        tokio::time::sleep(espera).await;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventoSupervisorPrivado {
    UnidoAlCanal,
    EventoRecibido(Value),
    /// Se empujó un JWT nuevo al canal SIN reconectar (ver
    /// `ClienteRealtime::renovar_token`).
    TokenRenovado,
    Desconectado {
        motivo: String,
    },
    Reintentando {
        intento: u32,
        espera: Duration,
    },
}

/// Agrupa lo que es dato de configuración fijo (a diferencia de
/// `obtener_token_fresco`/`eventos`/`detener`, que son comportamiento) --
/// sólo para no pasar demasiados parámetros sueltos a
/// `supervisar_canal_privado` (`clippy::too_many_arguments`).
pub struct ConfigCanalPrivado {
    pub url: String,
    pub topic: String,
    pub evento_esperado: String,
    pub backoff_base: Duration,
    pub backoff_tope: Duration,
    /// Cada cuánto mandar un heartbeat mientras no llegue ningún broadcast
    /// -- investigado: Phoenix cierra por defecto un socket que no manda
    /// NADA en ~60s (ver README.md, fuentes). Antes de esta función, este
    /// supervisor sólo escuchaba pasivo y nunca mandaba nada propio -- en
    /// un sitio silencioso (nada cambia por un rato largo) el servidor lo
    /// habría desconectado igual, sin que fuera un problema de red real.
    pub intervalo_heartbeat: Duration,
    /// `None` = nunca renovar el JWT proactivamente (sólo se pide uno
    /// fresco al reconectar, como en la versión anterior). `Some(intervalo)`
    /// empuja un `access_token` nuevo cada `intervalo` MIENTRAS el canal
    /// sigue unido, sin reconectar -- ver `ClienteRealtime::renovar_token`.
    pub renovar_token_cada: Option<Duration>,
}

/// Igual bucle que `supervisar_heartbeat`, pero uniéndose a un canal
/// PRIVADO real (`phx_join` con `access_token`) en vez de sólo latir.
///
/// `obtener_token_fresco` se llama de nuevo en CADA intento de unirse,
/// incluida cada reconexión -- nunca se cachea un token entre llamadas.
/// Esto no es un detalle menor: es la corrección estructural al bug real
/// encontrado en `supabase-py` (issue #1655, "`set_auth` doesn't update the
/// join payload, so rejoins send the old token") y `supabase-js` (issue
/// #274, "access token not refreshed... after being offline") -- ambos
/// clientes oficiales cachean el token en el payload de join y lo
/// reenvían tal cual en cada reconexión, incluso después de haberlo
/// renovado en otro lado. Acá es imposible cometer ese error porque no
/// existe ningún campo de estado donde un token viejo pueda sobrevivir
/// entre una conexión y la siguiente -- cada intento le vuelve a preguntar
/// a `obtener_token_fresco` cuál es el token de AHORA.
pub async fn supervisar_canal_privado(
    config: ConfigCanalPrivado,
    mut obtener_token_fresco: impl FnMut() -> String,
    eventos: UnboundedSender<EventoSupervisorPrivado>,
    mut detener: impl FnMut() -> bool,
) {
    let ConfigCanalPrivado {
        url,
        topic,
        evento_esperado,
        backoff_base,
        backoff_tope,
        intervalo_heartbeat,
        renovar_token_cada,
    } = config;
    let mut intentos_seguidos: u32 = 0;

    loop {
        let token = obtener_token_fresco();
        let resultado_join = match ClienteRealtime::conectar(&url).await {
            Ok(mut cliente) => match cliente.unirse_privado(&topic, &token).await {
                Ok(()) => Some(cliente),
                Err(error) => {
                    let _ = eventos.send(EventoSupervisorPrivado::Desconectado {
                        motivo: error.to_string(),
                    });
                    None
                }
            },
            Err(error) => {
                let _ = eventos.send(EventoSupervisorPrivado::Desconectado {
                    motivo: error.to_string(),
                });
                None
            }
        };

        if let Some(mut cliente) = resultado_join {
            intentos_seguidos = 0;
            let _ = eventos.send(EventoSupervisorPrivado::UnidoAlCanal);

            let mut proximo_heartbeat = std::time::Instant::now() + intervalo_heartbeat;
            let mut proxima_renovacion =
                renovar_token_cada.map(|intervalo| std::time::Instant::now() + intervalo);

            'canal: loop {
                let ahora = std::time::Instant::now();
                let mut espera = proximo_heartbeat.saturating_duration_since(ahora);
                if let Some(instante) = proxima_renovacion {
                    espera = espera.min(instante.saturating_duration_since(ahora));
                }
                // Nunca 0 -- un timeout de 0 haría que `esperar_evento`
                // reciba como mucho un frame ya en el buffer antes de
                // rendirse, en vez de esperar de verdad hasta el próximo
                // mantenimiento (heartbeat/renovación).
                let espera = espera.max(Duration::from_millis(1));

                match cliente
                    .esperar_evento(&topic, &evento_esperado, espera)
                    .await
                {
                    Ok(payload) => {
                        let _ = eventos.send(EventoSupervisorPrivado::EventoRecibido(payload));
                        if detener() {
                            return;
                        }
                    }
                    Err(ErrorCliente::Timeout(_)) => {
                        // Un timeout ACÁ no es un corte de red -- es
                        // simplemente que no pasó nada en el sitio durante
                        // `espera`. Se usa el hueco para el mantenimiento
                        // que corresponda; si no toca ninguno todavía, es
                        // un no-op y se vuelve a esperar el resto.
                        let ahora = std::time::Instant::now();
                        if ahora >= proximo_heartbeat {
                            if let Err(error) = cliente.latido().await {
                                let _ = eventos.send(EventoSupervisorPrivado::Desconectado {
                                    motivo: error.to_string(),
                                });
                                break 'canal;
                            }
                            proximo_heartbeat = std::time::Instant::now() + intervalo_heartbeat;
                        }
                        if let (Some(instante), Some(intervalo)) =
                            (proxima_renovacion, renovar_token_cada)
                            && ahora >= instante
                        {
                            let token_fresco = obtener_token_fresco();
                            if let Err(error) = cliente.renovar_token(&topic, &token_fresco).await {
                                let _ = eventos.send(EventoSupervisorPrivado::Desconectado {
                                    motivo: error.to_string(),
                                });
                                break 'canal;
                            }
                            let _ = eventos.send(EventoSupervisorPrivado::TokenRenovado);
                            proxima_renovacion = Some(std::time::Instant::now() + intervalo);
                        }
                    }
                    Err(error) => {
                        let _ = eventos.send(EventoSupervisorPrivado::Desconectado {
                            motivo: error.to_string(),
                        });
                        break 'canal;
                    }
                }
            }
        }

        let espera = proxima_espera_con_jitter(
            intentos_seguidos,
            backoff_base,
            backoff_tope,
            semilla_de_ahora(),
        );
        let _ = eventos.send(EventoSupervisorPrivado::Reintentando {
            intento: intentos_seguidos,
            espera,
        });
        intentos_seguidos += 1;
        tokio::time::sleep(espera).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    /// Servidor de prueba que acepta conexiones sin límite: responde el
    /// heartbeat normalmente, PERO en la conexión número 1 (la primera)
    /// corta el socket a propósito después de un solo heartbeat, simulando
    /// una caída real de red -- lo que fuerza a `supervisar_heartbeat` a
    /// pasar por `Desconectado` → `Reintentando` → reconectar antes de
    /// poder seguir latiendo.
    async fn servidor_que_corta_la_primera_conexion() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direccion = listener.local_addr().unwrap();
        let numero_de_conexion = Arc::new(AtomicUsize::new(0));

        tokio::spawn(async move {
            loop {
                let (flujo, _remoto) = listener.accept().await.unwrap();
                let esta_conexion = numero_de_conexion.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();
                    let mut heartbeats_respondidos = 0u32;
                    while let Some(Ok(Message::Text(texto))) = socket.next().await {
                        let entrante: serde_json::Value = serde_json::from_str(&texto).unwrap();
                        if entrante["event"] == "heartbeat" {
                            let reply = serde_json::json!({
                                "topic": "phoenix",
                                "event": "phx_reply",
                                "payload": { "status": "ok", "response": {} },
                                "ref": entrante["ref"],
                            });
                            socket.send(Message::Text(reply.to_string())).await.unwrap();
                            heartbeats_respondidos += 1;

                            // Sólo la conexión 0 (la primera) corta después
                            // del primer heartbeat -- cualquier reconexión
                            // posterior (conexión 1, 2, ...) sigue andando
                            // normal, así el test puede pedir un segundo
                            // `LatidoOk` tras la reconexión.
                            if esta_conexion == 0 && heartbeats_respondidos == 1 {
                                let _ = socket.close(None).await;
                                return;
                            }
                        }
                    }
                });
            }
        });

        format!("ws://{direccion}")
    }

    #[tokio::test]
    async fn se_reconecta_solo_tras_una_caida_y_sigue_latiendo() {
        let url = servidor_que_corta_la_primera_conexion().await;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // Backoff mínimo -- este test no está probando la ESPERA en sí (eso
        // lo prueba backoff.rs de forma pura y rápida), sino que el bucle
        // de reconexión efectivamente reconecta y retoma. Un backoff real
        // (2s-60s) sólo haría el test lento sin agregar nada.
        let base = Duration::from_millis(5);
        let tope = Duration::from_millis(50);

        let contador_latidos = Arc::new(AtomicUsize::new(0));
        let contador_para_cerrar = Arc::clone(&contador_latidos);

        let tarea = tokio::spawn(supervisar_heartbeat(url, base, tope, tx, move || {
            contador_para_cerrar.load(Ordering::SeqCst) >= 2
        }));

        let mut eventos = Vec::new();
        while let Some(evento) = rx.recv().await {
            if evento == EventoSupervisor::LatidoOk {
                contador_latidos.fetch_add(1, Ordering::SeqCst);
            }
            let detener = contador_latidos.load(Ordering::SeqCst) >= 2;
            eventos.push(evento);
            if detener {
                break;
            }
        }
        tarea.abort();

        assert!(
            eventos.contains(&EventoSupervisor::Conectado),
            "debería haberse conectado al menos una vez: {eventos:?}"
        );
        assert!(
            eventos
                .iter()
                .any(|e| matches!(e, EventoSupervisor::Desconectado { .. })),
            "debería haber detectado la caída de la primera conexión: {eventos:?}"
        );
        assert!(
            eventos
                .iter()
                .any(|e| matches!(e, EventoSupervisor::Reintentando { .. })),
            "debería haber programado un reintento: {eventos:?}"
        );
        let total_conectado = eventos
            .iter()
            .filter(|e| **e == EventoSupervisor::Conectado)
            .count();
        assert!(
            total_conectado >= 2,
            "debería haberse conectado DE NUEVO tras la caída, no sólo una vez: {eventos:?}"
        );
        assert_eq!(
            contador_latidos.load(Ordering::SeqCst),
            2,
            "un heartbeat antes de la caída y otro después de reconectar"
        );
    }

    #[tokio::test]
    async fn el_contador_de_intentos_se_reinicia_al_reconectar() {
        // Mismo servidor (corta sólo la conexión 0), pero acá lo que
        // importa es el `intento` que viaja en `Reintentando`: tiene que
        // ser 0 la primera vez (nunca reconectó antes), no un número que
        // se arrastre de una corrida anterior.
        let url = servidor_que_corta_la_primera_conexion().await;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let base = Duration::from_millis(5);
        let tope = Duration::from_millis(50);

        let contador_latidos = Arc::new(AtomicUsize::new(0));
        let contador_para_cerrar = Arc::clone(&contador_latidos);
        let tarea = tokio::spawn(supervisar_heartbeat(url, base, tope, tx, move || {
            contador_para_cerrar.load(Ordering::SeqCst) >= 2
        }));

        let mut primer_reintento_intento = None;
        while let Some(evento) = rx.recv().await {
            match &evento {
                EventoSupervisor::LatidoOk => {
                    let total = contador_latidos.fetch_add(1, Ordering::SeqCst) + 1;
                    if total >= 2 {
                        break;
                    }
                }
                EventoSupervisor::Reintentando { intento, .. }
                    if primer_reintento_intento.is_none() =>
                {
                    primer_reintento_intento = Some(*intento);
                }
                _ => {}
            }
        }
        tarea.abort();

        assert_eq!(primer_reintento_intento, Some(0));
    }

    /// Servidor que exige un `access_token` DISTINTO en cada conexión
    /// sucesiva (`"token-0"` en la primera, `"token-1"` en la segunda...) y
    /// rechaza el `phx_join` si no coincide -- si `supervisar_canal_privado`
    /// tuviera el bug real de `supabase-js`/`supabase-py` (cachear el
    /// token y reenviar el viejo al reconectar), este servidor lo
    /// detectaría rechazando la segunda conexión. Corta la conexión 0
    /// después de un evento, igual que el servidor de heartbeat.
    async fn servidor_que_exige_token_nuevo_en_cada_conexion() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direccion = listener.local_addr().unwrap();
        let numero_de_conexion = Arc::new(AtomicUsize::new(0));

        tokio::spawn(async move {
            loop {
                let (flujo, _remoto) = listener.accept().await.unwrap();
                let esta_conexion = numero_de_conexion.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();
                    let token_esperado = format!("token-{esta_conexion}");

                    while let Some(Ok(Message::Text(texto))) = socket.next().await {
                        let entrante: serde_json::Value = serde_json::from_str(&texto).unwrap();
                        if entrante["event"] != "phx_join" {
                            continue;
                        }
                        let token_recibido =
                            entrante["payload"]["access_token"].as_str().unwrap_or("");
                        let referencia = entrante["ref"].clone();

                        if token_recibido == token_esperado {
                            let reply = serde_json::json!({
                                "topic": entrante["topic"],
                                "event": "phx_reply",
                                "payload": { "status": "ok", "response": {} },
                                "ref": referencia,
                            });
                            socket.send(Message::Text(reply.to_string())).await.unwrap();

                            // Manda un broadcast (envelope real de Phoenix,
                            // ver cliente::esperar_evento) y corta -- sólo
                            // la conexión 0 corta, para forzar exactamente
                            // UNA reconexión con exactamente UN token nuevo.
                            let broadcast = serde_json::json!({
                                "topic": entrante["topic"],
                                "event": "broadcast",
                                "payload": { "event": "aviso_lab", "payload": { "conexion": esta_conexion } },
                                "ref": null,
                            });
                            socket
                                .send(Message::Text(broadcast.to_string()))
                                .await
                                .unwrap();
                            if esta_conexion == 0 {
                                let _ = socket.close(None).await;
                                return;
                            }
                        } else {
                            let reply = serde_json::json!({
                                "topic": entrante["topic"],
                                "event": "phx_reply",
                                "payload": {
                                    "status": "error",
                                    "response": { "reason": "token viejo -- se reenvió uno cacheado" },
                                },
                                "ref": referencia,
                            });
                            socket.send(Message::Text(reply.to_string())).await.unwrap();
                            let _ = socket.close(None).await;
                            return;
                        }
                    }
                });
            }
        });

        format!("ws://{direccion}")
    }

    #[tokio::test]
    async fn nunca_reenvia_un_token_viejo_al_reconectar() {
        let url = servidor_que_exige_token_nuevo_en_cada_conexion().await;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        // Un token DISTINTO cada vez que se llama -- si el supervisor
        // cacheara el primero y lo reusara en la reconexión, el servidor
        // (que espera "token-1" en la segunda conexión) lo rechazaría, y
        // este test fallaría con un `Desconectado` en vez de dos
        // `EventoRecibido`.
        let contador_tokens = Arc::new(AtomicUsize::new(0));
        let contador_para_closure = Arc::clone(&contador_tokens);
        let obtener_token_fresco = move || {
            let n = contador_para_closure.fetch_add(1, Ordering::SeqCst);
            format!("token-{n}")
        };

        let base = Duration::from_millis(5);
        let tope = Duration::from_millis(50);
        let eventos_recibidos = Arc::new(AtomicUsize::new(0));
        let contador_para_detener = Arc::clone(&eventos_recibidos);

        let tarea = tokio::spawn(supervisar_canal_privado(
            ConfigCanalPrivado {
                url,
                topic: "realtime:lab:token-rotation".to_string(),
                evento_esperado: "aviso_lab".to_string(),
                backoff_base: base,
                backoff_tope: tope,
                intervalo_heartbeat: Duration::from_secs(30),
                renovar_token_cada: None,
            },
            obtener_token_fresco,
            tx,
            move || contador_para_detener.load(Ordering::SeqCst) >= 2,
        ));

        let mut eventos = Vec::new();
        while let Some(evento) = rx.recv().await {
            if matches!(evento, EventoSupervisorPrivado::EventoRecibido(_)) {
                eventos_recibidos.fetch_add(1, Ordering::SeqCst);
            }
            let detener = eventos_recibidos.load(Ordering::SeqCst) >= 2;
            eventos.push(evento);
            if detener {
                break;
            }
        }
        tarea.abort();

        let total_recibidos = eventos
            .iter()
            .filter(|e| matches!(e, EventoSupervisorPrivado::EventoRecibido(_)))
            .count();
        assert_eq!(
            total_recibidos, 2,
            "debió recibir un evento antes de la caída y otro tras reconectar \
             con el token nuevo -- si esto es 0 o 1, probablemente reenvió un \
             token cacheado y el servidor lo rechazó: {eventos:?}"
        );
        assert!(
            !eventos
                .iter()
                .any(|e| matches!(e, EventoSupervisorPrivado::Desconectado { motivo } if motivo.contains("rechazó"))),
            "el servidor rechazó un join -- señal de que se reenvió un token viejo: {eventos:?}"
        );
    }

    /// Servidor de UNA sola conexión (si `supervisar_canal_privado`
    /// renovara reconectando en vez de empujar `access_token` in-band, este
    /// test se colgaría esperando una segunda conexión que nunca llega --
    /// `listener.accept()` se llama una única vez, a propósito). Acepta el
    /// join con `token_inicial_esperado`, después sólo entiende heartbeat
    /// (para no interferir con el intervalo de mantenimiento) y captura el
    /// primer `access_token` que llegue.
    async fn servidor_que_captura_renovacion_sin_reconectar(
        token_inicial_esperado: &'static str,
        token_renovado: Arc<tokio::sync::Mutex<Option<String>>>,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direccion = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (flujo, _remoto) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();

            let Some(Ok(Message::Text(texto))) = socket.next().await else {
                return;
            };
            let join: serde_json::Value = serde_json::from_str(&texto).unwrap();
            assert_eq!(
                join["payload"]["access_token"].as_str(),
                Some(token_inicial_esperado),
                "el join debió usar el token inicial, no uno ya renovado"
            );
            let reply = serde_json::json!({
                "topic": join["topic"], "event": "phx_reply",
                "payload": { "status": "ok", "response": {} }, "ref": join["ref"],
            });
            socket.send(Message::Text(reply.to_string())).await.unwrap();

            while let Some(Ok(Message::Text(texto))) = socket.next().await {
                let entrante: serde_json::Value = serde_json::from_str(&texto).unwrap();
                match entrante["event"].as_str() {
                    Some("access_token") => {
                        let token = entrante["payload"]["access_token"]
                            .as_str()
                            .unwrap()
                            .to_string();
                        *token_renovado.lock().await = Some(token);
                        return;
                    }
                    Some("heartbeat") => {
                        let reply = serde_json::json!({
                            "topic": "phoenix", "event": "phx_reply",
                            "payload": { "status": "ok", "response": {} }, "ref": entrante["ref"],
                        });
                        socket.send(Message::Text(reply.to_string())).await.unwrap();
                    }
                    _ => {}
                }
            }
        });

        format!("ws://{direccion}")
    }

    #[tokio::test]
    async fn renueva_el_token_sin_reconectar_y_pide_uno_fresco_no_el_del_join() {
        let token_capturado = Arc::new(tokio::sync::Mutex::new(None));
        let url =
            servidor_que_captura_renovacion_sin_reconectar("token-0", Arc::clone(&token_capturado))
                .await;

        // Un token DISTINTO cada vez que se llama -- "token-0" para el join,
        // "token-1" para la primera renovación. Si el supervisor renovara
        // con el mismo token del join (bug), este test lo detectaría
        // comparando contra "token-1" más abajo.
        let contador = Arc::new(AtomicUsize::new(0));
        let contador_closure = Arc::clone(&contador);
        let obtener_token_fresco = move || {
            let n = contador_closure.fetch_add(1, Ordering::SeqCst);
            format!("token-{n}")
        };

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let tarea = tokio::spawn(supervisar_canal_privado(
            ConfigCanalPrivado {
                url,
                topic: "realtime:lab:renovacion".to_string(),
                evento_esperado: "no_se_usa".to_string(),
                backoff_base: Duration::from_millis(10),
                backoff_tope: Duration::from_millis(100),
                // Bastante más grande que el tiempo total del test -- que
                // NO se dispare heartbeat es parte de lo que se prueba acá
                // (esta prueba es de renovación, no de heartbeat).
                intervalo_heartbeat: Duration::from_secs(10),
                renovar_token_cada: Some(Duration::from_millis(30)),
            },
            obtener_token_fresco,
            tx,
            || false,
        ));

        let resultado = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Some(token) = token_capturado.lock().await.clone() {
                    return token;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        tarea.abort();

        let token = resultado.expect("la renovación debió llegar dentro de 2s");
        assert_eq!(
            token, "token-1",
            "debió pedir un token NUEVO para renovar (token-1), no reenviar el del join (token-0)"
        );
    }

    /// Servidor que NUNCA manda ningún broadcast -- silencio total salvo lo
    /// que el propio cliente inicie. Si `supervisar_canal_privado` no
    /// mandara heartbeat durante ese silencio, un servidor Phoenix real
    /// cerraría la conexión por inactividad a los ~60s (investigado, ver
    /// README.md) -- acá se verifica que el cliente manda uno solo, antes
    /// de que haga falta ninguna desconexión real.
    async fn servidor_silencioso_que_exige_heartbeat(
        recibio_heartbeat: Arc<AtomicUsize>,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direccion = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (flujo, _remoto) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();

            let Some(Ok(Message::Text(texto))) = socket.next().await else {
                return;
            };
            let join: serde_json::Value = serde_json::from_str(&texto).unwrap();
            let reply = serde_json::json!({
                "topic": join["topic"], "event": "phx_reply",
                "payload": { "status": "ok", "response": {} }, "ref": join["ref"],
            });
            socket.send(Message::Text(reply.to_string())).await.unwrap();

            while let Some(Ok(Message::Text(texto))) = socket.next().await {
                let entrante: serde_json::Value = serde_json::from_str(&texto).unwrap();
                if entrante["event"] == "heartbeat" {
                    recibio_heartbeat.fetch_add(1, Ordering::SeqCst);
                    let reply = serde_json::json!({
                        "topic": "phoenix", "event": "phx_reply",
                        "payload": { "status": "ok", "response": {} }, "ref": entrante["ref"],
                    });
                    socket.send(Message::Text(reply.to_string())).await.unwrap();
                }
            }
        });

        format!("ws://{direccion}")
    }

    #[tokio::test]
    async fn manda_heartbeat_propio_durante_silencio_para_no_morir_por_inactividad() {
        let recibio_heartbeat = Arc::new(AtomicUsize::new(0));
        let url = servidor_silencioso_que_exige_heartbeat(Arc::clone(&recibio_heartbeat)).await;

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let tarea = tokio::spawn(supervisar_canal_privado(
            ConfigCanalPrivado {
                url,
                topic: "realtime:lab:silencio".to_string(),
                evento_esperado: "no_se_usa".to_string(),
                backoff_base: Duration::from_millis(10),
                backoff_tope: Duration::from_millis(100),
                intervalo_heartbeat: Duration::from_millis(30),
                renovar_token_cada: None,
            },
            || "token".to_string(),
            tx,
            || false,
        ));

        // Suficiente para que el intervalo de 30ms dispare varias veces sin
        // que ningún broadcast/timeout de reconexión de por medio lo tape.
        tokio::time::sleep(Duration::from_millis(200)).await;
        tarea.abort();

        assert!(
            recibio_heartbeat.load(Ordering::SeqCst) >= 1,
            "el cliente debió mandar al menos un heartbeat propio durante el silencio"
        );
    }
}
