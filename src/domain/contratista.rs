use crate::models::contratista::Contratista;
use crate::models::tipo_ingreso::TipoIngreso;

/// Regla de negocio (ver tabla "Reglas para PRAIND y gafete" en
/// `docs/diagrama-logico.md`): requiere PRAIND el personal de ruta (sin
/// importar el tipo de ingreso) y, entre los tipos de ingreso, `Praind` e
/// `InHouse`. `PorCorreo` y `Swat` no lo requieren.
pub fn requiere_praind(contratista: &Contratista) -> bool {
    contratista.es_personal_ruta
        || matches!(
            contratista.tipo_ingreso,
            TipoIngreso::Praind | TipoIngreso::InHouse
        )
}

/// Regla de negocio (misma tabla): el personal de ruta nunca requiere
/// gafete. Entre los tipos de ingreso, sólo `Praind` y `PorCorreo` lo
/// requieren; `InHouse` y `Swat` no.
pub fn requiere_gafete(contratista: &Contratista) -> bool {
    !contratista.es_personal_ruta
        && matches!(
            contratista.tipo_ingreso,
            TipoIngreso::Praind | TipoIngreso::PorCorreo
        )
}
