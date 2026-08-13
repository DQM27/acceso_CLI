#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BusquedaTexto {
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

        Self {
            modo: 2,
            patron_like: None,
            consulta_fts: Some(format!("\"{}\"", texto.replace('"', "\"\""))),
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
}
