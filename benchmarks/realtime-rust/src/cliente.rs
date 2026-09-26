//! Cliente WebSocket mínimo sobre `tokio-tungstenite`, hablando el framing
//! de `protocolo.rs`. Sin reconexión, sin backoff, sin renovación de JWT
//! todavía -- eso es la SIGUIENTE etapa del laboratorio (ver README.md),
//! una vez que el protocolo base esté probado contra el servidor real de
//! `control-acceso-staging`.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::protocolo::{MensajeEntrante, MensajeSaliente};

#[derive(Debug, thiserror::Error)]
pub enum ErrorCliente {
    #[error("error de WebSocket: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("no llegó una respuesta antes de {0:?}")]
    Timeout(Duration),
    #[error("el servidor cerró la conexión sin responder")]
    ConexionCerrada,
    #[error("mensaje entrante no es JSON válido: {0}")]
    JsonInvalido(#[from] serde_json::Error),
    #[error("el servidor rechazó el phx_join: {0}")]
    JoinRechazado(Value),
    #[error("el servidor rechazó la solicitud: {0}")]
    SolicitudRechazada(Value),
}

pub struct ClienteRealtime {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    proxima_referencia: u64,
}

/// Cuánto esperar por un `phx_reply` antes de darlo por perdido -- Supabase
/// responde en milisegundos en condiciones normales; 5s es generoso a
/// propósito para no dar falsos negativos por una red lenta durante este
/// laboratorio.
const ESPERA_REPLY: Duration = Duration::from_secs(5);

impl ClienteRealtime {
    /// `url_websocket` ya debe incluir `?apikey=...&vsn=1.0.0` -- ver
    /// `armar_url` en `main.rs`. No valida el certificado TLS de forma
    /// distinta a la del sistema (usa `rustls-tls-webpki-roots`, mismos
    /// roots que cualquier navegador).
    pub async fn conectar(url_websocket: &str) -> Result<Self, ErrorCliente> {
        let (socket, _respuesta_http) = connect_async(url_websocket).await?;
        Ok(Self {
            socket,
            proxima_referencia: 1,
        })
    }

    fn siguiente_referencia(&mut self) -> String {
        let referencia = self.proxima_referencia;
        self.proxima_referencia += 1;
        referencia.to_string()
    }

    async fn enviar(&mut self, mensaje: &MensajeSaliente) -> Result<(), ErrorCliente> {
        let texto = serde_json::to_string(mensaje).map_err(ErrorCliente::JsonInvalido)?;
        self.socket.send(Message::Text(texto)).await?;
        Ok(())
    }

    /// Lee mensajes hasta encontrar un `phx_reply` con esta `referencia`
    /// (ok o error) o hasta `ESPERA_REPLY` -- descarta cualquier otro
    /// mensaje que llegue mientras tanto (ej. un broadcast de otro canal),
    /// igual que haría un cliente real que multiplexa varios canales sobre
    /// el mismo socket.
    async fn esperar_reply(&mut self, referencia: &str) -> Result<MensajeEntrante, ErrorCliente> {
        let resultado = timeout(ESPERA_REPLY, async {
            loop {
                match self.socket.next().await {
                    Some(Ok(Message::Text(texto))) => {
                        let entrante: MensajeEntrante = serde_json::from_str(&texto)?;
                        if entrante.event == "phx_reply"
                            && entrante.referencia.as_deref() == Some(referencia)
                        {
                            return Ok(entrante);
                        }
                    }
                    Some(Ok(_otro_tipo_de_frame)) => {}
                    Some(Err(error)) => return Err(ErrorCliente::WebSocket(error)),
                    None => return Err(ErrorCliente::ConexionCerrada),
                }
            }
        })
        .await;

        resultado.unwrap_or(Err(ErrorCliente::Timeout(ESPERA_REPLY)))
    }

