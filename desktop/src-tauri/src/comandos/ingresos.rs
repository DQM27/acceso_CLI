use control_acceso::database::queries::ingresos::FiltroIngresosActivos;
use control_acceso::mensajes::{mensaje_ingreso, mensaje_salida};
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::services::registro_ingreso_service::{
    ListaIngresosActivosResumen, PreparacionIngreso, ResultadoRegistroEntrada,
};

use crate::estado::GuiState;

/// Sin filtro de entrada a propósito: la grilla de Activos filtra en el
/// cliente (columnas de AG Grid) sobre esta misma lista, no repite la
/// consulta contra `SQLite` por cada tecla — a diferencia de Contratistas,
/// que sí filtra en el servidor porque su universo no cabe entero en
/// memoria del lado del webview.
#[tauri::command]
pub fn listar_ingresos_activos(
    state: tauri::State<GuiState>,
) -> Result<ListaIngresosActivosResumen, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_ingresos_activos(&FiltroIngresosActivos::default())
        .map_err(mensaje_ingreso)
}

#[tauri::command]
pub fn preparar_ingreso(
    contratista_id: i64,
    state: tauri::State<GuiState>,
) -> Result<PreparacionIngreso, String> {
    state.sesion_activa()?;
    state
        .core()
        .preparar_ingreso(contratista_id)
        .map_err(mensaje_ingreso)
}

#[tauri::command]
pub fn registrar_ingreso(
    contratista_id: i64,
    medio: MedioIngreso,
    gafete: Option<i64>,
    state: tauri::State<GuiState>,
) -> Result<ResultadoRegistroEntrada, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_ingreso(&sesion, contratista_id, medio, gafete)
        .map_err(mensaje_ingreso)
}

#[tauri::command]
pub fn registrar_salida(id: i64, state: tauri::State<GuiState>) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_salida(&sesion, id)
        .map_err(mensaje_salida)
}
