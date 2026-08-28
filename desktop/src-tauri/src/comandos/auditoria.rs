use control_acceso::database::queries::auditoria::CambioAuditado;

use crate::estado::GuiState;

/// Todo el conjunto de auditoría (contratistas, empresas y usuarios) en un
/// solo `Vec`, sin paginar — la grilla (AG Grid) virtualiza del lado del
/// cliente, igual criterio que Historial (`comandos::historial::listar_historial`).
/// El núcleo exige sesión (valida `Operacion::VerAuditoria` contra el rol
/// del actor); no hace falta un chequeo redundante acá salvo `sesion_activa`
/// para no llamar al núcleo sin sesión.
#[tauri::command]
pub fn listar_auditoria(state: tauri::State<GuiState>) -> Result<Vec<CambioAuditado>, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .buscar_auditoria_completo(&sesion)
        .map_err(control_acceso::mensajes::mensaje_contratista)
}
