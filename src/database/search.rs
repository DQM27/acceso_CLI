#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusquedaTexto {
    pub modo: i64,
    pub patron_like: Option<String>,
    pub consulta_fts: Option<String>,
    pub texto_literal: Option<String>,
    pub numero_exacto: Option<i64>,
}

impl BusquedaTexto {
    pub fn preparar(texto: Option<&str>) -> Self {
        let Some(texto) = texto.map(str::trim).filter(|texto| !texto.is_empty()) else {
            return Self {
                modo: 0,
                patron_like: None,
                consulta_fts: None,
                texto_literal: None,
                numero_exacto: None,
            };
        };

        let numero_exacto = texto.parse::<i64>().ok();
        if texto.chars().count() < 3 {
            return Self {
                modo: 1,
                patron_like: Some(format!("%{texto}%")),
                consulta_fts: None,
                texto_literal: Some(texto.to_owned()),
                numero_exacto,
            };
        }

        // Cada palabra va como su propia frase entre comillas (en vez de
        // envolver el texto completo en una sola frase) -- el índice usa el
        // tokenizador `trigram` (`schema.rs`), donde una frase entre
        // comillas exige el substring literal, espacio incluido. Con una
        // sola frase, buscar "Carlos Sanches" no encontraba a "Carlos
        // Mauricio Sanches" porque esa secuencia exacta de caracteres no
        // existe (el nombre intermedio corta la adyacencia) -- reportado en
        // runtime real 2026-09-21. Frases separadas con espacio quedan unidas
        // por el AND implícito de la sintaxis de consulta de FTS5, así que
        // cada palabra debe aparecer en el texto pero no necesita estar
        // pegada a las demás.
        let consulta_fts = texto
            .split_whitespace()
            .map(|palabra| format!("\"{}\"", palabra.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ");

        Self {
            modo: 2,
            patron_like: None,
            consulta_fts: Some(consulta_fts),
            texto_literal: Some(texto.to_owned()),
            numero_exacto,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BusquedaTexto;

    #[test]
    fn clasifica_vacio_corto_y_fts_y_escapa_comillas() {
        assert_eq!(BusquedaTexto::preparar(Some("  ")).modo, 0);
        assert_eq!(BusquedaTexto::preparar(Some("ña")).modo, 1);
        assert_eq!(
            BusquedaTexto::preparar(Some(" a\"b ")).consulta_fts,
            Some("\"a\"\"b\"".to_owned())
        );
        assert_eq!(
            BusquedaTexto::preparar(Some(" 012 ")).numero_exacto,
            Some(12)
        );
    }

    /// Regresión del hallazgo runtime 2026-09-21: buscar "Carlos Sanches"
    /// debe encontrar a "Carlos Mauricio Sanches" -- cada palabra queda
    /// como su propia frase (AND implícito) en vez de una sola frase con
    /// todo el texto, que exigiría "Carlos Sanches" como substring literal
    /// contiguo.
    #[test]
    fn multiples_palabras_quedan_como_frases_independientes() {
        assert_eq!(
            BusquedaTexto::preparar(Some("Carlos  Sanches")).consulta_fts,
            Some("\"Carlos\" \"Sanches\"".to_owned())
        );
    }
}
