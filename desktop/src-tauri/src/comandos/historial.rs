use std::path::PathBuf;

use chrono::{Duration, TimeZone, Utc};
use control_acceso::database::queries::ingresos::{FiltroHistorial, MovimientoIngresoResumen};
use control_acceso::historial::exportacion::ColumnaHistorial;

use crate::estado::GuiState;

/// Rango de fechas abierto a propósito (año 2000 hasta mañana) — sin filtro
/// de fecha en esta primera versión de la pantalla (ver
/// `desktop/docs/pendientes.md`). `FiltroHistorial` lo exige siempre (no es
/// `Option`), así que hace falta un valor; AG Grid filtra/virtualiza del
/// lado del cliente, no hace falta acotar la consulta para que sea rápida.
fn filtro_sin_acotar() -> FiltroHistorial {
    let desde = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
    let hasta = Utc::now() + Duration::days(1);
    FiltroHistorial::nuevo(desde, hasta)
}

/// Todo el historial en un solo `Vec`, sin paginar — la grilla (AG Grid)
/// virtualiza del lado del cliente en vez de pedir páginas, a diferencia de
/// Activos/Contratistas. Ver `AppCore::buscar_historial_completo`.
#[tauri::command]
pub fn listar_historial(
    state: tauri::State<GuiState>,
) -> Result<Vec<MovimientoIngresoResumen>, String> {
    state.sesion_activa()?;
    state
        .core()
        .buscar_historial_completo(&filtro_sin_acotar())
        .map_err(|error| error.to_string())
}

/// Exporta el historial a un XLSX en `destino`, recortado a los
/// `registro_id` de `ids` (EN ESE ORDEN — el orden visible en la grilla
/// tras un reordenamiento de columna, no el cronológico de la consulta) y
/// sólo con las `columnas` pedidas (sus claves, ver
/// `ColumnaHistorial::clave`) — la grilla (AG Grid) filtra filas, las
/// ordena y oculta columnas del lado del cliente, `AppCore` no conoce nada
/// de eso, así que la pantalla manda exactamente lo que tiene visible en
/// ese momento en vez de siempre exportar todo en el orden de la consulta.
/// Una clave que no matchea ninguna columna se ignora en silencio (mismo
/// criterio que `SelectorColumnas::aplicar_preferencia`).
#[tauri::command]
pub fn exportar_historial(
    destino: String,
    ids: Vec<i64>,
    columnas: Vec<String>,
    state: tauri::State<GuiState>,
) -> Result<usize, String> {
    state.sesion_activa()?;
    let columnas: Vec<ColumnaHistorial> = columnas
        .iter()
        .filter_map(|clave| ColumnaHistorial::from_clave(clave))
        .collect();
    state
        .core()
        .exportar_historial_seleccion(
            &filtro_sin_acotar(),
            Some(&ids),
            &columnas,
            &PathBuf::from(destino),
        )
        .map_err(|error| error.to_string())
}
