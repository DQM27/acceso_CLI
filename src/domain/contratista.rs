use crate::models::contratista::Contratista;
use crate::models::tipo_ingreso::TipoIngreso;

/// Regla de negocio (ver tabla "Reglas para PRAIND y gafete" en
/// `docs/diagramas/diagrama-logico.md`): requiere PRAIND el personal de ruta (sin
/// importar el tipo de ingreso) y, entre los tipos de ingreso, `Praind` e
/// `InHouse`. `PorCorreo` y `Swat` no lo requieren.
pub fn requiere_praind(contratista: &Contratista) -> bool {
    requiere_praind_de(contratista.tipo_ingreso, contratista.es_personal_ruta)
}

/// Misma regla que [`requiere_praind`], sin depender de un `Contratista`
/// completo -- MV-10 (auditoría 2026-09-24): `mobile/rust-core` la expone
/// por `UniFFI` (`Nucleo::requiere_praind_para_formulario`) para que
/// `PantallaNuevoContratista` deje de reimplementarla a mano en Kotlin
/// (antes de tener un `Contratista` real que consultar, sólo con lo que la
/// persona ya tipeó en el formulario).
pub fn requiere_praind_de(tipo_ingreso: TipoIngreso, personal_ruta: bool) -> bool {
    personal_ruta || matches!(tipo_ingreso, TipoIngreso::Praind | TipoIngreso::InHouse)
}

/// Regla de negocio (misma tabla): el personal de ruta nunca requiere
/// gafete. Entre los tipos de ingreso, sólo `Praind` y `PorCorreo` lo
/// requieren; `InHouse` y `Swat` no.
pub fn requiere_gafete(contratista: &Contratista) -> bool {
    requiere_gafete_de(contratista.tipo_ingreso, contratista.es_personal_ruta)
}

/// Espejo de [`requiere_praind_de`] para la otra regla -- mismo motivo
/// (MV-10, expuesta como `Nucleo::requiere_gafete_para_formulario`).
pub fn requiere_gafete_de(tipo_ingreso: TipoIngreso, personal_ruta: bool) -> bool {
    !personal_ruta && matches!(tipo_ingreso, TipoIngreso::Praind | TipoIngreso::PorCorreo)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cobertura directa de la parte pura -- la cobertura existente contra
    // `Contratista` completo (`tests/domain.rs`) sigue pasando sin cambios
    // porque `requiere_praind`/`requiere_gafete` ahora sólo delegan acá.
    #[test]
    fn requiere_praind_de_personal_de_ruta_sin_importar_el_tipo() {
        assert!(requiere_praind_de(TipoIngreso::Swat, true));
        assert!(requiere_praind_de(TipoIngreso::PorCorreo, true));
    }

    #[test]
    fn requiere_praind_de_segun_tipo_de_ingreso_sin_personal_de_ruta() {
        assert!(requiere_praind_de(TipoIngreso::Praind, false));
        assert!(requiere_praind_de(TipoIngreso::InHouse, false));
        assert!(!requiere_praind_de(TipoIngreso::PorCorreo, false));
        assert!(!requiere_praind_de(TipoIngreso::Swat, false));
    }

    #[test]
    fn requiere_gafete_de_nunca_para_personal_de_ruta() {
        assert!(!requiere_gafete_de(TipoIngreso::Praind, true));
        assert!(!requiere_gafete_de(TipoIngreso::PorCorreo, true));
    }

    #[test]
    fn requiere_gafete_de_segun_tipo_de_ingreso_sin_personal_de_ruta() {
        assert!(requiere_gafete_de(TipoIngreso::Praind, false));
        assert!(requiere_gafete_de(TipoIngreso::PorCorreo, false));
        assert!(!requiere_gafete_de(TipoIngreso::InHouse, false));
        assert!(!requiere_gafete_de(TipoIngreso::Swat, false));
    }
}
