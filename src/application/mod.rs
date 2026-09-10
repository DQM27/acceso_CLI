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
// Reusa `lenguaje_comandos` (parser+resolver), que no depende de terminal —
// sin feature gate, a diferencia de `cli/` (el loop real).
mod catalogos;
mod gafetes;
mod historial;
#[cfg(feature = "nube")]
mod nube;
mod usuarios;

pub use catalogos::{buscar_auditoria_completo_con_conexion, buscar_auditoria_con_conexion};
pub use historial::{
    ExportarHistorialError, buscar_historial_completo_con_conexion,
    exportar_historial_seleccion_con_conexion,
};
#[cfg(feature = "nube")]
pub use nube::{
    GestionNubeError, MovimientoHistorialSitio, ResumenSincronizacion, SesionRealtimeNube,
};

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
pub struct AppCore {
    connection: Connection,
    reloj: Arc<dyn Reloj>,
    #[cfg(feature = "nube")]
    token_nube_cacheado: std::sync::Mutex<Option<nube::TokenCacheado>>,
}

impl AppCore {
    pub fn new(connection: Connection) -> Self {
        Self::con_reloj(connection, Arc::new(RelojSistema))
    }

    pub fn con_reloj(connection: Connection, reloj: Arc<dyn Reloj>) -> Self {
        Self {
            connection,
            reloj,
            #[cfg(feature = "nube")]
            token_nube_cacheado: std::sync::Mutex::new(None),
        }
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
