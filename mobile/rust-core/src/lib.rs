//! Puente `uniffi` sobre `control_acceso`: expone al Kotlin de la app móvil
//! sólo lo puntual que cada pantalla necesita, sin tocar la lógica del
//! crate raíz. Ver docs/plan-app-movil.md.

// Frontera FFI hacia Kotlin -- no hay ninguna razón legítima para que este
// crate en particular (a diferencia de la raíz o `desktop/src-tauri`, que sí
// llaman DPAPI/COM de Windows) necesite `unsafe`. `forbid`, no `deny`: ni
// siquiera un `#[allow(unsafe_code)]` local puede reabrirlo por accidente.
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::sync::Mutex;

use control_acceso::application::AppCore;
use control_acceso::services::autenticacion_service::UsuarioSesion as UsuarioSesionNucleo;
use control_acceso::tiempo::RelojCorregido;

mod mrz;
pub use mrz::{CampoMrz, CorreccionAplicada, FechaMrz, FormatoMrz, RegistroMrz, leer_mrz};
mod pdf417_cedula;
pub use pdf417_cedula::{
    DatosPdf417Cedula, LecturaPdf417, MotivoPdf417, largo_prefijo_pdf417_cedula,
    leer_pdf417_cedula, leer_pdf417_cedula_con_motivo,
};
mod votacion;
pub use votacion::{ConsensoVotacion, VotadorPorPosicion};
mod lectura_documentos;
pub use lectura_documentos::*;

uniffi::setup_scaffolding!();

mod catalogo;
mod error;
mod gafetes;
mod ingresos;
mod nube;
mod proveedores;
mod rutas;
mod sesion;
mod tipos;

pub use error::NucleoError;
use error::{FalloSincronizacion, convertir_fallo_sincronizacion, interno};
pub use tipos::{
    ConflictoGafeteActivo, ConflictoIngresoActivo, ConflictoIngresoProveedorActivo,
    ContratistaResumen, DatosContratista, DatosUsuario, Empresa, EmpresaProveedor, EncargadoRuta,
    IngresoActivoResumen, IngresoProveedorRemoto, IngresoRemoto, MedioIngreso, ModoBusquedaActivos,
    MotivoDenegacion, MotivoResultadoIngreso, PreparacionIngreso,
    PrestamoGafeteProvisionalActivoResumen, PrestamoGafeteProvisionalRemoto,
    RegistroIngresoProveedorActivoResumen, ResultadoAcceso, ResultadoIngresoRegistrado,
    ResultadoLogin, ResultadoRegistroEntrada, ResultadoRegistroSalidaRuta, ResultadoSalidaRuta,
    ResumenSincronizacion, RolUsuario, Ruta, SalidaRutaActivaResumen, SesionRealtimeNube,
    SolicitudSalidaRuta, TipoIngreso, UsuarioResumen, UsuarioSesion, VehiculoRuta,
};

/// Sesión de un usuario global contra Supabase Auth (Administrador/Operador,
/// o un ROOT ya sincronizado a otro sitio) -- ver
/// docs/planes-implementados/plan-autenticacion-supabase-auth.md. Distinta del
/// `TokenDispositivo` que cachea `Nucleo::cache_token`
/// (identidad del DISPOSITIVO): esto es la identidad de la PERSONA. Vive
/// sólo en memoria -- nunca se persiste a disco, mismo criterio que
/// `desktop/src-tauri/src/estado.rs::SesionSupabaseCacheada`: cerrar la app
/// siempre la pierde y el próximo arranque exige un login real de nuevo.
struct SesionSupabaseCacheada {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    /// Última vez que se confirmó de verdad contra Supabase -- la base del
    /// tope duro de `TOPE_PRESENCIA_SUPABASE`.
    confirmada_en: std::time::Instant,
}

/// Mismo tope que escritorio (ver `estado.rs`) -- aplicado por el cliente,
/// no depende de la configuración de expiración del proyecto de Supabase.
const TOPE_PRESENCIA_SUPABASE: std::time::Duration = std::time::Duration::from_secs(12 * 60 * 60);

