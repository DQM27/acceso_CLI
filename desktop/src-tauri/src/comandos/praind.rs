//! Carnés PRAIND de contratistas (ver
//! `docs/features-futuras/plan-fotos-praind.md`, fase 3).
//!
//! Ver un carné: si ya se abrió en este equipo, sale de la base local (al
//! instante y sin internet); si no, se baja del Worker y queda guardado. Nada
//! se baja por adelantado. El token del equipo nunca sale de Rust.

use control_acceso::database::queries::carnes_praind;
use control_acceso::nube;
use control_acceso::nube::praind::{CargaPraind, ErrorPraind, PermisosPraind};
use rusqlite::params;

use crate::estado::GuiState;

const SIN_INTERNET: &str = "Para ver este carné por primera vez, conectate a internet. \
     Después de verlo una vez queda guardado en este equipo.";
const SIN_NUBE: &str = "Este equipo no está conectado a la nube.";

/// Lo que la vista de detalles necesita saber antes de mostrar el carné.
#[derive(serde::Serialize)]
pub struct EstadoCarne {
    /// Ruta del carné vigente, o `None` si el contratista no tiene.
    pub clave: Option<String>,
    /// Si ya está guardado en este equipo (se ve sin internet).
    pub guardado: bool,
    /// Si el contratista ya existe en la nube (hace falta para cargar).
    pub sincronizado: bool,
}

/// Permisos según la nube. Sin conexión o sin nube, todo en falso: la
/// pantalla sólo deja de ofrecer "Cargar carné"; ver lo guardado no lo usa.
#[derive(serde::Serialize)]
pub struct PermisosCarne {
    #[serde(flatten)]
    pub permisos: PermisosPraind,
    pub sin_conexion: bool,
}

fn ahora() -> String {
    control_acceso::tiempo::serializar_utc(chrono::Utc::now())
}

fn token(state: &GuiState) -> Result<String, ErrorPraind> {
    if !state.nube_vinculada() {
        return Err(ErrorPraind::Rechazo {
            estado: 0,
            mensaje: SIN_NUBE.to_string(),
        });
    }
    let token = state.autenticar_con_cache().map_err(|error| {
        if error.es_falta_de_conexion() {
            ErrorPraind::SinConexion
        } else {
            ErrorPraind::Rechazo {
                estado: 401,
                mensaje: error.to_string(),
            }
        }
    })?;
    state.core().aplicar_token(&token);
    Ok(token.access_token)
}

fn estado_de(
    state: &GuiState,
    contratista_id: i64,
) -> Result<carnes_praind::CarneDeContratista, String> {
    let conexion = state.conexion_secundaria()?;
    carnes_praind::carne_de_contratista(&conexion, contratista_id)
        .map_err(super::mensaje_generico)?
        .ok_or_else(|| "El contratista no existe.".to_string())
}

#[tauri::command(async)]
pub fn estado_carne_praind(
    contratista_id: i64,
    state: tauri::State<GuiState>,
) -> Result<EstadoCarne, String> {
    state.sesion_activa()?;
    let carne = estado_de(&state, contratista_id)?;
    Ok(EstadoCarne {
        clave: carne.clave,
        guardado: carne.guardado,
        sincronizado: carne.uuid.is_some(),
    })
}

#[tauri::command(async)]
pub fn permisos_carne_praind(state: tauri::State<GuiState>) -> Result<PermisosCarne, String> {
    state.sesion_activa()?;
    let resultado = token(&state)
        .and_then(|token| nube::praind::permisos(nube::base_url(), nube::apikey(), &token));
    Ok(match resultado {
        Ok(permisos) => PermisosCarne {
            permisos,
            sin_conexion: false,
        },
        Err(error) => PermisosCarne {
            permisos: PermisosPraind::default(),
            sin_conexion: error.es_sin_conexion(),
        },
    })
}

