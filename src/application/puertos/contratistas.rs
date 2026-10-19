//! Puerto de los contratistas: crear, buscar, actualizar y listar.

use crate::database::error::DatabaseError;
use crate::models::contratista::Contratista;

pub trait ContratistaRepository {
    fn crear(&self, contratista: &Contratista) -> Result<i64, DatabaseError>;

    fn buscar_por_cedula(&self, cedula: &str) -> Result<Option<Contratista>, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<Contratista>, DatabaseError>;

    fn actualizar(&self, contratista: &Contratista) -> Result<(), DatabaseError>;

    fn listar(&self) -> Result<Vec<Contratista>, DatabaseError>;
}
