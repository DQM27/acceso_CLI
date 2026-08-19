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
    /// Regla de negocio (ver tabla "Reglas para PRAIND y gafete" en
    /// `docs/diagrama-logico.md`): requiere PRAIND el personal de ruta (sin
    /// importar el tipo de ingreso) y, entre los tipos de ingreso, `Praind` e
    /// `InHouse`. `PorCorreo` y `Swat` no lo requieren.
    pub fn requiere_praind(&self) -> bool {
        self.es_personal_ruta
            || matches!(
                self.tipo_ingreso,
                TipoIngreso::Praind | TipoIngreso::InHouse
            )
    }

    /// Regla de negocio (misma tabla): el personal de ruta nunca requiere
    /// gafete. Entre los tipos de ingreso, sólo `Praind` y `PorCorreo` lo
    /// requieren; `InHouse` y `Swat` no.
    pub fn requiere_gafete(&self) -> bool {
        !self.es_personal_ruta
            && matches!(
                self.tipo_ingreso,
                TipoIngreso::Praind | TipoIngreso::PorCorreo
            )
    }
}