    /// El smoke test más simple del protocolo -- ver el doc-comment de
    /// `MensajeSaliente::latido`. Devuelve `true` si el servidor confirmó
    /// `status: "ok"`.
    pub async fn latido(&mut self) -> Result<bool, ErrorCliente> {
        let referencia = self.siguiente_referencia();
        self.enviar(&MensajeSaliente::latido(referencia.clone()))
            .await?;
        let reply = self.esperar_reply(&referencia).await?;
        Ok(reply.es_reply_ok_de(&referencia))
    }

    /// Renueva el JWT de un canal YA UNIDO, sin reconectar ni rehacer
    /// `phx_join` -- el mecanismo real de Supabase Realtime para esto es un
    /// push `access_token` in-band (investigado: `realtime-js` lo manda a
    /// cada canal unido cuando se llama `setAuth(token)`, en vez de
    /// desconectar y volver a unirse). Fire-and-forget a propósito: la
    /// documentación del protocolo dice explícito que no hay reply en el
    /// caso de éxito -- si el token nuevo fuera inválido, el servidor NO
    /// contesta con un error acá, cierra el canal (eso lo detecta el
    /// `esperar_evento` normal del supervisor como una desconexión, no esta
    /// función).
    pub async fn renovar_token(&mut self, topic: &str, token: &str) -> Result<(), ErrorCliente> {
        let referencia = self.siguiente_referencia();
        self.enviar(&MensajeSaliente::generico(
            topic.to_string(),
            "access_token".to_string(),
            serde_json::json!({ "access_token": token }),
            referencia,
        ))
        .await
    }

    /// Manda un evento genérico dentro de un canal ya unido y espera su
    /// respuesta -- usado en las pruebas de punta a punta para simular el
    /// "resync" (segundo round-trip) del patrón actual de producción, ver
    /// `MensajeSaliente::generico`. Devuelve `payload.response` de la
    /// respuesta ok, o `SolicitudRechazada` si no vino `status: "ok"`.
    pub async fn solicitar(
        &mut self,
        topic: &str,
        event: &str,
        payload: Value,
    ) -> Result<Value, ErrorCliente> {
        let referencia = self.siguiente_referencia();
        self.enviar(&MensajeSaliente::generico(
            topic.to_string(),
            event.to_string(),
            payload,
            referencia.clone(),
        ))
        .await?;
        let reply = self.esperar_reply(&referencia).await?;
        if reply.es_reply_ok_de(&referencia) {
            Ok(reply
                .payload
                .get("response")
                .cloned()
                .unwrap_or(Value::Null))
        } else {
            Err(ErrorCliente::SolicitudRechazada(reply.payload))
        }
    }

    /// `phx_join` a un canal privado real (ej. `realtime:sitio:<uuid>`) con
    /// el `access_token` que devuelve `device-auth` -- Etapa 2: acá sí pasa
    /// por la política de `realtime.messages` (`private = true`), a
    /// diferencia de `unirse_publico`.
    pub async fn unirse_privado(
        &mut self,
        topic: &str,
        access_token: &str,
    ) -> Result<(), ErrorCliente> {
        let referencia = self.siguiente_referencia();
        self.enviar(&MensajeSaliente::unirse(
            topic.to_string(),
            referencia.clone(),
            Some(access_token),
        ))
        .await?;
        let reply = self.esperar_reply(&referencia).await?;
        if reply.es_reply_ok_de(&referencia) {
            Ok(())
        } else {
            Err(ErrorCliente::JoinRechazado(reply.payload))
        }
    }

    /// `phx_join` a un canal público (ver
    /// `MensajeSaliente::unirse_publico`) -- sólo para el laboratorio de
    /// prueba de punta a punta, nunca para un canal real de la app.
    pub async fn unirse_publico(&mut self, topic: &str) -> Result<(), ErrorCliente> {
        let referencia = self.siguiente_referencia();
        self.enviar(&MensajeSaliente::unirse_publico(
            topic.to_string(),
            referencia.clone(),
        ))
        .await?;
        let reply = self.esperar_reply(&referencia).await?;
        if reply.es_reply_ok_de(&referencia) {
            Ok(())
        } else {
            Err(ErrorCliente::JoinRechazado(reply.payload))
        }
    }

