//! Cliente HTTP del receptor en la nube (ver `docs/planes-implementados/plan-persistencia-nube.md`).
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

/// Único cliente HTTP bloqueante del crate -- construido una sola vez (ver
/// `TIMEOUT_HTTP`) y reutilizado desde entonces. Antes cada llamada a
/// `cliente_http()` levantaba un `Client` nuevo, tirando el pool de
/// conexiones/TLS de la llamada anterior; con `LazyLock` todas comparten el
/// mismo pool. `unwrap_or_else` en vez de `expect`: si el backend TLS del
/// sistema fallara al construirlo con el timeout (no debería, es la misma
/// config con la que ya se usaba `Client::new()` en todo el crate), cae al
/// cliente sin timeout en vez de entrar en pánico mitad de una
/// sincronización.
static CLIENTE_HTTP: std::sync::LazyLock<reqwest::blocking::Client> =
    std::sync::LazyLock::new(|| {
        reqwest::blocking::Client::builder()
            .timeout(TIMEOUT_HTTP)
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    });

/// Clon barato del cliente compartido -- `reqwest::blocking::Client` envuelve
/// su estado (pool de conexiones, config TLS) en un `Arc` por dentro, así que
/// clonarlo no repite ninguno de los dos trabajos.
pub(crate) fn cliente_http() -> reqwest::blocking::Client {
    CLIENTE_HTTP.clone()
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

/// Datos del dispositivo físico capturados en la activación inicial (ver
/// `Nucleo::configurar_dispositivo_inicial_con_secreto` en móvil,
/// `comandos::nube::configurar_dispositivo_inicial` en escritorio) -- sólo
/// viajan una vez, no en cada renovación de token. Sirven para que el panel
/// de administración distinga "el mismo dispositivo de siempre" de uno
/// distinto usando el mismo secreto, y como evidencia si hace falta
/// denunciar un intento de fraude (ver `docs/features-futuras/plan-sesion-unica-dispositivos.md`).
/// Nombres de campo neutrales a propósito -- esto lo usan tanto móvil como
/// escritorio, cada uno con su propio significado (ver los doc-comments de
/// cada campo). Ninguno es secreto en sí mismo -- todos observables por
/// cualquier app en el propio dispositivo -- así que viajan en texto plano
/// en el body, igual que el secreto.
#[derive(Default, serde::Serialize)]
pub struct MetadatosDispositivo {
    /// Identificador estable de hardware: `Settings.Secure.ANDROID_ID` en
    /// móvil, Machine GUID de Windows en escritorio.
    pub identificador_hardware: Option<String>,
    /// Nombre por el que el dispositivo se identifica a sí mismo:
    /// `Build.MODEL` en móvil, nombre de red (`COMPUTERNAME`) en escritorio.
    pub nombre_dispositivo: Option<String>,
    /// Quién/qué hizo el dispositivo o su sistema: `Build.MANUFACTURER` en
    /// móvil, sistema operativo ("Windows") en escritorio.
    pub plataforma: Option<String>,
    /// Huella más específica de la build exacta: `Build.FINGERPRINT` en
    /// móvil, sistema operativo + arquitectura en escritorio.
    pub version_build: Option<String>,
    pub app_version: Option<String>,
}

/// Intercambia el secreto de este dispositivo (ver `super::credenciales`)
/// por un `TokenDispositivo` firmado por el receptor. `metadata`, si viene,
/// se adjunta al mismo request -- ver `MetadatosDispositivo`.
pub fn autenticar_dispositivo(
    base_url: &str,
    secreto: &str,
    metadata: Option<&MetadatosDispositivo>,
) -> Result<TokenDispositivo, NubeError> {
    let url = format!("{base_url}/functions/v1/device-auth");
    let mut cuerpo = serde_json::json!({ "secret": secreto });
    if let Some(metadata) = metadata
        && let serde_json::Value::Object(mapa) = &mut cuerpo
    {
        mapa.insert(
            "metadata".to_string(),
            serde_json::to_value(metadata).unwrap_or_default(),
        );
    }
    let respuesta = cliente_http().post(url).json(&cuerpo).send()?;

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

        let token = autenticar_dispositivo(&base_url, "cualquier-secreto", None).expect("token ok");

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

        let token = autenticar_dispositivo(&base_url, "cualquier-secreto", None).expect("token ok");

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

        let resultado = autenticar_dispositivo(&base_url, "secreto-invalido", None);

        assert!(matches!(resultado, Err(NubeError::CredencialesInvalidas)));
    }

    #[test]
    fn error_de_servidor_se_reporta_como_error_de_red() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"boom\"}",
        );

        let resultado = autenticar_dispositivo(&base_url, "secreto", None);

        assert!(matches!(resultado, Err(NubeError::Red(_))));
    }

    #[test]
    fn cuerpo_invalido_se_reporta_como_error_de_red() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\nesto no es json",
        );

        let resultado = autenticar_dispositivo(&base_url, "secreto", None);

        assert!(matches!(resultado, Err(NubeError::Red(_))));
    }
}
