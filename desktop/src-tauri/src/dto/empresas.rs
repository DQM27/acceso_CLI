use control_acceso::database::queries::empresas::FiltroEmpresas;

#[derive(serde::Deserialize, Default)]
pub struct FiltroEmpresasEntrada {
    pub texto: Option<String>,
}

impl FiltroEmpresasEntrada {
    /// Lo pide la grilla de administración de Empresas, que tiene que
    /// mostrarlas todas: el límite se pide al máximo y el núcleo lo acota a
    /// su tope de listado. Con el predeterminado (100) la grilla se habría
    /// cortado en silencio, como pasó con las de proveedor en 20 (2026-10-30).
    pub fn construir(self) -> FiltroEmpresas {
        FiltroEmpresas {
            texto: self
                .texto
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty()),
            limite: usize::MAX,
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

    #[test]
    fn la_grilla_pide_todas_las_empresas_no_las_primeras_100() {
        let filtro = FiltroEmpresasEntrada::default().construir();

        assert!(filtro.limite > 100, "{}", filtro.limite);
    }
}
