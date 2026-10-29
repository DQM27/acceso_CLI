//! Quien está adentro por otra vía (contratista, proveedor o por correo):
//! el ingreso concreto, para mostrarlo y cerrarlo desde donde se lo
//! encontró, sin ir a su sección (pedido del usuario 2026-10-29).

use crate::database::error::DatabaseError;
use crate::database::queries::persona_adentro::{self, IngresoPorOtraVia};
use crate::models::via_ingreso::ViaIngreso;

use super::AppCore;

impl AppCore {
    /// El ingreso abierto de esta cédula por una vía distinta de `propia`,
    /// en este equipo o en el otro de la unidad. La misma vía que frena el
    /// registro (`AdentroPorOtraVia`). Una cédula que no se puede normalizar
    /// no está adentro. Sin actor: es una lectura.
    pub fn ingreso_por_otra_via(
        &self,
        cedula: &str,
        propia: ViaIngreso,
    ) -> Result<Option<IngresoPorOtraVia>, DatabaseError> {
        let Ok(cedula) = crate::domain::cedula::Cedula::normalizar(cedula) else {
            return Ok(None);
        };
        persona_adentro::ingreso_por_otra_via(&self.connection, cedula.as_str(), propia)
    }
}