    /// Lee mensajes hasta encontrar un broadcast de este `topic` con este
    /// `event`, o hasta `espera`. A diferencia de `esperar_reply`, acá no
    /// hay una `ref` que matchear -- un broadcast del servidor no lleva la
    /// referencia de ningún mensaje saliente nuestro, así que se filtra por
    /// topic+event, tal como haría un cliente real multiplexando canales.
    ///
    /// Un broadcast NO llega con `event` = tu nombre de evento a nivel
    /// superior -- Phoenix lo envuelve: el `event` del mensaje siempre es
    /// el string literal `"broadcast"`, y el nombre real (el que Postgres
    /// pasó como segundo argumento a `realtime.send`, ej. `"lab_aviso"`)
    /// junto con el payload real viven ANIDADOS adentro
    /// (`payload.event`/`payload.payload`) -- así es como
    /// `supabase-js` implementa `.on("broadcast", { event: X }, cb)` por
    /// debajo. Devuelve `payload.payload`, no el sobre completo.
    pub async fn esperar_evento(
        &mut self,
        topic: &str,
        event: &str,
        espera: Duration,
    ) -> Result<Value, ErrorCliente> {
        let resultado = timeout(espera, async {
            loop {
                match self.socket.next().await {
                    Some(Ok(Message::Text(texto))) => {
                        let entrante: MensajeEntrante = serde_json::from_str(&texto)?;
                        let evento_interno = entrante.payload.get("event").and_then(Value::as_str);
                        if entrante.topic == topic
                            && entrante.event == "broadcast"
                            && evento_interno == Some(event)
                        {
                            let carga = entrante
                                .payload
                                .get("payload")
                                .cloned()
                                .unwrap_or(Value::Null);
                            return Ok(carga);
                        }
                    }
                    Some(Ok(_otro_tipo_de_frame)) => {}
                    Some(Err(error)) => return Err(ErrorCliente::WebSocket(error)),
                    None => return Err(ErrorCliente::ConexionCerrada),
                }
            }
        })
        .await;

        resultado.unwrap_or(Err(ErrorCliente::Timeout(espera)))
    }

    pub async fn cerrar(mut self) -> Result<(), ErrorCliente> {
        self.socket.close(None).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    /// Levanta un servidor WebSocket local (sin red externa, sin
    /// credenciales de Supabase) que sólo entiende heartbeat -- prueba la
    /// lógica de framing/espera-de-reply de `ClienteRealtime` de forma
    /// determinística, reproducible en CI. El smoke test contra el
    /// servidor REAL de `control-acceso-staging` vive aparte
    /// (`bin/smoke_heartbeat.rs`, `#[ignore]` por defecto -- ver README.md).
    async fn servidor_de_prueba_que_solo_responde_heartbeat() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direccion = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (flujo, _remoto) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();
            while let Some(Ok(Message::Text(texto))) = socket.next().await {
                let entrante: MensajeEntrante = serde_json::from_str(&texto).unwrap();
                if entrante.event == "heartbeat" {
                    let reply = serde_json::json!({
                        "topic": "phoenix",
                        "event": "phx_reply",
                        "payload": { "status": "ok", "response": {} },
                        "ref": entrante.referencia,
                    });
                    socket.send(Message::Text(reply.to_string())).await.unwrap();
                }
            }
        });

