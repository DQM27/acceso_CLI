use chrono::NaiveDateTime;

use super::medio_ingreso::MedioIngreso;
use super::tipo_ingreso::TipoIngreso;

#[derive(Debug, Clone)]
pub struct RegistroIngreso {
    pub id: i64,
    pub contratista_id: i64,

    pub fecha_hora_ingreso: NaiveDateTime,
    pub medio_ingreso: MedioIngreso,
    pub tipo_ingreso: TipoIngreso,

    pub usuario_ingreso_id: i64,

    pub fecha_hora_salida: Option<NaiveDateTime>,
    pub usuario_salida_id: Option<i64>,
}