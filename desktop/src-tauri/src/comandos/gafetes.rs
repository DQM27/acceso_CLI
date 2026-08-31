use control_acceso::database::queries::gafetes::GafeteResumen;
use control_acceso::database::queries::gafetes_incidentes::IncidenteGafete;
use control_acceso::models::gafete::MotivoResolucionGafete;

use crate::dto::gafetes::FiltroGafetesEntrada;
use crate::estado::GuiState;

/// Catálogo completo de gafetes, sin restricción de rol a propósito (mismo
/// criterio que el núcleo — cualquier operador con sesión gestiona
/// alta/baja/perdido/resolver, a diferencia de Empresas/Usuarios).
#[tauri::command]
pub fn buscar_gafetes(
    filtro: FiltroGafetesEntrada,
    state: tauri::State<GuiState>,
) -> Result<Vec<GafeteResumen>, String> {
    state.sesion_activa()?;
    state
        .core()
        .buscar_gafetes(&filtro.construir())
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

/// Historial de un gafete puntual — misma falta de restricción de sesión
/// que `buscar_gafetes` (lectura del catálogo).
#[tauri::command]
pub fn historial_gafete(
    id: i64,
    state: tauri::State<GuiState>,
) -> Result<Vec<IncidenteGafete>, String> {
    state
        .core()
        .historial_gafete(id)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command]
pub fn crear_gafete(numero: i64, state: tauri::State<GuiState>) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_gafete(&sesion, numero)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command]
pub fn crear_gafetes_rango(
    desde: i64,
    hasta: i64,
    state: tauri::State<GuiState>,
) -> Result<Vec<i64>, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_gafetes_rango(&sesion, desde, hasta)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command]
pub fn dar_de_baja_gafete(id: i64, state: tauri::State<GuiState>) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .dar_de_baja_gafete(&sesion, id)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command]
pub fn marcar_gafete_perdido(
    id: i64,
    contratista_id: i64,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .marcar_gafete_perdido(&sesion, id, contratista_id)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command]
pub fn resolver_gafete(
    id: i64,
    motivo: MotivoResolucionGafete,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .resolver_gafete(&sesion, id, motivo)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}
