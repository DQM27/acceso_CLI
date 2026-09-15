use crate::database::error::DatabaseError;
use crate::domain::cita::MotivoDenegacionVisita;
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
    /// Ver `AppCore::fijar_password_inicial` -- una cuenta desactivada no
    /// puede arrancar sesión en un dispositivo nuevo fijando una contraseña,
    /// igual que tampoco podría con una contraseña ya fijada.
    #[error("Usuario inactivo")]
    UsuarioInactivo,
    /// Ídem -- `fijar_password_inicial` sólo es válido mientras el usuario
    /// siga con el centinela `SIN_PASSWORD_LOCAL`; una vez fijada, cambiarla
    /// pasa por `cambiar_password_propio`/`cambiar_password` (gestión), no
    /// por acá.
    #[error("Este usuario ya tiene contraseña en este dispositivo")]
    YaTienePasswordLocal,
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
    /// Usuario global (sincronizado, ver `nube::sincronizacion::recibir_usuarios`)
    /// que todavía no fijó contraseña en este dispositivo en particular --
    /// distinto de `CredencialesInvalidas`: acá no hubo contraseña
    /// incorrecta, todavía no existe ninguna que verificar. El llamador
    /// decide cómo pedir la contraseña nueva (`UsuarioService::cambiar_password`,
    /// que no exige conocer la anterior).
    #[error("Este usuario no tiene contraseña en este dispositivo todavía")]
    SinPasswordLocal,
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
pub enum CitaServiceError {
    /// Esta cédula no aparece en NINGUNA cita conocida por este
    /// dispositivo -- distinto de `SinCitaVigente`: acá no hay nada que
    /// mostrarle al guardia (ni anfitrión, ni motivo), la persona
    /// simplemente no tiene ninguna visita agendada.
    #[error("No hay ninguna visita agendada para esta cédula")]
    SinCitaRegistrada,
    /// Existe al menos una cita para esta cédula, pero ninguna aplica hoy
    /// -- el motivo viaja en la variante (de la última candidata
    /// evaluada) para que la interfaz pueda mostrar algo más útil que
    /// "no se puede" (ej. "esta cita fue cancelada" o "esta cita ya
    /// venció").
    #[error("No hay ninguna visita vigente para esta cédula: {0:?}")]
    SinCitaVigente(MotivoDenegacionVisita),
    /// Este visitante ya tiene un movimiento abierto -- mismo criterio que
    /// `RegistroIngresoServiceError::IngresoActivo`, no se puede entrar dos
    /// veces sin salir primero.
    #[error("Este visitante ya tiene un movimiento activo")]
    VisitanteYaEnSitio,
    /// El gafete ya está asignado a otro movimiento de visita abierto --
    /// mismo criterio que `RegistroIngresoServiceError::GafeteOcupado`.
    #[error("El gafete ya está asignado a otra visita")]
    GafeteOcupado,
    /// El número no existe en el catálogo (`gafetes`, tipo `VISITA`) --
    /// mismo criterio que `RegistroIngresoServiceError::GafeteNoRegistrado`.
    #[error("El gafete no está registrado en el catálogo")]
    GafeteNoRegistrado,
    /// El gafete existe pero no está `Disponible` (perdido o de baja).
    #[error("El gafete no está disponible: {0:?}")]
    GafeteNoDisponible(EstadoGafete),
    #[error("El movimiento no está activo")]
    MovimientoNoActivo,
    #[error("La salida no puede ser anterior a la entrada")]
    SalidaAnteriorAEntrada,
    /// Mismo criterio que `RegistroIngresoServiceError::RelojRetrocedido`:
    /// comprobación de sanidad de todo el sistema (¿el reloj de la máquina
    /// retrocedió respecto al último movimiento conocido?), no una regla de
    /// negocio de una entrada/salida puntual -- la genera `AppCore`, no
    /// `CitaService`.
    #[error("El reloj del equipo está atrasado respecto al último movimiento registrado")]
    RelojRetrocedido,
    /// Mismo criterio que `RegistroIngresoServiceError::OperadorNoAutorizado`.
    #[error("La sesión que registra el movimiento no existe o está inactiva")]
    OperadorNoAutorizado,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

#[derive(Debug, thiserror::Error)]
pub enum RutaServiceError {
    #[error("La placa del vehículo es obligatoria")]
    PlacaVacia,
    #[error("El nombre del encargado es obligatorio")]
    EncargadoVacio,
    #[error("El número de documento es obligatorio")]
    NumeroDocumentoVacio,
    /// Mismo criterio que `RegistroIngresoServiceError::IngresoActivo`, pero
    /// por placa (texto), no por catálogo -- un vehículo sin match en
    /// `vehiculos_ruta` igual puede quedar "ya en ruta".
    #[error("Este vehículo ya tiene una salida de ruta activa")]
    VehiculoYaEnRuta,
    #[error("Ya existe una salida registrada con ese número de documento")]
    DocumentoYaRegistrado,
    /// La fecha del documento no coincide con hoy y no se marcó tener el
    /// correo de autorización -- la UI ya debería haber bloqueado el botón
    /// de confirmar antes de llegar acá (ver
    /// `docs/planes-implementados/plan-control-rutas.md`, "Bloqueo
    /// transitorio por documento vencido"); esto es el resguardo del lado
    /// del servicio, mismo espíritu que `RegistroIngresoService` no confía
    /// en una verificación previa de la pantalla.
    #[error("El documento no es de hoy y no se indicó tener el correo de autorización")]
    DocumentoRequiereAutorizacion,
    #[error("La salida de ruta no está activa")]
    SalidaNoActiva,
    #[error("El retorno no puede ser anterior a la salida")]
    RetornoAnteriorASalida,
    /// Comprobación de sanidad de todo el sistema (¿el reloj de la máquina
    /// retrocedió respecto al último movimiento conocido, de cualquier
    /// dominio?), no una regla de negocio de una salida puntual -- mismo
    /// criterio que `RegistroIngresoServiceError::RelojRetrocedido`/
    /// `CitaServiceError::RelojRetrocedido`. La genera
    /// `application::rutas`, no `RutaService`.
    #[error("El reloj del equipo está atrasado respecto al último movimiento registrado")]
    RelojRetrocedido,
    /// Mismo criterio que `CitaServiceError::OperadorNoAutorizado`/
    /// `RegistroIngresoServiceError::OperadorNoAutorizado`.
    #[error("La sesión que registra el movimiento no existe o está inactiva")]
    OperadorNoAutorizado,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

/// Catálogo de vehículos de ruta -- mismo molde mínimo que
/// `EmpresaServiceError`, sin reglas de negocio propias más allá de "el
/// actor sigue activo" y lo que ya exige el esquema (placa/número de
/// unidad únicos).
#[derive(Debug, thiserror::Error)]
pub enum VehiculoRutaServiceError {
    #[error("Su sesión no está autorizada para esta operación")]
    OperacionNoAutorizada,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

/// Catálogo de encargados de ruta (personal KOF) -- mismo criterio que
/// `VehiculoRutaServiceError`.
#[derive(Debug, thiserror::Error)]
pub enum EncargadoRutaServiceError {
    #[error("Su sesión no está autorizada para esta operación")]
    OperacionNoAutorizada,
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
    #[error("Visitante no encontrado")]
    VisitaNoEncontrada,
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
