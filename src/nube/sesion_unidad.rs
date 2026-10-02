//! Sesión única por unidad: un usuario puede tener sesión en varios equipos
//! de la MISMA unidad, pero no en dos unidades a la vez. Gana el último
//! ingreso y se cierra la sesión de la unidad anterior (sólo la sesión: el
//! equipo sigue registrado). Ver la migración
//! `20260930130000_sesion_unica_por_unidad.sql`.
//!
//! El equipo llama [`sesion_en_unidad`] al iniciar sesión y en cada
//! sincronización. La misma llamada registra la sesión (y la bitácora del
//! panel) y responde si sigue vigente: así un ingreso sin conexión se
//! reconcilia solo al volver la red, con la hora real del ingreso.
//!
//! Quien llama estas funciones decide qué hacer ante un error de red: la
//! regla del sistema es fallar "abierto" (sin nube no se expulsa a nadie).

use chrono::{DateTime, Duration, Utc};

use super::cliente::{NubeError, TokenDispositivo, cliente_http};

/// Respuesta de `public.sesion_usuario_en_unidad`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoSesionUnidad {
    /// La sesión sigue; si el usuario tenía sesión en otra unidad, se cerró.
    Vigente,
    /// Hubo un ingreso más reciente en otra unidad: este equipo debe cerrar
    /// la sesión.
    Desplazada,
    /// La cédula no es un usuario activo de la nube (por ejemplo, el ROOT
    /// local del arranque): la regla no aplica.
    SinUsuario,
}

/// Registra la sesión de `cedula` en este equipo y dice si sigue vigente.
///
/// `iniciada_en` es la hora del ingreso según el reloj de ESTE equipo; se
/// lleva al reloj del servidor con `desfase_reloj_ms`, el último desfase
/// medido (`AppCore::desfase_reloj_ms`, guardado en la base). No sirve el
/// del token: un token de la caché no lo trae, y sin corrección un equipo
/// con el reloj atrasado registraría un ingreso "viejo" y la nube lo daría
/// por desplazado aunque el usuario acabara de entrar.
pub fn sesion_en_unidad(
    base_url: &str,
    apikey: &str,
    token: &TokenDispositivo,
    cedula: &str,
    iniciada_en: DateTime<Utc>,
    desfase_reloj_ms: Option<i64>,
) -> Result<EstadoSesionUnidad, NubeError> {
    let cuerpo = serde_json::json!({
        "p_cedula": cedula,
        "p_iniciada_en": hora_del_servidor(iniciada_en, desfase_reloj_ms).to_rfc3339(),
    });
    let respuesta = cliente_http()
        .post(format!("{base_url}/rest/v1/rpc/sesion_usuario_en_unidad"))
        .header("apikey", apikey)
        .bearer_auth(&token.access_token)
        .json(&cuerpo)
        .send()?
        .error_for_status()?;
    let estado: String = respuesta.json()?;
    Ok(interpretar_estado(&estado))
}

/// Salida voluntaria: quita la sesión de este equipo en la nube y la cierra
/// en la bitácora. Best-effort: si falla, un ingreso posterior en otra
/// unidad la reemplaza igual.
pub fn cerrar_sesion_en_unidad(
    base_url: &str,
    apikey: &str,
    token: &TokenDispositivo,
    cedula: &str,
) -> Result<(), NubeError> {
    cliente_http()
        .post(format!(
            "{base_url}/rest/v1/rpc/cerrar_sesion_usuario_en_unidad"
        ))
        .header("apikey", apikey)
        .bearer_auth(&token.access_token)
        .json(&serde_json::json!({ "p_cedula": cedula }))
        .send()?
        .error_for_status()?;
    Ok(())
}

/// Hora local llevada al reloj del servidor (desfase positivo = equipo
/// adelantado). Sin desfase medido, se usa tal cual.
fn hora_del_servidor(local: DateTime<Utc>, desfase_ms: Option<i64>) -> DateTime<Utc> {
    desfase_ms.map_or(local, |ms| local - Duration::milliseconds(ms))
}

