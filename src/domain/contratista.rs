use crate::models::contratista::Contratista;

// Las reglas puras del contratista viven en el crate compartido
// (`control_acceso_reglas::contratista`), para que el panel web y las Edge
// Functions usen el mismo código. Acá quedan sólo las variantes que reciben
// un `Contratista` del núcleo.
pub use control_acceso_reglas::contratista::{
    ContratistaValido, DatosContratista, ErrorContratista, EstadoAnterior, admite_personal_ruta,
    normalizar_nombre, praind_vencido, requiere_gafete_de, requiere_praind_de,
    tipo_ingreso_seleccionable, validar_contratista,
};

/// Regla de negocio (ver tabla "Reglas para PRAIND y gafete" en
/// `docs/diagramas/diagrama-logico.md`): requiere PRAIND el personal de ruta (sin
/// importar el tipo de ingreso) y, entre los tipos de ingreso, `Praind` e
/// `InHouse`. Ver [`requiere_praind_de`].
pub fn requiere_praind(contratista: &Contratista) -> bool {
    requiere_praind_de(contratista.tipo_ingreso, contratista.es_personal_ruta)
}

/// Regla de negocio (misma tabla): el personal de ruta nunca requiere
/// gafete; entre los tipos, sólo `Praind` y `PorCorreo`. Ver
/// [`requiere_gafete_de`].
pub fn requiere_gafete(contratista: &Contratista) -> bool {
    requiere_gafete_de(contratista.tipo_ingreso, contratista.es_personal_ruta)
}
