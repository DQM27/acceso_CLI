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
    /// Estado de `Empresa::activo` de `empresa_id`, resuelto por quien
    /// construye este `Contratista` (repositorio o quien lo arma a mano) —
    /// no es un campo propio de `contratistas`, viaja aquí para que
    /// `domain::acceso::verificar_acceso` no necesite consultar otra tabla.
    pub empresa_activa: bool,
}

impl Contratista {
    /// Regla de negocio: ver `domain::contratista::requiere_praind`, donde
    /// vive la definición real. Delegada acá para no tocar los ~20 call
    /// sites que ya usan `contratista.requiere_praind()` como método.
    pub fn requiere_praind(&self) -> bool {
        crate::domain::contratista::requiere_praind(self)
    }

    /// Regla de negocio: ver `domain::contratista::requiere_gafete`, donde
    /// vive la definición real. Delegada acá por el mismo motivo.
    pub fn requiere_gafete(&self) -> bool {
        crate::domain::contratista::requiere_gafete(self)
    }
}
