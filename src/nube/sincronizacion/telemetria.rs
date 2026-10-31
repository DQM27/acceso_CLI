//! La orden de telemetría de este equipo (`telemetria_ordenes` en la nube).
//!
//! Llega por dos vías, las dos aplicadas con la misma regla (sólo una versión
//! mayor a la vigente):
//!
//! - El aviso en vivo `telemetria_config` ([`aplicar_aviso_telemetria`]), al
//!   instante.
//! - La sincronización ([`recibir_orden_telemetria_propia`]), que cubre al
//!   equipo que estaba apagado o sin red cuando se dio la orden, o que perdió
//!   el aviso.
//!
//! La orden se guarda en la base local (`database::config_telemetria`) y se
//! aplica a la copia en memoria (`telemetria::config`), que es la que consulta
//! cada medición. Ver `docs/features-futuras/plan-telemetria-y-registro-de-errores.md`.

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use super::{ContextoSincronizacion, SincronizacionError, obtener_json};
use crate::database::config_telemetria;
use crate::nube::cliente::cliente_http;
use crate::telemetria::{ConfigTelemetria, Grupos, Orden, config};

/// La orden tal como la manda la nube, ya sea la fila de `PostgREST` o el aviso
/// en vivo (mismas claves).
fn leer_orden(valor: &serde_json::Value) -> Option<(String, Orden)> {
    let dispositivo_id = valor.get("dispositivo_id")?.as_str()?.to_owned();
    let version = valor.get("version")?.as_i64().filter(|v| *v >= 1)?;
    let mut nombres = Vec::new();
    for grupo in valor.get("grupos")?.as_array()? {
        nombres.push(grupo.as_str()?);
    }
    // Un vencimiento ilegible no se adivina: se rechaza toda la orden.
    let vence_en = match valor.get("vence_en") {
        None | Some(serde_json::Value::Null) => None,
        Some(texto) => Some(crate::tiempo::parsear_utc(texto.as_str()?).ok()?),
    };
    Some((
        dispositivo_id,
        Orden {
            version,
            grupos: Grupos::desde_nombres(nombres),
            vence_en,
        },
    ))
}

/// Guarda la orden y la aplica a `configuracion`, si es para este equipo.
/// `true` si cambió lo que se mide. Una falla al guardar en la base no impide
/// aplicarla: la orden rige igual y la próxima sincronización la vuelve a
/// intentar.
fn aplicar_orden(
    conexion: &Connection,
    configuracion: &ConfigTelemetria,
    dispositivo_propio: &str,
    valor: &serde_json::Value,
    ahora: DateTime<Utc>,
) -> bool {
    let Some((dispositivo_id, orden)) = leer_orden(valor) else {
        return false;
    };
    if dispositivo_id != dispositivo_propio {
        return false;
    }
    // Al abrir se cargó la orden guardada sin saber de qué equipo era; si era de
    // otra identidad, su versión no debe bloquear a la de este equipo.
    if config_telemetria::leer(conexion)
        .ok()
        .flatten()
        .is_some_and(|guardada| guardada.dispositivo_id != dispositivo_id)
    {
        configuracion.reiniciar();
    }
    if let Err(error) = config_telemetria::guardar(conexion, &dispositivo_id, &orden, ahora) {
        log::warn!("no se pudo guardar la orden de telemetría: {error}");
    }
    configuracion.aplicar(&orden)
}

/// Aviso en vivo `telemetria_config`, tal cual llega por el canal. `false` si
/// no es una orden legible, si es de otro equipo o si no es más nueva que la
/// vigente. Nunca falla: un aviso perdido lo cubre la sincronización.
pub fn aplicar_aviso_telemetria(
    conexion: &Connection,
    dispositivo_propio: &str,
    aviso: &serde_json::Value,
) -> bool {
    aplicar_orden(conexion, config(), dispositivo_propio, aviso, Utc::now())
}

