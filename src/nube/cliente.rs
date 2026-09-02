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

/// Token que autoriza a este dispositivo a leer/escribir únicamente los
/// datos de su propio sitio. Vence a los `expires_in` segundos — hay que
/// volver a llamar a `autenticar_dispositivo` para renovarlo, no se refresca
/// solo.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenDispositivo {
    pub access_token: String,
    pub expires_in: u64,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
}

/// Intercambia el secreto de este dispositivo (ver `super::credenciales`)
/// por un `TokenDispositivo` firmado por el receptor.
pub fn autenticar_dispositivo(
    base_url: &str,
    secreto: &str,
) -> Result<TokenDispositivo, NubeError> {
    let url = format!("{base_url}/functions/v1/device-auth");
    let respuesta = reqwest::blocking::Client::new()
        .post(url)
        .json(&serde_json::json!({ "secret": secreto }))
        .send()?;

    if respuesta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(NubeError::CredencialesInvalidas);
    }
    let respuesta = respuesta.error_for_status()?;
    Ok(respuesta.json::<TokenDispositivo>()?)
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
    fn servidor_de_una_respuesta(respuesta: &'static str) -> String {
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
