use crate::database::error::DatabaseError;
use crate::domain::cita::MotivoDenegacionVisita;
use crate::domain::resultado_acceso::MotivoDenegacion;
use crate::models::gafete::EstadoGafete;
use crate::models::via_ingreso::ViaIngreso;

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
    #[error("Usuario inactivo")]
    UsuarioInactivo,
    /// Sin productor hoy -- existía para `AppCore::fijar_password_inicial`
    /// (alta de contraseña de un usuario global con sólo la cédula, sin
    /// verificar nada contra Supabase), eliminada en la auditoría
    /// 2026-09-24 (NR-07/NS-25) por peligrosa si algo volvía a llamarla.
    /// La variante queda por compatibilidad de la API pública (`mensajes.rs`
    /// todavía la mapea a un mensaje), pero ningún camino actual la
    /// produce -- el alta de contraseña de un usuario global pasa por
    /// Supabase Auth (`login_supabase`/`autenticar_supabase`), no por acá.
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
    #[error("La cédula debe tener sólo números, entre 9 y 13 dígitos")]
    CedulaInvalida,
    #[error("El nombre es obligatorio")]
    NombreVacio,
    #[error("El nombre no puede tener números ni símbolos")]
    NombreInvalido,
    #[error("La fecha de PRAIND es obligatoria")]
    PraindRequerido,
    #[error("El PRAIND está vencido")]
    PraindVencido,
    #[error("Este tipo de ingreso no admite personal de ruta")]
    PersonalRutaNoAdmitido,
    /// "Por correo" dejó de ser un tipo de contratista (pedido del usuario
    /// 2026-10-03): esas visitas se registran como ingreso por correo.
    #[error("El tipo de ingreso ya no se puede elegir")]
    TipoIngresoRetirado,
    #[error("La cédula del contratista ya existe")]
    CedulaDuplicada,
    /// No se cambia la cédula de quien está adentro: su ingreso abierto
    /// quedaría con la cédula vieja, y con la nueva podría volver a entrar
    /// sin que nada lo frene (el "¿ya está adentro?" compara por cédula).
    #[error("No se puede cambiar la cédula de un contratista que está adentro")]
    CedulaConIngresoActivo,
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

