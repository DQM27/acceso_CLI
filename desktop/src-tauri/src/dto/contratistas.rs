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
