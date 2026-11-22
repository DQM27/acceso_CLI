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
    /// Sólo en la vista previa pedida con la imagen (el teléfono la lee con
    /// OCR antes de guardar). No se manda a las pantallas.
    #[serde(default, skip_serializing)]
    pub imagen_base64: Option<String>,
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
                "No hay conexión con la nube. Revise la conexión a internet."
            ),
            Self::Rechazo { mensaje, .. } => write!(f, "{mensaje}"),
            Self::Inesperado(detalle) => write!(
                f,
                "Respuesta inesperada del servicio de gafetes PRAIND ({detalle})."
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
            404 => "El gafete PRAIND no existe.".to_string(),
            _ => format!("El servicio de gafetes PRAIND respondió {estado}."),
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
///
/// Sin `contratista_uuid` sólo sirve para previsualizar: es el alta de un
/// contratista nuevo, que todavía no existe en la nube.
pub fn cargar_desde_qr(
    token: &str,
    contratista_uuid: Option<&str>,
    enlace: &str,
    solo_previsualizar: bool,
    incluir_imagen: bool,
) -> Result<CargaPraind, ErrorPraind> {
    let mut cuerpo = serde_json::json!({
        "enlace": enlace,
        "solo_previsualizar": solo_previsualizar,
        "incluir_imagen": incluir_imagen,
    });
    if let Some(uuid) = contratista_uuid {
        cuerpo["contratista_id"] = serde_json::Value::from(uuid);
    }
    let respuesta = CLIENTE
        .post(format!("{}/cargar-desde-qr", praind_url()))
        .bearer_auth(token)
        .json(&cuerpo)
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

// ---- Lo que comparten escritorio y móvil ----
//
// Ver un carné: si ya se abrió en este equipo sale de la base local (al
// instante y sin internet); si no, se baja y queda guardado. Nada se baja por
// adelantado. `token` sólo se llama si hace falta la nube.

pub const SIN_CARNE: &str = "Este contratista no tiene el gafete PRAIND cargado.";
pub const SIN_INTERNET_PRIMERA_VEZ: &str = "Para ver este gafete PRAIND por primera vez, conéctese a internet. \
     Después de verlo una vez queda guardado en este equipo.";
const SIN_SINCRONIZAR: &str =
    "Este contratista todavía no llegó a la nube. Espere a que sincronice y vuelva a intentar.";

fn ahora() -> String {
    crate::tiempo::serializar_utc(chrono::Utc::now())
}

/// Si el nombre de la página del QR corresponde al contratista. La regla
/// vive en el dominio ([`crate::domain::gafete_praind::nombres_coinciden`]),
/// la misma que compara el nombre leído del gafete.
pub use crate::domain::gafete_praind::nombres_coinciden;

fn carne_de(
    connection: &rusqlite::Connection,
    contratista_id: i64,
) -> Result<crate::database::queries::carnes_praind::CarneDeContratista, String> {
    crate::database::queries::carnes_praind::carne_de_contratista(connection, contratista_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "El contratista no existe.".to_string())
}

/// Estado del carné de un contratista según la base local.
pub fn estado_carne(
    connection: &rusqlite::Connection,
    contratista_id: i64,
) -> Result<crate::database::queries::carnes_praind::CarneDeContratista, String> {
    carne_de(connection, contratista_id)
}

/// Bytes y tipo del carné: de la base local si ya se abrió; si no, de la
/// nube (y queda guardado).
pub fn ver_carne(
    connection: &rusqlite::Connection,
    contratista_id: i64,
    token: impl FnOnce() -> Result<String, ErrorPraind>,
) -> Result<CarneDescargado, String> {
    use crate::database::queries::carnes_praind;

    let clave = carne_de(connection, contratista_id)?
        .clave
        .ok_or_else(|| SIN_CARNE.to_string())?;
    if let Some(guardado) = carnes_praind::abrir_guardado(connection, &clave, &ahora())
        .map_err(|error| error.to_string())?
    {
        return Ok(CarneDescargado {
            tipo_mime: guardado.tipo_mime,
            bytes: guardado.bytes,
        });
    }
    let descargado = token()
        .and_then(|token| descargar(&token, &clave))
        .map_err(|error| {
            if error.es_sin_conexion() {
                SIN_INTERNET_PRIMERA_VEZ.to_string()
            } else {
                error.to_string()
            }
        })?;
    // Si no se pudo guardar, igual se muestra: la próxima vez se vuelve a bajar.
    let _ = carnes_praind::guardar(
        connection,
        &clave,
        &descargado.tipo_mime,
        &descargado.bytes,
        &ahora(),
    );
    Ok(descargado)
}

fn anotar_clave(
    connection: &rusqlite::Connection,
    contratista_id: i64,
    clave: &str,
) -> Result<(), String> {
    connection
        .execute(
            "UPDATE contratistas SET praind_foto = ?2 WHERE id = ?1",
            rusqlite::params![contratista_id, clave],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Carga el carné desde el enlace del QR. Con `solo_previsualizar` no guarda
/// nada: devuelve el nombre de la página para compararlo antes. Al guardar,
/// la ruta nueva queda anotada sin esperar a la sincronización.
pub fn cargar_carne_desde_qr(
    connection: &rusqlite::Connection,
    contratista_id: i64,
    enlace: &str,
    solo_previsualizar: bool,
    token: impl FnOnce() -> Result<String, ErrorPraind>,
) -> Result<CargaPraind, String> {
    let uuid = carne_de(connection, contratista_id)?
        .uuid
        .ok_or_else(|| SIN_SINCRONIZAR.to_string())?;
    let carga = token()
        .and_then(|token| cargar_desde_qr(&token, Some(&uuid), enlace.trim(), solo_previsualizar, false))
        .map_err(|error| error.to_string())?;
    if let Some(clave) = carga.clave.as_deref() {
        anotar_clave(connection, contratista_id, clave)?;
    }
    Ok(carga)
}

/// Vista previa del gafete del QR **con su imagen**, sin guardar nada: el
/// teléfono la lee con OCR para compararla con la ficha antes de guardar.
pub fn previsualizar_carne_con_imagen(
    connection: &rusqlite::Connection,
    contratista_id: i64,
    enlace: &str,
    token: impl FnOnce() -> Result<String, ErrorPraind>,
) -> Result<(CargaPraind, Vec<u8>), String> {
    let uuid = carne_de(connection, contratista_id)?
        .uuid
        .ok_or_else(|| SIN_SINCRONIZAR.to_string())?;
    previsualizar_con_imagen(Some(&uuid), enlace, token)
}

/// Vista previa del gafete del QR con su imagen para dar de alta a un
/// contratista nuevo: todavía no existe, ni en el equipo ni en la nube. No
/// guarda nada.
pub fn previsualizar_gafete_para_alta(
    enlace: &str,
    token: impl FnOnce() -> Result<String, ErrorPraind>,
) -> Result<(CargaPraind, Vec<u8>), String> {
    previsualizar_con_imagen(None, enlace, token)
}

fn previsualizar_con_imagen(
    uuid: Option<&str>,
    enlace: &str,
    token: impl FnOnce() -> Result<String, ErrorPraind>,
) -> Result<(CargaPraind, Vec<u8>), String> {
    use base64::Engine as _;

    let mut carga = token()
        .and_then(|token| cargar_desde_qr(&token, uuid, enlace.trim(), true, true))
        .map_err(|error| error.to_string())?;
    let imagen = carga
        .imagen_base64
        .take()
        .and_then(|texto| base64::engine::general_purpose::STANDARD.decode(texto).ok())
        .ok_or_else(|| "La vista previa no trajo la imagen del gafete PRAIND.".to_string())?;
    Ok((carga, imagen))
}

/// Carga la imagen del carné. Queda guardada en el equipo para verla sin
/// volver a bajarla.
pub fn cargar_carne_archivo(
    connection: &rusqlite::Connection,
    contratista_id: i64,
    bytes: Vec<u8>,
    token: impl FnOnce() -> Result<String, ErrorPraind>,
) -> Result<CargaPraind, String> {
    let uuid = carne_de(connection, contratista_id)?
        .uuid
        .ok_or_else(|| SIN_SINCRONIZAR.to_string())?;
    let carga = token()
        .and_then(|token| cargar_archivo(&token, &uuid, bytes.clone()))
        .map_err(|error| error.to_string())?;
    if let Some(clave) = carga.clave.as_deref() {
        anotar_clave(connection, contratista_id, clave)?;
        let _ = crate::database::queries::carnes_praind::guardar(
            connection,
            clave,
            &carga.tipo_mime,
            &bytes,
            &ahora(),
        );
    }
    Ok(carga)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_con_contratista(praind_foto: Option<&str>, uuid: Option<&str>) -> rusqlite::Connection {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        crate::database::schema::initialize_database(&connection).unwrap();
        connection
            .execute_batch("INSERT INTO empresas (id, nombre, activo) VALUES (1, 'E', 1);")
            .unwrap();
        connection
            .execute(
                "INSERT INTO contratistas (id, cedula, nombre, empresa_id, tipo_ingreso, tiene_acceso, uuid, praind_foto)
                 VALUES (1, '111111111', 'ANA', 1, 'PRAIND', 1, ?1, ?2)",
                rusqlite::params![uuid, praind_foto],
            )
            .unwrap();
        connection
    }

    #[test]
    fn compara_nombres_como_escritorio_y_panel() {
        assert!(nombres_coinciden(
            Some("Mindy  Murillo Guzmán"),
            "MINDY MURILLO GUZMAN"
        ));
        assert!(nombres_coinciden(
            Some("Ana María Pérez Soto"),
            "ANA PEREZ SOTO"
        ));
        assert!(!nombres_coinciden(
            Some("Ana Pérez Soto"),
            "LUIS PEREZ SOTO"
        ));
        assert!(!nombres_coinciden(Some("Ana"), "ANA PEREZ SOTO"));
        assert!(!nombres_coinciden(None, "ANA PEREZ"));
    }

    #[test]
    fn ver_carne_sin_carne_no_va_a_la_nube() {
        let connection = base_con_contratista(None, Some("u1"));
        let error = ver_carne(&connection, 1, || panic!("no debía pedir token")).unwrap_err();
        assert_eq!(error, SIN_CARNE);
    }

    #[test]
    fn ver_carne_guardado_sale_de_la_base_sin_internet() {
        let clave = "contratistas/u1/abc.png";
        let connection = base_con_contratista(Some(clave), Some("u1"));
        crate::database::queries::carnes_praind::guardar(
            &connection,
            clave,
            "image/png",
            &[7, 8],
            "2026-11-15T00:00:00Z",
        )
        .unwrap();
        let carne = ver_carne(&connection, 1, || panic!("no debía pedir token")).unwrap();
        assert_eq!(carne.bytes, vec![7, 8]);
    }

    #[test]
    fn ver_carne_no_guardado_y_sin_internet_lo_dice() {
        let connection = base_con_contratista(Some("contratistas/u1/abc.png"), Some("u1"));
        let error = ver_carne(&connection, 1, || Err(ErrorPraind::SinConexion)).unwrap_err();
        assert_eq!(error, SIN_INTERNET_PRIMERA_VEZ);
    }

    #[test]
    fn cargar_sin_sincronizar_lo_dice_sin_ir_a_la_nube() {
        let connection = base_con_contratista(None, None);
        let error = cargar_carne_desde_qr(&connection, 1, "https://prd.pw/x", true, || {
            panic!("no debía pedir token")
        })
        .unwrap_err();
        assert_eq!(error, SIN_SINCRONIZAR);
    }

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