/// Las reglas de criterio del contratista viven en el crate compartido
/// (`ErrorContratista`); acá cada motivo se traduce a su variante de
/// siempre, así los mensajes y quien los maneja no cambian.
impl From<crate::domain::contratista::ErrorContratista> for ContratistaServiceError {
    fn from(error: crate::domain::contratista::ErrorContratista) -> Self {
        use crate::domain::contratista::ErrorContratista;
        match error {
            ErrorContratista::CedulaVacia => Self::CedulaVacia,
            ErrorContratista::CedulaInvalida => Self::CedulaInvalida,
            ErrorContratista::NombreVacio => Self::NombreVacio,
            ErrorContratista::NombreInvalido => Self::NombreInvalido,
            ErrorContratista::TipoIngresoRetirado => Self::TipoIngresoRetirado,
            ErrorContratista::PersonalRutaNoAdmitido => Self::PersonalRutaNoAdmitido,
            ErrorContratista::PraindRequerido => Self::PraindRequerido,
            ErrorContratista::PraindVencido => Self::PraindVencido,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RegistroIngresoServiceError {
    #[error("Contratista no encontrado")]
    ContratistaNoEncontrado,
    #[error("Acceso denegado: {0:?}")]
    AccesoDenegado(MotivoDenegacion),
    #[error("El contratista ya tiene un ingreso activo")]
    IngresoActivo,
    #[error("El contratista ya tiene un ingreso activo en el otro dispositivo del sitio")]
    IngresoActivoEnOtroDispositivo,
    /// La misma persona ya está adentro como proveedor o por correo, en este
    /// equipo o en el otro de la unidad (`queries::persona_adentro`).
    #[error("Esta persona ya está adentro por otra vía: {0:?}")]
    AdentroPorOtraVia(ViaIngreso),
    /// El medio de ingreso es `Vehiculo` y no se indicó placa
    /// (`MIGRACION_49`).
    #[error("La placa es obligatoria cuando el ingreso es en vehículo")]
    PlacaRequerida,
    /// El medio de ingreso es `Caminando` pero se indicó una placa -- el
    /// `CHECK` de `registro_ingresos` la rechazaría de todos modos, pero se
    /// valida acá para devolver un error de dominio legible en vez de un
    /// `DatabaseError::Sqlite` crudo.
    #[error("No se puede indicar placa cuando el ingreso es a pie")]
    PlacaNoAplica,
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
    /// Esta cédula tiene el acceso negado como contratista: aunque tenga
    /// una visita agendada, no entra.
    #[error("Esta persona tiene el acceso denegado")]
    AccesoNegado,
    /// Existe al menos una cita para esta cédula, pero ninguna aplica hoy
    /// -- el motivo viaja en la variante (de la cita más relevante, ver
    /// `MotivoDenegacionVisita::relevancia`) para que la interfaz diga
    /// algo útil: "tiene visita para el martes 6 de octubre", "fue
    /// cancelada", "venció el...".
    #[error("No hay ninguna visita vigente para esta cédula: {motivo:?}")]
    SinCitaVigente {
        motivo: MotivoDenegacionVisita,
        /// De la cita que explica el motivo, para que la portería sepa a
        /// quién llamar.
        anfitrion: String,
    },
    /// Esta cédula ya tiene un movimiento de visita abierto -- mismo criterio
    /// que `RegistroIngresoServiceError::IngresoActivo`, no se puede entrar
    /// dos veces sin salir primero. Lleva el nombre del visitante para el
    /// mensaje.
    #[error("Este visitante ya tiene un movimiento activo: {nombre}")]
    VisitanteYaEnSitio { nombre: String },
    /// Eligió "Vehículo" pero no escribió la placa.
    #[error("Falta la placa del vehículo")]
    PlacaRequerida,
    /// La placa tiene más de 20 caracteres o caracteres de control.
    #[error("La placa no es válida")]
    PlacaInvalida,
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

impl CitaServiceError {
    /// No hay cita que valga hoy, pero tampoco algo que lo impida: el
    /// guarda puede registrarla como visita autorizada por correo (con el
    /// correo que la respalde). No se ofrece si el anfitrión la canceló, si
    /// la persona tiene el acceso negado o si ya está adentro.
    pub fn admite_registro_por_correo(&self) -> bool {
        matches!(
            self,
            Self::SinCitaRegistrada
                | Self::SinCitaVigente {
                    motivo: MotivoDenegacionVisita::TodaviaNoEmpieza { .. }
                        | MotivoDenegacionVisita::Vencida { .. },
                    ..
                }
        )
    }

    /// No es una falla: la visita existe pero es para otro día. La pantalla
    /// lo muestra como aviso, no como error (pedido del dueño 2026-10-05).
    pub fn es_informativo(&self) -> bool {
        matches!(
            self,
            Self::SinCitaVigente {
                motivo: MotivoDenegacionVisita::TodaviaNoEmpieza { .. },
                ..
            }
        )
    }
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
    /// Bloqueante a propósito (pedido explícito del usuario, 2026-09-15):
    /// a diferencia de vehículo/encargado, el número de ruta debe existir
    /// en el catálogo (`rutas`) -- mismo criterio que
    /// `RegistroIngresoServiceError::ContratistaNoEncontrado`.
    #[error("El número de ruta no existe en el catálogo")]
    RutaNoEncontrada,
    #[error("El número de ruta está dado de baja")]
    RutaInactiva,
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

/// Catálogo de números de ruta -- mismo molde que `GafeteServiceError`
/// (alta individual/por rango, dar de baja con resguardo si está en uso),
/// pero sin los estados de "portador" que sí tiene un gafete (una ruta no
/// se pierde, sólo se habilita/deshabilita).
#[derive(Debug, thiserror::Error)]
pub enum RutaCatalogoServiceError {
    #[error("El número de ruta debe ser mayor a cero")]
    NumeroInvalido,
    #[error("Ya existe una ruta con ese número")]
    NumeroDuplicado,
    #[error("El rango de números no es válido")]
    RangoInvalido,
    #[error("La ruta ya no existe")]
    RutaNoEncontrada,
    /// Dar de baja una ruta con una salida activa dejaría el catálogo
    /// contradiciendo un movimiento en curso -- mismo criterio que
    /// `GafeteServiceError::GafeteConIngresoActivo`.
    #[error("La ruta tiene una salida activa en este momento")]
    RutaConSalidaActiva,
    #[error("La sesión actual no está autorizada para realizar esta operación")]
    OperacionNoAutorizada,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

/// Entrega/devolución de gafetes provisionales KOF -- mucho más simple que
/// `RutaServiceError`: sin PRAIND, sin bloqueo por documento, sin reloj
/// cruzado entre dominios (pedido explícito del usuario: "no hay más
/// verificación que la humana"). Ver
/// `docs/features-futuras/plan-gafetes-provisionales-kof.md`.
#[derive(Debug, thiserror::Error)]
pub enum GafeteProvisionalServiceError {
    #[error("El número de gafete debe ser mayor a cero")]
    NumeroInvalido,
    #[error("El encargado no existe en el catálogo")]
    EncargadoNoEncontrado,
    #[error("El encargado está dado de baja en el catálogo")]
    EncargadoInactivo,
    #[error("Este encargado ya tiene un gafete provisional prestado")]
    EncargadoYaTienePrestamoActivo,
    #[error("Ese número de gafete ya está prestado a otra persona")]
    GafeteYaPrestado,
    /// El número no existe en el inventario (`gafetes`, tipo
    /// `PROVISIONAL_KOF`).
    #[error("El gafete provisional no está registrado en el catálogo")]
    GafeteNoRegistrado,
    #[error("El gafete provisional no está disponible: {0:?}")]
    GafeteNoDisponible(EstadoGafete),
    #[error("El préstamo no está activo")]
    PrestamoNoActivo,
    #[error("La sesión actual no está autorizada para realizar esta operación")]
    OperacionNoAutorizada,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

/// Catálogo de empresas proveedoras
/// (`docs/features-futuras/plan-control-proveedores.md`) -- mismo molde
/// mínimo que `EmpresaServiceError`, catálogo separado a propósito.
#[derive(Debug, thiserror::Error)]
pub enum EmpresaProveedorServiceError {
    #[error("Empresa proveedora no encontrada")]
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

/// Ingreso/salida de proveedores
/// (`docs/features-futuras/plan-control-proveedores.md`) -- mismo espíritu
/// que `CitaServiceError`: sin PRAIND/bloqueos de contratista, el gafete es
/// siempre obligatorio (a diferencia de `RegistroIngresoServiceError`, que
/// lo hace condicional a `requiere_gafete`).
#[derive(Debug, thiserror::Error)]
pub enum IngresoProveedorServiceError {
    #[error("La cédula es obligatoria")]
    CedulaVacia,
    /// Esta cédula tiene el acceso negado como contratista.
    #[error("Esta persona tiene el acceso denegado")]
    AccesoNegado,
    /// Misma regla que contratistas: sólo cédula nacional o de extranjero
    /// (`Cedula::es_nacional_o_de_extranjero`).
    #[error("La cédula debe tener sólo números, entre 9 y 13 dígitos")]
    CedulaInvalida,
    #[error("El nombre es obligatorio")]
    NombreVacio,
    #[error("Empresa proveedora no encontrada")]
    EmpresaNoEncontrada,
    #[error("La empresa proveedora está dada de baja")]
    EmpresaInactiva,
    #[error("Esta cédula ya tiene un ingreso de proveedor activo")]
    IngresoActivo,
    /// La misma persona ya está adentro como contratista o por correo
    /// (`queries::persona_adentro`).
    #[error("Esta persona ya está adentro por otra vía: {0:?}")]
    AdentroPorOtraVia(ViaIngreso),
    #[error("El gafete ya está asignado a otro ingreso de proveedor")]
    GafeteOcupado,
    /// El número no existe en el catálogo (`gafetes`, tipo `PROVEEDOR`).
    #[error("El gafete no está registrado en el catálogo")]
    GafeteNoRegistrado,
    #[error("El gafete no está disponible: {0:?}")]
    GafeteNoDisponible(EstadoGafete),
    #[error("El ingreso de proveedor no está activo")]
    RegistroNoActivo,
    #[error("La salida no puede ser anterior al ingreso")]
    SalidaAnteriorAIngreso,
    #[error("El reloj del equipo está atrasado respecto al último movimiento registrado")]
    RelojRetrocedido,
    #[error("La sesión que registra el movimiento no existe o está inactiva")]
    OperadorNoAutorizado,
    #[error(transparent)]
    Database(#[from] DatabaseError),
}

/// Ingreso "por correo" (visita autorizada por correo) -- mismo criterio
/// que `IngresoProveedorServiceError`, con motivo en vez de empresa y
/// gafete de visita.
#[derive(Debug, thiserror::Error)]
pub enum IngresoCorreoServiceError {
    #[error("La cédula es obligatoria")]
    CedulaVacia,
    /// Esta cédula tiene el acceso negado como contratista.
    #[error("Esta persona tiene el acceso denegado")]
    AccesoNegado,
    #[error("La cédula debe tener sólo números, entre 9 y 13 dígitos")]
    CedulaInvalida,
    #[error("El nombre es obligatorio")]
    NombreVacio,
    #[error("El motivo de la visita es obligatorio")]
    MotivoVacio,
    #[error("Esta cédula ya tiene un ingreso por correo activo")]
    IngresoActivo,
    /// La misma persona ya está adentro como contratista o como proveedor
    /// (`queries::persona_adentro`).
    #[error("Esta persona ya está adentro por otra vía: {0:?}")]
    AdentroPorOtraVia(ViaIngreso),
    #[error("El gafete de visita ya está en uso")]
    GafeteOcupado,
    /// El número no existe en el catálogo (`gafetes`, tipo `VISITA`).
    #[error("El gafete no está registrado en el catálogo")]
    GafeteNoRegistrado,
    #[error("El gafete no está disponible: {0:?}")]
    GafeteNoDisponible(EstadoGafete),
    #[error("El ingreso por correo no está activo")]
    RegistroNoActivo,
    #[error("La salida no puede ser anterior al ingreso")]
    SalidaAnteriorAIngreso,
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
    #[error("Visitante no encontrado")]
    VisitaNoEncontrada,
    #[error("Encargado de ruta no encontrado")]
    EncargadoRutaNoEncontrado,
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