/// Sesión del núcleo: dueña de la única conexión `SQLite` del teléfono. Se
/// abre una vez al arrancar la app y se reusa en todas las pantallas (login,
/// buscar contratista, registrar entrada/salida) — nunca se reabre por
/// pantalla.
#[derive(uniffi::Object)]
pub struct Nucleo {
    core: Mutex<AppCore>,
    /// Actor autenticado — lo necesitan `registrar_ingreso`/`registrar_salida`
    /// como `usuario_ingreso_id`/`usuario_salida_id`. Se llena en
    /// `autenticar_con_secreto` y vive mientras dure el proceso (no hay "cerrar sesión"
    /// todavía en el piloto).
    sesion: Mutex<Option<UsuarioSesionNucleo>>,
    /// Caché del último `TokenDispositivo`, deliberadamente FUERA del
    /// `Mutex<AppCore>` de arriba -- ver el doc-comment de
    /// `control_acceso::nube::CacheTokenDispositivo`. Antes de esto,
    /// el login y el chequeo de gafete llamaban a los métodos de
    /// red de `AppCore` a través de `core_lock()`, que quedaba tomado
    /// durante toda la llamada HTTP: cualquier otra pantalla (buscar,
    /// listar activos, otro registro) se quedaba esperando ese mismo
    /// candado mientras tanto -- se sentía como que la app se congelaba al
    /// iniciar sesión o al confirmar un ingreso con gafete, sobre todo si
    /// la sincronización periódica estaba en curso al mismo tiempo.
    cache_token: control_acceso::nube::CacheTokenDispositivo,
    /// Serializa las sincronizaciones completas (`sincronizar_con_nube_con_secreto`,
    /// llamada desde el timer periódico, un aviso Realtime Y el botón
    /// manual -- ver `SincronizacionPeriodica.kt`/`NubeViewModel.kt`) para
    /// que nunca corran dos en simultáneo pisándose la cola de salida --
    /// mismo motivo que el `static SINCRONIZACION: Mutex<()>` de
    /// `desktop/src-tauri/src/comandos/nube.rs::ejecutar_sincronizacion`.
    /// Deliberadamente NO es el mismo candado que `core`: mientras una
    /// sincronización espera acá (o corre su red), cualquier búsqueda o
    /// registro sigue andando con total normalidad.
    sincronizacion_en_curso: Mutex<()>,
    /// Capturada una sola vez en `abrir` -- el núcleo ya no expone
    /// `ruta_base_datos()` como método (ver `database::connection::ruta_base_datos`,
    /// que resuelve el path por defecto; acá ya llega como parámetro del
    /// constructor). Misma idea que `GuiState::ruta_base_datos` en escritorio.
    ruta_base_datos: PathBuf,
    /// MV-03 (auditoría 2026-09-24) -- `None` si se abrió con `abrir()`
    /// (sin cifrar), `Some` si se abrió con `abrir_cifrado()`. Bug real
    /// encontrado en la prueba de fuego en dispositivo (2026-09-26):
    /// `conexion_secundaria()` pasaba `None` siempre, sin importar con qué
    /// clave se hubiera abierto la conexión principal -- toda la
    /// sincronización (que usa una `Connection` secundaria para no
    /// competir por el `Mutex<AppCore>` de la principal) intentaba abrir
    /// el archivo YA cifrado sin clave, y `SQLite` lo rechazaba con "file
    /// is not a database" apenas se tocaba cualquier tabla. Guardada acá
    /// para que esa conexión secundaria pueda aplicar la misma clave que
    /// ya usa la principal.
    clave: Option<[u8; 32]>,
    /// Ver `SesionSupabaseCacheada`.
    sesion_supabase: Mutex<Option<SesionSupabaseCacheada>>,
}