/// Pide a la nube la orden de este equipo y la aplica. Sin fila (el panel nunca
/// le dio una orden) no hace nada: el equipo mide lo de siempre.
pub fn recibir_orden_telemetria_propia(
    conexion: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<bool, SincronizacionError> {
    recibir_orden_en(conexion, contexto, config())
}

fn recibir_orden_en(
    conexion: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    configuracion: &ConfigTelemetria,
) -> Result<bool, SincronizacionError> {
    let url = format!(
        "{}/rest/v1/telemetria_ordenes?dispositivo_id=eq.{}\
         &select=dispositivo_id,version,grupos,vence_en&limit=1",
        contexto.base_url, contexto.dispositivo_id,
    );
    let filas: Vec<serde_json::Value> = obtener_json(&cliente_http(), contexto, &url)?;
    Ok(filas.first().is_some_and(|fila| {
        aplicar_orden(
            conexion,
            configuracion,
            contexto.dispositivo_id,
            fila,
            Utc::now(),
        )
    }))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    use serde_json::json;

    use super::*;
    use crate::database::schema::initialize_database;
    use crate::telemetria::Grupo;

    const PROPIO: &str = "dispositivo-propio";
    const AHORA: i64 = 1_800_000_000;

    fn base() -> Connection {
        let conexion = Connection::open_in_memory().unwrap();
        initialize_database(&conexion).unwrap();
        conexion
    }

    fn aplicar(
        conexion: &Connection,
        configuracion: &ConfigTelemetria,
        valor: &serde_json::Value,
    ) -> bool {
        aplicar_orden(conexion, configuracion, PROPIO, valor, Utc::now())
    }

    #[test]
    fn el_aviso_de_este_equipo_se_aplica_y_se_guarda() {
        let c = base();
        let config = ConfigTelemetria::nueva();
        let aviso = json!({
            "dispositivo_id": PROPIO, "version": 4,
            "grupos": ["rendimiento", "fluidez"], "vence_en": null,
        });
        assert!(aplicar(&c, &config, &aviso));
        assert!(config.permite(Grupo::Fluidez, AHORA));
        assert_eq!(config.version(), 4);
        let guardada = config_telemetria::leer(&c).unwrap().unwrap();
        assert_eq!(guardada.dispositivo_id, PROPIO);
        assert_eq!(guardada.orden.version, 4);
    }

    #[test]
    fn el_aviso_de_otro_equipo_no_hace_nada() {
        let c = base();
        let config = ConfigTelemetria::nueva();
        let aviso = json!({
            "dispositivo_id": "otro", "version": 4, "grupos": ["fluidez"], "vence_en": null,
        });
        assert!(!aplicar(&c, &config, &aviso));
        assert!(!config.permite(Grupo::Fluidez, AHORA));
        assert_eq!(config.version(), 0);
        assert_eq!(config_telemetria::leer(&c).unwrap(), None);
    }

    #[test]
    fn la_orden_cargada_de_otra_identidad_no_bloquea_a_la_de_este_equipo() {
        let c = base();
        let anterior = Orden {
            version: 900,
            grupos: Grupos::desde_nombres(["fluidez"]),
            vence_en: None,
        };
        config_telemetria::guardar(&c, "identidad-anterior", &anterior, Utc::now()).unwrap();
        let config = ConfigTelemetria::nueva();
        config_telemetria::cargar_guardada(&c, &config).unwrap();
        assert!(config.permite(Grupo::Fluidez, AHORA));

        let aviso = json!({
            "dispositivo_id": PROPIO, "version": 3, "grupos": ["escaneo"], "vence_en": null,
        });
        assert!(aplicar(&c, &config, &aviso));
        assert!(!config.permite(Grupo::Fluidez, AHORA));
        assert!(config.permite(Grupo::Escaneo, AHORA));
        assert_eq!(config.version(), 3);
    }

    #[test]
    fn un_aviso_repetido_o_viejo_no_cambia_nada() {
        let c = base();
        let config = ConfigTelemetria::nueva();
        let nuevo = json!({
            "dispositivo_id": PROPIO, "version": 9, "grupos": ["fluidez"], "vence_en": null,
        });
        let viejo = json!({
            "dispositivo_id": PROPIO, "version": 8, "grupos": ["recursos"], "vence_en": null,
        });
        assert!(aplicar(&c, &config, &nuevo));
        assert!(!aplicar(&c, &config, &nuevo));
        assert!(!aplicar(&c, &config, &viejo));
        assert!(config.permite(Grupo::Fluidez, AHORA));
        assert!(!config.permite(Grupo::Recursos, AHORA));
        assert_eq!(
            config_telemetria::leer(&c).unwrap().unwrap().orden.version,
            9
        );
    }

    #[test]
    fn un_aviso_ilegible_se_descarta_sin_tumbar_nada() {
        let c = base();
        let config = ConfigTelemetria::nueva();
        for aviso in [
            json!(null),
            json!("texto"),
            json!({}),
            json!({"dispositivo_id": PROPIO, "grupos": [], "vence_en": null}),
            json!({"dispositivo_id": PROPIO, "version": 0, "grupos": [], "vence_en": null}),
            json!({"dispositivo_id": PROPIO, "version": 1.5, "grupos": [], "vence_en": null}),
            json!({"dispositivo_id": PROPIO, "version": 1, "grupos": "fluidez", "vence_en": null}),
            json!({"dispositivo_id": PROPIO, "version": 1, "grupos": [1], "vence_en": null}),
            json!({"dispositivo_id": PROPIO, "version": 1, "grupos": [], "vence_en": "ayer"}),
            json!({"dispositivo_id": PROPIO, "version": 1, "grupos": [], "vence_en": 12}),
            json!({"dispositivo_id": 7, "version": 1, "grupos": [], "vence_en": null}),
        ] {
            assert!(!aplicar(&c, &config, &aviso), "{aviso}");
        }
        assert_eq!(config.version(), 0);
        assert_eq!(config_telemetria::leer(&c).unwrap(), None);
    }

    #[test]
    fn el_vencimiento_con_microsegundos_de_postgres_se_lee() {
        let c = base();
        let config = ConfigTelemetria::nueva();
        let aviso = json!({
            "dispositivo_id": PROPIO, "version": 2, "grupos": ["fluidez"],
            "vence_en": "2099-01-01T00:00:00.123456+00:00",
        });
        assert!(aplicar(&c, &config, &aviso));
        assert!(config.permite(Grupo::Fluidez, AHORA));
    }

    #[test]
    fn una_orden_ya_vencida_se_guarda_pero_no_enciende_nada() {
        let c = base();
        let config = ConfigTelemetria::nueva();
        let aviso = json!({
            "dispositivo_id": PROPIO, "version": 2, "grupos": ["fluidez"],
            "vence_en": "2020-01-01T00:00:00+00:00",
        });
        assert!(aplicar(&c, &config, &aviso));
        assert!(!config.permite(Grupo::Fluidez, AHORA));
        assert!(config.permite(Grupo::Rendimiento, AHORA));
    }

    #[test]
    fn un_grupo_que_esta_version_no_conoce_se_ignora() {
        let c = base();
        let config = ConfigTelemetria::nueva();
        let aviso = json!({
            "dispositivo_id": PROPIO, "version": 2,
            "grupos": ["fluidez", "grupo_del_futuro"], "vence_en": null,
        });
        assert!(aplicar(&c, &config, &aviso));
        assert!(config.permite(Grupo::Fluidez, AHORA));
    }

    /// Servidor HTTP de una sola respuesta que devuelve el pedido recibido.
    fn servidor(cuerpo: &'static str, estado: &'static str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
        let direccion = listener.local_addr().expect("dirección local");
        let (enviar, recibir) = mpsc::channel();
        thread::spawn(move || {
            let (mut conexion, _) = listener.accept().expect("conexión");
            conexion
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("set_read_timeout");
            let mut leido = Vec::new();
            let mut buffer = [0_u8; 2048];
            while let Ok(n) = conexion.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                leido.extend_from_slice(&buffer[..n]);
                if String::from_utf8_lossy(&leido).contains("\r\n\r\n") {
                    break;
                }
            }
            let _ = enviar.send(String::from_utf8_lossy(&leido).to_string());
            let respuesta = format!(
                "HTTP/1.1 {estado}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cuerpo}",
                cuerpo.len()
            );
            let _ = conexion.write_all(respuesta.as_bytes());
        });
        (format!("http://{direccion}"), recibir)
    }

    fn contexto(base_url: &str) -> ContextoSincronizacion<'_> {
        ContextoSincronizacion {
            base_url,
            apikey: "apikey-de-prueba",
            token: "token-de-prueba",
            dispositivo_id: PROPIO,
            sitio_id: "sitio-1",
        }
    }

    #[test]
    fn la_sincronizacion_pide_solo_la_fila_de_este_equipo_y_la_aplica() {
        let (url, pedido) = servidor(
            r#"[{"dispositivo_id":"dispositivo-propio","version":12,"grupos":["rendimiento","recursos"],"vence_en":null}]"#,
            "200 OK",
        );
        let c = base();
        let config = ConfigTelemetria::nueva();
        assert!(recibir_orden_en(&c, &contexto(&url), &config).unwrap());
        assert!(config.permite(Grupo::Recursos, AHORA));
        assert_eq!(config.version(), 12);

        let pedido = pedido.recv().unwrap();
        let primera = pedido.lines().next().unwrap();
        assert!(
            primera.starts_with(
                "GET /rest/v1/telemetria_ordenes?dispositivo_id=eq.dispositivo-propio&"
            ),
            "{primera}"
        );
        assert!(
            primera.contains("select=dispositivo_id,version,grupos,vence_en"),
            "{primera}"
        );
        let minusculas = pedido.to_lowercase();
        assert!(minusculas.contains("authorization: bearer token-de-prueba"));
        assert!(minusculas.contains("apikey: apikey-de-prueba"));
    }

    #[test]
    fn sin_fila_en_la_nube_el_equipo_sigue_como_estaba() {
        let (url, _) = servidor("[]", "200 OK");
        let c = base();
        let config = ConfigTelemetria::nueva();
        assert!(!recibir_orden_en(&c, &contexto(&url), &config).unwrap());
        assert_eq!(config.version(), 0);
        assert_eq!(config_telemetria::leer(&c).unwrap(), None);
    }

    #[test]
    fn una_respuesta_de_error_de_la_nube_es_un_error_de_sincronizacion() {
        let (url, _) = servidor(r#"{"message":"no autorizado"}"#, "401 Unauthorized");
        let c = base();
        let config = ConfigTelemetria::nueva();
        let error = recibir_orden_en(&c, &contexto(&url), &config).unwrap_err();
        assert!(error.token_dispositivo_vencido(), "{error}");
        assert_eq!(config.version(), 0);
    }
}
