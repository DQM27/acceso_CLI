use control_acceso::models::empresa::Empresa;

use crate::estado::GuiState;

#[tauri::command]
pub fn listar_empresas(state: tauri::State<GuiState>) -> Result<Vec<Empresa>, String> {
    state
        .core
        .lock()
        .unwrap()
        .listar_empresas()
        .map_err(|error| error.to_string())
}
