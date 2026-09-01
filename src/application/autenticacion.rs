//! Arranque (configuración inicial / ROOT inicial) y autenticación.

use crate::database::repositories::usuario_repository::SqliteUsuarioRepository;
use crate::services::autenticacion_service::{
    AutenticacionService, CandidatoAutenticacion, UsuarioSesion,
};
use crate::services::error::{AutenticacionError, UsuarioServiceError};
use crate::services::usuario_service::{CrearRootInicialInput, UsuarioService};

use super::AppCore;

impl AppCore {
    pub fn requiere_configuracion_inicial(&self) -> Result<bool, UsuarioServiceError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        UsuarioService::new(&repository).requiere_configuracion_inicial()
    }

    pub fn crear_root_inicial(
        &self,
        input: CrearRootInicialInput,
    ) -> Result<i64, UsuarioServiceError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        UsuarioService::new(&repository).crear_root_inicial(input)
    }

    pub fn validar_datos_para_root_inicial(
        &self,
        input: &CrearRootInicialInput,
    ) -> Result<(), UsuarioServiceError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        UsuarioService::new(&repository).validar_datos_para_root_inicial(input)
    }

    pub fn crear_root_inicial_con_hash(
        &self,
        input: CrearRootInicialInput,
        password_hash: String,
    ) -> Result<i64, UsuarioServiceError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        UsuarioService::new(&repository).crear_root_inicial_con_hash(input, password_hash)
    }

    pub fn autenticar(
        &self,
        cedula: &str,
        password: &str,
    ) -> Result<UsuarioSesion, AutenticacionError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        AutenticacionService::new(&repository).autenticar(cedula, password)
    }

    /// Resuelve la cédula sin verificar todavía la contraseña — rápido, sólo `SQLite`. Permite
    /// correr la verificación de Argon2 (lenta) en un hilo aparte sin compartir la conexión.
    pub fn buscar_candidato_autenticacion(
        &self,
        cedula: &str,
    ) -> Result<CandidatoAutenticacion, AutenticacionError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        AutenticacionService::new(&repository).buscar_candidato(cedula)
    }
}
