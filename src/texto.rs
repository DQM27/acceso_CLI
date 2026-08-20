//! Plegado de diacríticos, compartido entre la TUI (comparaciones en
//! memoria, p. ej. el filtro `empresa:`) y la base de datos (función SQL
//! `PLEGAR`, registrada en `database::schema::initialize_database` para las
//! búsquedas `LIKE` de menos de 3 caracteres — ver `docs/hallazgos-buscador.md`).
//! FTS5 ya pliega diacríticos por su cuenta vía `remove_diacritics = 1`; esta
//! función sólo cubre los caminos que no pasan por FTS.
//!
//! Cubre el alfabeto latino con acentos que puede aparecer en nombres reales
//! de esta app (español); no es una normalización Unicode NFD general.
//!
//! `ñ` se pliega a `n` a propósito, igual que el tokenizador FTS5 —
//! confirmado como comportamiento intencional en
//! `docs/hallazgos-buscador.md`, no un descuido de esta función.
pub fn plegar_diacriticos(texto: &str) -> String {
    texto
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'Á' | 'À' | 'Ä' | 'Â' => 'A',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'É' | 'È' | 'Ë' | 'Ê' => 'E',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'Ó' | 'Ò' | 'Ö' | 'Ô' => 'O',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'Ú' | 'Ù' | 'Ü' | 'Û' => 'U',
            'ñ' => 'n',
            'Ñ' => 'N',
            'ç' => 'c',
            'Ç' => 'C',
            otro => otro,
        })
        .collect()
}

/// Minúsculas + diacríticos plegados — el criterio de comparación completo
/// usado por la función SQL `PLEGAR` (equivalente a `COLLATE NOCASE` más
/// tolerancia a tildes/eñes, ninguna de las dos cosas por separado).
pub fn plegar_para_busqueda(texto: &str) -> String {
    plegar_diacriticos(&texto.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pliega_vocales_acentuadas_y_dieresis() {
        assert_eq!(plegar_diacriticos("Álvarez Ingeniería"), "Alvarez Ingenieria");
    }

    #[test]
    fn pliega_ene_a_n_igual_que_fts() {
        assert_eq!(plegar_diacriticos("Niño"), "Nino");
    }

    #[test]
    fn deja_intacto_el_texto_sin_diacriticos() {
        assert_eq!(plegar_diacriticos("Brisas del Oeste"), "Brisas del Oeste");
    }

    #[test]
    fn plegar_para_busqueda_combina_minusculas_y_diacriticos() {
        assert_eq!(plegar_para_busqueda("ÁLVAREZ"), "alvarez");
        assert_eq!(plegar_para_busqueda("Niño"), "nino");
    }
}
