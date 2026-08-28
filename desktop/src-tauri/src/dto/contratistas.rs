use chrono::NaiveDate;

use control_acceso::database::queries::Igualdad;
use control_acceso::database::queries::contratistas::{FiltroContratistas, FiltroPraind};
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::services::contratista_service::{
    DatosActualizacionContratista, DatosContratista,
};
use control_acceso::tiempo::ahora_costa_rica;

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstadoPraind {
    Vencido,
    Proximo,
    SinFecha,
}

#[derive(serde::Deserialize, Default)]
pub struct FiltroContratistasEntrada {
    pub texto: Option<String>,
    pub empresa_id: Option<i64>,
    pub tipos: Option<Vec<TipoIngreso>>,
    pub praind: Option<EstadoPraind>,
    pub personal_ruta: Option<bool>,
    pub tiene_acceso: Option<bool>,
}

impl FiltroContratistasEntrada {
    pub fn construir(self) -> FiltroContratistas {
        let hoy = ahora_costa_rica().date_naive();
        FiltroContratistas {
            texto: self
                .texto
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty()),
            empresa_id: self.empresa_id.map(Igualdad::Incluye),
            tipos_incluidos: self.tipos.filter(|tipos| !tipos.is_empty()),
            praind: self.praind.map(|estado| match estado {
                EstadoPraind::Vencido => FiltroPraind::Vencido { hoy },
                EstadoPraind::Proximo => FiltroPraind::ProximoAVencer { hoy },
                EstadoPraind::SinFecha => FiltroPraind::SinFecha,
            }),
            praind_negado: false,
            personal_ruta: self.personal_ruta,
            tiene_acceso: self.tiene_acceso,
            ..Default::default()
        }
    }
}

/// Espejo de `DatosContratista`/`DatosActualizacionContratista` — el core ya
/// modela crear y editar con dos structs idénticos en forma (distinto nombre
/// nada más), así que del lado del webview alcanza con uno solo. Cubre tanto
/// el formulario completo (crear/editar) como el toggle rápido de
/// "es de ruta"/"tiene acceso" desde la grilla (la fila ya tiene todos los
/// demás campos, se mandan de vuelta sin cambios).
#[derive(serde::Deserialize)]
pub struct DatosContratistaEntrada {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

impl From<DatosContratistaEntrada> for DatosContratista {
    fn from(entrada: DatosContratistaEntrada) -> Self {
        DatosContratista {
            cedula: entrada.cedula,
            nombre: entrada.nombre,
            empresa_id: entrada.empresa_id,
            tipo_ingreso: entrada.tipo_ingreso,
            fecha_vencimiento_praind: entrada.fecha_vencimiento_praind,
            es_personal_ruta: entrada.es_personal_ruta,
            tiene_acceso: entrada.tiene_acceso,
        }
    }
}

impl From<DatosContratistaEntrada> for DatosActualizacionContratista {
    fn from(entrada: DatosContratistaEntrada) -> Self {
        DatosActualizacionContratista {
            cedula: entrada.cedula,
            nombre: entrada.nombre,
            empresa_id: entrada.empresa_id,
            tipo_ingreso: entrada.tipo_ingreso,
            fecha_vencimiento_praind: entrada.fecha_vencimiento_praind,
            es_personal_ruta: entrada.es_personal_ruta,
            tiene_acceso: entrada.tiene_acceso,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_texto_en_blanco_se_convierte_en_ninguno() {
        let filtro = FiltroContratistasEntrada {
            texto: Some("   ".to_owned()),
            ..Default::default()
        }
        .construir();

        assert_eq!(filtro.texto, None);
    }

    #[test]
    fn el_texto_se_recorta_de_espacios() {
        let filtro = FiltroContratistasEntrada {
            texto: Some("  ana  ".to_owned()),
            ..Default::default()
        }
        .construir();

        assert_eq!(filtro.texto, Some("ana".to_owned()));
    }

    #[test]
    fn el_empresa_id_se_mapea_a_igualdad_incluye() {
        let filtro = FiltroContratistasEntrada {
            empresa_id: Some(7),
            ..Default::default()
        }
        .construir();

        assert_eq!(filtro.empresa_id, Some(Igualdad::Incluye(7)));
    }

    #[test]
    fn la_lista_de_tipos_vacia_se_convierte_en_ninguno() {
        let filtro = FiltroContratistasEntrada {
            tipos: Some(Vec::new()),
            ..Default::default()
        }
        .construir();

        assert_eq!(filtro.tipos_incluidos, None);
    }

    #[test]
    fn la_lista_de_tipos_no_vacia_se_conserva() {
        let filtro = FiltroContratistasEntrada {
            tipos: Some(vec![TipoIngreso::Praind, TipoIngreso::Swat]),
            ..Default::default()
        }
        .construir();

        assert_eq!(
            filtro.tipos_incluidos,
            Some(vec![TipoIngreso::Praind, TipoIngreso::Swat])
        );
    }

    #[test]
    fn cada_estado_praind_se_mapea_a_su_variante_con_la_fecha_de_hoy() {
        let hoy = ahora_costa_rica().date_naive();

        let vencido = FiltroContratistasEntrada {
            praind: Some(EstadoPraind::Vencido),
            ..Default::default()
        }
        .construir();
        assert_eq!(vencido.praind, Some(FiltroPraind::Vencido { hoy }));

        let proximo = FiltroContratistasEntrada {
            praind: Some(EstadoPraind::Proximo),
            ..Default::default()
        }
        .construir();
        assert_eq!(proximo.praind, Some(FiltroPraind::ProximoAVencer { hoy }));

        let sin_fecha = FiltroContratistasEntrada {
            praind: Some(EstadoPraind::SinFecha),
            ..Default::default()
        }
        .construir();
        assert_eq!(sin_fecha.praind, Some(FiltroPraind::SinFecha));
    }

    /// `praind_negado` no tiene equivalente en la entrada — el filtro de la
    /// GUI siempre pregunta por la condición positiva ("está vencido"), nunca
    /// por su negación ("-praind:vencido", que sólo existe en el lenguaje de
    /// `--comandos`). Fija ese `false` para que un cambio futuro que lo
    /// rompa falle acá y no en producción.
    #[test]
    fn praind_negado_siempre_es_falso() {
        let filtro = FiltroContratistasEntrada::default().construir();

        assert!(!filtro.praind_negado);
    }

    #[test]
    fn personal_ruta_y_tiene_acceso_pasan_directo() {
        let filtro = FiltroContratistasEntrada {
            personal_ruta: Some(true),
            tiene_acceso: Some(false),
            ..Default::default()
        }
        .construir();

        assert_eq!(filtro.personal_ruta, Some(true));
        assert_eq!(filtro.tiene_acceso, Some(false));
    }
}
