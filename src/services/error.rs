use crate::database::error::DatabaseError;
use crate::domain::resultado_acceso::MotivoDenegacion;

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("No se pudo generar el hash")]
    GeneracionHash,
    #[error("El hash almacenado no es válido")]
    HashInvalido,
}

#[derive(Debug, thiserror::Error)]
pub enum UsuarioServiceError {
    #[error("La cédula es obligatoria")]
    CedulaVacia,
    #[error("El nombre es obligatorio")]
    NombreVacio,
    #[error("La contraseña debe tener al menos 8 caracteres")]
    PasswordDemasiadoCorto,
    #[error("Usuario no encontrado")]
    UsuarioNoEncontrado,
    #[error("Se requiere crear el usuario ROOT inicial")]
    ConfiguracionInicialRequerida,
    #[error("La configuración inicial ya fue realizada")]
    ConfiguracionInicialYaRealizada,
    #[error("No se puede desactivar o degradar al último ROOT activo")]
    UltimoRootActivo,
    #[error("La cédula del usuario ya existe")]
    CedulaDuplicada,
    #[error("La sesión actual no está autorizada para gestionar ese usuario")]
    OperacionNoAutorizada,
    #[error("La contraseña actual es incorrecta")]
    PasswordActualIncorrecta,
    #[error(transparent)]
    Password(#[from] PasswordError),
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

#[derive(Debug, thiserror::Error)]
pub enum AutenticacionError {
    #[error("Credenciales inválidas")]
    CredencialesInvalidas,
    #[error("Usuario inactivo")]
    UsuarioInactivo,
    #[error("El hash almacenado no es válido")]
    HashInvalido,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

#[derive(Debug, thiserror::Error)]
pub enum ContratistaServiceError {
    #[error("Contratista no encontrado")]
    ContratistaNoEncontrado,
    #[error("Empresa no encontrada")]
    EmpresaNoEncontrada,
    #[error("La cédula es obligatoria")]
    CedulaVacia,
    #[error("El nombre es obligatorio")]
    NombreVacio,
    #[error("La fecha de PRAIND es obligatoria")]
    PraindRequerido,
    #[error("La cédula del contratista ya existe")]
    CedulaDuplicada,
    #[error("La sesión actual no está autorizada para realizar esta operación")]
    OperacionNoAutorizada,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

#[derive(Debug, thiserror::Error)]
pub enum EmpresaServiceError {
    #[error("Empresa no encontrada")]
    EmpresaNoEncontrada,
    #[error("El nombre de la empresa es obligatorio")]
    NombreEmpresaVacio,
    #[error("El nombre de la empresa ya existe")]
    NombreDuplicado,
    #[error("La sesión actual no está autorizada para realizar esta operación")]
    OperacionNoAutorizada,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

#[derive(Debug, thiserror::Error)]
pub enum RegistroIngresoServiceError {
    #[error("Contratista no encontrado")]
    ContratistaNoEncontrado,
    #[error("Acceso denegado: {0:?}")]
    AccesoDenegado(MotivoDenegacion),
    #[error("El contratista ya tiene un ingreso activo")]
    IngresoActivo,
    #[error("El contratista requiere gafete")]
    GafeteRequerido,
    #[error("El gafete ya está asignado")]
    GafeteOcupado,
    #[error("El gafete no está asignado actualmente")]
    GafeteNoAsignado,
    #[error("El registro de ingreso no está activo")]
    RegistroNoActivo,
    #[error("La salida no puede ser anterior al ingreso")]
    SalidaAnteriorAIngreso,
    #[error("El reloj del equipo está atrasado respecto al último movimiento registrado")]
    RelojRetrocedido,
    #[error("El rango de fechas del historial no es válido")]
    RangoFechasInvalido,
    /// El usuario que figura como operador del movimiento no existe o está
    /// inactivo — revisado dentro de la misma transacción que el
    /// movimiento, así que una desactivación concurrente no puede colarse
    /// entre la verificación y la escritura.
    #[error("La sesión que registra el movimiento no existe o está inactiva")]
    OperadorNoAutorizado,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}
