use std::path::PathBuf;

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use control_acceso::database::queries::ingresos::{FiltroHistorial, MovimientoIngresoResumen};
use control_acceso::historial::exportacion::ColumnaHistorial;
use control_acceso::tiempo::{self, TiempoError};

use crate::estado::GuiState;
use crate::pdf;

/// Rango de fechas abierto a propósito (año 2000 hasta mañana) — usado como
/// techo para exportar (ver `exportar_historial`) y como valor por defecto
/// de `rango_utc` cuando la pantalla no manda `desde`/`hasta`.
fn filtro_sin_acotar() -> FiltroHistorial {
    let desde = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
    let hasta = Utc::now() + Duration::days(1);
    FiltroHistorial::nuevo(desde, hasta)
}

/// Convierte el rango de fecha (calendario, Costa Rica) que manda la
/// pantalla al rango UTC que espera `FiltroHistorial`. `hasta` es inclusive
/// del día completo — se resuelve como el inicio del día SIGUIENTE, no
/// `hasta` a medianoche (que dejaría afuera todo ese día). `None` en
/// cualquiera de los dos extremos reproduce el rango abierto de siempre
/// (2000-01-01 → mañana), mismo criterio que antes de tener el filtro.
fn rango_utc(
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
) -> Result<(DateTime<Utc>, DateTime<Utc>), TiempoError> {
    let sin_acotar = filtro_sin_acotar();
    let desde_utc = match desde {
        Some(fecha) => tiempo::inicio_dia_costa_rica_utc(fecha)?,
        None => sin_acotar.desde,
    };
    let hasta_utc = match hasta {
        Some(fecha) => tiempo::inicio_dia_costa_rica_utc(fecha + Duration::days(1))?,
        None => sin_acotar.hasta,
    };
    Ok((desde_utc, hasta_utc))
}

/// Todo el historial dentro de `[desde, hasta]` en un solo `Vec`, sin
/// paginar — la grilla (AG Grid) virtualiza del lado del cliente en vez de
/// pedir páginas, a diferencia de Activos/Contratistas. `desde`/`hasta`
/// ausentes traen el historial completo (ver `rango_utc`). Ver
/// `AppCore::buscar_historial_completo`.
#[tauri::command]
pub fn listar_historial(
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    state: tauri::State<GuiState>,
) -> Result<Vec<MovimientoIngresoResumen>, String> {
    state.sesion_activa()?;
    let (desde_utc, hasta_utc) = rango_utc(desde, hasta).map_err(|error| error.to_string())?;
    state
        .core()
        .buscar_historial_completo(&FiltroHistorial::nuevo(desde_utc, hasta_utc))
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

/// Exporta a PDF los mismos `ids`/`columnas` que ya resuelve
/// `exportar_historial` para Excel — mismo criterio de "lo que la grilla
/// tiene visible ahora", pero HTML/CSS renderizado por WebView2
/// (`pdf::generador`) en vez de `rust_xlsxwriter`. `filtro_descripcion` es
/// el texto ya formateado que arma la pantalla (ej. "Filtro: rango de
/// fechas 30/07/2026 – 29/08/2026") — no se recalcula acá para no duplicar
/// el formateo de fechas que ya vive en el frontend. `generado_por` sale de
/// la sesión activa, no del frontend — no hay razón para confiar en un
/// nombre que mande el cliente para algo que es, en la práctica, un dato de
/// auditoría.
#[tauri::command]
pub async fn exportar_historial_pdf(
    destino: String,
    ids: Vec<i64>,
    columnas: Vec<String>,
    filtro_descripcion: String,
    state: tauri::State<'_, GuiState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    let columnas: Vec<ColumnaHistorial> = columnas
        .iter()
        .filter_map(|clave| ColumnaHistorial::from_clave(clave))
        .collect();
    let movimientos = state
        .core()
        .movimientos_en_orden(&filtro_sin_acotar(), &ids)
        .map_err(|error| error.to_string())?;
    let html =
        pdf::html::generar_html(&movimientos, &columnas, &sesion.nombre, &filtro_descripcion);
    pdf::generador::generar_pdf(&app, html, PathBuf::from(destino)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambos_extremos_ausentes_reproduce_el_rango_abierto_de_siempre() {
        let sin_acotar = filtro_sin_acotar();
        let (desde_utc, hasta_utc) = rango_utc(None, None).unwrap();
        assert_eq!(desde_utc, sin_acotar.desde);
        // `hasta` involucra `Utc::now()`, así que comparamos con tolerancia
        // (un segundo alcanza de sobra) en vez de igualdad exacta.
        assert!((hasta_utc - sin_acotar.hasta).num_seconds().abs() < 1);
    }

    #[test]
    fn desde_es_el_inicio_del_dia_y_hasta_incluye_el_dia_completo() {
        let desde = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let hasta = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        let (desde_utc, hasta_utc) = rango_utc(Some(desde), Some(hasta)).unwrap();

        assert_eq!(desde_utc, tiempo::inicio_dia_costa_rica_utc(desde).unwrap());
        // Un movimiento a las 23:59 del 31/03 debe caer ANTES de `hasta_utc`
        // (límite superior exclusivo, ver el campo `hasta` de
        // `FiltroHistorial`) — por eso `hasta_utc` es el inicio del 1/04, no
        // el inicio del 31/03.
        let un_dia_despues = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        assert_eq!(
            hasta_utc,
            tiempo::inicio_dia_costa_rica_utc(un_dia_despues).unwrap()
        );
        assert!(hasta_utc > desde_utc);
    }
}
