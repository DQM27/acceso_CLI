//! Sesión única por unidad: un usuario puede tener sesión en varios equipos
//! de la MISMA unidad, pero no en dos unidades a la vez. Gana el último
//! ingreso y se cierra la sesión de la unidad anterior (sólo la sesión: el
//! equipo sigue registrado). Ver las migraciones
//! `20260930130000_sesion_unica_por_unidad.sql` y
//! `20261002150000_sesion_unica_hora_del_servidor.sql`.
//!
//! El equipo llama [`sesion_en_unidad`] al iniciar sesión y en cada
//! sincronización. La misma llamada registra la sesión (y la bitácora del
//! panel) y responde si sigue vigente: así un ingreso sin conexión se
//! reconcilia solo al volver la red.
//!
//! La hora del ingreso NO sale del reloj del equipo: el equipo manda cuánto
//! tiempo pasó desde el ingreso según su contador de arranque
//! ([`crate::reloj_arranque`], no se puede mover y sigue contando con el
//! equipo suspendido) y la nube la calcula con su propio reloj. Así un
//! reloj atrasado o cambiado a mano no puede dejar a nadie afuera.
//!
//! Quien llama estas funciones decide qué hacer ante un error de red: la
//! regla del sistema es fallar "abierto" (sin nube no se expulsa a nadie).

use super::cliente::{NubeError, TokenDispositivo, cliente_http};

/// Identidad y momento de un inicio de sesión en este equipo: un
/// identificador propio (la nube reconoce "la misma sesión" por él, no por
/// la hora) y la lectura del contador de arranque al entrar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InicioSesion {
    id: String,
    arranque_ms: u64,
}

impl InicioSesion {
    /// Un ingreso que ocurre ahora.
    pub fn ahora() -> Self {
        Self {
            id: crate::database::identificador::generar_uuid_v4(),
            arranque_ms: crate::reloj_arranque::ms_desde_arranque(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// Milisegundos desde el ingreso según el contador de arranque. La
    /// sesión vive en la memoria de la app, así que nunca cruza un reinicio
    /// del equipo (que pondría el contador en cero).
    pub fn transcurrido_ms(&self) -> u64 {
        crate::reloj_arranque::ms_desde_arranque().saturating_sub(self.arranque_ms)
    }
}

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

/// Registra la sesión `inicio` de `cedula` en este equipo y dice si sigue
/// vigente. Ante ingresos demasiado cercanos para saber cuál fue primero,
/// la nube no cierra a nadie (responde `Vigente`).
pub fn sesion_en_unidad(
    base_url: &str,
    apikey: &str,
    token: &TokenDispositivo,
    cedula: &str,
    inicio: &InicioSesion,
) -> Result<EstadoSesionUnidad, NubeError> {
    let cuerpo = serde_json::json!({
        "p_cedula": cedula,
        "p_sesion_id": inicio.id(),
        "p_transcurrido_ms": inicio.transcurrido_ms(),
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

    fn token() -> TokenDispositivo {
        TokenDispositivo {
            access_token: "token-del-equipo".to_string(),
            expires_in: 3600,
            sitio_id: "sitio".to_string(),
            dispositivo_id: "equipo".to_string(),
            tipo: "pc".to_string(),
            desfase_reloj_ms: None,
            sitio_nombre: None,
            etiqueta: None,
        }
    }

    #[test]
    fn manda_el_id_de_la_sesion_y_el_tiempo_desde_el_ingreso_no_una_hora() {
        let (url, pedido) = servidor("\"desplazada\"", "200 OK");
        let inicio = InicioSesion {
            id: "11111111-0000-0000-0000-000000000001".to_string(),
            arranque_ms: crate::reloj_arranque::ms_desde_arranque().saturating_sub(90_000),
        };

        let estado =
            sesion_en_unidad(&url, "clave-publica", &token(), "900000301", &inicio).unwrap();

        assert_eq!(estado, EstadoSesionUnidad::Desplazada);
        let pedido = pedido.recv().unwrap();
        assert!(pedido.starts_with("POST /rest/v1/rpc/sesion_usuario_en_unidad "));
        assert!(
            pedido
                .to_ascii_lowercase()
                .contains("authorization: bearer token-del-equipo")
        );
        assert!(pedido.contains("\"p_cedula\":\"900000301\""));
        assert!(pedido.contains("\"p_sesion_id\":\"11111111-0000-0000-0000-000000000001\""));
        let cuerpo: serde_json::Value =
            serde_json::from_str(pedido.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        let transcurrido = cuerpo["p_transcurrido_ms"].as_u64().unwrap();
        assert!(
            (90_000..95_000).contains(&transcurrido),
            "transcurrido {transcurrido}"
        );
        assert!(
            !pedido.contains("p_iniciada_en"),
            "no viaja ninguna hora del equipo"
        );
    }

    #[test]
    fn cada_inicio_tiene_su_propio_id_y_su_duracion_avanza() {
        let primero = InicioSesion::ahora();
        let segundo = InicioSesion::ahora();
        assert_ne!(primero.id(), segundo.id());
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(primero.transcurrido_ms() >= 15);
    }

    #[test]
    fn un_error_del_servidor_es_error_no_expulsion() {
        let (url, _pedido) = servidor("{\"message\":\"no\"}", "403 Forbidden");
        let resultado =
            sesion_en_unidad(&url, "clave", &token(), "900000301", &InicioSesion::ahora());
        assert!(resultado.is_err());
    }

    #[test]
    fn cerrar_llama_la_funcion_de_salida() {
        let (url, pedido) = servidor("", "204 No Content");
        cerrar_sesion_en_unidad(&url, "clave", &token(), "900000301").unwrap();
        let pedido = pedido.recv().unwrap();
        assert!(pedido.starts_with("POST /rest/v1/rpc/cerrar_sesion_usuario_en_unidad "));
        assert!(pedido.contains("\"p_cedula\":\"900000301\""));
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
