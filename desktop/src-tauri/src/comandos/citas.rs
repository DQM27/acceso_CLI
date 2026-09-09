use control_acceso::mensajes::mensaje_cita;
use control_acceso::models::cita::{Cita, CitaVisitante};
use control_acceso::models::movimiento_visita::MovimientoVisitaActivoResumen;

use crate::estado::GuiState;

/// DTO de presentación -- `AppCore::verificar_check_in_visita` devuelve una
/// tupla `(Cita, CitaVisitante)`; acá se nombra para que el lado TypeScript
/// tenga campos, no un array posicional.
#[derive(serde::Serialize)]
pub struct PreparacionVisita {
    pub cita: Cita,
    pub visitante: CitaVisitante,
}

#[tauri::command]
pub fn verificar_check_in_visita(
    cedula: String,
    state: tauri::State<GuiState>,
) -> Result<PreparacionVisita, String> {
    state.sesion_activa()?;
    let (cita, visitante) = state
        .core()
        .verificar_check_in_visita(&cedula)
        .map_err(mensaje_cita)?;
    Ok(PreparacionVisita { cita, visitante })
}

#[tauri::command]
pub fn registrar_entrada_visita(
    cedula: String,
    gafete: Option<i64>,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_entrada_visita(&sesion, &cedula, gafete)
        .map_err(mensaje_cita)
}

#[tauri::command]
pub fn registrar_salida_visita(
    movimiento_id: i64,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_salida_visita(&sesion, movimiento_id)
        .map_err(mensaje_cita)
}

#[tauri::command]
pub fn listar_visitas_activas(
    state: tauri::State<GuiState>,
) -> Result<Vec<MovimientoVisitaActivoResumen>, String> {
    state.sesion_activa()?;
    // Sin `mensaje_*` propio a propósito: `AppCore::listar_visitas_activas`
    // devuelve `DatabaseError` directo (es una lectura simple, no pasa por
    // un `*ServiceError` de negocio) -- no exponer su `Display` (interpola
    // el error crudo de `SQLite`), mismo criterio que el resto de mensajes
    // de este módulo.
    state
        .core()
        .listar_visitas_activas()
        .map_err(|_| "No se pudo cargar la lista de visitas activas".to_string())
}
