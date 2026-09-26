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
    ///
    /// `"presence": {"key": ""}` en el `config`: sin esto, el servidor NO
    /// manda `presence_state` al unirse ni `presence_diff` después --
    /// Presence, igual que Broadcast, hay que declararlo explícito en el
    /// join, no es automático por el sólo hecho de unirse al canal. Bug
    /// real encontrado al implementar Presence: el primer intento sin este
    /// campo se quedó esperando `presence_state` para siempre (timeout).
    /// `realtime-js` lo manda siempre, se use o no Presence -- se replica
    /// ese mismo comportamiento acá.
    pub fn unirse(topic: String, referencia: String, access_token: Option<&str>) -> Self {
        let payload = match access_token {
            Some(token) => serde_json::json!({
                "config": {
                    "broadcast": { "self": false },
                    "presence": { "key": "" },
                    "private": true,
                },
                "access_token": token,
            }),
            None => serde_json::json!({
                "config": {
                    "broadcast": { "self": false },
                    "presence": { "key": "" },
                    "private": true,
                },
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

    /// Un mensaje genérico dentro de un canal ya unido -- ej. el "resync"
    /// que hoy dispara `sincronizarConNube()` tras un aviso vacío
    /// (`EventoSupervisor::Conectado`/un broadcast de metadata). No es
    /// parte del protocolo Phoenix en sí (Phoenix no tiene un evento
    /// "resync"), es un evento de aplicación como cualquier otro que uno
    /// define -- `ClienteRealtime::solicitar` lo usa para simular ese
    /// segundo viaje en las pruebas de punta a punta (ver
    /// `tests/e2e_aviso_vacio_y_resync.rs`).
    pub fn generico(topic: String, event: String, payload: Value, referencia: String) -> Self {
        Self {
            topic,
            event,
            payload,
            referencia,
            referencia_join: None,
        }
    }

    /// El "track" de Presence -- publica tu propio estado en el canal
    /// (ej. `{cedula, nombre}`, lo mismo que ya hace `nubeRealtime.ts` con
    /// `track()` para el panel de "quién está conectado"). Formato
    /// verificado contra la documentación oficial del protocolo (no
    /// adivinado -- ya nos mordió una vez asumir el formato del broadcast):
    /// `event` es literalmente `"presence"`, y el `type`/`event` REALES
    /// ("track") van anidados adentro del `payload`, con la metadata del
    /// usuario un nivel más adentro todavía.
    pub fn presencia_track(topic: String, referencia: String, metadata: Value) -> Self {
        Self {
            topic,
            event: "presence".to_string(),
            payload: serde_json::json!({ "type": "presence", "event": "track", "payload": metadata }),
            referencia,
            referencia_join: None,
        }
    }

    /// `phx_join` a un canal PÚBLICO (`realtime.send(..., private => false)`
    /// del lado de Postgres) -- sin `access_token` ni `private: true` en el
    /// `config`, porque un canal público no pasa por ninguna política de
    /// `realtime.messages`: cualquiera con la `apikey` puede escucharlo.
    /// Sólo para el laboratorio de prueba (`_lab_lattis_avisos`, ver
    /// README.md) -- un canal real de la app siempre es privado.
    pub fn unirse_publico(topic: String, referencia: String) -> Self {
        Self {
            topic,
            event: "phx_join".to_string(),
            payload: serde_json::json!({ "config": { "broadcast": { "self": false } } }),
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
        let mensaje =
            MensajeSaliente::unirse("realtime:sitio:abc".to_string(), "7".to_string(), None);
        assert_eq!(mensaje.referencia, "7");
        assert_eq!(mensaje.referencia_join.as_deref(), Some("7"));
    }

    #[test]
    fn unirse_publico_no_lleva_private_ni_access_token() {
        let mensaje = MensajeSaliente::unirse_publico("lab:lattis".to_string(), "3".to_string());
        let json = serde_json::to_value(&mensaje).unwrap();
        assert!(json["payload"].get("access_token").is_none());
        assert!(json["payload"]["config"].get("private").is_none());
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
        assert!(
            !entrante.es_reply_ok_de("2"),
            "no debe matchear otra referencia"
        );
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

/// Pruebas basadas en propiedades (`proptest`): en vez de elegir a mano
/// cada string rara (vacía, con comillas, con emoji, kilométrica...),
/// `proptest` genera cientos de entradas al azar por corrida y verifica que
/// la propiedad se sostiene siempre -- si alguna falla, la reduce
/// automáticamente al caso mínimo que la rompe ("shrinking"), en vez de
/// dejarte un string de 3000 caracteres para depurar a mano.
#[cfg(test)]
mod propiedades {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Cualquier `topic`/`referencia` -- incluidos unicode, comillas,
        /// backslashes, saltos de línea -- tiene que sobrevivir intactos un
        /// viaje de ida y vuelta por JSON. Si esto fallara, algún caracter
        /// estaría rompiendo el escapado de `serde_json` en `enviar()`.
        #[test]
        fn latido_sobrevive_serializar_y_deserializar(referencia in ".*") {
            let mensaje = MensajeSaliente::latido(referencia.clone());
            let texto = serde_json::to_string(&mensaje).unwrap();
            let releido: MensajeEntrante = serde_json::from_str(&texto).unwrap();
            prop_assert_eq!(releido.topic, "phoenix");
            prop_assert_eq!(releido.event, "heartbeat");
            prop_assert_eq!(releido.referencia, Some(referencia));
        }

        /// Mismo criterio para `unirse_publico` -- el topic es justamente
        /// donde más plata de negocio circula (`sitio:<uuid>`), así que acá
        /// es donde más duele un bug de escapado.
        #[test]
        fn unirse_publico_preserva_el_topic_exacto(
            topic in ".*",
            referencia in ".*",
        ) {
            let mensaje = MensajeSaliente::unirse_publico(topic.clone(), referencia.clone());
            let texto = serde_json::to_string(&mensaje).unwrap();
            let releido: MensajeEntrante = serde_json::from_str(&texto).unwrap();
            prop_assert_eq!(releido.topic, topic);
            prop_assert_eq!(releido.event, "phx_join");
        }

        /// `presencia_track` arma exactamente el sobre que confirma la
        /// documentación oficial (`event: "presence"`, con `type`/`event`
        /// reales anidados) -- una metadata arbitraria (cédula con
        /// caracteres raros, nombre con unicode) llega intacta.
        #[test]
        fn presencia_track_arma_el_sobre_correcto(cedula in ".*", nombre in ".*") {
            let metadata = serde_json::json!({ "cedula": cedula, "nombre": nombre });
            let mensaje = MensajeSaliente::presencia_track(
                "realtime:sitio:abc".to_string(),
                "1".to_string(),
                metadata,
            );
            let json = serde_json::to_value(&mensaje).unwrap();
            prop_assert_eq!(json["event"].as_str(), Some("presence"));
            prop_assert_eq!(json["payload"]["type"].as_str(), Some("presence"));
            prop_assert_eq!(json["payload"]["event"].as_str(), Some("track"));
            prop_assert_eq!(json["payload"]["payload"]["cedula"].as_str(), Some(cedula.as_str()));
            prop_assert_eq!(json["payload"]["payload"]["nombre"].as_str(), Some(nombre.as_str()));
        }

        /// `unirse` con un `access_token` arbitrario (un JWT real tiene
        /// puntos, símbolos base64url y es largo) -- el token tiene que
        /// llegar exacto adentro de `payload.access_token`, ni truncado ni
        /// escapado de más.
        #[test]
        fn unirse_privado_preserva_el_access_token_exacto(
            topic in ".*",
            referencia in ".*",
            token in ".*",
        ) {
            let mensaje = MensajeSaliente::unirse(topic, referencia, Some(&token));
            let json = serde_json::to_value(&mensaje).unwrap();
            prop_assert_eq!(
                json["payload"]["access_token"].as_str(),
                Some(token.as_str())
            );
        }

        /// `es_reply_ok_de` nunca debe entrar en pánico sin importar qué
        /// forma tenga `payload` -- ni un `status` que no sea string, ni un
        /// payload que directamente no sea un objeto (`null`, un array, un
        /// número). Un servidor comprometido o un bug del lado de Supabase
        /// no debería poder tumbar este cliente con una respuesta rara.
        #[test]
        fn es_reply_ok_de_nunca_entra_en_panico(
            evento in ".*",
            referencia_recibida in proptest::option::of(".*"),
            payload_es_objeto in any::<bool>(),
            status in proptest::option::of(".*"),
            referencia_esperada in ".*",
        ) {
            let payload = if payload_es_objeto {
                match &status {
                    Some(s) => serde_json::json!({ "status": s }),
                    None => serde_json::json!({}),
                }
            } else {
                serde_json::json!(null)
            };
            let entrante = MensajeEntrante {
                topic: "t".to_string(),
                event: evento,
                payload,
                referencia: referencia_recibida,
            };
            // No importa el resultado -- lo único que se verifica es que
            // llegar hasta acá no entró en pánico.
            let _ = entrante.es_reply_ok_de(&referencia_esperada);
        }
    }
}
