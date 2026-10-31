//! Manda la cola de telemetría a la nube: lote columnar comprimido con brotli,
//! por HTTP/2 y con el token del equipo, a la Edge Function receptora
//! (`/functions/v1/observabilidad/telemetria`), que lo descomprime y llama a
//! `observabilidad.registrar_telemetria` con ese mismo token
//! (`docs/features-futuras/plan-observabilidad-en-produccion.md`).
//!
//! Bloqueante: sólo desde un hilo propio, nunca desde un runtime de tokio.
//! Quien llama pasa el cliente HTTP (el del núcleo, con la conexión ya
//! abierta) y el token vigente.

use std::sync::atomic::{AtomicI64, Ordering};

use reqwest::StatusCode;
use serde_json::Value;

use super::cola::ColaTelemetria;
use super::disparador::{Perfil, Situacion, debe_enviar};
use super::lote;

/// Ruta de la receptora, relativa a la URL base del proyecto.
pub const RUTA_RECEPTORA: &str = "/functions/v1/observabilidad/telemetria";

/// Qué pasó con un lote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Respuesta {
    /// Llegó (o ya había llegado: `duplicado`). `filas` las guardadas y
    /// `descartadas` las que la nube no guardó (tipo desconocido, fuera de
    /// ventana, demasiado grande).
    Recibido { filas: u64, descartadas: u64 },
    /// La nube lo rechazó por mal formado (400, 413, 422): reintentarlo daría
    /// lo mismo. Se descarta.
    Rechazado(String),
    /// El token venció o el equipo no está vigente (401, 403): renovar y
    /// probar en la próxima vuelta.
    SinAutorizacion,
    /// Red, ritmo (429) o la nube caída (5xx): se reintenta el mismo lote más
    /// tarde.
    Reintentar(String),
}

/// Manda un lote ya comprimido.
pub fn enviar_lote(
    cliente: &reqwest::blocking::Client,
    base_url: &str,
    apikey: &str,
    token: &str,
    comprimido: Vec<u8>,
) -> Respuesta {
    let resultado = cliente
        .post(format!("{base_url}{RUTA_RECEPTORA}"))
        .header("apikey", apikey)
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::CONTENT_ENCODING, "br")
        .body(comprimido)
        .send();
    let respuesta = match resultado {
        Ok(respuesta) => respuesta,
        Err(error) => return Respuesta::Reintentar(error.to_string()),
    };
    let estado = respuesta.status();
    match estado {
        _ if estado.is_success() => {
            let cuerpo: Value = respuesta.json().unwrap_or(Value::Null);
            let filas = cuerpo["filas"].as_u64().unwrap_or(0);
            let descartadas = cuerpo["descartadas_tipo"].as_u64().unwrap_or(0)
                + cuerpo["descartadas_otras"].as_u64().unwrap_or(0);
            Respuesta::Recibido { filas, descartadas }
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Respuesta::SinAutorizacion,
        StatusCode::BAD_REQUEST
        | StatusCode::PAYLOAD_TOO_LARGE
        | StatusCode::UNPROCESSABLE_ENTITY => {
            Respuesta::Rechazado(format!("la receptora respondió {estado}"))
        }
        _ => Respuesta::Reintentar(format!("la receptora respondió {estado}")),
    }
}

/// Resultado de una vuelta de envío.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResumenEnvio {
    pub lotes: u64,
    pub filas: u64,
    /// Filas que la nube no guardó, más las de lotes rechazados.
    pub descartadas: u64,
    /// Por qué se cortó antes de vaciar la cola, si se cortó.
    pub pendiente: Option<String>,
}

