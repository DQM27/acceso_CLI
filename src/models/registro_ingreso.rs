use chrono::NaiveDateTime;

use super::medio_ingreso::MedioIngreso;
use super::tipo_ingreso::TipoIngreso;

pub const VERSION_REGLAS_ACCESO: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoIngresoRegistrado {
    Permitido,
    PermitidoConAdvertencia,
    /// Registro anterior a la captura de fotografías históricas.
    Migrado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotivoResultadoIngreso {
    PraindProximoVencer,
    DatosReconstruidos,
}

#[derive(Debug, Clone)]
pub struct DatosHistoricosEntrada {
    pub contratista_cedula: String,
    pub contratista_nombre: String,
    pub fecha_vencimiento_praind: Option<chrono::NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
    pub resultado_acceso: ResultadoIngresoRegistrado,
    pub reglas_version: i64,
}

#[derive(Debug, Clone)]
pub struct NuevoRegistroIngreso {
    pub contratista_id: i64,
    pub empresa_id: i64,
    pub fecha_hora_ingreso: NaiveDateTime,
    pub medio_ingreso: MedioIngreso,
    pub tipo_ingreso: TipoIngreso,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_id: i64,
    pub datos_historicos: DatosHistoricosEntrada,
}

#[derive(Debug, Clone)]
pub struct RegistroIngreso {
    pub id: i64,

    pub contratista_id: i64,
    pub empresa_id: i64,

    pub fecha_hora_ingreso: NaiveDateTime,

    pub medio_ingreso: MedioIngreso,
    pub tipo_ingreso: TipoIngreso,

    /// Número de gafete asignado durante este ingreso.
    ///
    /// `Some(numero)` = tiene gafete.
    /// `None` = sin gafete (S/G).
    pub gafete_numero: Option<i64>,

    pub usuario_ingreso_id: i64,

    pub fecha_hora_salida: Option<NaiveDateTime>,
    pub usuario_salida_id: Option<i64>,
}