/// Un valor desconocido (servidor más nuevo que la app) no expulsa a nadie:
/// se toma como vigente, igual que un error de red.
fn interpretar_estado(estado: &str) -> EstadoSesionUnidad {
    match estado {
        "desplazada" => EstadoSesionUnidad::Desplazada,
        "sin_usuario" => EstadoSesionUnidad::SinUsuario,
        _ => EstadoSesionUnidad::Vigente,
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    use super::*;

    /// Servidor HTTP de una sola respuesta que devuelve el pedido recibido.
    fn servidor(
        cuerpo_respuesta: &'static str,
        estado: &'static str,
    ) -> (String, mpsc::Receiver<String>) {
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
                let texto = String::from_utf8_lossy(&leido).to_string();
                if let Some(fin) = texto.find("\r\n\r\n") {
                    let largo = texto[..fin]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                        })
                        .unwrap_or(0);
                    if leido.len() >= fin + 4 + largo {
                        break;
                    }
                }
            }
            let _ = enviar.send(String::from_utf8_lossy(&leido).to_string());
            let respuesta = format!(
                "HTTP/1.1 {estado}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{cuerpo_respuesta}",
                cuerpo_respuesta.len()
            );
            let _ = conexion.write_all(respuesta.as_bytes());
        });
        (format!("http://{direccion}"), recibir)
    }

    fn token(desfase_reloj_ms: Option<i64>) -> TokenDispositivo {
        TokenDispositivo {
            access_token: "token-del-equipo".to_string(),
            expires_in: 3600,
            sitio_id: "sitio".to_string(),
            dispositivo_id: "equipo".to_string(),
            tipo: "pc".to_string(),
            desfase_reloj_ms,
            sitio_nombre: None,
            etiqueta: None,
        }
    }

    #[test]
    fn llama_la_funcion_con_el_token_del_equipo_y_la_hora_corregida_con_el_desfase_guardado() {
        let (url, pedido) = servidor("\"desplazada\"", "200 OK");
        let local = DateTime::parse_from_rfc3339("2026-09-30T12:00:02Z")
            .unwrap()
            .with_timezone(&Utc);

        // El token de la caché no trae desfase: se usa el guardado.
        let estado = sesion_en_unidad(
            &url,
            "clave-publica",
            &token(None),
            "900000301",
            local,
            Some(2_000),
        )
        .unwrap();

        assert_eq!(estado, EstadoSesionUnidad::Desplazada);
        let pedido = pedido.recv().unwrap();
        assert!(pedido.starts_with("POST /rest/v1/rpc/sesion_usuario_en_unidad "));
        assert!(
            pedido
                .to_ascii_lowercase()
                .contains("authorization: bearer token-del-equipo")
        );
        assert!(pedido.contains("\"p_cedula\":\"900000301\""));
        assert!(pedido.contains("\"p_iniciada_en\":\"2026-09-30T12:00:00+00:00\""));
    }

    #[test]
    fn un_error_del_servidor_es_error_no_expulsion() {
        let (url, _pedido) = servidor("{\"message\":\"no\"}", "403 Forbidden");
        let resultado =
            sesion_en_unidad(&url, "clave", &token(None), "900000301", Utc::now(), None);
        assert!(resultado.is_err());
    }

    #[test]
    fn cerrar_llama_la_funcion_de_salida() {
        let (url, pedido) = servidor("", "204 No Content");
        cerrar_sesion_en_unidad(&url, "clave", &token(None), "900000301").unwrap();
        let pedido = pedido.recv().unwrap();
        assert!(pedido.starts_with("POST /rest/v1/rpc/cerrar_sesion_usuario_en_unidad "));
        assert!(pedido.contains("\"p_cedula\":\"900000301\""));
    }

    #[test]
    fn corrige_la_hora_con_el_desfase_del_equipo() {
        let local = DateTime::parse_from_rfc3339("2026-09-30T12:00:05Z")
            .unwrap()
            .with_timezone(&Utc);
        // Equipo 5 s adelantado: en el servidor eran las 12:00:00.
        assert_eq!(
            hora_del_servidor(local, Some(5_000)).to_rfc3339(),
            "2026-09-30T12:00:00+00:00"
        );
        assert_eq!(hora_del_servidor(local, None), local);
    }

    #[test]
    fn solo_desplazada_expulsa() {
        assert_eq!(interpretar_estado("vigente"), EstadoSesionUnidad::Vigente);
        assert_eq!(
            interpretar_estado("desplazada"),
            EstadoSesionUnidad::Desplazada
        );
        assert_eq!(
            interpretar_estado("sin_usuario"),
            EstadoSesionUnidad::SinUsuario
        );
        assert_eq!(interpretar_estado("otra_cosa"), EstadoSesionUnidad::Vigente);
    }
}