/// Backend real de `log` (ver `interno()` más arriba) -- vuelca a Logcat,
/// filtrable con `adb logcat -s control_acceso_mobile`. Se llama desde
/// ambos constructores (`abrir`/`abrir_cifrado`), lo primero que Kotlin
/// invoca (ver `ARQUITECTURA.md`) -- sin backend, todo `log::` de este
/// crate era no-op. Mismo criterio de nivel que
/// `configurar_plugins_condicionales` en escritorio: más ruido en debug,
/// sólo advertencias/errores reales en release. `init_once` tolera
/// llamadas repetidas (no rompe si Kotlin llega a instanciar `Nucleo` más
/// de una vez en el mismo proceso).
fn iniciar_log_android() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(if cfg!(debug_assertions) {
                log::LevelFilter::Info
            } else {
                log::LevelFilter::Warn
            })
            .with_tag("control_acceso_mobile"),
    );
}

/// Borra el archivo de base de datos y sus sidecars WAL/journal -- MV-03
/// (auditoría 2026-09-24), usado por `Nucleo::abrir_cifrado` cuando el
/// archivo existente no es legible con la clave nueva (ver su
/// doc-comment). Mejor esfuerzo: un `remove_file` que falla porque el
/// sidecar no existe no debe abortar nada.
fn borrar_archivo_y_sidecars(ruta_base_datos: &str) {
    let base = std::path::Path::new(ruta_base_datos);
    let _ = std::fs::remove_file(base);
    for sufijo in ["-wal", "-shm", "-journal"] {
        let mut ruta_sidecar = base.as_os_str().to_owned();
        ruta_sidecar.push(sufijo);
        let _ = std::fs::remove_file(std::path::Path::new(&ruta_sidecar));
    }
}

#[uniffi::export]
impl Nucleo {
    #[uniffi::constructor]
    pub fn abrir(ruta_base_datos: String) -> Result<Self, NucleoError> {
        iniciar_log_android();

        // `RelojCorregido`, no `RelojSistema` -- un teléfono con la hora mal
        // puesta manualmente (o sin datos/GPS para que Android la ajuste
        // solo) tiene el mismo problema que se vio en escritorio: cada
        // autenticación contra la nube mide el desfase real y lo aplica acá
        // (ver `application::nube::AppCore::actualizar_desfase_reloj`).
        let core = AppCore::abrir_con_reloj(
            &ruta_base_datos,
            std::sync::Arc::new(RelojCorregido::nuevo()),
        )
        .map_err(|origen| NucleoError::Apertura {
            mensaje: interno(origen),
        })?;
        Ok(Self::desde_core(&ruta_base_datos, core, None))
    }

    /// MV-03 (auditoría 2026-09-24): variante cifrada de [`Self::abrir`] --
    /// `clave` es la clave AES de 32 bytes que Kotlin resuelve del Android
    /// Keystore (`AndroidKeystoreClaveBaseDatosStore.kt`), nunca derivada
    /// acá. Único punto de entrada real desde `AplicacionViewModel`; `abrir`
    /// se queda sin tocar para los tests de Kotlin (`NucleoDePrueba`).
    ///
    /// Si el archivo en `ruta_base_datos` ya existe pero NO es legible con
    /// esta clave -- el caso real de todo teléfono con la app instalada
    /// antes de este cambio, que hoy tiene la base en texto plano -- se
    /// descarta y se reconstruye vacía en vez de migrar el archivo byte a
    /// byte. Decisión explícita del usuario (2026-09-26): la app ya
    /// depende de la sincronización con la nube como fuente de verdad
    /// (mismo criterio que `confirmar_reconstruccion_desde_nube` en
    /// desktop/src-tauri/src/lib.rs para un archivo dañado); el costo es
    /// perder el historial/auditoría LOCAL de ese dispositivo que todavía
    /// no se hubiera subido, a cambio de no escribir ni probar en este
    /// momento una migración byte a byte sin verificar todavía contra un
    /// dispositivo real.
    #[uniffi::constructor]
    pub fn abrir_cifrado(ruta_base_datos: String, clave: Vec<u8>) -> Result<Self, NucleoError> {
        iniciar_log_android();

        let clave: [u8; 32] = clave.try_into().map_err(|_| NucleoError::Interno {
            mensaje: "La clave de cifrado debe tener 32 bytes".to_string(),
        })?;

        match Self::abrir_cifrado_intento(&ruta_base_datos, &clave) {
            Ok(nucleo) => Ok(nucleo),
            Err(error) if std::path::Path::new(&ruta_base_datos).exists() => {
                log::warn!(
                    "abrir_cifrado: archivo existente no legible con la clave nueva ({error:?}), \
                     se descarta y se reconstruye vacío"
                );
                borrar_archivo_y_sidecars(&ruta_base_datos);
                Self::abrir_cifrado_intento(&ruta_base_datos, &clave)
            }
            Err(error) => Err(error),
        }
    }
}