/// Devuelve los bytes del carné (binario, no JSON). El tipo sale de la
/// extensión de la ruta, que la pantalla ya conoce.
#[tauri::command(async)]
pub fn ver_carne_praind(
    contratista_id: i64,
    state: tauri::State<GuiState>,
) -> Result<tauri::ipc::Response, String> {
    state.sesion_activa()?;
    let carne = estado_de(&state, contratista_id)?;
    let Some(clave) = carne.clave else {
        return Err("Este contratista no tiene carné PRAIND cargado.".to_string());
    };

    let conexion = state.conexion_secundaria()?;
    if let Some(guardado) = carnes_praind::abrir_guardado(&conexion, &clave, &ahora())
        .map_err(super::mensaje_generico)?
    {
        return Ok(tauri::ipc::Response::new(guardado.bytes));
    }

    let descargado = token(&state)
        .and_then(|token| nube::praind::descargar(&token, &clave))
        .map_err(|error| {
            if error.es_sin_conexion() {
                SIN_INTERNET.to_string()
            } else {
                error.to_string()
            }
        })?;
    // Si no se pudo guardar, igual se muestra: la próxima vez se vuelve a bajar.
    if let Err(error) = carnes_praind::guardar(
        &conexion,
        &clave,
        &descargado.tipo_mime,
        &descargado.bytes,
        &ahora(),
    ) {
        log::warn!("no se pudo guardar el carné PRAIND en el equipo: {error}");
    }
    Ok(tauri::ipc::Response::new(descargado.bytes))
}

fn uuid_para_cargar(state: &GuiState, contratista_id: i64) -> Result<String, String> {
    estado_de(state, contratista_id)?.uuid.ok_or_else(|| {
        "Este contratista todavía no llegó a la nube. Esperá a que sincronice y volvé a intentar.".to_string()
    })
}

/// Anota la ruta nueva en la base local sin esperar a la sincronización (que
/// después trae la misma).
fn anotar_clave(state: &GuiState, contratista_id: i64, clave: &str) -> Result<(), String> {
    let conexion = state.conexion_secundaria()?;
    conexion
        .execute(
            "UPDATE contratistas SET praind_foto = ?2 WHERE id = ?1",
            params![contratista_id, clave],
        )
        .map_err(super::mensaje_generico)?;
    Ok(())
}

/// Carga el carné desde el enlace del QR. Con `solo_previsualizar` no guarda
/// nada: devuelve el nombre de la página para compararlo antes.
#[tauri::command(async)]
pub fn cargar_carne_praind_desde_qr(
    contratista_id: i64,
    enlace: String,
    solo_previsualizar: bool,
    state: tauri::State<GuiState>,
) -> Result<CargaPraind, String> {
    state.sesion_activa()?;
    let uuid = uuid_para_cargar(&state, contratista_id)?;
    let carga = token(&state)
        .and_then(|token| {
            nube::praind::cargar_desde_qr(&token, &uuid, enlace.trim(), solo_previsualizar)
        })
        .map_err(|error| error.to_string())?;
    if let Some(clave) = carga.clave.as_deref() {
        anotar_clave(&state, contratista_id, clave)?;
    }
    Ok(carga)
}

/// Carga la imagen del carné. Llega en binario (cuerpo crudo del pedido) con
/// el contratista en la cabecera `x-contratista-id`.
#[tauri::command(async)]
pub fn cargar_carne_praind_archivo(
    request: tauri::ipc::Request<'_>,
    state: tauri::State<'_, GuiState>,
) -> Result<CargaPraind, String> {
    state.sesion_activa()?;
    let contratista_id: i64 = request
        .headers()
        .get("x-contratista-id")
        .and_then(|valor| valor.to_str().ok())
        .and_then(|valor| valor.parse().ok())
        .ok_or_else(|| "Falta el contratista.".to_string())?;
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("El archivo no llegó.".to_string());
    };
    let bytes = bytes.clone();
    let uuid = uuid_para_cargar(&state, contratista_id)?;
    let carga = token(&state)
        .and_then(|token| nube::praind::cargar_archivo(&token, &uuid, bytes.clone()))
        .map_err(|error| error.to_string())?;
    if let Some(clave) = carga.clave.as_deref() {
        anotar_clave(&state, contratista_id, clave)?;
        // Ya lo tenemos: queda guardado para verlo sin bajarlo de nuevo.
        let conexion = state.conexion_secundaria()?;
        if let Err(error) =
            carnes_praind::guardar(&conexion, clave, &carga.tipo_mime, &bytes, &ahora())
        {
            log::warn!("no se pudo guardar el carné PRAIND en el equipo: {error}");
        }
    }
    Ok(carga)
}
