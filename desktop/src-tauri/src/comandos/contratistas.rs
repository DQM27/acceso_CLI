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
        .map_err(super::mensaje_generico)
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

/// Lo que el formulario de contratista necesita saber mientras se llena,
/// calculado por el núcleo (`domain::contratista`) -- el frontend no
/// replica ninguna regla. Al guardar, `ContratistaService` las vuelve a
/// aplicar todas.
#[derive(serde::Serialize)]
pub struct ReglasFormularioContratista {
    /// Mostrar y exigir la fecha de vencimiento PRAIND.
    requiere_praind: bool,
    /// Mostrar la casilla "Personal de ruta" (sólo PRAIND / IN HOUSE).
    admite_personal_ruta: bool,
    /// Aviso si la fecha escrita ya venció (regla y reloj del núcleo);
    /// `None` si está vigente o todavía no hay fecha.
    aviso_praind: Option<String>,
}

/// Los tipos de ingreso que se pueden elegir para un contratista, en orden.
/// Los decide el núcleo (`tipo_ingreso_seleccionable`): "Por correo" no está
/// porque el núcleo lo retiró, no porque la pantalla lo filtre.
#[tauri::command]
pub fn tipos_ingreso_seleccionables(
    state: tauri::State<GuiState>,
) -> Result<Vec<control_acceso::models::tipo_ingreso::TipoIngreso>, String> {
    use control_acceso::domain::contratista::tipo_ingreso_seleccionable;
    use control_acceso::models::tipo_ingreso::TipoIngreso;
    state.sesion_activa()?;
    Ok(TipoIngreso::ALL
        .into_iter()
        .filter(|tipo| tipo_ingreso_seleccionable(*tipo))
        .collect())
}

#[tauri::command]
pub fn reglas_formulario_contratista(
    tipo_ingreso: control_acceso::models::tipo_ingreso::TipoIngreso,
    es_personal_ruta: bool,
    fecha_vencimiento_praind: Option<chrono::NaiveDate>,
    state: tauri::State<GuiState>,
) -> Result<ReglasFormularioContratista, String> {
    use control_acceso::domain::contratista::{admite_personal_ruta, requiere_praind_de};
    state.sesion_activa()?;
    let admite = admite_personal_ruta(tipo_ingreso);
    let requiere = requiere_praind_de(tipo_ingreso, es_personal_ruta && admite);
    Ok(ReglasFormularioContratista {
        requiere_praind: requiere,
        admite_personal_ruta: admite,
        aviso_praind: (requiere
            && fecha_vencimiento_praind.is_some_and(|fecha| state.core().praind_vencido(fecha)))
        .then(|| {
            control_acceso::mensajes::mensaje_contratista(
                control_acceso::services::error::ContratistaServiceError::PraindVencido,
            )
        }),
    })
}