impl Nucleo {
    /// Helper de `abrir_cifrado` -- separado en su propia función (no
    /// dentro del `impl` exportado por uniffi de más arriba) porque
    /// `#[uniffi::export]` no soporta funciones asociadas sin `&self` que
    /// no sean `#[uniffi::constructor]` (error real visto al probar esto:
    /// "associated functions are not currently supported", que además
    /// rompía la exportación de TODO el resto del `impl`, no sólo la de
    /// esta función).
    fn abrir_cifrado_intento(ruta_base_datos: &str, clave: &[u8; 32]) -> Result<Self, NucleoError> {
        let core = AppCore::abrir_con_reloj_cifrado(
            ruta_base_datos,
            clave,
            std::sync::Arc::new(RelojCorregido::nuevo()),
        )
        .map_err(|origen| NucleoError::Apertura {
            mensaje: interno(origen),
        })?;
        Ok(Self::desde_core(ruta_base_datos, core, Some(*clave)))
    }

    /// Construye `Self` a partir de un `AppCore` ya abierto -- compartido
    /// por `abrir`/`abrir_cifrado_intento`. Mismo motivo que la función de
    /// arriba para no vivir en el `impl` exportado. `clave` se guarda
    /// (bug real encontrado en la prueba de fuego en dispositivo,
    /// 2026-09-26: `conexion_secundaria()` pasaba `None` siempre, sin
    /// importar con qué clave se hubiera abierto la principal) para que
    /// `conexion_secundaria()` pueda aplicar la misma clave.
    fn desde_core(ruta_base_datos: &str, core: AppCore, clave: Option<[u8; 32]>) -> Self {
        Self {
            core: Mutex::new(core),
            sesion: Mutex::new(None),
            cache_token: control_acceso::nube::CacheTokenDispositivo::new(),
            sincronizacion_en_curso: Mutex::new(()),
            ruta_base_datos: PathBuf::from(ruta_base_datos),
            clave,
            sesion_supabase: Mutex::new(None),
        }
    }

    /// Recupera el guard aunque el mutex haya quedado "envenenado" (un
    /// panic anterior mientras alguien lo sostenía) en vez de propagar ese
    /// panic a cada llamada futura — con `uniffi` cada método público es una
    /// frontera FFI: un solo bug de una llamada no debe dejar inservibles
    /// todas las demás pantallas hasta reiniciar la app. `AppCore` no deja
    /// datos a medio escribir visibles tras un panic a mitad de operación
    /// (`SQLite` ya maneja sus propias transacciones), así que el estado
    /// recuperado sigue siendo válido para seguir operando.
    fn core_lock(&self) -> std::sync::MutexGuard<'_, AppCore> {
        self.core
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Reusa el último `TokenDispositivo` mientras siga vigente en vez de
    /// autenticar de cero -- delega en `Nucleo::cache_token`, que
    /// reemplaza para móvil lo que antes hacía `AppCore::autenticar_con_cache`
    /// (ver el comentario de ese campo). La red corre sin `core_lock()`;
    /// sólo se toma DESPUÉS, para aplicar el desfase de reloj medido (ver
    /// [`Nucleo::aplicar_desfase_de`]). Quien llama no debe tener tomado
    /// `core_lock()`.
    fn autenticar_con_cache(
        &self,
        secreto: &str,
    ) -> Result<control_acceso::nube::TokenDispositivo, control_acceso::nube::NubeError> {
        let token = self.cache_token.autenticar_con_cache(secreto)?;
        self.aplicar_desfase_de(&token);
        Ok(token)
    }

