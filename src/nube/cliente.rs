//! Cliente HTTP del receptor en la nube (ver `docs/planes-implementados/plan-persistencia-nube.md`).
//!
//! Bloqueante a propósito: el resto del crate es síncrono; traer un runtime
//! async (tokio) solo para esto no se justifica todavía.

use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum NubeError {
    #[error("No se pudo contactar al receptor: {0}")]
    Red(#[from] reqwest::Error),
    /// `device-auth` no reconoce la clave del equipo, o la firma no es de
    /// esa clave (`401`). NO quiere decir que lo hayan retirado: un retiro
    /// llega sólo como [`Self::EquipoRetirado`]. Antes del incidente del
    /// 2026-10-26 este error marcaba el equipo como retirado, y un `401`
    /// pasajero de la nube lo dejó así sin estarlo.
    #[error("La nube no reconoce la clave de este dispositivo")]
    CredencialesInvalidas,
    /// `device-auth` confirmó que este equipo fue retirado en el panel
    /// (`403 dispositivo_retirado`): la nube sólo lo responde con la fila
    /// retirada del equipo y su firma válida a la vista. Es lo único que
    /// marca un equipo como retirado (ver `AppCore::anotar_autenticacion`).
    #[error("Este dispositivo fue retirado en el panel")]
    EquipoRetirado,
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
    /// El equipo nunca se vinculó (no tiene clave aceptada por el servidor).
    #[error("Este dispositivo todavía no está vinculado a la nube")]
    SinCredencial,
    #[error(transparent)]
    Firmante(#[from] super::firmante::ErrorFirmante),
}

impl NubeError {
    /// La nube no se pudo alcanzar: no hubo conexión, no respondió a
    /// tiempo o está caída (`5xx`, por ejemplo `device-auth` sin poder
    /// atender). No incluye que la nube conteste "no" (dispositivo retirado,
    /// versión vieja, sin credencial) ni una respuesta que no se entendió:
    /// esos son rechazos de verdad, no falta de red.
    pub fn es_falta_de_conexion(&self) -> bool {
        matches!(
            self,
            Self::Red(error) if error.is_connect()
                || error.is_timeout()
                || error.status().is_some_and(|status| status.is_server_error())
        )
    }
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

/// Cliente HTTP bloqueante general del crate -- construido una sola vez (ver
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

/// Tope de las consultas en vivo que se hacen justo antes de registrar en
/// la portería (`sincronizacion::consultas_en_vivo`): quien opera está
/// esperando con la persona enfrente. Pasado este tiempo la consulta cuenta
/// como "sin conexión" y el registro sigue con las reglas locales (ver
/// `application::con_nube`); la sincronización, que no tiene a nadie
/// esperando, conserva el tope general de [`TIMEOUT_HTTP`].
const TIMEOUT_VERIFICACION: std::time::Duration = std::time::Duration::from_secs(3);

/// Segundo cliente compartido, igual que [`CLIENTE_HTTP`] pero con
/// [`TIMEOUT_VERIFICACION`]. Aparte para no acortar el tope de la
/// sincronización, que sí puede tardar más sin estorbar a nadie.
static CLIENTE_HTTP_VERIFICACION: std::sync::LazyLock<reqwest::blocking::Client> =
    std::sync::LazyLock::new(|| {
        reqwest::blocking::Client::builder()
            .timeout(TIMEOUT_VERIFICACION)
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    });

/// Clon barato del cliente de las consultas en vivo (ver
/// [`TIMEOUT_VERIFICACION`]).
pub(crate) fn cliente_http_verificacion() -> reqwest::blocking::Client {
    CLIENTE_HTTP_VERIFICACION.clone()
}

/// Cuántas conexiones se calientan: las consultas de la verificación de un
/// ingreso salen a la vez (la persona, la otra vía, el gafete). Con HTTP/2
/// comparten una sola conexión y las demás no abren nada nuevo; se dejan 3
/// por si la conexión baja a HTTP/1.1 (un proxy que no lo negocia), donde
/// cada consulta necesita la suya.
const CONEXIONES_A_CALENTAR: usize = 3;

/// Mínimo entre dos calentamientos: si el modal de ingreso se abre varias
/// veces seguidas, las conexiones del segundo todavía están calientes.
const PAUSA_ENTRE_CALENTAMIENTOS: std::time::Duration = std::time::Duration::from_secs(20);

static ULTIMO_CALENTAMIENTO: std::sync::Mutex<Option<std::time::Instant>> =
    std::sync::Mutex::new(None);

/// Abre de antemano las conexiones del cliente de verificación, para que la
/// primera consulta de un ingreso no pague conexión nueva más TLS.
///
/// El grupo de conexiones se enfría tras ~1 minuto sin usarse, y en una
/// portería los ingresos llegan con minutos de diferencia: medido contra
/// staging (`tests/medir_conexion_nube.rs`), tres consultas en paralelo
/// tardan ~135 ms con las conexiones calientes y 450-590 ms con el grupo
/// frío, así que casi todo ingreso pagaba ese costo al elegir a la persona.
/// Quien llama lo dispara al abrir el formulario de ingreso, cuando la persona
/// todavía está escribiendo, y el costo queda escondido.
///
/// Mejor esfuerzo: un `OPTIONS` a la raíz de la API, que la puerta de entrada
/// de Supabase contesta 200 sin cuerpo, sin token y sin tocar la base, sólo
/// para que las conexiones queden en el grupo; los errores no importan. (Antes
/// era un `GET` con la clave pública, que contestaba 401 y ensuciaba los logs
/// con cientos de "no autorizado" por día.) Bloquea hasta que terminan, así que quien llama lo
/// corre en un hilo aparte. `false` si no corrió porque se calentó hace poco.
pub fn precalentar_conexiones_de_verificacion(base_url: &str, apikey: &str) -> bool {
    precalentar_si_corresponde(
        &ULTIMO_CALENTAMIENTO,
        std::time::Instant::now(),
        base_url,
        apikey,
    )
}

/// Cada cuánto [`mantener_conexiones_vivas`] toca la nube. Menos que lo que el
/// grupo de conexiones aguanta una sin usar (90 s por defecto en reqwest),
/// así nunca llega a enfriarse.
const INTERVALO_CONEXIONES_VIVAS: std::time::Duration = std::time::Duration::from_secs(45);

/// Mantiene abiertas, mientras viva el proceso, las conexiones de los dos
/// clientes (la sincronización y las salidas que cierran en la nube, y las
/// verificaciones de un ingreso): un `OPTIONS` sin token cada
/// [`INTERVALO_CONEXIONES_VIVAS`], el mismo que usa el precalentamiento (200
/// sin cuerpo, sin tocar la base). Sin esto, la primera operación después de
/// un rato quieto pagaba conexión nueva más TLS.
///
/// Sólo para el escritorio: en el teléfono despertaría la radio cada 45 s y
/// gastaría batería (ahí sigue el precalentamiento al abrir un formulario).
/// Corre en un hilo propio; llamarla de nuevo no abre otro.
pub fn mantener_conexiones_vivas(base_url: &'static str, apikey: &'static str) {
    static INICIADO: std::sync::Once = std::sync::Once::new();
    INICIADO.call_once(|| {
        let lanzado = std::thread::Builder::new()
            .name("conexiones-vivas".to_string())
            .spawn(move || {
                loop {
                    tocar_conexiones(base_url, apikey);
                    std::thread::sleep(INTERVALO_CONEXIONES_VIVAS);
                }
            });
        if let Err(error) = lanzado {
            log::warn!("no se pudo lanzar el hilo que mantiene las conexiones: {error}");
        }
    });
}

/// Una vuelta de [`mantener_conexiones_vivas`]: un `OPTIONS` por cliente. Los
/// errores no importan (sin red, la próxima vuelta lo intenta de nuevo).
pub(crate) fn tocar_conexiones(base_url: &str, apikey: &str) {
    let url = format!("{base_url}/rest/v1/");
    for cliente in [cliente_http(), cliente_http_verificacion()] {
        let _ = cliente
            .request(reqwest::Method::OPTIONS, &url)
            .header("apikey", apikey)
            .send();
    }
}

/// La parte testeable de [`precalentar_conexiones_de_verificacion`]: el estado
/// y el instante llegan como parámetro.
pub(crate) fn precalentar_si_corresponde(
    ultimo: &std::sync::Mutex<Option<std::time::Instant>>,
    ahora: std::time::Instant,
    base_url: &str,
    apikey: &str,
) -> bool {
    {
        let mut ultimo = ultimo
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if ultimo.is_some_and(|antes| {
            ahora.saturating_duration_since(antes) < PAUSA_ENTRE_CALENTAMIENTOS
        }) {
            return false;
        }
        *ultimo = Some(ahora);
    }
    let cliente = cliente_http_verificacion();
    let url = format!("{base_url}/rest/v1/");
    std::thread::scope(|hilos| {
        for _ in 0..CONEXIONES_A_CALENTAR {
            hilos.spawn(|| {
                let _ = cliente
                    .request(reqwest::Method::OPTIONS, &url)
                    .header("apikey", apikey)
                    .send();
            });
        }
    });
    true
}

/// Token que autoriza a este dispositivo a leer/escribir únicamente los
/// datos de su propio sitio. Vence a los `expires_in` segundos — hay que
/// volver a autenticarse para renovarlo (ver `CacheTokenDispositivo`), no se
/// refresca solo.
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
    /// Nombre de la unidad a la que quedó registrado el equipo, para
    /// mostrarlo (login, barra de estado). Sólo informativo: ningún permiso
    /// depende de él. `None` si el servidor no lo manda (versión anterior).
    #[serde(default)]
    pub sitio_nombre: Option<String>,
    /// Etiqueta con la que el panel registró el equipo. Igual que
    /// `sitio_nombre`: sólo para mostrar.
    #[serde(default)]
    pub etiqueta: Option<String>,
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
            .field("sitio_nombre", &self.sitio_nombre)
            .field("etiqueta", &self.etiqueta)
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

/// Autenticación del equipo: pide un desafío al servidor, lo firma con la
/// clave del equipo y lo canjea por un token. Dos idas y vueltas, a cambio
/// de que ningún secreto viaje y de no depender del reloj del equipo.
///
/// `cliente` decide el tope de espera de cada ida y vuelta: el general
/// ([`cliente_http`]) o el de la verificación en la portería
/// ([`cliente_http_verificacion`]).
pub(crate) fn autenticar_con_firmante(
    cliente: &reqwest::blocking::Client,
    base_url: &str,
    firmante: &dyn super::firmante::FirmanteDispositivo,
    metadata: Option<&MetadatosDispositivo>,
) -> Result<TokenDispositivo, NubeError> {
    #[derive(Deserialize)]
    struct RespuestaDesafio {
        desafio: String,
    }

    let url = format!("{base_url}/functions/v1/device-auth");
    // Un desafío vencido (`400 desafio_invalido`: el equipo tardó entre los
    // dos pasos) se repite UNA vez con uno nuevo.
    for intento in 0..2 {
        let desafio = cliente
            .post(&url)
            .json(&serde_json::json!({ "desafio": true }))
            .send()?
            .error_for_status()?
            .json::<RespuestaDesafio>()?
            .desafio;

        let asercion = super::firmante::construir_asercion(firmante, &desafio)?;
        let mut cuerpo = serde_json::json!({ "asercion": asercion });
        agregar_metadata(&mut cuerpo, metadata);
        let respuesta = cliente.post(&url).json(&cuerpo).send()?;
        if intento == 0 && respuesta.status() == reqwest::StatusCode::BAD_REQUEST {
            let error_http = respuesta.error_for_status_ref().err();
            if codigo_de_error(respuesta).as_deref() == Some("desafio_invalido") {
                continue;
            }
            return Err(error_http.map_or(NubeError::CredencialesInvalidas, NubeError::Red));
        }
        return token_de_respuesta(respuesta, NubeError::CredencialesInvalidas);
    }
    unreachable!("el segundo intento siempre devuelve")
}

/// `error` del cuerpo JSON de una respuesta de error (`{ "error": "..." }`).
/// Quien lo llama toma antes el error HTTP con `error_for_status_ref`, que
/// para un `4xx` siempre existe (el respaldo de su `map_or` no se usa).
fn codigo_de_error(respuesta: reqwest::blocking::Response) -> Option<String> {
    let cuerpo: serde_json::Value = respuesta.json().ok()?;
    cuerpo.get("error")?.as_str().map(str::to_string)
}

/// Canjea el código de vinculación que emitió el panel, atando la clave
/// pública de este equipo al dispositivo. Devuelve el primer token.
pub(crate) fn vincular_con_codigo(
    base_url: &str,
    codigo: &str,
    clave_publica_jwk: &str,
    metadata: Option<&MetadatosDispositivo>,
) -> Result<TokenDispositivo, NubeError> {
    let mut cuerpo = serde_json::json!({
        "codigo": codigo,
        "clave_publica_jwk": jwk_como_valor(clave_publica_jwk)?,
    });
    agregar_metadata(&mut cuerpo, metadata);
    let respuesta = cliente_http()
        .post(format!("{base_url}/functions/v1/device-vincular"))
        .json(&cuerpo)
        .send()?;
    token_de_respuesta(respuesta, NubeError::CodigoVinculacionInvalido)
}

/// A dónde va un equipo retirado con un código de reconexión (ver
/// `device-reconectar`). Lo responde la nube sin gastar el código.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DestinoReconexion {
    pub sitio_id: String,
    pub sitio_nombre: String,
    pub etiqueta: String,
    /// `true` si va a otra unidad: el equipo tiene que borrar su base y
    /// bajar la de la unidad nueva.
    pub cambio_de_unidad: bool,
}

/// Lo que el equipo confirma al reconectarse: queda en el evento del panel.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct DetalleReconexion {
    /// Registros sin subir que se pierden al cambiar de unidad.
    pub pendientes_descartados: u64,
    /// Quién confirmó en el equipo (la sesión abierta), si hubo.
    pub confirmado_por: Option<String>,
}

/// Consulta a qué unidad va el equipo con `codigo`, sin gastarlo. Sólo
/// responde si el código es de reconexión, está vigente y el equipo sigue
/// retirado; si no, [`NubeError::CodigoVinculacionInvalido`].
pub(crate) fn consultar_reconexion(
    base_url: &str,
    codigo: &str,
) -> Result<DestinoReconexion, NubeError> {
    let respuesta = cliente_http()
        .post(format!("{base_url}/functions/v1/device-reconectar"))
        .json(&serde_json::json!({ "codigo": codigo, "confirmar": false }))
        .send()?;
    if respuesta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(NubeError::CodigoVinculacionInvalido);
    }
    Ok(respuesta.error_for_status()?.json::<DestinoReconexion>()?)
}

