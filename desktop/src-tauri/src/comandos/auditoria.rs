use control_acceso::application::{CargaCompleta, buscar_auditoria_completo_con_conexion};
use control_acceso::database::queries::auditoria::CambioAuditado;
use control_acceso::database::queries::gafetes_incidentes::IncidenteGafete;

use crate::estado::GuiState;

/// Todo el conjunto de auditoría (contratistas, empresas y usuarios) en un
/// solo `Vec`, sin paginar — la grilla (AG Grid) virtualiza del lado del
/// cliente, igual criterio que Historial (`comandos::historial::listar_historial`).
/// El núcleo exige sesión (valida `Operacion::VerAuditoria` contra el rol
/// del actor); no hace falta un chequeo redundante acá salvo `sesion_activa`
/// para no llamar al núcleo sin sesión.
#[tauri::command]
pub fn listar_auditoria(
    state: tauri::State<GuiState>,
) -> Result<CargaCompleta<CambioAuditado>, String> {
    let sesion = state.sesion_activa()?;
    // Conexión propia (ver `GuiState::conexion_secundaria`): esta consulta
    // puede tardar ~750ms con auditoría grande (`docs/pendientes.md`, tope
    // `LIMITE_CARGA_COMPLETA_MAXIMO`) — retener acá el `Mutex<AppCore>`
    // compartido dejaría sin núcleo a cualquier otro comando mientras dura.
    let conexion = state.conexion_secundaria()?;
    buscar_auditoria_completo_con_conexion(&conexion, &sesion)
        .map_err(control_acceso::mensajes::mensaje_contratista)
}

/// Incidentes de gafetes (marcar perdido/resolver) para la misma pantalla —
/// tabla aparte (`gafetes_incidentes`), mismo gate (`Operacion::VerAuditoria`).
#[tauri::command]
pub fn listar_auditoria_gafetes(
    state: tauri::State<GuiState>,
) -> Result<Vec<IncidenteGafete>, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .buscar_auditoria_gafetes(&sesion)
        .map_err(control_acceso::mensajes::mensaje_contratista)
}
