//! Número de cédula nacional en texto crudo de OCR.

use std::sync::LazyLock;

use regex::Regex;

use super::texto::{patron, renglones};

static CARACTERES_NO_CEDULA: LazyLock<Regex> = LazyLock::new(|| patron(r"[^0-9\n -]"));
// El primer dígito de una cédula es la provincia (1-9), nunca 0: así un
// número de control impreso junto a la cédula ("001234567" bajo el PDF417
// del reverso anterior) no se toma por cédula, y si hay ambos en el mismo
// texto se sigue buscando hasta la cédula real en vez de rendirse.
static CEDULA_CON_SEPARADORES: LazyLock<Regex> =
    LazyLock::new(|| patron(r"\b[1-9][- ]?[0-9]{4}[- ]?[0-9]{4}\b"));
static CEDULA_PEGADA: LazyLock<Regex> = LazyLock::new(|| patron(r"\b[1-9][0-9]{8}\b"));
// El renglón de "Expediente No." del DIMEX trae otro número: nunca es la
// cédula.
static RENGLON_NUMERO_AJENO: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)\bEXPEDIENTE\s*(?:N[O°º.]*)?"));

/// Número de cédula nacional (9 dígitos, sin separadores) en el texto. Se
/// prefiere el impreso con separadores ("1 2345 6789", como va en la
/// tarjeta) y, si no hay, 9 dígitos seguidos.
#[uniffi::export]
pub fn extraer_cedula_de_texto(texto: String) -> Option<String> {
    extraer_cedula(&texto)
}

pub fn extraer_cedula(texto: &str) -> Option<String> {
    let candidatos: Vec<String> = renglones(texto)
        .into_iter()
        .filter(|r| !RENGLON_NUMERO_AJENO.is_match(r))
        .map(|r| CARACTERES_NO_CEDULA.replace_all(r, " ").into_owned())
        .collect();
    candidatos
        .iter()
        .find_map(|r| CEDULA_CON_SEPARADORES.find(r))
        .map(|m| m.as_str().chars().filter(char::is_ascii_digit).collect())
        .or_else(|| {
            candidatos
                .iter()
                .find_map(|r| CEDULA_PEGADA.find(r))
                .map(|m| m.as_str().to_owned())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_la_cedula_con_espacios_guiones_o_pegada() {
        assert_eq!(extraer_cedula("1 2345 6789").as_deref(), Some("123456789"));
        assert_eq!(
            extraer_cedula("Cédula 1-2345-6789").as_deref(),
            Some("123456789")
        );
        assert_eq!(
            extraer_cedula("x\n123456789\n").as_deref(),
            Some("123456789")
        );
    }

    #[test]
    fn la_provincia_nunca_es_cero_ni_se_lee_del_expediente() {
        assert_eq!(extraer_cedula("001234567"), None);
        assert_eq!(
            extraer_cedula("001234567\n1 2345 6789").as_deref(),
            Some("123456789")
        );
        assert_eq!(extraer_cedula("Expediente No.: 123456789"), None);
    }

    #[test]
    fn diez_digitos_seguidos_no_son_cedula() {
        assert_eq!(extraer_cedula("1234567890"), None);
    }
}
