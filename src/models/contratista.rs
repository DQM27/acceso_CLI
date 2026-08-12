use chrono::NaiveDate;

use super::tipo_ingreso::TipoIngreso;

#[derive(Debug, Clone)]
pub struct Contratista {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub tiene_acceso: bool,
}