//! Framing de mensajes Phoenix Channels (el protocolo que habla Supabase
//! Realtime por debajo del WebSocket) -- versión "v1.0.0" (JSON, no el
//! binario "v2.0.0"). Referencia: `docs/arquitectura/arquitectura-supabase.md`
//! sección 4 del crate raíz, y el propio código fuente de Phoenix
//! (`lib/phoenix/socket/serializer.ex`, no vendorizado acá).
//!
//! Forma de cada mensaje (array posicional en la v2, objeto con las mismas
//! claves en la v1 que usamos acá): `topic`, `event`, `payload`, `ref` y,
//! sólo en `phx_join`, `join_ref` -- Supabase exige `join_ref` igual a
//! `ref` en el join inicial, y lo repite en cada mensaje subsiguiente del
//! mismo canal para que el servidor pueda distinguir un `phx_join` viejo
//! (de una reconexión anterior) de uno nuevo.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct MensajeSaliente {
    pub topic: String,
    pub event: String,
    pub payload: Value,
    #[serde(rename = "ref")]
    pub referencia: String,
    #[serde(rename = "join_ref", skip_serializing_if = "Option::is_none")]
    pub referencia_join: Option<String>,
}

impl MensajeSaliente {
    /// El heartbeat no pertenece a ningún canal -- Phoenix lo especial-casea
    /// en el topic fijo `"phoenix"` y lo responde SIEMPRE, incluso sin haber
    /// hecho `phx_join` a nada todavía. Es el mensaje más simple posible del
    /// protocolo -- por eso es el primer smoke test (`bin/smoke_heartbeat.rs`
    /// vía `Cliente::latido`): si esto no responde `status: "ok"`, no tiene
    /// sentido intentar nada más complejo (join a un canal privado, etc.).
    pub fn latido(referencia: String) -> Self {
        Self {
            topic: "phoenix".to_string(),
            event: "heartbeat".to_string(),
            payload: serde_json::json!({}),
            referencia,
            referencia_join: None,
        }
    }

    /// `phx_join` a un canal cualquiera (ej. `realtime:sitio:<uuid>`).
    /// `access_token` es el JWT del dispositivo -- las políticas de
    /// `realtime.messages` (ver arquitectura-supabase.md, 4.2) lo exigen
    /// para canales privados; queda `None` acá porque este laboratorio
    /// todavía no integra el flujo de `device-auth` real.
    pub fn unirse(topic: String, referencia: String, access_token: Option<&str>) -> Self {
        let payload = match access_token {
            Some(token) => serde_json::json!({
                "config": { "broadcast": { "self": false }, "private": true },
                "access_token": token,
            }),
            None => serde_json::json!({
                "config": { "broadcast": { "self": false }, "private": true },
            }),
        };
        Self {
            topic,
            event: "phx_join".to_string(),
            payload,
            referencia: referencia.clone(),
            referencia_join: Some(referencia),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MensajeEntrante {
    pub topic: String,
    pub event: String,
    pub payload: Value,
    #[serde(rename = "ref")]
    pub referencia: Option<String>,
}

impl MensajeEntrante {
    /// `true` si esta es una respuesta de éxito (`phx_reply` con
    /// `payload.status == "ok"`) a un mensaje con esta misma `referencia`.
    /// Phoenix también manda `phx_reply` con `status: "error"` -- eso NO
    /// cuenta como éxito, aunque el `event` coincida.
    pub fn es_reply_ok_de(&self, referencia_esperada: &str) -> bool {
        self.event == "phx_reply"
            && self.referencia.as_deref() == Some(referencia_esperada)
            && self.payload.get("status").and_then(Value::as_str) == Some("ok")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_heartbeat_no_lleva_join_ref() {
        let mensaje = MensajeSaliente::latido("1".to_string());
        let json = serde_json::to_value(&mensaje).unwrap();
        assert_eq!(json["topic"], "phoenix");
        assert_eq!(json["event"], "heartbeat");
        assert!(json.get("join_ref").is_none(), "{json}");
    }

    #[test]
    fn unirse_repite_la_referencia_como_join_ref() {
        let mensaje = MensajeSaliente::unirse("realtime:sitio:abc".to_string(), "7".to_string(), None);
        assert_eq!(mensaje.referencia, "7");
        assert_eq!(mensaje.referencia_join.as_deref(), Some("7"));
    }

    #[test]
    fn reconoce_un_reply_ok_de_la_referencia_correcta() {
        let entrante: MensajeEntrante = serde_json::from_value(serde_json::json!({
            "topic": "phoenix",
            "event": "phx_reply",
            "payload": { "status": "ok", "response": {} },
            "ref": "1",
        }))
        .unwrap();
        assert!(entrante.es_reply_ok_de("1"));
        assert!(!entrante.es_reply_ok_de("2"), "no debe matchear otra referencia");
    }

    #[test]
    fn un_reply_de_error_no_cuenta_como_ok() {
        let entrante: MensajeEntrante = serde_json::from_value(serde_json::json!({
            "topic": "realtime:sitio:abc",
            "event": "phx_reply",
            "payload": { "status": "error", "response": { "reason": "unauthorized" } },
            "ref": "1",
        }))
        .unwrap();
        assert!(!entrante.es_reply_ok_de("1"));
    }
}
