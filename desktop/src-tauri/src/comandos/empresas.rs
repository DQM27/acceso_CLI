use control_acceso::database::queries::empresas::EmpresaResumen;
use control_acceso::models::empresa::Empresa;

use crate::dto::empresas::FiltroEmpresasEntrada;
use crate::estado::GuiState;

/// Para la grilla de administración -- trae activas e inactivas.
/// `listar_empresas_seleccionables` es la contraparte para el desplegable de
/// "Empresa" al crear/editar un contratista (`FormularioContratista.tsx`),
/// donde una empresa desactivada nunca es una opción válida. El núcleo no
/// exige sesión para esta lectura, pero la GUI sí la exige acá — ver el
/// comentario equivalente en `comandos::contratistas::buscar_contratistas`.
#[tauri::command(async)]
pub fn listar_empresas(state: tauri::State<GuiState>) -> Result<Vec<Empresa>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_empresas()
        .map_err(super::mensaje_generico)
}

/// Para un selector (el desplegable de "Empresa" al crear/editar un
/// contratista) -- una empresa desactivada nunca es una opción válida, ver
/// `EmpresaService::listar_seleccionables`.
#[tauri::command(async)]
pub fn listar_empresas_seleccionables(
    state: tauri::State<GuiState>,
) -> Result<Vec<Empresa>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_empresas_seleccionables()
        .map_err(super::mensaje_generico)
}

#[tauri::command(async)]
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

#[tauri::command(async)]
pub fn crear_empresa(nombre: String, state: tauri::State<GuiState>) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_empresa(&sesion, &nombre)
        .map_err(control_acceso::mensajes::mensaje_empresa)
}

#[tauri::command(async)]
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

// Sin comando para activar/desactivar una empresa: desde el escritorio nadie
// lo hace, es sólo del panel web (decisión del dueño 2026-10-24).
