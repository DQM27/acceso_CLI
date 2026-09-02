use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;

use crate::database::connection::open_database;
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
// Reusa `lenguaje_comandos` (parser+resolver), que no depende de terminal —
// sin feature gate, a diferencia de `cli/` (el loop real).
mod catalogos;
mod gafetes;
mod historial;
#[cfg(feature = "nube")]
mod nube;
mod respaldos;
mod usuarios;

pub use catalogos::{buscar_auditoria_completo_con_conexion, buscar_auditoria_con_conexion};
pub use historial::{
    ExportarHistorialError, buscar_historial_completo_con_conexion,
    exportar_historial_seleccion_con_conexion,
};
#[cfg(feature = "nube")]
pub use nube::{GestionNubeError, ResumenSincronizacion};
pub use respaldos::EstadoRespaldoAutomatico;

/// Tope de seguridad para las cargas "todo en un `Vec`" que alimentan AG
/// Grid (`buscar_historial_completo`, `buscar_auditoria_completo`) — la
/// virtualización del lado del cliente sigue siendo la estrategia elegida
/// (ver `docs/pendientes.md`), pero sin un tope una tabla append-only sin
/// filtro de fecha acotado (Auditoría no tiene selector de rango) podría
/// intentar traer años de datos en un solo mensaje IPC y congelar la UI.
/// `CargaCompleta::truncado` avisa a la pantalla en vez de devolver menos
/// filas en silencio.
pub(crate) const LIMITE_CARGA_COMPLETA_MAXIMO: usize = 20_000;

/// Resultado de una carga "todo de una vez" acotada por
/// [`LIMITE_CARGA_COMPLETA_MAXIMO`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CargaCompleta<T> {
    pub items: Vec<T>,
    /// `true` si el conjunto real supera el tope y se cortó antes de
    /// traerlo completo — la pantalla debe pedir acotar el filtro en vez de
    /// asumir que ve todo.
    pub truncado: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("No se pudo preparar SQLite: {0}")]
    Database(#[from] SchemaError),
}

/// Fachada de aplicación y propietario único de la conexión `SQLite`.
pub struct AppCore {
    connection: Connection,
    reloj: Arc<dyn Reloj>,
    /// Ruta del archivo activo, sólo conocida cuando se abre con [`AppCore::abrir`].
    /// Se usa exclusivamente para ubicar el directorio de respaldos junto a la base.
    ruta_base_datos: PathBuf,
}

impl AppCore {
    /// Construye un `AppCore` sin ruta de archivo asociada (pensado para `SQLite` en
    /// memoria y tests). **Los respaldos usan `directorio_respaldos()`, que sin una
    /// ruta real cae en `./backups` relativo al directorio de trabajo del proceso**
    /// — para producción, usar [`AppCore::abrir`], que sí registra la ruta real.
    pub fn new(connection: Connection) -> Self {
        Self::con_reloj(connection, Arc::new(RelojSistema))
    }

    /// Igual que [`AppCore::new`] con un reloj inyectado — mismo aviso sobre
    /// `directorio_respaldos()` sin ruta real.
    pub fn con_reloj(connection: Connection, reloj: Arc<dyn Reloj>) -> Self {
        Self {
            connection,
            reloj,
            ruta_base_datos: PathBuf::new(),
        }
    }

    pub fn abrir(path: impl AsRef<Path>) -> Result<Self, BootstrapError> {
        Self::abrir_con_reloj(path, Arc::new(RelojSistema))
    }

    pub fn abrir_con_reloj(
        path: impl AsRef<Path>,
        reloj: Arc<dyn Reloj>,
    ) -> Result<Self, BootstrapError> {
        let mut core = Self::con_reloj(open_database(&path)?, reloj);
        core.ruta_base_datos = path.as_ref().to_path_buf();
        Ok(core)
    }
}

impl Drop for AppCore {
    /// Deja al planificador de consultas estadísticas frescas para la
    /// próxima apertura. Es mantenimiento, no corrección: un fallo aquí
    /// (conexión ya en mal estado, por ejemplo) no debe impedir el cierre.
    fn drop(&mut self) {
        let _ = self.connection.execute_batch("PRAGMA optimize;");
    }
}

/// Comprobación de sanidad del actor compartida por todos los dominios: "¿quién dice
/// que está haciendo esto sigue siendo un usuario real y activo?". Autoriza contra el
/// rol y estado que existen ahora en `SQLite` — el rol del snapshot de sesión nunca
/// decide permisos, una degradación surte efecto en la siguiente operación sin
/// necesidad de reiniciar la TUI.
fn verificar_actor_activo(
    connection: &Connection,
    actor: &UsuarioSesion,
) -> Result<Option<Usuario>, DatabaseError> {
    Ok(SqliteUsuarioRepository::new(connection)
        .buscar_por_id(actor.id)?
        .filter(|usuario| usuario.activo))
}
