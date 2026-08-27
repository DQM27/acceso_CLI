use control_acceso::database::queries::contratistas::PaginaContratistas;

use crate::dto::{DatosContratistaEntrada, FiltroContratistasEntrada};
use crate::estado::GuiState;

#[tauri::command]
pub fn buscar_contratistas(
    filtro: FiltroContratistasEntrada,
    state: tauri::State<GuiState>,
) -> Result<PaginaContratistas, String> {
    state
        .core
        .lock()
        .unwrap()
        .buscar_contratistas(&filtro.construir())
        .map_err(|error| error.to_string())
}

fn sesion_activa(state: &GuiState) -> Result<control_acceso::services::autenticacion_service::UsuarioSesion, String> {
    state
        .sesion
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "No hay una sesión activa".to_string())
}

/// Formulario de alta — usa el mismo DTO que editar (ver dto.rs).
#[tauri::command]
pub fn crear_contratista(
    datos: DatosContratistaEntrada,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = sesion_activa(&state)?;
    state
        .core
        .lock()
        .unwrap()
        .crear_contratista(&sesion, datos.into())
        .map_err(control_acceso::mensajes::mensaje_contratista)
}

/// Cubre tanto el formulario completo (crear/editar) como el toggle rápido de
/// "es de ruta"/"tiene acceso" desde la grilla. El core exige el registro
/// completo (`DatosActualizacionContratista` reemplaza, no aplica un parche).
#[tauri::command]
pub fn actualizar_contratista(
    id: i64,
    datos: DatosContratistaEntrada,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = sesion_activa(&state)?;
    state
        .core
        .lock()
        .unwrap()
        .actualizar_contratista(&sesion, id, datos.into())
        .map_err(control_acceso::mensajes::mensaje_contratista)
}
