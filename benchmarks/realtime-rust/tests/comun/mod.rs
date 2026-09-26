//! Helpers compartidos por las pruebas de integración/punta-a-punta de
//! `tests/*.rs`. Vive en un subdirectorio (`tests/comun/`) a propósito --
//! es la convención de Rust para que esto NO se compile como su propio
//! binario de test (sólo los `.rs` directamente bajo `tests/` lo son), sino
//! como un módulo que cada archivo de test importa con `mod comun;`.
//!
//! A diferencia de los mocks en `src/*/tests`, que prueban DETALLES
//! internos, estos simulan un servidor Phoenix completo desde el punto de
//! vista de alguien que sólo usa la API pública del crate
//! (`ClienteRealtime`, `supervisar_*`) -- ninguna función de acá es
//! `pub(crate)` ni toca internals.
//!
//! `#![allow(dead_code)]`: cada binario de test (`cargo test` compila
//! CADA `tests/*.rs` como un binario separado) importa este módulo entero
//! pero sólo usa un subconjunto de sus funciones -- sin esto, cada binario
//! avisaría "función nunca usada" sobre las que otro sí usa.
#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

/// Arma el string de un `phx_reply` ok para la `referencia` dada.
fn reply_ok(topic: &serde_json::Value, referencia: &serde_json::Value) -> String {
    serde_json::json!({
        "topic": topic,
        "event": "phx_reply",
        "payload": { "status": "ok", "response": {} },
        "ref": referencia,
    })
    .to_string()
}

/// Arma el envelope real de un broadcast de Phoenix (ver el doc-comment de
/// `ClienteRealtime::esperar_evento` en `src/cliente.rs` para por qué tiene
/// esta forma anidada).
fn envelope_broadcast(
    topic: &serde_json::Value,
    evento: &str,
    payload: serde_json::Value,
) -> String {
    serde_json::json!({
        "topic": topic,
        "event": "broadcast",
        "payload": { "event": evento, "payload": payload },
        "ref": Option::<String>::None,
    })
    .to_string()
}

/// Patrón A -- el que usa HOY `private.emitir_cambio_nube_sitio` en
/// producción: el broadcast sólo trae metadata vacía (`operation`/`table`,
/// sin los datos reales). Quien escucha necesita un segundo viaje
/// (`resync_fetch` acá, un `sincronizarConNube()` completo en la app real)
/// para averiguar qué cambió de verdad.
///
/// `contador_mensajes_post_join` cuenta cuántos mensajes cliente→servidor
/// llegaron DESPUÉS del `phx_join` -- es la métrica dura de "cuántos
/// round-trips hicieron falta para tener el dato", sin depender de
/// cronometrar nada (un timing sobre loopback es ruido; contar mensajes
/// no lo es).
pub async fn servidor_aviso_vacio_con_resync(
    contador_mensajes_post_join: Arc<AtomicUsize>,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let direccion = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (flujo, _remoto) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();

        // 1) phx_join
        let Some(Ok(Message::Text(texto))) = socket.next().await else {
            return;
        };
        let join: serde_json::Value = serde_json::from_str(&texto).unwrap();
        socket
            .send(Message::Text(reply_ok(&join["topic"], &join["ref"])))
            .await
            .unwrap();

        // 2) el aviso vacío -- sólo metadata, nada de datos reales
        socket
            .send(Message::Text(envelope_broadcast(
                &join["topic"],
                "cambio_nube",
                serde_json::json!({ "operation": "INSERT", "table": "_lab" }),
            )))
            .await
            .unwrap();

        // 3) espera el "resync" (segundo round-trip) y ENTONCES manda el dato real
        while let Some(Ok(Message::Text(texto))) = socket.next().await {
            contador_mensajes_post_join.fetch_add(1, Ordering::SeqCst);
            let entrante: serde_json::Value = serde_json::from_str(&texto).unwrap();
            if entrante["event"] == "resync_fetch" {
                let reply = serde_json::json!({
                    "topic": join["topic"],
                    "event": "phx_reply",
                    "payload": {
                        "status": "ok",
                        "response": { "id": 1, "nombre": "dato via resync", "cedula": "1-0000-0000" },
                    },
                    "ref": entrante["ref"],
                });
                socket.send(Message::Text(reply.to_string())).await.unwrap();
                return;
            }
        }
    });

    format!("ws://{direccion}")
}

