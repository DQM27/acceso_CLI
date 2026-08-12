use crate::database::error::DatabaseError;
use crate::domain::resultado_acceso::MotivoDenegacion;

#[derive(Debug)]
pub enum PasswordError {
    GeneracionHash,
    HashInvalido,
}

impl std::fmt::Display for PasswordError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GeneracionHash => write!(formatter, "No se pudo generar el hash"),
            Self::HashInvalido => write!(formatter, "El hash almacenado no es válido"),
        }
    }
}

impl std::error::Error for PasswordError {}

#[derive(Debug)]
pub enum UsuarioServiceError {
    CedulaVacia,
    NombreVacio,
    PasswordDemasiadoCorto,
    UsuarioNoEncontrado,
    ConfiguracionInicialRequerida,
    ConfiguracionInicialYaRealizada,
    UltimoRootActivo,
    Password(PasswordError),
    Database(DatabaseError),
}

impl std::fmt::Display for UsuarioServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CedulaVacia => write!(formatter, "La cédula es obligatoria"),
            Self::NombreVacio => write!(formatter, "El nombre es obligatorio"),
            Self::PasswordDemasiadoCorto => {
                write!(formatter, "La contraseña debe tener al menos 8 caracteres")
            }
            Self::UsuarioNoEncontrado => write!(formatter, "Usuario no encontrado"),
            Self::ConfiguracionInicialRequerida => {
                write!(formatter, "Se requiere crear el usuario ROOT inicial")
            }
            Self::ConfiguracionInicialYaRealizada => {
                write!(formatter, "La configuración inicial ya fue realizada")
            }
            Self::UltimoRootActivo => write!(
                formatter,
                "No se puede desactivar o degradar al último ROOT activo"
            ),
            Self::Password(error) => write!(formatter, "{error}"),
            Self::Database(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for UsuarioServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Password(error) => Some(error),
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<PasswordError> for UsuarioServiceError {
    fn from(error: PasswordError) -> Self {
        Self::Password(error)
    }
}

impl From<DatabaseError> for UsuarioServiceError {
    fn from(error: DatabaseError) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug)]
pub enum AutenticacionError {
    CredencialesInvalidas,
    UsuarioInactivo,
    HashInvalido,
    Database(DatabaseError),
}

impl std::fmt::Display for AutenticacionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CredencialesInvalidas => write!(formatter, "Credenciales inválidas"),
            Self::UsuarioInactivo => write!(formatter, "Usuario inactivo"),
            Self::HashInvalido => write!(formatter, "El hash almacenado no es válido"),
            Self::Database(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for AutenticacionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DatabaseError> for AutenticacionError {
    fn from(error: DatabaseError) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug)]
pub enum ContratistaServiceError {
    ContratistaNoEncontrado,
    EmpresaNoEncontrada,
    CedulaVacia,
    NombreVacio,
    PraindRequerido,
    Database(DatabaseError),
}

impl std::fmt::Display for ContratistaServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContratistaNoEncontrado => write!(formatter, "Contratista no encontrado"),
            Self::EmpresaNoEncontrada => write!(formatter, "Empresa no encontrada"),
            Self::CedulaVacia => write!(formatter, "La cédula es obligatoria"),
            Self::NombreVacio => write!(formatter, "El nombre es obligatorio"),
            Self::PraindRequerido => write!(formatter, "La fecha de PRAIND es obligatoria"),
            Self::Database(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ContratistaServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DatabaseError> for ContratistaServiceError {
    fn from(error: DatabaseError) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug)]
pub enum EmpresaServiceError {
    EmpresaNoEncontrada,
    NombreEmpresaVacio,
    Database(DatabaseError),
}

impl std::fmt::Display for EmpresaServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmpresaNoEncontrada => write!(formatter, "Empresa no encontrada"),
            Self::NombreEmpresaVacio => write!(formatter, "El nombre de la empresa es obligatorio"),
            Self::Database(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for EmpresaServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DatabaseError> for EmpresaServiceError {
    fn from(error: DatabaseError) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug)]
pub enum RegistroIngresoServiceError {
    ContratistaNoEncontrado,
    AccesoDenegado(MotivoDenegacion),
    IngresoActivo,
    GafeteRequerido,
    GafeteOcupado,
    GafeteNoAsignado,
    RegistroNoActivo,
    SalidaAnteriorAIngreso,
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
            Self::RegistroNoActivo => write!(formatter, "El registro de ingreso no está activo"),
            Self::SalidaAnteriorAIngreso => {
                write!(formatter, "La salida no puede ser anterior al ingreso")
            }
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
