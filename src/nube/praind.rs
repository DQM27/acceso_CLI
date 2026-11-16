//! Cliente del Worker de carnés PRAIND (`worker-praind/`, ver
//! `docs/features-futuras/plan-fotos-praind.md`).
//!
//! El Worker sirve y recibe las imágenes del bucket privado de R2; los
//! permisos los decide la nube con el token del equipo. Acá sólo se arman los
//! pedidos y se traducen las respuestas a mensajes para la pantalla.

use serde::{Deserialize, Serialize};

const PRAIND_URL_ENV: &str = "CONTROL_ACCESO_PRAIND_URL";
const PROYECTO_STAGING: &str = "pmrytjktlyiuikxuuxpr";

/// Bajar un carné de ~1,2 MB o pedirle al Worker que lo traiga de la página
/// del QR tarda más que una consulta corta: tope propio, más largo que el
/// del cliente general.
const TIMEOUT_PRAIND: std::time::Duration = std::time::Duration::from_secs(30);

static CLIENTE: std::sync::LazyLock<reqwest::blocking::Client> = std::sync::LazyLock::new(|| {
    reqwest::blocking::Client::builder()
        .timeout(TIMEOUT_PRAIND)
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
});

/// Dirección del Worker. Por defecto sigue al proyecto de Supabase activo
/// (staging o producción); `CONTROL_ACCESO_PRAIND_URL` la reemplaza.
pub fn praind_url() -> &'static str {
    static VALOR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VALOR.get_or_init(|| resolver_praind_url(std::env::var(PRAIND_URL_ENV).ok(), super::base_url()))
}

fn resolver_praind_url(valor_env: Option<String>, base_url: &str) -> String {
    valor_env.map_or_else(
        || {
            if base_url.contains(PROYECTO_STAGING) {
                "https://praind-staging.megabrisas.com".to_string()
            } else {
                "https://praind.megabrisas.com".to_string()
            }
        },
        |valor| valor.trim_end_matches('/').to_string(),
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermisosPraind {
    pub puede_ver: bool,
    pub puede_cargar: bool,
    #[serde(default)]
    pub es_admin: bool,
}

/// Resultado de una carga (o de su previsualización, sin `clave`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargaPraind {
    #[serde(default)]
    pub clave: Option<String>,
    #[serde(default)]
    pub nombre_en_origen: Option<String>,
    pub bytes: u64,
    pub tipo_mime: String,
}

/// Un carné bajado del Worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarneDescargado {
    pub tipo_mime: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum ErrorPraind {
    /// No hubo respuesta (sin internet, tiempo agotado).
    SinConexion,
    /// El Worker o la nube rechazaron el pedido, con el texto para mostrar.
    Rechazo { estado: u16, mensaje: String },
    /// Respuesta que no se entendió.
    Inesperado(String),
}

impl ErrorPraind {
    pub fn es_sin_conexion(&self) -> bool {
        matches!(self, Self::SinConexion)
    }
}

impl std::fmt::Display for ErrorPraind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SinConexion => write!(
                f,
                "No hay conexión con la nube. Revisá la conexión a internet."
            ),
            Self::Rechazo { mensaje, .. } => write!(f, "{mensaje}"),
            Self::Inesperado(detalle) => write!(
                f,
                "Respuesta inesperada del servicio de carnés ({detalle})."
            ),
        }
    }
}

impl std::error::Error for ErrorPraind {}

fn error_de_red(error: &reqwest::Error) -> ErrorPraind {
    if error.is_connect() || error.is_timeout() || error.is_request() {
        ErrorPraind::SinConexion
    } else {
        ErrorPraind::Inesperado(error.to_string())
    }
}

#[derive(Deserialize)]
struct CuerpoError {
    detail: Option<String>,
    message: Option<String>,
}

/// Convierte una respuesta no exitosa en `Rechazo` con el texto que mandó
/// el Worker (`detail`) o `PostgREST` (`message`).
fn rechazo(respuesta: reqwest::blocking::Response) -> ErrorPraind {
    let estado = respuesta.status().as_u16();
    let mensaje = respuesta
        .json::<CuerpoError>()
        .ok()
        .and_then(|cuerpo| cuerpo.detail.or(cuerpo.message))
        .unwrap_or_else(|| match estado {
            401 => "La sesión del equipo venció. Intentá de nuevo.".to_string(),
            403 => "Este equipo no tiene permiso para esta acción.".to_string(),
            404 => "El carné no existe.".to_string(),
            _ => format!("El servicio de carnés respondió {estado}."),
        });
    ErrorPraind::Rechazo { estado, mensaje }
}