/// Canjea un código de reconexión atando `clave_publica_jwk` (la clave NUEVA
/// del equipo). Devuelve el primer token y si cambió de unidad.
pub(crate) fn canjear_reconexion(
    base_url: &str,
    codigo: &str,
    clave_publica_jwk: &str,
    metadata: Option<&MetadatosDispositivo>,
    detalle: &DetalleReconexion,
) -> Result<(TokenDispositivo, bool), NubeError> {
    #[derive(Deserialize)]
    struct RespuestaReconexion {
        #[serde(flatten)]
        token: TokenDispositivo,
        #[serde(default)]
        cambio_de_unidad: bool,
    }

    let mut cuerpo = serde_json::json!({
        "codigo": codigo,
        "confirmar": true,
        "clave_publica_jwk": jwk_como_valor(clave_publica_jwk)?,
        "pendientes_descartados": detalle.pendientes_descartados,
        "confirmado_por": detalle.confirmado_por,
    });
    agregar_metadata(&mut cuerpo, metadata);
    let respuesta = cliente_http()
        .post(format!("{base_url}/functions/v1/device-reconectar"))
        .json(&cuerpo)
        .send()?;
    match respuesta.status() {
        reqwest::StatusCode::UNAUTHORIZED => return Err(NubeError::CodigoVinculacionInvalido),
        reqwest::StatusCode::CONFLICT => return Err(NubeError::ClaveEnUso),
        _ => {}
    }
    let respuesta = respuesta.error_for_status()?;
    let desfase_reloj_ms = desfase_reloj_ms_desde_header(&respuesta);
    let RespuestaReconexion {
        mut token,
        cambio_de_unidad,
    } = respuesta.json::<RespuestaReconexion>()?;
    token.desfase_reloj_ms = desfase_reloj_ms;
    Ok((token, cambio_de_unidad))
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
) -> Result<TokenDispositivo, NubeError> {
    match respuesta.status() {
        reqwest::StatusCode::UNAUTHORIZED => return Err(no_autorizado),
        reqwest::StatusCode::CONFLICT => return Err(NubeError::ClaveEnUso),
        // Sólo `dispositivo_retirado` es un retiro; cualquier otro 403 queda
        // como error HTTP, sin marcar nada.
        reqwest::StatusCode::FORBIDDEN => {
            let error_http = respuesta.error_for_status_ref().err();
            return Err(
                if codigo_de_error(respuesta).as_deref() == Some("dispositivo_retirado") {
                    NubeError::EquipoRetirado
                } else {
                    error_http.map_or(NubeError::CredencialesInvalidas, NubeError::Red)
                },
            );
        }
        // 426 Upgrade Required -- lo que `device-auth` manda cuando
        // `VERSION_MINIMA_ACEPTADA` rechaza esta versión. Ver
        // `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`, punto 9.
        reqwest::StatusCode::UPGRADE_REQUIRED => return Err(NubeError::VersionDesactualizada),
        _ => {}
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
    /// para pasarle a [`canjear`].
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

    /// Canje con una clave pública de prueba: una sola petición, así que
    /// sirve para probar la lectura del token y la traducción de estados
    /// HTTP que comparte con `autenticar_con_firmante`.
    fn canjear(base_url: &str) -> Result<TokenDispositivo, NubeError> {
        vincular_con_codigo(
            base_url,
            "K7QMR4XT2P",
            r#"{"crv":"P-256","kty":"EC","x":"f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU","y":"x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0"}"#,
            None,
        )
    }

    /// Una nube caída (`5xx`, por ejemplo `device-auth` sin poder atender)
    /// cuenta como falta de conexión: la portería no espera por ella (ver
    /// `application::con_nube`).
    #[test]
    fn un_5xx_cuenta_como_falta_de_conexion() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );

        let error = canjear(&base_url).expect_err("un 503 no trae token");

        assert!(error.es_falta_de_conexion(), "{error:?}");
    }

    /// Que la nube conteste que no (`401`: clave o código rechazado) es un
    /// rechazo de verdad, no falta de red.
    #[test]
    fn un_rechazo_no_cuenta_como_falta_de_conexion() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );

        let error = canjear(&base_url).expect_err("un 401 no trae token");

        assert!(!error.es_falta_de_conexion(), "{error:?}");
    }

    #[test]
    fn sin_servidor_cuenta_como_falta_de_conexion() {
        let cerrado = {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
            format!("http://{}", listener.local_addr().expect("dirección local"))
        };

        let error = canjear(&cerrado).expect_err("sin servidor no hay token");

        assert!(error.es_falta_de_conexion(), "{error:?}");
    }

    #[test]
    fn lee_el_token_de_la_respuesta() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             {\"access_token\":\"abc\",\"expires_in\":3600,\"sitio_id\":\"s1\",\
             \"dispositivo_id\":\"d1\",\"tipo\":\"pc\"}",
        );

        let token = canjear(&base_url).expect("token ok");

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
            sitio_nombre: None,
            etiqueta: None,
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

        let token = canjear(&base_url).expect("token ok");

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
    fn codigo_rechazado_da_codigo_invalido() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"codigo_invalido\"}",
        );

        let resultado = canjear(&base_url);

        assert!(matches!(
            resultado,
            Err(NubeError::CodigoVinculacionInvalido)
        ));
    }

    #[test]
    fn clave_de_otro_dispositivo_da_clave_en_uso() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"clave_en_uso\"}",
        );

        assert!(matches!(canjear(&base_url), Err(NubeError::ClaveEnUso)));
    }

    #[test]
    fn version_desactualizada_se_reporta_como_tal() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 426 Upgrade Required\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"version_desactualizada\",\"version_minima\":\"2.0.0\"}",
        );

        let resultado = canjear(&base_url);

        assert!(matches!(resultado, Err(NubeError::VersionDesactualizada)));
    }

    #[test]
    fn error_de_servidor_se_reporta_como_error_de_red() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"boom\"}",
        );

        let resultado = canjear(&base_url);

        assert!(matches!(resultado, Err(NubeError::Red(_))));
    }

    // ---- device-auth: qué respuesta es un retiro y cuál no ----
    //
    // Incidente del 2026-10-26: un 401 pasajero de la nube marcó un teléfono
    // como retirado sin estarlo. Desde entonces sólo `403
    // dispositivo_retirado` es un retiro; todo lo demás es otro error.

    /// Firmante con una clave pública válida (la firma da igual: el
    /// servidor de estas pruebas no la verifica).
    struct FirmanteDePrueba;

    impl super::super::firmante::FirmanteDispositivo for FirmanteDePrueba {
        fn clave_publica_jwk(&self) -> Result<String, super::super::firmante::ErrorFirmante> {
            Ok(r#"{"crv":"P-256","kty":"EC","x":"f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU","y":"x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0"}"#.to_string())
        }
        fn firmar(&self, _datos: &[u8]) -> Result<Vec<u8>, super::super::firmante::ErrorFirmante> {
            Ok(vec![0; 64])
        }
        fn regenerar(&self) -> Result<(), super::super::firmante::ErrorFirmante> {
            Ok(())
        }
        fn dispositivo_vinculado(&self) -> Option<String> {
            Some("d1".to_string())
        }
        fn marcar_vinculada(
            &self,
            _dispositivo_id: &str,
        ) -> Result<(), super::super::firmante::ErrorFirmante> {
            Ok(())
        }
    }

    const DESAFIO: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
        Connection: close\r\n\r\n{\"desafio\":\"jwt-del-servidor\",\"expires_in\":120}";
    const TOKEN: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
        Connection: close\r\n\r\n{\"access_token\":\"abc\",\"expires_in\":3600,\
        \"sitio_id\":\"s1\",\"dispositivo_id\":\"d1\",\"tipo\":\"mobile\"}";
    const RETIRADO: &str = "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\n\
        Connection: close\r\n\r\n{\"error\":\"dispositivo_retirado\",\"retirado_en\":\"2026-10-26T12:10:49Z\"}";
    const NO_RECONOCIDO: &str = "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\n\
        Connection: close\r\n\r\n{\"error\":\"equipo_desconocido\"}";
    const NO_VERIFICABLE: &str = "HTTP/1.1 503 Service Unavailable\r\n\
        Content-Type: application/json\r\nConnection: close\r\n\r\n{\"error\":\"no_verificable\"}";
    const DESAFIO_VENCIDO: &str = "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\n\
        Connection: close\r\n\r\n{\"error\":\"desafio_invalido\"}";

    /// Responde `respuestas` en orden, una por conexión, y cuenta los
    /// pedidos que llegaron.
    fn servidor(
        respuestas: Vec<&'static str>,
    ) -> (String, std::sync::Arc<std::sync::Mutex<usize>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
        let direccion = listener.local_addr().expect("dirección local");
        let pedidos = std::sync::Arc::new(std::sync::Mutex::new(0));
        let contador = std::sync::Arc::clone(&pedidos);
        thread::spawn(move || {
            for respuesta in respuestas {
                let Ok((mut conexion, _)) = listener.accept() else {
                    return;
                };
                conexion
                    .set_read_timeout(Some(Duration::from_millis(200)))
                    .expect("set_read_timeout");
                let mut buffer = [0_u8; 4096];
                while let Ok(n) = conexion.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                }
                *contador.lock().unwrap() += 1;
                let _ = conexion.write_all(respuesta.as_bytes());
                let _ = conexion.flush();
            }
        });
        (format!("http://{direccion}"), pedidos)
    }

    fn autenticar(base_url: &str) -> Result<TokenDispositivo, NubeError> {
        autenticar_con_firmante(&cliente_http(), base_url, &FirmanteDePrueba, None)
    }

    #[test]
    fn device_auth_403_dispositivo_retirado_es_un_retiro() {
        let (url, _) = servidor(vec![DESAFIO, RETIRADO]);

        let error = autenticar(&url).expect_err("retirado no trae token");

        assert!(matches!(error, NubeError::EquipoRetirado), "{error:?}");
        assert!(!error.es_falta_de_conexion());
    }

    #[test]
    fn device_auth_401_no_es_un_retiro() {
        let (url, _) = servidor(vec![DESAFIO, NO_RECONOCIDO]);

        let error = autenticar(&url).expect_err("401 no trae token");

        assert!(
            matches!(error, NubeError::CredencialesInvalidas),
            "{error:?}"
        );
    }

    #[test]
    fn device_auth_403_sin_dispositivo_retirado_no_es_un_retiro() {
        let (url, _) = servidor(vec![
            DESAFIO,
            "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"otra_cosa\"}",
        ]);

        let error = autenticar(&url).expect_err("403 no trae token");

        assert!(matches!(error, NubeError::Red(_)), "{error:?}");
    }

    #[test]
    fn device_auth_403_sin_cuerpo_no_es_un_retiro() {
        let (url, _) = servidor(vec![
            DESAFIO,
            "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        ]);

        let error = autenticar(&url).expect_err("403 no trae token");

        assert!(!matches!(error, NubeError::EquipoRetirado), "{error:?}");
    }

    #[test]
    fn device_auth_503_no_verificable_es_falta_de_conexion() {
        let (url, _) = servidor(vec![DESAFIO, NO_VERIFICABLE]);

        let error = autenticar(&url).expect_err("503 no trae token");

        assert!(error.es_falta_de_conexion(), "{error:?}");
        assert!(!matches!(error, NubeError::EquipoRetirado));
    }

    #[test]
    fn device_auth_caido_al_pedir_el_desafio_es_falta_de_conexion() {
        let (url, _) = servidor(vec![NO_VERIFICABLE]);

        let error = autenticar(&url).expect_err("sin desafío no hay token");

        assert!(error.es_falta_de_conexion(), "{error:?}");
    }

    #[test]
    fn device_auth_desafio_vencido_se_reintenta_una_vez_con_uno_nuevo() {
        let (url, pedidos) = servidor(vec![DESAFIO, DESAFIO_VENCIDO, DESAFIO, TOKEN]);

        let token = autenticar(&url).expect("el segundo intento funciona");

        assert_eq!(token.dispositivo_id, "d1");
        assert_eq!(*pedidos.lock().unwrap(), 4, "dos desafíos y dos aserciones");
    }

    #[test]
    fn device_auth_desafio_vencido_dos_veces_se_rinde_sin_marcar_retiro() {
        let (url, pedidos) = servidor(vec![DESAFIO, DESAFIO_VENCIDO, DESAFIO, DESAFIO_VENCIDO]);

        let error = autenticar(&url).expect_err("dos desafíos vencidos");

        assert!(matches!(error, NubeError::Red(_)), "{error:?}");
        assert_eq!(*pedidos.lock().unwrap(), 4, "un solo reintento, sin bucle");
    }

    #[test]
    fn device_auth_otro_400_no_se_reintenta() {
        let (url, pedidos) = servidor(vec![
            DESAFIO,
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"bad_request\"}",
        ]);

        let error = autenticar(&url).expect_err("400 no trae token");

        assert!(matches!(error, NubeError::Red(_)), "{error:?}");
        assert_eq!(*pedidos.lock().unwrap(), 2);
    }

    #[test]
    fn device_auth_vigente_trae_el_token() {
        let (url, _) = servidor(vec![DESAFIO, TOKEN]);
        assert_eq!(autenticar(&url).expect("vigente").access_token, "abc");
    }

    /// De punta a punta, nube falsa → cliente → núcleo, como hacen las apps
    /// (`autenticar_con_cache` + `AppCore::anotar_autenticacion`).
    fn nucleo_en_memoria() -> crate::application::AppCore {
        let conexion = rusqlite::Connection::open_in_memory().unwrap();
        crate::database::schema::initialize_database(&conexion).unwrap();
        crate::application::AppCore::new(conexion)
    }

    /// El incidente del 2026-10-26 tal cual: un 401 pasajero, la nube que no
    /// puede verificar (503) y después una autenticación normal. El equipo
    /// no se muestra como retirado en ningún momento.
    #[test]
    fn incidente_de_punta_a_punta_nunca_se_muestra_como_retirado() {
        let core = nucleo_en_memoria();
        let (url, _) = servidor(vec![
            DESAFIO,
            NO_RECONOCIDO,
            DESAFIO,
            NO_VERIFICABLE,
            DESAFIO,
            TOKEN,
        ]);

        for paso in ["401 pasajero", "503", "vigente"] {
            core.anotar_autenticacion(&autenticar(&url));
            assert!(!core.equipo_retirado(), "después de: {paso}");
        }
    }

    /// Retiro real: la franja aparece con el 403 y se va sola apenas la nube
    /// vuelve a aceptar al equipo.
    #[test]
    fn retiro_de_punta_a_punta_aparece_con_el_403_y_se_va_con_el_200() {
        let core = nucleo_en_memoria();
        let (url, _) = servidor(vec![
            DESAFIO,
            RETIRADO,
            DESAFIO,
            NO_VERIFICABLE,
            DESAFIO,
            TOKEN,
        ]);

        core.anotar_autenticacion(&autenticar(&url));
        assert!(core.equipo_retirado(), "403 dispositivo_retirado");
        core.anotar_autenticacion(&autenticar(&url));
        assert!(core.equipo_retirado(), "un 503 no des-retira");
        core.anotar_autenticacion(&autenticar(&url));
        assert!(!core.equipo_retirado(), "la nube lo volvió a aceptar");
    }

    #[test]
    fn cuerpo_invalido_se_reporta_como_error_de_red() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\nesto no es json",
        );

        let resultado = canjear(&base_url);

        assert!(matches!(resultado, Err(NubeError::Red(_))));
    }
}