/// Patrón B -- `realtime.broadcast_changes()`: el dato real viaja DENTRO
/// del broadcast (`record`), cero round-trips extra. Mismo contador que el
/// patrón A, para comparar manzanas con manzanas.
pub async fn servidor_broadcast_changes(contador_mensajes_post_join: Arc<AtomicUsize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let direccion = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (flujo, _remoto) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();

        let Some(Ok(Message::Text(texto))) = socket.next().await else {
            return;
        };
        let join: serde_json::Value = serde_json::from_str(&texto).unwrap();
        socket
            .send(Message::Text(reply_ok(&join["topic"], &join["ref"])))
            .await
            .unwrap();

        // La fila completa, de una -- nada más que enviar después de esto.
        socket
            .send(Message::Text(envelope_broadcast(
                &join["topic"],
                "fila_completa",
                serde_json::json!({
                    "operation": "INSERT",
                    "table": "_lab",
                    "record": { "id": 1, "nombre": "dato directo", "cedula": "1-1111-1111" },
                    "old_record": null,
                }),
            )))
            .await
            .unwrap();

        // Sigue escuchando por si el test manda algo más (no debería) --
        // cualquier mensaje que llegue acá cuenta en contra del patrón B,
        // así que si el cliente hiciera un resync innecesario, el test lo
        // detectaría.
        while let Some(Ok(Message::Text(_texto))) = socket.next().await {
            contador_mensajes_post_join.fetch_add(1, Ordering::SeqCst);
        }
    });

    format!("ws://{direccion}")
}

/// Servidor mínimo que sólo entiende heartbeat -- para las pruebas de
/// integración que ejercitan `ClienteRealtime`/`supervisar_heartbeat` por
/// su API pública, sin necesitar ningún canal.
pub async fn servidor_heartbeat_simple() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let direccion = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (flujo, _remoto) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();
        while let Some(Ok(Message::Text(texto))) = socket.next().await {
            let entrante: serde_json::Value = serde_json::from_str(&texto).unwrap();
            let reply = serde_json::json!({
                "topic": "phoenix",
                "event": "phx_reply",
                "payload": { "status": "ok", "response": {} },
                "ref": entrante["ref"],
            });
            socket.send(Message::Text(reply.to_string())).await.unwrap();
        }
    });
    format!("ws://{direccion}")
}

/// Servidor mínimo para las pruebas de integración de autorización: exige
/// (o no) un `access_token` exacto en el `phx_join`.
pub async fn servidor_con_autorizacion(token_requerido: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let direccion = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let Ok((flujo, _remoto)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();
                while let Some(Ok(Message::Text(texto))) = socket.next().await {
                    let entrante: serde_json::Value = serde_json::from_str(&texto).unwrap();
                    if entrante["event"] != "phx_join" {
                        continue;
                    }
                    let token = entrante["payload"]["access_token"].as_str().unwrap_or("");
                    let ok = token == token_requerido;
                    let reply = serde_json::json!({
                        "topic": entrante["topic"],
                        "event": "phx_reply",
                        "payload": if ok {
                            serde_json::json!({ "status": "ok", "response": {} })
                        } else {
                            serde_json::json!({ "status": "error", "response": { "reason": "unauthorized" } })
                        },
                        "ref": entrante["ref"],
                    });
                    socket.send(Message::Text(reply.to_string())).await.unwrap();
                }
            });
        }
    });

    format!("ws://{direccion}")
}