    /// Igual que [`Nucleo::autenticar_con_cache`], pero permite adjuntar
    /// `metadata` cuando hace falta mandarla (sólo la activación inicial,
    /// ver [`Nucleo::configurar_dispositivo_inicial_con_secreto`]). El resto
    /// de los llamadores pasan `None` a través de `autenticar_con_cache`.
    fn autenticar_y_cachear(
        &self,
        secreto: &str,
        metadata: Option<&control_acceso::nube::MetadatosDispositivo>,
    ) -> Result<control_acceso::nube::TokenDispositivo, control_acceso::nube::NubeError> {
        let token = self.cache_token.autenticar_y_cachear(secreto, metadata)?;
        self.aplicar_desfase_de(&token);
        Ok(token)
    }

    /// Aplica (y guarda) el desfase de reloj que trae un token recién
    /// medido; en un acierto de caché no trae nada y no hace nada. En un
    /// solo lugar a propósito: antes cada llamador lo aplicaba por su cuenta
    /// y el login (`Nucleo::autenticar`) se lo saltaba -- medía el desfase
    /// y lo descartaba, y las autenticaciones siguientes, desde el caché, ya
    /// no lo traían, así que quedaba el de una medición vieja.
    fn aplicar_desfase_de(&self, token: &control_acceso::nube::TokenDispositivo) {
        if let Some(desfase_ms) = token.desfase_reloj_ms {
            self.core_lock().actualizar_desfase_reloj(desfase_ms);
        }
    }

    /// Conexión propia al mismo archivo, independiente de `core` -- mismo
    /// patrón y mismo motivo que `GuiState::conexion_secundaria` en
    /// escritorio: la sincronización hace varias llamadas HTTP
    /// seguidas (drenar cola, cierres, ingresos abiertos, catálogo,
    /// historial) y cada una escribe lo que trae -- sin esto, esa cadena
    /// entera retendría `core_lock()`, bloqueando cualquier otra pantalla
    /// mientras dura. Sólo funciona sin pisarse con la conexión principal
    /// porque la base está en `journal_mode=WAL` (ver `database::schema`):
    /// con el rollback journal clásico, la primera escritura de cualquiera
    /// de las dos conexiones bloquearía a la otra igual que si compartieran
    /// el mismo candado. Reusa la fábrica central de escritura (mismos
    /// pragmas que `GuiState::conexion_secundaria` en escritorio: antes
    /// esta sólo aplicaba `busy_timeout`/`foreign_keys`, le faltaban
    /// `synchronous`/`trusted_schema`/`secure_delete`). Usa la misma clave
    /// de `SQLite3MC` que la conexión principal (`None` sólo con `abrir`,
    /// sin cifrar).
    fn conexion_secundaria(&self) -> Result<rusqlite::Connection, NucleoError> {
        control_acceso::database::connection::abrir_conexion_secundaria_escritura(
            &self.ruta_base_datos,
            self.clave.as_ref(),
        )
        .map_err(|error| NucleoError::Interno {
            mensaje: interno(error),
        })
    }

    fn sesion_lock(&self) -> std::sync::MutexGuard<'_, Option<UsuarioSesionNucleo>> {
        self.sesion
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn actor_autenticado(&self) -> Result<UsuarioSesionNucleo, NucleoError> {
        self.sesion_lock().clone().ok_or(NucleoError::NoAutenticado)
    }

    /// Descarta el `TokenDispositivo` cacheado -- ver
    /// `SincronizacionError::token_dispositivo_vencido` y el mismo método en
    /// `desktop/src-tauri/src/estado.rs::GuiState`: el receptor lo rechazó a
    /// mitad de una sincronización aunque `autenticar_con_cache` lo creía
    /// vigente. La próxima llamada pide uno nuevo sin esperar a que el
    /// "`vigente_por`" calculado localmente se cumpla solo.
    fn invalidar_token_cacheado(&self) {
        self.cache_token.invalidar();
    }
}

#[cfg(test)]
mod tests;