/// Manda lotes hasta vaciar la cola o hasta que la nube no pueda recibir
/// más (red, ritmo, token). Cada lote confirmado sale de la cola; uno que
/// falla queda en vuelo y se reintenta igual, con el mismo `lote_id`.
pub fn vaciar_cola(
    cola: &ColaTelemetria,
    sesion: &Value,
    cliente: &reqwest::blocking::Client,
    base_url: &str,
    apikey: &str,
    token: &str,
    nuevo_id: impl Fn() -> String,
) -> ResumenEnvio {
    let mut resumen = ResumenEnvio::default();
    loop {
        let lote = match cola.lote_a_enviar(sesion, &nuevo_id) {
            Ok(Some(lote)) => lote,
            Ok(None) => return resumen,
            Err(error) => {
                resumen.pendiente = Some(format!("cola de telemetría: {error}"));
                return resumen;
            }
        };
        let filas_del_lote = lote["bloques"].as_array().map_or(0, |bloques| {
            bloques
                .iter()
                .map(|b| b["filas"].as_array().map_or(0, Vec::len) as u64)
                .sum()
        });
        match enviar_lote(cliente, base_url, apikey, token, lote::comprimir(&lote)) {
            Respuesta::Recibido { filas, descartadas } => {
                resumen.lotes += 1;
                resumen.filas += filas;
                resumen.descartadas += descartadas;
            }
            Respuesta::Rechazado(motivo) => {
                log::warn!("telemetría: lote rechazado ({motivo}); se descarta");
                resumen.descartadas += filas_del_lote;
            }
            Respuesta::SinAutorizacion => {
                resumen.pendiente = Some("sin autorización".to_owned());
                return resumen;
            }
            Respuesta::Reintentar(motivo) => {
                resumen.pendiente = Some(motivo);
                return resumen;
            }
        }
        if let Err(error) = cola.confirmar(sesion, &nuevo_id) {
            resumen.pendiente = Some(format!("cola de telemetría: {error}"));
            return resumen;
        }
    }
}

/// Cuándo fue el último envío (o el arranque, si todavía no hubo). Lo
/// comparten las vueltas del hilo de telemetría.
pub struct RitmoEnvio {
    ultimo_envio_ms: AtomicI64,
}

impl RitmoEnvio {
    pub fn desde(inicio_ms: i64) -> Self {
        Self {
            ultimo_envio_ms: AtomicI64::new(inicio_ms),
        }
    }

    pub fn ultimo_envio_ms(&self) -> i64 {
        self.ultimo_envio_ms.load(Ordering::Relaxed)
    }
}

/// A dónde y con qué cliente se manda.
pub struct Destino<'a> {
    pub cliente: &'a reqwest::blocking::Client,
    pub base_url: &'a str,
    pub apikey: &'a str,
}

