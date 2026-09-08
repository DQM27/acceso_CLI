//! Cliente HTTP del receptor en la nube (ver `docs/plan-persistencia-nube.md`).
//!
//! Bloqueante a propósito: el resto del crate es síncrono; traer un runtime
//! async (tokio) solo para esto no se justifica todavía.

use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum NubeError {
    #[error("No se pudo contactar al receptor: {0}")]
    Red(#[from] reqwest::Error),
    #[error("El secreto de este dispositivo fue rechazado o revocado")]
    CredencialesInvalidas,
}

/// Tope por operación de red contra el receptor (conexión + respuesta
/// completa). Sin esto, `reqwest::blocking::Client::new()` no tiene ningún
/// límite -- una conexión que se queda colgada a mitad de camino (no
/// rechazada, no un error, simplemente muda) bloquearía a quien llama para
/// siempre. Importa especialmente en el login del celular
/// (`Nucleo::autenticar`, que intenta un sync corto antes de dejar entrar):
/// sin este tope, esa sincronización "best effort" podría no ser tan
/// "best effort".
const TIMEOUT_HTTP: std::time::Duration = std::time::Duration::from_secs(10);

/// Único punto de construcción del cliente HTTP bloqueante -- ver
/// `TIMEOUT_HTTP`. `unwrap_or_else` en vez de `expect`: si el backend TLS
/// del sistema fallara al construirlo con el timeout (no debería, es la
/// misma config con la que ya se usaba `Client::new()` en todo el crate),
/// cae al cliente sin timeout en vez de entrar en pánico mitad de una
/// sincronización.
pub(crate) fn cliente_http() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(TIMEOUT_HTTP)
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

/// Token que autoriza a este dispositivo a leer/escribir únicamente los
/// datos de su propio sitio. Vence a los `expires_in` segundos — hay que
/// volver a llamar a `autenticar_dispositivo` para renovarlo, no se refresca
/// solo.
#[derive(Clone, Deserialize)]
pub struct TokenDispositivo {
    pub access_token: String,
    pub expires_in: u64,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
    /// Reloj de este dispositivo MENOS el del receptor, en milisegundos, en
    /// el instante en que llegó esta respuesta -- positivo si el reloj
    /// local está adelantado. Nunca viaja en el cuerpo JSON (`#[serde(skip,
    /// default)]`): se calcula acá mismo a partir del header HTTP `Date` de
    /// la respuesta, no de nada que mande el receptor. `None` si el header
    /// vino ausente o no se pudo parsear -- quien reciba esto no debe
    /// tocar el reloj corregido en ese caso, no asumir desfase cero.
    #[serde(skip, default)]
    pub desfase_reloj_ms: Option<i64>,
}

impl std::fmt::Debug for TokenDispositivo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenDispositivo")
            .field("access_token", &"<redactado>")
            .field("expires_in", &self.expires_in)
            .field("sitio_id", &self.sitio_id)
            .field("dispositivo_id", &self.dispositivo_id)
            .field("tipo", &self.tipo)
            .field("desfase_reloj_ms", &self.desfase_reloj_ms)
            .finish()
    }
}

/// Intercambia el secreto de este dispositivo (ver `super::credenciales`)
/// por un `TokenDispositivo` firmado por el receptor.
pub fn autenticar_dispositivo(
    base_url: &str,
    secreto: &str,
) -> Result<TokenDispositivo, NubeError> {
    let url = format!("{base_url}/functions/v1/device-auth");
    let respuesta = cliente_http()
        .post(url)
        .json(&serde_json::json!({ "secret": secreto }))
        .send()?;

    if respuesta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(NubeError::CredencialesInvalidas);
    }
    let respuesta = respuesta.error_for_status()?;
    let desfase_reloj_ms = desfase_reloj_ms_desde_header(&respuesta);
    let mut token = respuesta.json::<TokenDispositivo>()?;
    token.desfase_reloj_ms = desfase_reloj_ms;
    Ok(token)
}

