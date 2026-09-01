use control_acceso::database::queries::Igualdad;
use control_acceso::database::queries::gafetes::FiltroGafetes;
use control_acceso::models::gafete::EstadoGafete;

/// Espejo de `EstadoGafete` — un enum propio en vez de reusar el del núcleo
/// directamente en el filtro de entrada porque el frontend nunca pide la
/// negación (`-estado:...`, sólo existe en el lenguaje `clave:valor` de
/// `--cli`).
#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstadoGafeteEntrada {
    Disponible,
    Perdido,
    DeBaja,
}

impl From<EstadoGafeteEntrada> for EstadoGafete {
    fn from(estado: EstadoGafeteEntrada) -> Self {
        match estado {
            EstadoGafeteEntrada::Disponible => EstadoGafete::Disponible,
            EstadoGafeteEntrada::Perdido => EstadoGafete::Perdido,
            EstadoGafeteEntrada::DeBaja => EstadoGafete::DeBaja,
        }
    }
}

#[derive(serde::Deserialize, Default)]
pub struct FiltroGafetesEntrada {
    pub numero: Option<i64>,
    pub estado: Option<EstadoGafeteEntrada>,
}

impl FiltroGafetesEntrada {
    pub fn construir(self) -> FiltroGafetes {
        FiltroGafetes {
            numero: self.numero,
            estado: self.estado.map(|e| Igualdad::Incluye(e.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_filtro_queda_vacio() {
        let filtro = FiltroGafetesEntrada::default().construir();
        assert_eq!(filtro, FiltroGafetes::default());
    }

    #[test]
    fn numero_pasa_directo() {
        let filtro = FiltroGafetesEntrada {
            numero: Some(9),
            estado: None,
        }
        .construir();
        assert_eq!(filtro.numero, Some(9));
    }

    #[test]
    fn estado_se_mapea_a_igualdad_incluye() {
        let filtro = FiltroGafetesEntrada {
            numero: None,
            estado: Some(EstadoGafeteEntrada::Perdido),
        }
        .construir();
        assert_eq!(
            filtro.estado,
            Some(Igualdad::Incluye(EstadoGafete::Perdido))
        );
    }
}
