//! Puente `uniffi` sobre `control_acceso`: expone al Kotlin de la app móvil
//! sólo lo puntual que cada pantalla necesita, sin tocar la lógica del
//! crate raíz. Ver docs/plan-app-movil.md.

uniffi::setup_scaffolding!();

use control_acceso::application::AppCore;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum NucleoError {
    #[error("no se pudo abrir la base de datos: {mensaje}")]
    Apertura { mensaje: String },
}

/// Primera prueba de vida del puente: abre el núcleo real de Rust contra una
/// base de datos SQLite existente y confirma que el enlace Kotlin<->Rust
/// funciona de punta a punta antes de escribir cualquier pantalla.
#[uniffi::export]
pub fn abrir_nucleo(ruta_base_datos: String) -> Result<String, NucleoError> {
    AppCore::abrir(&ruta_base_datos).map_err(|origen| NucleoError::Apertura {
        mensaje: origen.to_string(),
    })?;
    Ok("núcleo abierto correctamente".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abre_una_base_de_datos_temporal() {
        let archivo = tempfile::NamedTempFile::new().unwrap();
        let ruta = archivo.path().to_str().unwrap().to_string();

        let resultado = abrir_nucleo(ruta);

        assert!(resultado.is_ok());
    }
}
