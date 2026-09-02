//! Gestión de la persistencia en la nube (`docs/plan-persistencia-nube.md`)
//! desde la fachada de aplicación. Exclusivo de ROOT -- el secreto de
//! dispositivo es la identidad de todo el equipo ante el receptor, no una
//! preferencia que un Administrador deba poder tocar.

use crate::database::error::DatabaseError;
use crate::domain::autorizacion::Operacion;
use crate::services::autenticacion_service::UsuarioSesion;

use super::{AppCore, verificar_actor_activo};

#[derive(Debug, thiserror::Error)]
pub enum GestionNubeError {
    #[error("Sólo una sesión ROOT activa puede gestionar la nube")]
    OperacionNoAutorizada,
    #[error("Error de SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Todavía no se guardó el secreto de este dispositivo")]
    SinSecreto,
    #[error("No se pudo guardar el secreto localmente: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Autenticacion(#[from] crate::nube::NubeError),
    #[error(transparent)]
    Sincronizacion(#[from] crate::nube::SincronizacionError),
}

impl AppCore {
    pub fn guardar_secreto_dispositivo(
        &self,
        actor: &UsuarioSesion,
        secreto: &str,
    ) -> Result<(), GestionNubeError> {
        self.autorizar_gestion_nube(actor)?;
        crate::nube::credenciales::guardar_secreto(secreto)?;
        Ok(())
    }

    /// No revela el secreto ya guardado -- sólo si hay uno o no, para que
    /// la pantalla sepa si mostrar "pegá el secreto" o "dispositivo ya
    /// configurado".
    pub fn secreto_dispositivo_guardado(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<bool, GestionNubeError> {
        self.autorizar_gestion_nube(actor)?;
        Ok(crate::nube::credenciales::cargar_secreto().is_some())
    }

    /// Sólo autoriza -- no sincroniza nada. Separado de la sincronización
    /// real por el mismo motivo que `autorizar_creacion_respaldo`: la
    /// sincronización hace red (varios cientos de milisegundos, tal vez
    /// más con conexión lenta) y no debe retener el `Mutex<AppCore>`
    /// compartido mientras tanto -- quien llama autoriza acá, con el
    /// candado, y ejecuta `crate::nube::drenar_cola` sobre una conexión
    /// propia (ver `GuiState::conexion_secundaria` en la GUI de escritorio).
    pub fn autorizar_gestion_nube(&self, actor: &UsuarioSesion) -> Result<(), GestionNubeError> {
        let usuario = verificar_actor_activo(&self.connection, actor)
            .map_err(|error| match error {
                DatabaseError::Sqlite(error) => GestionNubeError::Sqlite(error),
                _ => GestionNubeError::OperacionNoAutorizada,
            })?
            .ok_or(GestionNubeError::OperacionNoAutorizada)?;
        if !usuario.rol.puede(Operacion::GestionarNube) {
            return Err(GestionNubeError::OperacionNoAutorizada);
        }
        Ok(())
    }
}
