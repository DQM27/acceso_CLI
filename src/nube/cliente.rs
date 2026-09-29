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
    /// El dispositivo existe y su secreto es válido, pero un admin lo
    /// suspendió temporalmente (`dispositivos.suspended_at` en `device-auth`)
    /// -- distinto de `CredencialesInvalidas` (baja permanente).
    #[error("Este dispositivo fue suspendido temporalmente")]
    DispositivoSuspendido,
    /// `device-auth` rechazó esta versión por estar debajo del mínimo
    /// aceptado (`VERSION_MINIMA_ACEPTADA` en el receptor) -- ver
    /// `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`, punto 9.
    #[error("Esta versión de la app ya no es compatible -- hace falta actualizar")]
    VersionDesactualizada,
    /// `device-vincular` rechazó el código: no existe, ya se usó, venció o
    /// se anuló. El motivo exacto queda registrado en el panel, no acá.
    #[error("El código de vinculación no es válido o ya venció")]
    CodigoVinculacionInvalido,
    /// La clave pública que mandó este equipo ya está atada a otro
    /// dispositivo (índice único de `clave_huella`).
    #[error("La clave de este equipo ya está vinculada a otro dispositivo")]
    ClaveEnUso,
    /// Ni clave vinculada ni secreto legado: el equipo nunca se vinculó.
    #[error("Este dispositivo todavía no está vinculado a la nube")]
    SinCredencial,
    #[error(transparent)]
    Firmante(#[from] super::firmante::ErrorFirmante),
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

/// Datos del dispositivo físico capturados al vincularlo (ver
/// `Nucleo::vincular_dispositivo_inicial` en móvil,
/// `comandos::nube::vincular_dispositivo_inicial` en escritorio). Sirven
/// para que el panel de administración identifique el equipo físico, y el
/// servidor nunca pisa el identificador de hardware ya registrado: si llega
/// otro, lo deja como evento de seguridad (ver `device-auth`).
/// Nombres de campo neutrales a propósito -- esto lo usan tanto móvil como
/// escritorio, cada uno con su propio significado (ver los doc-comments de
/// cada campo). Ninguno es secreto en sí mismo -- todos observables por
/// cualquier app en el propio dispositivo -- así que viajan en texto plano
/// en el body.
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
///
/// Camino legado: los equipos nuevos se autentican con su clave (ver
/// [`autenticar_con_firmante`]).
pub fn autenticar_dispositivo(
    base_url: &str,
    secreto: &str,
    metadata: Option<&MetadatosDispositivo>,
) -> Result<TokenDispositivo, NubeError> {
    autenticar_con_secreto(base_url, secreto, None, metadata).map(|autenticado| autenticado.token)
}

/// Token recién emitido, más si el servidor aprovechó la llamada para
/// registrar la clave pública del equipo (migración desde el secreto).
pub(crate) struct Autenticado {
    pub token: TokenDispositivo,
    pub clave_registrada: bool,
}

#[derive(Deserialize)]
struct RespuestaToken {
    #[serde(flatten)]
    token: TokenDispositivo,
    #[serde(default)]
    clave_registrada: bool,
}

/// Camino legado por secreto. Si viene `clave_publica_jwk`, el servidor
/// migra al equipo en la misma llamada: guarda la clave y deja de aceptar
/// el secreto (ver `Autenticado::clave_registrada`).
pub(crate) fn autenticar_con_secreto(
    base_url: &str,
    secreto: &str,
    clave_publica_jwk: Option<&str>,
    metadata: Option<&MetadatosDispositivo>,
) -> Result<Autenticado, NubeError> {
    let mut cuerpo = serde_json::json!({ "secret": secreto });
    if let Some(jwk) = clave_publica_jwk {
        cuerpo["clave_publica_jwk"] = jwk_como_valor(jwk)?;
    }
    agregar_metadata(&mut cuerpo, metadata);
    let respuesta = cliente_http()
        .post(format!("{base_url}/functions/v1/device-auth"))
        .json(&cuerpo)
        .send()?;
    token_de_respuesta(respuesta, NubeError::CredencialesInvalidas)
}

/// Camino normal: pide un desafío al servidor, lo firma con la clave del
/// equipo y lo canjea por un token. Dos idas y vueltas, a cambio de que
/// ningún secreto viaje y de no depender del reloj del equipo.
pub(crate) fn autenticar_con_firmante(
    base_url: &str,
    firmante: &dyn super::firmante::FirmanteDispositivo,
    metadata: Option<&MetadatosDispositivo>,
) -> Result<TokenDispositivo, NubeError> {
    #[derive(Deserialize)]
    struct RespuestaDesafio {
        desafio: String,
    }

    let url = format!("{base_url}/functions/v1/device-auth");
    let desafio = cliente_http()
        .post(&url)
        .json(&serde_json::json!({ "desafio": true }))
        .send()?
        .error_for_status()?
        .json::<RespuestaDesafio>()?
        .desafio;

    let asercion = super::firmante::construir_asercion(firmante, &desafio)?;
    let mut cuerpo = serde_json::json!({ "asercion": asercion });
    agregar_metadata(&mut cuerpo, metadata);
    let respuesta = cliente_http().post(url).json(&cuerpo).send()?;
    token_de_respuesta(respuesta, NubeError::CredencialesInvalidas)
        .map(|autenticado| autenticado.token)
}

/// Canjea el código de vinculación que emitió el panel, atando la clave
/// pública de este equipo al dispositivo. Devuelve el primer token. Con
/// `dispositivo_esperado`, un código de otro dispositivo se rechaza sin
/// consumirse.
pub(crate) fn vincular_con_codigo(
    base_url: &str,
    codigo: &str,
    clave_publica_jwk: &str,
    dispositivo_esperado: Option<&str>,
    metadata: Option<&MetadatosDispositivo>,
) -> Result<TokenDispositivo, NubeError> {
    let mut cuerpo = serde_json::json!({
        "codigo": codigo,
        "clave_publica_jwk": jwk_como_valor(clave_publica_jwk)?,
    });
    if let Some(dispositivo_id) = dispositivo_esperado {
        cuerpo["dispositivo_esperado"] = serde_json::Value::from(dispositivo_id);
    }
    agregar_metadata(&mut cuerpo, metadata);
    let respuesta = cliente_http()
        .post(format!("{base_url}/functions/v1/device-vincular"))
        .json(&cuerpo)
        .send()?;
    token_de_respuesta(respuesta, NubeError::CodigoVinculacionInvalido)
        .map(|autenticado| autenticado.token)
}

fn jwk_como_valor(jwk: &str) -> Result<serde_json::Value, NubeError> {
    serde_json::from_str(jwk).map_err(|error| {
        NubeError::Firmante(super::firmante::ErrorFirmante::ClaveInvalida(
            error.to_string(),
        ))
    })
}

fn agregar_metadata(cuerpo: &mut serde_json::Value, metadata: Option<&MetadatosDispositivo>) {
    if let Some(metadata) = metadata {
        cuerpo["metadata"] = serde_json::to_value(metadata).unwrap_or_default();
    }
}

/// Traduce el estado HTTP de `device-auth`/`device-vincular` y lee el token.
/// `no_autorizado` es lo que significa un 401 para quien llama: credencial
/// rechazada al autenticar, código inválido al vincular.
fn token_de_respuesta(
    respuesta: reqwest::blocking::Response,
    no_autorizado: NubeError,
) -> Result<Autenticado, NubeError> {
    match respuesta.status() {
        reqwest::StatusCode::UNAUTHORIZED => return Err(no_autorizado),
        reqwest::StatusCode::FORBIDDEN => return Err(NubeError::DispositivoSuspendido),
        reqwest::StatusCode::CONFLICT => return Err(NubeError::ClaveEnUso),
        // 426 Upgrade Required -- lo que `device-auth` manda cuando
        // `VERSION_MINIMA_ACEPTADA` rechaza esta versión. Ver
        // `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`, punto 9.
        reqwest::StatusCode::UPGRADE_REQUIRED => return Err(NubeError::VersionDesactualizada),
        _ => {}
    }
    let respuesta = respuesta.error_for_status()?;
    let desfase_reloj_ms = desfase_reloj_ms_desde_header(&respuesta);
    let RespuestaToken {
        mut token,
        clave_registrada,
    } = respuesta.json::<RespuestaToken>()?;
    token.desfase_reloj_ms = desfase_reloj_ms;
    Ok(Autenticado {
        token,
        clave_registrada,
    })
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
    fn dispositivo_suspendido_se_reporta_como_tal() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"device_suspended\"}",
        );

        let resultado = autenticar_dispositivo(&base_url, "secreto", None);

        assert!(matches!(resultado, Err(NubeError::DispositivoSuspendido)));
    }

    #[test]
    fn version_desactualizada_se_reporta_como_tal() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 426 Upgrade Required\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"version_desactualizada\",\"version_minima\":\"2.0.0\"}",
        );

        let resultado = autenticar_dispositivo(&base_url, "secreto", None);

        assert!(matches!(resultado, Err(NubeError::VersionDesactualizada)));
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
