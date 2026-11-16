//! Carnés PRAIND de contratistas (ver
//! `docs/features-futuras/plan-fotos-praind.md`, fase 3).
//!
//! La lógica (base local o descarga, carga por QR o archivo) vive en
//! `control_acceso::nube::praind`, compartida con el móvil. Acá sólo se
//! consigue el token del equipo, que nunca sale de Rust.

use control_acceso::nube;
use control_acceso::nube::praind::{CargaPraind, ErrorPraind, PermisosPraind};

use crate::estado::GuiState;

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

#[tauri::command(async)]
pub fn estado_carne_praind(
    contratista_id: i64,
    state: tauri::State<GuiState>,
) -> Result<EstadoCarne, String> {
    state.sesion_activa()?;
    let carne = nube::praind::estado_carne(&state.conexion_secundaria()?, contratista_id)?;
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
    let carne = nube::praind::ver_carne(&state.conexion_secundaria()?, contratista_id, || {
        token(&state)
    })?;
    Ok(tauri::ipc::Response::new(carne.bytes))
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
    nube::praind::cargar_carne_desde_qr(
        &state.conexion_secundaria()?,
        contratista_id,
        &enlace,
        solo_previsualizar,
        || token(&state),
    )
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
    nube::praind::cargar_carne_archivo(
        &state.conexion_secundaria()?,
        contratista_id,
        bytes.clone(),
        || token(&state),
    )
}
