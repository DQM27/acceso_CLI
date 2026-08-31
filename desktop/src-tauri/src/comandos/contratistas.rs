use control_acceso::database::queries::contratistas::PaginaContratistas;

use crate::dto::contratistas::{DatosContratistaEntrada, FiltroContratistasEntrada};
use crate::estado::GuiState;

/// El núcleo no exige sesión para esta lectura (ver
/// `application::catalogos::buscar_contratistas`), pero la GUI sí la exige
/// acá: a diferencia de la TUI, donde la navegación es la barrera, cualquier
/// pantalla del webview puede invocar este comando directamente, así que el
/// chequeo tiene que vivir en el comando mismo.
///
/// `filtro.texto` es lo único que sigue viajando desde el frontend — lo usan
/// el buscador en vivo de `NuevoIngresoModal`/`GestionGafeteModal`, con
/// `filtro: {}` (sin texto) para la grilla de Contratistas, que ya no manda
/// cédula/empresa/tipo/PRAIND/etc.: carga el universo completo una sola vez
/// y filtra/ordena del lado del cliente (ver
/// `desktop/src/pantallas/Contratistas.tsx`). Ninguno de los dos casos pagina
/// — `FiltroContratistasEntrada::construir` siempre pide `limite: usize::MAX`.
#[tauri::command]
pub fn buscar_contratistas(
    filtro: FiltroContratistasEntrada,
    state: tauri::State<GuiState>,
) -> Result<PaginaContratistas, String> {
    state.sesion_activa()?;
    state
        .core()
        .buscar_contratistas(&filtro.construir())
        .map_err(|error| error.to_string())
}

/// Formulario de alta — usa el mismo DTO que editar (ver dto/contratistas.rs).
#[tauri::command]
pub fn crear_contratista(
    datos: DatosContratistaEntrada,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
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
    let sesion = state.sesion_activa()?;
    state
        .core()
        .actualizar_contratista(&sesion, id, datos.into())
        .map_err(control_acceso::mensajes::mensaje_contratista)
}
