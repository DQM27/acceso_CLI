use crate::estado::GuiState;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agrega_el_bom_una_sola_vez() {
        assert_eq!(con_bom("a;b"), "\u{feff}a;b");
        assert_eq!(con_bom("\u{feff}a;b"), "\u{feff}a;b");
    }
}
