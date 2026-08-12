use crate::database::error::DatabaseError;
use crate::domain::resultado_acceso::MotivoDenegacion;

#[derive(Debug)]
pub enum RegistroIngresoServiceError {
    ContratistaNoEncontrado,
    AccesoDenegado(MotivoDenegacion),
    IngresoActivo,
    GafeteRequerido,
    GafeteOcupado,
    GafeteNoAsignado,
    Database(DatabaseError),
}

impl std::fmt::Display for RegistroIngresoServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContratistaNoEncontrado => write!(formatter, "Contratista no encontrado"),
            Self::AccesoDenegado(motivo) => {
                write!(formatter, "Acceso denegado: {motivo:?}")
            }
            Self::IngresoActivo => write!(formatter, "El contratista ya tiene un ingreso activo"),
            Self::GafeteRequerido => write!(formatter, "El contratista requiere gafete"),
            Self::GafeteOcupado => write!(formatter, "El gafete ya está asignado"),
            Self::GafeteNoAsignado => write!(formatter, "El gafete no está asignado actualmente"),
            Self::Database(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for RegistroIngresoServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DatabaseError> for RegistroIngresoServiceError {
    fn from(error: DatabaseError) -> Self {
        Self::Database(error)
    }
}
