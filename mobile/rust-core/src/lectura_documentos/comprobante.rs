//! "Comprobante de Carga de Ruta" (Coca-Cola FEMSA). No identifica a una
//! persona: su forma (ruta, sub-número, documento) no tiene nada en común
//! con un documento de identidad. Basado en 4 fotos reales (2026-09-15).
//! "Fecha de Entrega" se completa a mano en el formulario, no por OCR.

use std::sync::LazyLock;

use regex::Regex;

use super::texto::patron;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ComprobanteRutaDetectado {
    /// Código antes de la barra en "Ruta / No.de Carga:" (ej. `CRR079`).
    pub numero_ruta: String,
    /// Sub-número tras la barra, sin ceros a la izquierda: `1` es la ruta
    /// principal; `2`, `3`... las recargas (H2, H3...).
    pub sub_numero: i32,
    /// Campo "Transporte:": el número de documento de esa carga.
    pub numero_documento: String,
}

// "Ruta / No.de Carga:" es la señal de clasificación (sólo la trae este
// comprobante). Ruta real vista: `CRR079`; se admite 2-6 letras + 2-6
// dígitos sin atarse a un solo formato.
static RUTA_NUMERO_CARGA: LazyLock<Regex> = LazyLock::new(|| {
    patron(r"(?i)Ruta\s*/\s*No\.?\s*de\s*Carga:?\s*([A-Z]{2,6}[0-9]{2,6})\s*/\s*0*([0-9]+)")
});
// Como máximo UN salto de línea entre "Transporte:" y su valor: con
// ninguno el A25 no reconocía nada; con cualquier cantidad saltaba hasta la
// tabla de materiales. Mínimo 7 dígitos: el código de material es SIEMPRE
// de 6 (`164145`) y el transporte de 9 en las 4 fotos reales.
static TRANSPORTE: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)Transporte:?[ \t]*\r?\n?[ \t]*([0-9]{7,15})"));

/// ¿Es un Comprobante de Carga de Ruta? Separado de la extracción para que
/// la pantalla siga buscando sin tratar cada frame parcial como fallo.
#[uniffi::export]
pub fn es_comprobante_carga_ruta(texto: String) -> bool {
    RUTA_NUMERO_CARGA.is_match(&texto)
}

/// Ruta y documento son obligatorios: sin alguno no hay nada que registrar.
#[uniffi::export]
pub fn extraer_comprobante_ruta(texto: String) -> Option<ComprobanteRutaDetectado> {
    let ruta = RUTA_NUMERO_CARGA.captures(&texto)?;
    let numero_documento = TRANSPORTE.captures(&texto)?[1].to_owned();
    Some(ComprobanteRutaDetectado {
        numero_ruta: ruta[1].to_uppercase(),
        sub_numero: ruta[2].parse().unwrap_or(1),
        numero_documento,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_ruta_subnumero_y_transporte() {
        let texto = "Coca Cola FEMSA\nRuta / No.de Carga: CRR079 / 002\nTransporte:\n700101452\nMaterial 164145";
        let c = extraer_comprobante_ruta(texto.to_owned()).unwrap();
        assert_eq!(c.numero_ruta, "CRR079");
        assert_eq!(c.sub_numero, 2);
        assert_eq!(c.numero_documento, "700101452");
    }

    #[test]
    fn un_sku_de_seis_digitos_nunca_es_el_transporte() {
        let texto = "Ruta / No.de Carga: CRR079 / 1\nTransporte:\n164145";
        assert_eq!(extraer_comprobante_ruta(texto.to_owned()), None);
        assert!(es_comprobante_carga_ruta(texto.to_owned()));
    }
}
