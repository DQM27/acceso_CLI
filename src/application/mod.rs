use std::path::Path;
use std::sync::Arc;

use rusqlite::Connection;

use crate::database::connection::{open_database, open_database_cifrada};
use crate::database::error::DatabaseError;
use crate::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use crate::database::schema::SchemaError;
use crate::models::usuario::Usuario;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::tiempo::{Reloj, RelojSistema};

mod accesos;
mod autenticacion;
mod citas;
#[cfg(feature = "nube")]
mod con_nube;
mod correo;
// Reusa `lenguaje_comandos` (parser+resolver), que no depende de terminal —
// sin feature gate, a diferencia de `cli/` (el loop real).
mod catalogos;
mod gafetes;
mod gafetes_provisionales;
mod historial;
#[cfg(feature = "nube")]
mod nube;
mod proveedores;
mod rutas;

pub use catalogos::{buscar_auditoria_completo_con_conexion, buscar_auditoria_con_conexion};
#[cfg(feature = "nube")]
pub use con_nube::{
    EntradaVisitaVerificadaError, EntregaGafeteProvisionalVerificadaError,
    IngresoCorreoVerificadoError, IngresoProveedorVerificadoError, IngresoVerificadoError,
    NuevoIngresoCorreo, NuevoIngresoProveedor, entregar_gafete_provisional_verificado,
    preparar_ingreso_verificado, registrar_entrada_visita_verificada,
    registrar_ingreso_correo_verificado, registrar_ingreso_proveedor_verificado,
    registrar_ingreso_verificado,
};
pub use historial::{
    ExportarHistorialError, buscar_historial_completo_con_conexion,
    exportar_historial_seleccion_con_conexion, exportar_tabla_xlsx,
    movimientos_completos_con_conexion, movimientos_en_orden_con_conexion,
};
#[cfg(feature = "nube")]
pub use nube::{GestionNubeError, MovimientoHistorialSitio, ResumenSincronizacion};

/// Tope de seguridad para las cargas "todo en un `Vec`" que alimentan AG
/// Grid (`buscar_historial_completo`, `buscar_auditoria_completo`) — la
/// virtualización del lado del cliente sigue siendo la estrategia elegida
/// (ver `docs/pendientes.md`), pero sin un tope una tabla append-only sin
/// filtro de fecha acotado (Auditoría no tiene selector de rango) podría
/// intentar traer años de datos en un solo mensaje IPC y congelar la UI.
/// `CargaCompleta::truncado` avisa a la pantalla en vez de devolver menos
/// filas en silencio.
pub(crate) const LIMITE_CARGA_COMPLETA_MAXIMO: usize = 20_000;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CargaCompleta<T> {
    pub items: Vec<T>,
    pub truncado: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("No se pudo preparar SQLite: {0}")]
    Database(#[from] SchemaError),
}

/// Fachada de aplicación y propietario único de la conexión `SQLite`.
///
/// Regla: **nunca red con el candado del núcleo tomado.** Escritorio y
/// móvil guardan `AppCore` detrás de un `Mutex` compartido por toda la app;
/// una llamada HTTP con ese candado tomado congela cualquier otra pantalla
/// hasta el timeout (10 s). Por eso `AppCore` no hace red: la hacen
/// `nube::sincronizar`/`nube::recibir` sobre una conexión secundaria y las
/// funciones de `application::con_nube`, que toman el candado sólo para lo
/// local. La única excepción, documentada, es
/// [`AppCore::vincular_dispositivo_inicial`].
pub struct AppCore {
    connection: Connection,
    reloj: Arc<dyn Reloj>,
    /// Campo HERMANO de `connection`, nunca protegido por el mismo candado
    /// que envuelve a este `AppCore` entero (`core_lock()`/`GuiState::core()`
    /// del lado de quien lo sostiene) -- ver el doc-comment de
    /// `crate::nube::CacheTokenDispositivo` sobre por qué.
    #[cfg(feature = "nube")]
    cache_token: crate::nube::CacheTokenDispositivo,
    /// Versión de la app que abrió este `AppCore` (`env!("CARGO_PKG_VERSION")`
    /// de escritorio/móvil, cada uno la suya -- este crate no puede saberla
    /// solo). `None` hasta que quien llama la fija con
    /// [`Self::establecer_version_app`]. Ver esa función para el porqué.
    #[cfg(feature = "nube")]
    version_app: Option<String>,
}

impl AppCore {
    pub fn new(connection: Connection) -> Self {
        Self::con_reloj(connection, Arc::new(RelojSistema))
    }