/// Una vuelta del hilo de telemetría: decide con [`disparador::debe_enviar`]
/// si conviene mandar ahora y, si conviene, vacía la cola. `None` si no tocó
/// la red (no convenía, o todavía no hay token: se vuelve a probar en la
/// próxima vuelta). El token se pide sólo si va a hacer falta.
///
/// `ultima_actividad_red_ms` es [`disparador::ultima_actividad_red`] (un
/// parámetro para poder probarlo sin estado global).
#[allow(clippy::too_many_arguments)]
pub fn enviar_si_conviene(
    perfil: Perfil,
    ritmo: &RitmoEnvio,
    ahora_ms: i64,
    ultima_actividad_red_ms: Option<i64>,
    cola: &ColaTelemetria,
    sesion: &Value,
    token: impl FnOnce() -> Option<String>,
    destino: &Destino<'_>,
    nuevo_id: impl Fn() -> String,
) -> Option<ResumenEnvio> {
    if !cola.hay_pendientes() {
        return None;
    }
    let situacion = Situacion {
        ahora_ms,
        pendientes: cola.pendientes_aprox().max(1),
        ultimo_envio_ms: ritmo.ultimo_envio_ms(),
        ultima_actividad_red_ms,
    };
    if !debe_enviar(perfil, situacion) {
        return None;
    }
    let token = token()?;
    // Aunque falle, no se reintenta en la vuelta siguiente: se espera a la
    // próxima ventana (radio despierta, o el máximo sin enviar).
    ritmo.ultimo_envio_ms.store(ahora_ms, Ordering::Relaxed);
    Some(vaciar_cola(
        cola,
        sesion,
        destino.cliente,
        destino.base_url,
        destino.apikey,
        &token,
        nuevo_id,
    ))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    use serde_json::json;

    use super::*;
    use crate::telemetria::lote::EventoTelemetria;

    /// Lo que la receptora de prueba vio en cada petición.
    struct Peticion {
        ruta: String,
        cabeceras: Vec<(String, String)>,
        cuerpo: Vec<u8>,
    }

    impl Peticion {
        fn cabecera(&self, nombre: &str) -> Option<&str> {
            self.cabeceras
                .iter()
                .find(|(n, _)| n == nombre)
                .map(|(_, v)| v.as_str())
        }
    }

    /// Responde `respuestas` (estado, cuerpo) en orden, una por petición.
    fn receptora(respuestas: Vec<(u16, &'static str)>) -> (String, mpsc::Receiver<Peticion>) {
        let escucha = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", escucha.local_addr().unwrap());
        let (enviar, recibir) = mpsc::channel();
        std::thread::spawn(move || {
            for (estado, cuerpo_respuesta) in respuestas {
                let Ok((conexion, _)) = escucha.accept() else {
                    return;
                };
                let mut lector = BufReader::new(conexion);
                let mut primera = String::new();
                lector.read_line(&mut primera).unwrap();
                let ruta = primera
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_owned();
                let mut cabeceras = Vec::new();
                let mut largo = 0;
                loop {
                    let mut linea = String::new();
                    lector.read_line(&mut linea).unwrap();
                    let linea = linea.trim_end().to_owned();
                    if linea.is_empty() {
                        break;
                    }
                    let (nombre, valor) = linea.split_once(": ").unwrap_or_default();
                    let nombre = nombre.to_ascii_lowercase();
                    if nombre == "content-length" {
                        largo = valor.parse().unwrap();
                    }
                    cabeceras.push((nombre, valor.to_owned()));
                }
                let mut cuerpo = vec![0; largo];
                lector.read_exact(&mut cuerpo).unwrap();
                enviar
                    .send(Peticion {
                        ruta,
                        cabeceras,
                        cuerpo,
                    })
                    .unwrap();
                write!(
                    lector.get_mut(),
                    "HTTP/1.1 {estado} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{cuerpo_respuesta}",
                    cuerpo_respuesta.len()
                )
                .unwrap();
            }
        });
        (url, recibir)
    }

    fn evento(i: i64) -> EventoTelemetria {
        EventoTelemetria {
            ocurrido_ms: 1_793_470_000_000 + i,
            tipo: "reloj".to_owned(),
            datos: json!({ "i": i }).as_object().cloned().unwrap(),
        }
    }

    fn sesion() -> Value {
        json!({ "id": "dddddddd-0000-0000-0000-000000000001" })
    }

    fn ids() -> impl Fn() -> String {
        let siguiente = Cell::new(0);
        move || {
            siguiente.set(siguiente.get() + 1);
            format!("lote-{}", siguiente.get())
        }
    }

    const RECIBIDO_3: &str =
        r#"{"estado":"recibido","filas":3,"descartadas_tipo":0,"descartadas_otras":0}"#;

    #[test]
    fn manda_brotli_con_token_a_la_receptora_y_vacia_la_cola() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        cola.agregar(&[evento(1), evento(2), evento(3)]).unwrap();
        let (url, peticiones) = receptora(vec![(200, RECIBIDO_3)]);

        let resumen = vaciar_cola(
            &cola,
            &sesion(),
            &reqwest::blocking::Client::new(),
            &url,
            "clave-publica",
            "jwt-del-equipo",
            ids(),
        );

        assert_eq!(
            resumen,
            ResumenEnvio {
                lotes: 1,
                filas: 3,
                descartadas: 0,
                pendiente: None
            }
        );
        assert!(!cola.hay_pendientes());
        let peticion = peticiones.recv().unwrap();
        assert_eq!(peticion.ruta, RUTA_RECEPTORA);
        assert_eq!(
            peticion.cabecera("authorization"),
            Some("Bearer jwt-del-equipo")
        );
        assert_eq!(peticion.cabecera("apikey"), Some("clave-publica"));
        assert_eq!(peticion.cabecera("content-encoding"), Some("br"));
        let lote: Value =
            serde_json::from_slice(&lote::descomprimir(&peticion.cuerpo, 1 << 20).unwrap())
                .unwrap();
        assert_eq!(lote["lote_id"], "lote-1");
        assert_eq!(lote::reconstruir(&lote).unwrap().len(), 3);
    }

    #[test]
    fn si_la_nube_no_responde_el_mismo_lote_se_reintenta_despues() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        cola.agregar(&[evento(1), evento(2), evento(3)]).unwrap();
        let (url, peticiones) = receptora(vec![(503, "{}"), (200, RECIBIDO_3)]);
        let cliente = reqwest::blocking::Client::new();
        let nuevo_id = ids();

        let primero = vaciar_cola(&cola, &sesion(), &cliente, &url, "k", "t", &nuevo_id);
        assert_eq!(primero.lotes, 0);
        assert!(primero.pendiente.is_some());
        assert!(cola.hay_pendientes(), "el lote sigue en vuelo");

        let segundo = vaciar_cola(&cola, &sesion(), &cliente, &url, "k", "t", &nuevo_id);
        assert_eq!(segundo.lotes, 1);
        assert!(!cola.hay_pendientes());

        let id_de = |p: Peticion| -> Value {
            serde_json::from_slice::<Value>(&lote::descomprimir(&p.cuerpo, 1 << 20).unwrap())
                .unwrap()["lote_id"]
                .clone()
        };
        assert_eq!(
            id_de(peticiones.recv().unwrap()),
            id_de(peticiones.recv().unwrap()),
            "mismo lote_id: la nube no duplica"
        );
    }

    #[test]
    fn un_lote_rechazado_se_descarta_y_se_sigue_con_el_resto() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        let total = i64::try_from(crate::telemetria::cola::FILAS_POR_LOTE).unwrap() + 3;
        cola.agregar(&(0..total).map(evento).collect::<Vec<_>>())
            .unwrap();
        let (url, _peticiones) =
            receptora(vec![(400, r#"{"error":"mal_formado"}"#), (200, RECIBIDO_3)]);

        let resumen = vaciar_cola(
            &cola,
            &sesion(),
            &reqwest::blocking::Client::new(),
            &url,
            "k",
            "t",
            ids(),
        );

        assert_eq!(resumen.lotes, 1);
        assert_eq!(
            resumen.descartadas,
            u64::try_from(crate::telemetria::cola::FILAS_POR_LOTE).unwrap()
        );
        assert_eq!(resumen.pendiente, None);
        assert!(!cola.hay_pendientes());
    }

    #[test]
    fn sin_autorizacion_corta_y_deja_todo() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        cola.agregar(&[evento(1)]).unwrap();
        let (url, _peticiones) = receptora(vec![(401, "{}")]);

        let resumen = vaciar_cola(
            &cola,
            &sesion(),
            &reqwest::blocking::Client::new(),
            &url,
            "k",
            "t",
            ids(),
        );

        assert_eq!(resumen.pendiente.as_deref(), Some("sin autorización"));
        assert!(cola.hay_pendientes());
    }

    #[test]
    fn el_movil_espera_la_radio_despierta_y_no_pide_token_si_no_va_a_mandar() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        cola.agregar(&[evento(1), evento(2), evento(3)]).unwrap();
        let (url, peticiones) = receptora(vec![(200, RECIBIDO_3)]);
        let cliente = reqwest::blocking::Client::new();
        let destino = Destino {
            cliente: &cliente,
            base_url: &url,
            apikey: "k",
        };
        let ahora = 1_793_470_000_000;
        let ritmo = RitmoEnvio::desde(ahora - 60_000);
        let pidio_token = Cell::new(false);
        let token = || {
            pidio_token.set(true);
            Some("t".to_owned())
        };

        // Sin actividad de red reciente: no manda ni pide el token.
        let quieto = enviar_si_conviene(
            Perfil::Movil,
            &ritmo,
            ahora,
            None,
            &cola,
            &sesion(),
            token,
            &destino,
            ids(),
        );
        assert_eq!(quieto, None);
        assert!(!pidio_token.get());

        // Una verificación despertó la radio hace 1 s: aprovecha.
        let a_caballo = enviar_si_conviene(
            Perfil::Movil,
            &ritmo,
            ahora,
            Some(ahora - 1_000),
            &cola,
            &sesion(),
            || Some("t".to_owned()),
            &destino,
            ids(),
        );
        assert_eq!(a_caballo.map(|r| r.lotes), Some(1));
        assert_eq!(ritmo.ultimo_envio_ms(), ahora);
        assert!(peticiones.recv().is_ok());
        assert!(!cola.hay_pendientes());
    }

    #[test]
    fn sin_token_no_toca_la_red_y_vuelve_a_probar() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        cola.agregar(&[evento(1)]).unwrap();
        let cliente = reqwest::blocking::Client::new();
        let destino = Destino {
            cliente: &cliente,
            base_url: "http://127.0.0.1:9",
            apikey: "k",
        };
        let ahora = 1_793_470_000_000;
        let ritmo = RitmoEnvio::desde(ahora - 120_000);

        let resultado = enviar_si_conviene(
            Perfil::Escritorio,
            &ritmo,
            ahora,
            None,
            &cola,
            &sesion(),
            || None,
            &destino,
            ids(),
        );

        assert_eq!(resultado, None);
        assert_eq!(
            ritmo.ultimo_envio_ms(),
            ahora - 120_000,
            "no cuenta como envío"
        );
        assert!(cola.hay_pendientes());
    }

    #[test]
    fn sin_pendientes_no_hace_ninguna_peticion() {
        let dir = tempfile::tempdir().unwrap();
        let cola = ColaTelemetria::en(dir.path());
        // Una receptora que no responde nada: si se le pidiera algo, la
        // prueba quedaría esperando.
        let resumen = vaciar_cola(
            &cola,
            &sesion(),
            &reqwest::blocking::Client::new(),
            "http://127.0.0.1:9",
            "k",
            "t",
            ids(),
        );
        assert_eq!(resumen, ResumenEnvio::default());
    }
}
