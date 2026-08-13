use std::path::Path;

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::database::connection::open_database;
use crate::database::queries::contratistas::{
    ContratistaResumen, FiltroContratistas, SqliteContratistasQuery,
};
use crate::database::repositories::contratista_repository::SqliteContratistaRepository;
use crate::database::repositories::empresa_repository::SqliteEmpresaRepository;
use crate::database::repositories::registro_ingreso_repository::SqliteRegistroIngresoRepository;
use crate::database::repositories::usuario_repository::SqliteUsuarioRepository;
use crate::services::autenticacion_service::{AutenticacionService, UsuarioSesion};
use crate::services::contratista_service::ContratistaConsultaService;
use crate::services::error::{
    AutenticacionError, ContratistaServiceError, RegistroIngresoServiceError, UsuarioServiceError,
};
use crate::services::registro_ingreso_service::{PreparacionIngreso, RegistroIngresoService};
use crate::services::usuario_service::{CrearRootInicialInput, UsuarioService};

#[derive(Debug)]
pub enum BootstrapError {
    Database(rusqlite::Error),
}

impl std::fmt::Display for BootstrapError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "No se pudo preparar SQLite: {error}"),
        }
    }
}

impl std::error::Error for BootstrapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
        }
    }
}

impl From<rusqlite::Error> for BootstrapError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

/// Fachada de aplicación y propietario único de la conexión SQLite.
pub struct AppCore {
    connection: Connection,
}

impl AppCore {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub fn abrir(path: impl AsRef<Path>) -> Result<Self, BootstrapError> {
        Ok(Self::new(open_database(path)?))
    }

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

    pub fn autenticar(
        &self,
        cedula: &str,
        password: &str,
    ) -> Result<UsuarioSesion, AutenticacionError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        AutenticacionService::new(&repository).autenticar(cedula, password)
    }

    pub fn buscar_contratistas(
        &self,
        filtro: &FiltroContratistas,
    ) -> Result<Vec<ContratistaResumen>, ContratistaServiceError> {
        let query = SqliteContratistasQuery::new(&self.connection);
        ContratistaConsultaService::new(&query).buscar_para_tabla(filtro)
    }

    pub fn preparar_ingreso(
        &self,
        contratista_id: i64,
        hoy: NaiveDate,
    ) -> Result<PreparacionIngreso, RegistroIngresoServiceError> {
        let contratistas = SqliteContratistaRepository::new(&self.connection);
        let empresas = SqliteEmpresaRepository::new(&self.connection);
        let registros = SqliteRegistroIngresoRepository::new(&self.connection);
        RegistroIngresoService::new(&contratistas, &registros).preparar_ingreso(
            &empresas,
            contratista_id,
            hoy,
        )
    }
}
