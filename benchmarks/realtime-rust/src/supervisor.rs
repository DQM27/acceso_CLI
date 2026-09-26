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

use std::time::Duration;

use tokio::sync::mpsc::UnboundedSender;

use crate::backoff::proxima_espera;
use crate::cliente::ClienteRealtime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventoSupervisor {
    Conectado,
    LatidoOk,
    /// La conexión se perdió (falló al conectar, o se cayó en medio de la
    /// escucha) -- `motivo` es sólo para diagnóstico humano, no se compara
    /// en los tests.
    Desconectado { motivo: String },
    Reintentando { intento: u32, espera: Duration },
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

        let espera = proxima_espera(intentos_seguidos, backoff_base, backoff_tope);
        let _ = eventos.send(EventoSupervisor::Reintentando {
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
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
                            socket
                                .send(Message::Text(reply.to_string()))
                                .await
                                .unwrap();
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
                EventoSupervisor::Reintentando { intento, .. } if primer_reintento_intento.is_none() => {
                    primer_reintento_intento = Some(*intento);
                }
                _ => {}
            }
        }
        tarea.abort();

        assert_eq!(primer_reintento_intento, Some(0));
    }
}
