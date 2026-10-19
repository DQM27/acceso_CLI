//! Puerto del catálogo de gafetes (`docs/plan-gafetes.md`). Un solo trait
//! para lectura y escritura puntual (a diferencia de Empresas y
//! Contratistas, que separan `*Query` de `*Repository`) porque el catálogo es
//! chico y no hay una proyección cara que justifique separarlos --
//! `GafetesQuery` (`queries/gafetes.rs`) sigue aparte sólo para la lista
//! completa con datos del deudor, que sí es una proyección propia.

use crate::database::error::DatabaseError;
use crate::models::gafete::{Gafete, PortadorGafete, TipoGafete};

pub trait GafeteRepository {
    fn crear(&self, numero: i64, tipo: TipoGafete) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<Gafete>, DatabaseError>;

    fn buscar_por_numero(
        &self,
        numero: i64,
        tipo: TipoGafete,
    ) -> Result<Option<Gafete>, DatabaseError>;

    fn dar_de_baja(&self, id: i64) -> Result<(), DatabaseError>;

    fn marcar_perdido(&self, id: i64, portador: PortadorGafete) -> Result<(), DatabaseError>;

    fn resolver(&self, id: i64) -> Result<(), DatabaseError>;

    /// Números de los gafetes que un contratista debe actualmente
    /// (`estado = 'PERDIDO'` con `contratista_portador_id` apuntándolo). Un
    /// `Vec` y no `Option<i64>`: nada impide más de una deuda simultánea.
    fn deuda_de_contratista(&self, contratista_id: i64) -> Result<Vec<i64>, DatabaseError>;
}
