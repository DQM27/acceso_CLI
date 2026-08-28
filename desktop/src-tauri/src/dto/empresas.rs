use control_acceso::database::queries::empresas::FiltroEmpresas;

#[derive(serde::Deserialize, Default)]
pub struct FiltroEmpresasEntrada {
    pub texto: Option<String>,
}

impl FiltroEmpresasEntrada {
    pub fn construir(self) -> FiltroEmpresas {
        FiltroEmpresas {
            texto: self
                .texto
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_texto_en_blanco_se_convierte_en_ninguno() {
        let filtro = FiltroEmpresasEntrada {
            texto: Some("   ".to_owned()),
        }
        .construir();

        assert_eq!(filtro.texto, None);
    }

    #[test]
    fn el_texto_se_recorta_de_espacios() {
        let filtro = FiltroEmpresasEntrada {
            texto: Some("  brisas  ".to_owned()),
        }
        .construir();

        assert_eq!(filtro.texto, Some("brisas".to_owned()));
    }
}