/// Lo que este equipo puede hacer con los carnés, según la nube.
pub fn permisos(base_url: &str, apikey: &str, token: &str) -> Result<PermisosPraind, ErrorPraind> {
    let respuesta = CLIENTE
        .post(format!("{base_url}/rest/v1/rpc/praind_permisos"))
        .header("apikey", apikey)
        .bearer_auth(token)
        .json(&serde_json::json!({}))
        .send()
        .map_err(|error| error_de_red(&error))?;
    if !respuesta.status().is_success() {
        return Err(rechazo(respuesta));
    }
    respuesta
        .json()
        .map_err(|error| ErrorPraind::Inesperado(error.to_string()))
}

/// Baja el original de un carné.
pub fn descargar(token: &str, clave: &str) -> Result<CarneDescargado, ErrorPraind> {
    let respuesta = CLIENTE
        .get(format!("{}/foto/{clave}", praind_url()))
        .bearer_auth(token)
        .send()
        .map_err(|error| error_de_red(&error))?;
    if !respuesta.status().is_success() {
        return Err(rechazo(respuesta));
    }
    let tipo_mime = respuesta
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|valor| valor.to_str().ok())
        .unwrap_or("image/png")
        .to_string();
    let bytes = respuesta
        .bytes()
        .map_err(|error| error_de_red(&error))?
        .to_vec();
    Ok(CarneDescargado { tipo_mime, bytes })
}

/// Pide al Worker que baje el carné del enlace del QR. Con
/// `solo_previsualizar` no guarda nada y devuelve el nombre de la página,
/// para compararlo con el contratista antes de confirmar.
pub fn cargar_desde_qr(
    token: &str,
    contratista_uuid: &str,
    enlace: &str,
    solo_previsualizar: bool,
) -> Result<CargaPraind, ErrorPraind> {
    let respuesta = CLIENTE
        .post(format!("{}/cargar-desde-qr", praind_url()))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "contratista_id": contratista_uuid,
            "enlace": enlace,
            "solo_previsualizar": solo_previsualizar,
        }))
        .send()
        .map_err(|error| error_de_red(&error))?;
    if !respuesta.status().is_success() {
        return Err(rechazo(respuesta));
    }
    respuesta
        .json()
        .map_err(|error| ErrorPraind::Inesperado(error.to_string()))
}

/// Sube la imagen del carné directamente (PNG o JPEG).
pub fn cargar_archivo(
    token: &str,
    contratista_uuid: &str,
    bytes: Vec<u8>,
) -> Result<CargaPraind, ErrorPraind> {
    let respuesta = CLIENTE
        .put(format!("{}/cargar/{contratista_uuid}", praind_url()))
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
        .body(bytes)
        .send()
        .map_err(|error| error_de_red(&error))?;
    if !respuesta.status().is_success() {
        return Err(rechazo(respuesta));
    }
    respuesta
        .json()
        .map_err(|error| ErrorPraind::Inesperado(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_direccion_sigue_al_proyecto_activo() {
        assert_eq!(
            resolver_praind_url(None, "https://xidaepyaljzkpbsxrqsm.supabase.co"),
            "https://praind.megabrisas.com"
        );
        assert_eq!(
            resolver_praind_url(None, "https://pmrytjktlyiuikxuuxpr.supabase.co"),
            "https://praind-staging.megabrisas.com"
        );
        assert_eq!(
            resolver_praind_url(
                Some("http://localhost:8787/".to_string()),
                "https://x.supabase.co"
            ),
            "http://localhost:8787"
        );
    }

    #[test]
    fn la_carga_acepta_la_previsualizacion_sin_clave() {
        let carga: CargaPraind = serde_json::from_str(
            r#"{"nombre_en_origen":"Ana","bytes":10,"tipo_mime":"image/png"}"#,
        )
        .unwrap();
        assert_eq!(carga.clave, None);
        assert_eq!(carga.nombre_en_origen.as_deref(), Some("Ana"));
    }
}