    /// Aplica al reloj lo último guardado contra la hora del servidor (ver
    /// `database::queries::desfase_reloj`): el ancla, si sigue siendo de
    /// este arranque del equipo, y el desfase como respaldo. Así el equipo
    /// sella con la hora del servidor desde que abre, y no sólo tras la
    /// primera respuesta de la nube. Con un reloj que no se corrige
    /// (`RelojSistema`, `RelojFijo`), no cambia nada.
    pub fn con_reloj(connection: Connection, reloj: Arc<dyn Reloj>) -> Self {
        use crate::database::queries::desfase_reloj;

        let desfase = desfase_reloj::leer(&connection).unwrap_or_else(|error| {
            log::warn!("no se pudo leer el desfase de reloj guardado: {error}");
            None
        });
        let ancla = desfase_reloj::leer_ancla(&connection).unwrap_or_else(|error| {
            log::warn!("no se pudo leer el ancla de hora guardada: {error}");
            None
        });
        reloj.restaurar(desfase, ancla);
        Self {
            connection,
            reloj,
            #[cfg(feature = "nube")]
            cache_token: crate::nube::CacheTokenDispositivo::new(),
            #[cfg(feature = "nube")]
            version_app: None,
        }
    }

    /// El reloj de este núcleo, para quien necesite la misma hora fuera del
    /// candado de `AppCore` (la telemetría de diagnóstico sella sus eventos
    /// con él: así todos los tiempos del sistema salen del mismo reloj).
    pub fn reloj(&self) -> Arc<dyn Reloj> {
        Arc::clone(&self.reloj)
    }

    /// Estado del reloj confiable (ver [`crate::tiempo::EstadoReloj`]), para
    /// la telemetría de los builds de diagnóstico.
    pub fn estado_reloj(&self) -> crate::tiempo::EstadoReloj {
        let hora = self.reloj.ahora_con_margen();
        crate::tiempo::EstadoReloj {
            confiable: hora.margen_ms.is_some(),
            margen_ms: hora.margen_ms,
            diferencia_equipo_ms: (chrono::Utc::now() - hora.instante).num_milliseconds(),
            ancla_hace_ms: self.reloj.ancla().map(|ancla| {
                crate::reloj_arranque::ms_desde_arranque().saturating_sub(ancla.arranque_ms)
            }),
        }
    }

    /// Fija la versión de la app para que viaje en CADA renovación de token
    /// de dispositivo (`nube::autenticar_y_cachear`), no sólo en la
    /// activación inicial -- para que el receptor pueda rechazar una versión
    /// por debajo del mínimo aceptado en cualquier momento de la vida del
    /// dispositivo, no sólo al configurarlo. Sin llamar a esto, el
    /// comportamiento es exactamente el de antes (ninguna versión viaja
    /// fuera de la activación). Ver
    /// `docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md`, punto 9.
    #[cfg(feature = "nube")]
    pub fn establecer_version_app(&mut self, version: impl Into<String>) {
        self.version_app = Some(version.into());
    }

    pub fn abrir(path: impl AsRef<Path>) -> Result<Self, BootstrapError> {
        Self::abrir_con_reloj(path, Arc::new(RelojSistema))
    }

    pub fn abrir_con_reloj(
        path: impl AsRef<Path>,
        reloj: Arc<dyn Reloj>,
    ) -> Result<Self, BootstrapError> {
        Ok(Self::con_reloj(open_database(path)?, reloj))
    }

    /// Igual que [`Self::abrir_con_reloj`], pero cifrada con `SQLCipher`.
    /// `clave` es responsabilidad de quien llama: en escritorio sale de un
    /// blob protegido con DPAPI (ver `desktop/src-tauri/src/clave_cifrado.rs`);
    /// en Android, del Keystore.
    pub fn abrir_con_reloj_cifrado(
        path: impl AsRef<Path>,
        clave: &[u8; 32],
        reloj: Arc<dyn Reloj>,
    ) -> Result<Self, BootstrapError> {
        Ok(Self::con_reloj(open_database_cifrada(path, clave)?, reloj))
    }
}

impl Drop for AppCore {
    fn drop(&mut self) {
        let _ = self.connection.execute_batch("PRAGMA optimize;");
    }
}

fn verificar_actor_activo(
    connection: &Connection,
    actor: &UsuarioSesion,
) -> Result<Option<Usuario>, DatabaseError> {
    Ok(SqliteUsuarioRepository::new(connection)
        .buscar_por_id(actor.id)?
        .filter(|usuario| usuario.activo))
}
