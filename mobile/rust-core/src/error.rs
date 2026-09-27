//! `NucleoError`: el único error que ve Kotlin, y su traducción desde cada error del núcleo.

use control_acceso::application::GestionNubeError as GestionNubeErrorNucleo;
use control_acceso::services::error::AutenticacionError as AutenticacionErrorNucleo;
use control_acceso::services::error::ContratistaServiceError as ContratistaServiceErrorNucleo;
use control_acceso::services::error::EmpresaProveedorServiceError as EmpresaProveedorServiceErrorNucleo;
use control_acceso::services::error::EmpresaServiceError as EmpresaServiceErrorNucleo;
use control_acceso::services::error::GafeteProvisionalServiceError as GafeteProvisionalServiceErrorNucleo;
use control_acceso::services::error::IngresoProveedorServiceError as IngresoProveedorServiceErrorNucleo;
use control_acceso::services::error::RegistroIngresoServiceError as RegistroIngresoServiceErrorNucleo;
use control_acceso::services::error::RutaServiceError as RutaServiceErrorNucleo;
use control_acceso::services::error::UsuarioServiceError as UsuarioServiceErrorNucleo;

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum NucleoError {
    #[error("no se pudo abrir la base de datos: {mensaje}")]
    Apertura { mensaje: String },
    #[error("credenciales inválidas")]
    CredencialesInvalidas,
    #[error("usuario inactivo")]
    UsuarioInactivo,
    /// Usuario global (sincronizado) que todavía no fijó contraseña en
    /// este teléfono. `Nucleo::autenticar`/`autenticar_con_secreto`
    /// interceptan esto internamente y redirigen a `autenticar_supabase`
    /// (ver el comentario ahí) -- Kotlin nunca ve este error para un
    /// usuario global. La variante sigue existiendo por el contrato FFI
    /// -- confirmado en la auditoría 2026-09-24 (NR-07/NS-25) que el único
    /// camino que fijaba contraseña con sólo la cédula era
    /// `AppCore::fijar_password_inicial`, ya eliminada por no tener
    /// llamadores reales; el alta de contraseña de un usuario global pasa
    /// siempre por `autenticar_supabase` (Supabase Auth), así que esta
    /// variante no debería escapar sin interceptar desde acá.
    #[error("todavía no tenés contraseña en este dispositivo")]
    SinPasswordLocal,
    #[error("no hay una sesión iniciada")]
    NoAutenticado,
    /// La sesión de Supabase Auth (la de la PERSONA, distinta del token del
    /// DISPOSITIVO) venció o nunca se abrió -- Kotlin debe mandar de vuelta
    /// al login. Ver `Nucleo::cambiar_password_supabase`.
    #[error("la sesión de nube venció -- iniciá sesión de nuevo")]
    SesionSupabaseVencida,
    #[error("fecha de PRAIND inválida: {mensaje}")]
    FechaInvalida { mensaje: String },
    /// El gafete ya está activo en este sitio del lado de OTRO
    /// dispositivo -- chequeo en vivo (`application::registrar_ingreso_verificado`),
    /// nunca llega a tocar `registrar_ingreso` en el núcleo, se corta acá
    /// mismo. Reemplaza `GafeteOcupadoEnSitioException`, que antes vivía
    /// sólo del lado de Kotlin (`PantallaConfirmarIngreso.kt`).
    #[error("El gafete {numero} ya está en uso en otro dispositivo de la unidad operativa")]
    GafeteOcupadoEnSitio { numero: i64 },
    /// La cédula ya tiene un ingreso de proveedor abierto en OTRO sitio
    /// (chequeo en vivo contra la nube, de mejor esfuerzo). Mismo texto que
    /// `desktop/src-tauri/src/comandos/proveedores.rs`.
    #[error("Esta cédula ya tiene un ingreso de proveedor activo en {sitio}")]
    ProveedorActivoEnOtroSitio { sitio: String },
    /// Una regla de negocio rechazó la operación (dato inválido, PRAIND
    /// vencido, ...). `mensaje` ya viene listo para mostrar tal cual, sin
    /// prefijo técnico -- sale de `control_acceso::mensajes`.
    #[error("{mensaje}")]
    Rechazado { mensaje: String },
    #[error("error interno: {mensaje}")]
    Interno { mensaje: String },
}

/// Punto de paso de casi todo `NucleoError::Interno` de este archivo --
/// loguea el detalle técnico antes de convertirlo a texto para Kotlin
/// (ver `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`, punto
/// 5.1: mismo criterio que `mensaje_generico` del lado de escritorio y las
/// variantes técnicas de `control_acceso::mensajes::mensaje_*`). Sin
/// backend de logging instalado (`inicializar_logging`, más abajo, todavía
/// no se llamó desde Kotlin) esto es no-op: no cambia nada por sí solo.
pub fn interno<E: std::fmt::Display>(error: E) -> String {
    let mensaje = error.to_string();
    log::error!("{mensaje}");
    mensaje
}

impl From<AutenticacionErrorNucleo> for NucleoError {
    fn from(error: AutenticacionErrorNucleo) -> Self {
        match error {
            AutenticacionErrorNucleo::CredencialesInvalidas => Self::CredencialesInvalidas,
            AutenticacionErrorNucleo::UsuarioInactivo => Self::UsuarioInactivo,
            AutenticacionErrorNucleo::SinPasswordLocal => Self::SinPasswordLocal,
            otro => Self::Interno {
                mensaje: interno(otro),
            },
        }
    }
}

