use std::path::PathBuf;

use control_acceso::application::exportar_tabla_xlsx as exportar_tabla_xlsx_nucleo;
use control_acceso::historial::exportacion::ColumnaTabla;

use crate::comandos::historial::RespaldoDestino;
use crate::estado::GuiState;
use crate::pdf;

/// BOM de UTF-8: sin él, Excel abre el CSV como ANSI y rompe tildes/eñes
/// ("Cédula" → "CÃ©dula").
const BOM_UTF8: &str = "\u{feff}";

/// Contenido final del archivo: el CSV tal cual lo armó la grilla (AG Grid,
/// `getDataAsCsv`), con el BOM adelante si no lo trae ya.
fn con_bom(contenido: &str) -> String {
    if contenido.starts_with(BOM_UTF8) {
        contenido.to_owned()
    } else {
        format!("{BOM_UTF8}{contenido}")
    }
}

/// Guarda en `destino` el CSV que armó la grilla del lado del cliente (ver
/// `exportarCsv` en `Tabla.tsx`). El frontend no tiene permiso de escribir
/// archivos por su cuenta; la ruta ya la eligió la persona en el diálogo
/// nativo de "Guardar como", que también preguntó si reemplazar un archivo
/// existente, así que acá se escribe directo.
#[tauri::command]
pub fn guardar_csv(
    destino: String,
    contenido: String,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    state.sesion_activa()?;
    std::fs::write(&destino, con_bom(&contenido)).map_err(super::mensaje_generico)
}

/// Exporta a XLSX una tabla que armó la grilla (ver `datosVisibles` en
/// `Tabla.tsx`): columnas visibles, filas filtradas y ordenadas, valores
/// ya formateados. Para las grillas sin exportador propio (historial de
/// proveedores y de KOF); mismo estilo que el Excel de Historial. El
/// destino ya lo confirmó la persona en "Guardar como" -- se aparta el
/// existente igual que en `exportar_historial`.
#[tauri::command]
pub fn exportar_tabla_xlsx(
    destino: String,
    columnas: Vec<ColumnaTabla>,
    filas: Vec<Vec<String>>,
    state: tauri::State<GuiState>,
) -> Result<usize, String> {
    state.sesion_activa()?;
    let destino = PathBuf::from(destino);
    let respaldo = RespaldoDestino::apartar(&destino)?;
    let resultado =
        exportar_tabla_xlsx_nucleo(&columnas, &filas, &destino).map_err(super::mensaje_generico);
    if resultado.is_ok() {
        respaldo.confirmar();
    }
    resultado
}

/// Igual que [`exportar_tabla_xlsx`], a PDF (`pdf::html::generar_html_tabla`,
/// mismo documento que el PDF de Historial). `generado_por` sale de la
/// sesión, no del cliente -- mismo criterio que `exportar_historial_pdf`.
#[tauri::command]
pub async fn exportar_tabla_pdf(
    destino: String,
    titulo: String,
    filtro_descripcion: String,
    columnas: Vec<ColumnaTabla>,
    filas: Vec<Vec<String>>,
    state: tauri::State<'_, GuiState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    if columnas.is_empty() {
        return Err("Seleccione al menos una columna".to_owned());
    }
    let html = pdf::html::generar_html_tabla(
        &titulo,
        &columnas,
        &filas,
        &sesion.nombre,
        &filtro_descripcion,
    );
    let destino = PathBuf::from(destino);
    let respaldo = RespaldoDestino::apartar(&destino)?;
    let resultado = pdf::generador::generar_pdf(&app, html, destino).await;
    if resultado.is_ok() {
        respaldo.confirmar();
    }
    resultado
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agrega_el_bom_una_sola_vez() {
        assert_eq!(con_bom("a;b"), "\u{feff}a;b");
        assert_eq!(con_bom("\u{feff}a;b"), "\u{feff}a;b");
    }
}
