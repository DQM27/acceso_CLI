use control_acceso::database::queries::Igualdad;
use control_acceso::database::queries::gafetes::FiltroGafetes;
use control_acceso::models::gafete::{EstadoGafete, TipoGafete};

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
            EstadoGafeteEntrada::Disponible => Self::Disponible,
            EstadoGafeteEntrada::Perdido => Self::Perdido,
            EstadoGafeteEntrada::DeBaja => Self::DeBaja,
        }
    }
}

/// Espejo de `TipoGafete` -- mismo criterio que `EstadoGafeteEntrada`. El
/// catálogo (crear/listar/filtrar) es genérico sobre `TipoGafete`
/// (`AppCore::crear_gafete`/`crear_gafetes_rango`), así que agregar un
/// valor acá sólo abre la puerta en este DTO de entrada -- ya lo hizo
/// `ProvisionalKof`, y ahora `Proveedor` (docs/features-futuras/plan-control-proveedores.md)
/// desde que existe la entidad `registro_ingresos_proveedor`.
#[derive(serde::Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum TipoGafeteEntrada {
    Contratista,
    Visita,
    ProvisionalKof,
    Proveedor,
}

impl From<TipoGafeteEntrada> for TipoGafete {
    fn from(tipo: TipoGafeteEntrada) -> Self {
        match tipo {
            TipoGafeteEntrada::Contratista => Self::Contratista,
            TipoGafeteEntrada::Visita => Self::Visita,
            TipoGafeteEntrada::ProvisionalKof => Self::ProvisionalKof,
            TipoGafeteEntrada::Proveedor => Self::Proveedor,
        }
    }
}

#[derive(serde::Deserialize, Default)]
pub struct FiltroGafetesEntrada {
    pub numero: Option<i64>,
    pub tipo: Option<TipoGafeteEntrada>,
    pub estado: Option<EstadoGafeteEntrada>,
}

impl FiltroGafetesEntrada {
    pub fn construir(self) -> FiltroGafetes {
        FiltroGafetes {
            numero: self.numero,
            tipo: self.tipo.map(|t| Igualdad::Incluye(t.into())),
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
            tipo: None,
            estado: None,
        }
        .construir();
        assert_eq!(filtro.numero, Some(9));
    }

    #[test]
    fn estado_se_mapea_a_igualdad_incluye() {
        let filtro = FiltroGafetesEntrada {
            numero: None,
            tipo: None,
            estado: Some(EstadoGafeteEntrada::Perdido),
        }
        .construir();
        assert_eq!(
            filtro.estado,
            Some(Igualdad::Incluye(EstadoGafete::Perdido))
        );
    }

    #[test]
    fn tipo_se_mapea_a_igualdad_incluye() {
        let filtro = FiltroGafetesEntrada {
            numero: None,
            tipo: Some(TipoGafeteEntrada::Visita),
            estado: None,
        }
        .construir();
        assert_eq!(filtro.tipo, Some(Igualdad::Incluye(TipoGafete::Visita)));
    }

    #[test]
    fn tipo_provisional_kof_se_mapea_a_igualdad_incluye() {
        let filtro = FiltroGafetesEntrada {
            numero: None,
            tipo: Some(TipoGafeteEntrada::ProvisionalKof),
            estado: None,
        }
        .construir();
        assert_eq!(
            filtro.tipo,
            Some(Igualdad::Incluye(TipoGafete::ProvisionalKof))
        );
    }

    #[test]
    fn tipo_proveedor_se_mapea_a_igualdad_incluye() {
        let filtro = FiltroGafetesEntrada {
            numero: None,
            tipo: Some(TipoGafeteEntrada::Proveedor),
            estado: None,
        }
        .construir();
        assert_eq!(filtro.tipo, Some(Igualdad::Incluye(TipoGafete::Proveedor)));
    }
}
