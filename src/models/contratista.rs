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
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

impl Contratista {
    pub fn requiere_praind(&self) -> bool {
        self.es_personal_ruta
            || matches!(
                self.tipo_ingreso,
                TipoIngreso::Praind | TipoIngreso::InHouse
            )
    }

    pub fn requiere_gafete(&self) -> bool {
        !self.es_personal_ruta
            && matches!(
                self.tipo_ingreso,
                TipoIngreso::Praind | TipoIngreso::PorCorreo
            )
    }
}