/// Lee el header `Date` de la respuesta (hora del receptor al responder) y
/// lo compara contra el reloj de este dispositivo EN ESE MISMO INSTANTE --
/// antes de gastar tiempo parseando el cuerpo, que ya movería el reloj
/// local hacia adelante y ensuciaría la medición. `Date` es un header HTTP
/// estándar (RFC 7231), formato IMF-fixdate -- el mismo que acepta
/// `parse_from_rfc2822` (incluye "GMT" como huso con nombre).
fn desfase_reloj_ms_desde_header(respuesta: &reqwest::blocking::Response) -> Option<i64> {
    let ahora_local = chrono::Utc::now();
    let hora_receptor = respuesta
        .headers()
        .get(reqwest::header::DATE)?
        .to_str()
        .ok()?;
    let hora_receptor = chrono::DateTime::parse_from_rfc2822(hora_receptor)
        .ok()?
        .with_timezone(&chrono::Utc);
    Some((ahora_local - hora_receptor).num_milliseconds())
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };

    use super::*;

    /// Levanta un servidor HTTP mínimo de un solo uso en `localhost`: acepta
    /// una conexión, drena el pedido (sin parsearlo — no hace falta para
    /// estas pruebas) y responde exactamente `respuesta` (línea de estado +
    /// headers + cuerpo, ya armados por quien llama). Devuelve la URL base
    /// para pasarle a `autenticar_dispositivo`.
    fn servidor_de_una_respuesta(respuesta: impl Into<String>) -> String {
        let respuesta = respuesta.into();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
        let direccion = listener.local_addr().expect("dirección local");
        thread::spawn(move || {
            let Ok((mut conexion, _)) = listener.accept() else {
                return;
            };
            conexion
                .set_read_timeout(Some(Duration::from_millis(200)))
                .expect("set_read_timeout");
            let mut buffer = [0_u8; 4096];
            loop {
                match conexion.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(_leidos) => {}
                }
            }
            let _ = conexion.write_all(respuesta.as_bytes());
            let _ = conexion.flush();
        });
        format!("http://{direccion}")
    }

    #[test]
    fn autentica_exitosamente_y_devuelve_el_token() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             {\"access_token\":\"abc\",\"expires_in\":3600,\"sitio_id\":\"s1\",\
             \"dispositivo_id\":\"d1\",\"tipo\":\"pc\"}",
        );

        let token = autenticar_dispositivo(&base_url, "cualquier-secreto").expect("token ok");

        assert_eq!(token.access_token, "abc");
        assert_eq!(token.expires_in, 3600);
        assert_eq!(token.sitio_id, "s1");
        assert_eq!(token.dispositivo_id, "d1");
        assert_eq!(token.tipo, "pc");
        assert_eq!(
            token.desfase_reloj_ms, None,
            "sin header Date no hay nada que medir"
        );
    }

    #[test]
    fn debug_de_token_no_expone_access_token() {
        let token = TokenDispositivo {
            access_token: "token-super-secreto".to_string(),
            expires_in: 3600,
            sitio_id: "s1".to_string(),
            dispositivo_id: "d1".to_string(),
            tipo: "pc".to_string(),
            desfase_reloj_ms: Some(25),
        };

        let debug = format!("{token:?}");

        assert!(!debug.contains("token-super-secreto"));
        assert!(debug.contains("<redactado>"));
        assert!(debug.contains("s1"));
        assert!(debug.contains("d1"));
    }

    #[test]
    fn mide_el_desfase_de_reloj_contra_el_header_date_de_la_respuesta() {
        // "hace 10 minutos" en formato HTTP-date (RFC 7231) -- el mismo que
        // manda cualquier servidor real en el header `Date`.
        let hace_10_min = (chrono::Utc::now() - chrono::Duration::minutes(10))
            .format("%a, %d %b %Y %H:%M:%S GMT")
            .to_string();
        let base_url = servidor_de_una_respuesta(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nDate: {hace_10_min}\r\n\
             Connection: close\r\n\r\n\
             {{\"access_token\":\"abc\",\"expires_in\":3600,\"sitio_id\":\"s1\",\
             \"dispositivo_id\":\"d1\",\"tipo\":\"pc\"}}"
        ));

        let token = autenticar_dispositivo(&base_url, "cualquier-secreto").expect("token ok");

        let desfase_ms = token
            .desfase_reloj_ms
            .expect("con header Date presente, se mide el desfase");
        // ~10 minutos = 600_000 ms -- tolerancia amplia por el tiempo real
        // que tarda la petición de prueba.
        assert!(
            (595_000..605_000).contains(&desfase_ms),
            "desfase medido fuera de rango: {desfase_ms} ms"
        );
    }

    #[test]
    fn secreto_rechazado_da_credenciales_invalidas() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"invalid_credentials\"}",
        );

        let resultado = autenticar_dispositivo(&base_url, "secreto-invalido");

        assert!(matches!(resultado, Err(NubeError::CredencialesInvalidas)));
    }

    #[test]
    fn error_de_servidor_se_reporta_como_error_de_red() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"boom\"}",
        );

        let resultado = autenticar_dispositivo(&base_url, "secreto");

        assert!(matches!(resultado, Err(NubeError::Red(_))));
    }

    #[test]
    fn cuerpo_invalido_se_reporta_como_error_de_red() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\nesto no es json",
        );

        let resultado = autenticar_dispositivo(&base_url, "secreto");

        assert!(matches!(resultado, Err(NubeError::Red(_))));
    }
}