impl From<control_acceso::nube::AuthSupabaseError> for NucleoError {
    fn from(error: control_acceso::nube::AuthSupabaseError) -> Self {
        match error {
            control_acceso::nube::AuthSupabaseError::CredencialesInvalidas => {
                Self::CredencialesInvalidas
            }
            otro => Self::Interno {
                mensaje: interno(otro),
            },
        }
    }
}

/// Distingue "el token de dispositivo cacheado quedó vencido a mitad de
/// camino" (ver `SincronizacionError::token_dispositivo_vencido`, mismo
/// patrón que `desktop/src-tauri/src/comandos/nube.rs::FalloSincronizacion`)
/// de cualquier otro fallo -- sólo el primero amerita invalidar el caché de
/// `Nucleo` y reintentar la sincronización una vez. No es un tipo `uniffi`
/// (nunca cruza la frontera FFI, sólo vive entre `intentar_sincronizar_*` y
/// su wrapper con reintento).
pub enum FalloSincronizacion {
    TokenVencido,
    Nucleo(NucleoError),
}

impl From<control_acceso::nube::SincronizacionError> for FalloSincronizacion {
    fn from(error: control_acceso::nube::SincronizacionError) -> Self {
        if error.token_dispositivo_vencido() {
            Self::TokenVencido
        } else {
            Self::Nucleo(NucleoError::Interno {
                mensaje: interno(error),
            })
        }
    }
}

impl From<NucleoError> for FalloSincronizacion {
    fn from(error: NucleoError) -> Self {
        Self::Nucleo(error)
    }
}

impl From<GestionNubeErrorNucleo> for FalloSincronizacion {
    fn from(error: GestionNubeErrorNucleo) -> Self {
        Self::Nucleo(NucleoError::from(error))
    }
}

pub fn convertir_fallo_sincronizacion(fallo: FalloSincronizacion) -> NucleoError {
    match fallo {
        FalloSincronizacion::TokenVencido => NucleoError::Interno {
            mensaje:
                "El token de este dispositivo venció y no se pudo renovar -- revisá la conexión"
                    .to_string(),
        },
        FalloSincronizacion::Nucleo(error) => error,
    }
}

/// Mismo criterio que contratistas: las reglas llegan con su mensaje
/// (`mensaje_ingreso`), sólo la falla de base es `Interno`.
impl From<RegistroIngresoServiceErrorNucleo> for NucleoError {
    fn from(error: RegistroIngresoServiceErrorNucleo) -> Self {
        match error {
            RegistroIngresoServiceErrorNucleo::Database(_) => Self::Interno {
                mensaje: interno(error),
            },
            regla => Self::Rechazado {
                mensaje: control_acceso::mensajes::mensaje_ingreso(regla),
            },
        }
    }
}

/// Las reglas del alta se muestran tal cual a quien opera; sólo la falla
/// de base queda como `Interno`.
impl From<ContratistaServiceErrorNucleo> for NucleoError {
    fn from(error: ContratistaServiceErrorNucleo) -> Self {
        match error {
            ContratistaServiceErrorNucleo::Database(_) => Self::Interno {
                mensaje: interno(error),
            },
            regla => Self::Rechazado {
                mensaje: control_acceso::mensajes::mensaje_contratista(regla),
            },
        }
    }
}

impl From<EmpresaServiceErrorNucleo> for NucleoError {
    fn from(error: EmpresaServiceErrorNucleo) -> Self {
        Self::Interno {
            mensaje: interno(error),
        }
    }
}

impl From<UsuarioServiceErrorNucleo> for NucleoError {
    fn from(error: UsuarioServiceErrorNucleo) -> Self {
        Self::Interno {
            mensaje: interno(error),
        }
    }
}

impl From<RutaServiceErrorNucleo> for NucleoError {
    fn from(error: RutaServiceErrorNucleo) -> Self {
        Self::Interno {
            mensaje: interno(error),
        }
    }
}

/// Mismo criterio que contratistas y proveedores.
impl From<GafeteProvisionalServiceErrorNucleo> for NucleoError {
    fn from(error: GafeteProvisionalServiceErrorNucleo) -> Self {
        match error {
            GafeteProvisionalServiceErrorNucleo::Database(_) => Self::Interno {
                mensaje: interno(error),
            },
            regla => Self::Rechazado {
                mensaje: control_acceso::mensajes::mensaje_gafete_provisional(regla),
            },
        }
    }
}

impl From<EmpresaProveedorServiceErrorNucleo> for NucleoError {
    fn from(error: EmpresaProveedorServiceErrorNucleo) -> Self {
        Self::Interno {
            mensaje: interno(error),
        }
    }
}

/// Mismo criterio que contratistas: las reglas llegan con su mensaje,
/// sólo la falla de base es `Interno`.
impl From<IngresoProveedorServiceErrorNucleo> for NucleoError {
    fn from(error: IngresoProveedorServiceErrorNucleo) -> Self {
        match error {
            IngresoProveedorServiceErrorNucleo::Database(_) => Self::Interno {
                mensaje: interno(error),
            },
            regla => Self::Rechazado {
                mensaje: control_acceso::mensajes::mensaje_ingreso_proveedor(regla),
            },
        }
    }
}

impl From<GestionNubeErrorNucleo> for NucleoError {
    fn from(error: GestionNubeErrorNucleo) -> Self {
        Self::Interno {
            mensaje: control_acceso::mensajes::mensaje_gestion_nube(error),
        }
    }
}
