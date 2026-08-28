use control_acceso::database::queries::empresas::EmpresaResumen;
use control_acceso::models::empresa::Empresa;

use crate::dto::empresas::FiltroEmpresasEntrada;
use crate::estado::GuiState;

/// Lista completa sin filtro — usada por los desplegables de "Empresa" en
/// otras pantallas (Contratistas hoy). El núcleo no exige sesión para esta
/// lectura, pero la GUI sí la exige acá — ver el comentario equivalente en
/// `comandos::contratistas::buscar_contratistas`.
#[tauri::command]
pub fn listar_empresas(state: tauri::State<GuiState>) -> Result<Vec<Empresa>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_empresas()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn buscar_empresas(
    filtro: FiltroEmpresasEntrada,
    state: tauri::State<GuiState>,
) -> Result<Vec<EmpresaResumen>, String> {
    state.sesion_activa()?;
    state
        .core()
        .buscar_empresas(&filtro.construir())
        .map_err(control_acceso::mensajes::mensaje_empresa)
}

#[tauri::command]
pub fn crear_empresa(nombre: String, state: tauri::State<GuiState>) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_empresa(&sesion, &nombre)
        .map_err(control_acceso::mensajes::mensaje_empresa)
}

#[tauri::command]
pub fn actualizar_empresa(
    id: i64,
    nombre: String,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .actualizar_empresa(&sesion, id, &nombre)
        .map_err(control_acceso::mensajes::mensaje_empresa)
}

#[tauri::command]
pub fn establecer_empresa_activa(
    id: i64,
    activa: bool,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    let core = state.core();
    let resultado = if activa {
        core.activar_empresa(&sesion, id)
    } else {
        core.desactivar_empresa(&sesion, id)
    };
    resultado.map_err(control_acceso::mensajes::mensaje_empresa)
}
