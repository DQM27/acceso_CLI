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
