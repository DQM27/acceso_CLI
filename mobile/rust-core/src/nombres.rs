//! La regla de los nombres (todo nombre de persona o empresa en mayúscula)
//! para Kotlin: las pantallas la llaman mientras se escribe en vez de pasar
//! a mayúscula por su cuenta. La que manda al guardar es la del núcleo.

/// Mientras se escribe un nombre: en mayúscula, sin tocar los espacios.
#[uniffi::export]
pub fn nombre_mientras_se_escribe(texto: String) -> String {
    control_acceso::texto::nombre_mientras_se_escribe(&texto)
}

/// El nombre como lo guarda el núcleo: espacios de más fuera y en mayúscula.
#[uniffi::export]
pub fn nombre_en_mayusculas(texto: String) -> String {
    control_acceso::texto::nombre_en_mayusculas(&texto)
}
