use crate::database::error::DatabaseError;
use crate::domain::resultado_acceso::MotivoDenegacion;
use crate::models::gafete::EstadoGafete;

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
    /// El número no existe en el catálogo (`gafetes`) — distinto de
    /// `GafeteOcupado` (existe, pero ya está en uso en otro ingreso activo).
    #[error("El gafete no está registrado en el catálogo")]
    GafeteNoRegistrado,
    /// Existe en el catálogo pero su estado actual no permite asignarlo
    /// (`Perdido`/`DeBaja`) — el estado concreto viaja en la variante para
    /// que cada interfaz arme su propio mensaje sin volver a consultar.
    #[error("El gafete no está disponible: {0:?}")]
    GafeteNoDisponible(EstadoGafete),
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

#[derive(Debug, thiserror::Error)]
pub enum GafeteServiceError {
    #[error("El número de gafete debe ser mayor a cero")]
    NumeroInvalido,
    #[error("Ya existe un gafete con ese número")]
    NumeroDuplicado,
    #[error("Gafete no encontrado")]
    GafeteNoEncontrado,
    #[error("El rango de números no es válido")]
    RangoInvalido,
    #[error("Marcar un gafete perdido requiere indicar el contratista deudor")]
    ContratistaDeudorRequerido,
    #[error("Contratista no encontrado")]
    ContratistaNoEncontrado,
    /// La transición pedida no aplica al estado actual (ej. dar de baja uno
    /// ya perdido, o resolver uno que no está perdido).
    #[error("El gafete no está en un estado válido para esta operación")]
    EstadoInvalido,
    /// Hay un ingreso activo con este gafete asignado — dar de baja o
    /// marcar perdido dejaría el inventario contradiciendo un movimiento en
    /// curso. Se revisa dentro de la misma transacción que la transición.
    #[error("El gafete está asignado a un ingreso activo")]
    GafeteConIngresoActivo,
    #[error("La sesión actual no está autorizada para realizar esta operación")]
    OperacionNoAutorizada,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}