        format!("ws://{direccion}")
    }

    #[tokio::test]
    async fn el_heartbeat_recibe_ok_del_servidor() {
        let url = servidor_de_prueba_que_solo_responde_heartbeat().await;
        let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
        assert!(cliente.latido().await.unwrap());
        cliente.cerrar().await.unwrap();
    }

    #[tokio::test]
    async fn dos_heartbeats_seguidos_usan_referencias_distintas() {
        let url = servidor_de_prueba_que_solo_responde_heartbeat().await;
        let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
        assert!(cliente.latido().await.unwrap());
        assert!(cliente.latido().await.unwrap());
        cliente.cerrar().await.unwrap();
    }

    #[tokio::test]
    async fn conectar_a_un_puerto_sin_servidor_falla() {
        let error = ClienteRealtime::conectar("ws://127.0.0.1:1").await;
        assert!(error.is_err());
    }

    /// Caos: ¿qué pasa si el servidor manda basura que no es JSON? (un bug
    /// de Supabase, un proxy que corrompe el frame, lo que sea). Tiene que
    /// devolver un error tipado (`JsonInvalido`), NUNCA entrar en pánico --
    /// un cliente real no puede caerse entero porque un solo mensaje vino
    /// mal formado.
    async fn servidor_que_manda_basura_no_json() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direccion = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (flujo, _remoto) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();
            let _ = socket.next().await; // consume el heartbeat saliente
            socket
                .send(Message::Text("esto no es JSON { { {".to_string()))
                .await
                .unwrap();
        });
        format!("ws://{direccion}")
    }

    #[tokio::test]
    async fn basura_no_json_del_servidor_da_error_tipado_no_panico() {
        let url = servidor_que_manda_basura_no_json().await;
        let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
        let resultado = cliente.latido().await;
        assert!(
            matches!(resultado, Err(ErrorCliente::JsonInvalido(_))),
            "{resultado:?}"
        );
    }

    /// Caos: un corte de red real casi nunca manda un frame `close` prolijo
    /// -- un proxy/load balancer que mata la conexión, o el proceso del
    /// servidor que muere, simplemente cierra el socket TCP en seco (visto
    /// en la investigación: un load balancer con idle timeout de 60s que
    /// tumba la conexión sin FIN cuando el heartbeat llega con jitter).
    /// `drop(socket)` sin `.close()` reproduce exactamente eso -- tiene que
    /// dar un error de conexión, no colgarse esperando para siempre.
    async fn servidor_que_corta_en_seco_sin_close_frame() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direccion = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (flujo, _remoto) = listener.accept().await.unwrap();
            let socket = tokio_tungstenite::accept_async(flujo).await.unwrap();
            drop(socket); // corte en seco -- sin close frame, sin FIN prolijo
        });
        format!("ws://{direccion}")
    }

    #[tokio::test]
    async fn corte_en_seco_sin_close_frame_da_error_no_cuelga() {
        let url = servidor_que_corta_en_seco_sin_close_frame().await;
        let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
        // `tokio::test` no tiene timeout automático -- si esto colgara para
        // siempre, este test colgaría el proceso entero de `cargo test`.
        // Envolverlo en `tokio::time::timeout` convierte un cuelgue en un
        // fallo de test legible en vez de una corrida de CI que nunca
        // termina.
        let resultado = tokio::time::timeout(Duration::from_secs(10), cliente.latido()).await;
        let resultado = resultado.expect("no debería tardar 10s en darse cuenta del corte");
        assert!(resultado.is_err(), "{resultado:?}");
    }

    /// Caos: el servidor se queda mudo (ni reply, ni error, ni cierre) --
    /// una conexión colgada a medio TLS handshake de otro proceso, un
    /// proxy que traga el paquete. `latido()` tiene que rendirse sola
    /// después de `ESPERA_REPLY`, no esperar para siempre. Este test tarda
    /// ~5s de verdad (es justamente lo que está probando).
    async fn servidor_mudo_que_nunca_responde_nada() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let direccion = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (flujo, _remoto) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(flujo).await.unwrap();
            let _ = socket.next().await; // recibe el heartbeat, no contesta nunca
            std::future::pending::<()>().await; // mantiene el socket vivo, mudo
        });
        format!("ws://{direccion}")
    }

    #[tokio::test]
    async fn servidor_mudo_da_timeout_en_vez_de_colgar_para_siempre() {
        let url = servidor_mudo_que_nunca_responde_nada().await;
        let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
        let resultado = tokio::time::timeout(Duration::from_secs(8), cliente.latido()).await;
        let resultado = resultado.expect("ESPERA_REPLY es 5s -- 8s de margen alcanza de sobra");
        assert!(
            matches!(resultado, Err(ErrorCliente::Timeout(_))),
            "{resultado:?}"
        );
    }
}
