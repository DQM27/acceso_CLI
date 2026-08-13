use std::path::Path;

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::database::connection::open_database;
use crate::database::queries::contratistas::{
    ContratistaResumen, FiltroContratistas, SqliteContratistasQuery,
};
use crate::database::queries::empresas::{EmpresaResumen, FiltroEmpresas, SqliteEmpresasQuery};
use crate::database::queries::ingresos::{
    FiltroHistorial, FiltroIngresosActivos, PaginaHistorial, SqliteIngresosQuery,
};
use crate::database::queries::usuarios::{FiltroUsuarios, SqliteUsuariosQuery, UsuarioResumen};
use crate::database::repositories::contratista_repository::SqliteContratistaRepository;
use crate::database::repositories::empresa_repository::SqliteEmpresaRepository;
use crate::database::repositories::registro_ingreso_repository::SqliteRegistroIngresoRepository;
use crate::database::repositories::usuario_repository::SqliteUsuarioRepository;
use crate::services::autenticacion_service::{AutenticacionService, UsuarioSesion};
use crate::services::contratista_service::{
    ContratistaConsultaService, ContratistaService, DatosContratista,
};
use crate::services::empresa_service::{EmpresaConsultaService, EmpresaService};
use crate::services::error::{
    AutenticacionError, ContratistaServiceError, EmpresaServiceError, RegistroIngresoServiceError,
    UsuarioServiceError,
};
use crate::services::registro_ingreso_service::{
    IngresoActivoResumen, PreparacionIngreso, RegistroIngresoConsultaService,
    RegistroIngresoService, ResultadoRegistroEntrada,
};
use crate::services::usuario_service::{
    ActualizarUsuarioInput, CrearRootInicialInput, CrearUsuarioInput, UsuarioConsultaService,
    UsuarioService,
};

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

    pub fn crear_contratista(
        &self,
        datos: DatosContratista,
    ) -> Result<i64, ContratistaServiceError> {
        ContratistaService::new(
            &SqliteContratistaRepository::new(&self.connection),
            &SqliteEmpresaRepository::new(&self.connection),
        )
        .crear(datos)
    }

    pub fn actualizar_contratista(
        &self,
        id: i64,
        datos: DatosContratista,
    ) -> Result<(), ContratistaServiceError> {
        ContratistaService::new(
            &SqliteContratistaRepository::new(&self.connection),
            &SqliteEmpresaRepository::new(&self.connection),
        )
        .actualizar(id, datos)
    }

    pub fn listar_empresas(
        &self,
    ) -> Result<Vec<crate::models::empresa::Empresa>, EmpresaServiceError> {
        EmpresaService::new(&SqliteEmpresaRepository::new(&self.connection)).listar()
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

    pub fn gafete_esta_ocupado(&self, numero: i64) -> Result<bool, RegistroIngresoServiceError> {
        let contratistas = SqliteContratistaRepository::new(&self.connection);
        let registros = SqliteRegistroIngresoRepository::new(&self.connection);
        match RegistroIngresoService::new(&contratistas, &registros)
            .buscar_ingreso_activo_por_gafete(numero)
        {
            Ok(_) => Ok(true),
            Err(RegistroIngresoServiceError::GafeteNoAsignado) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn registrar_ingreso(
        &self,
        contratista_id: i64,
        medio: crate::models::medio_ingreso::MedioIngreso,
        gafete: Option<i64>,
        usuario_id: i64,
        fecha_hora: chrono::NaiveDateTime,
    ) -> Result<ResultadoRegistroEntrada, RegistroIngresoServiceError> {
        RegistroIngresoService::new(
            &SqliteContratistaRepository::new(&self.connection),
            &SqliteRegistroIngresoRepository::new(&self.connection),
        )
        .registrar_entrada(contratista_id, medio, gafete, usuario_id, fecha_hora)
    }

    pub fn listar_ingresos_activos(
        &self,
        filtro: &FiltroIngresosActivos,
        hoy: NaiveDate,
    ) -> Result<Vec<IngresoActivoResumen>, RegistroIngresoServiceError> {
        RegistroIngresoConsultaService::new(&SqliteIngresosQuery::new(&self.connection))
            .listar_activos(filtro, hoy)
    }

    pub fn buscar_historial(
        &self,
        filtro: &FiltroHistorial,
    ) -> Result<PaginaHistorial, RegistroIngresoServiceError> {
        RegistroIngresoConsultaService::new(&SqliteIngresosQuery::new(&self.connection))
            .buscar_historial(filtro)
    }

    pub fn buscar_activo_por_gafete(
        &self,
        numero: i64,
    ) -> Result<i64, RegistroIngresoServiceError> {
        let contratistas = SqliteContratistaRepository::new(&self.connection);
        let registros = SqliteRegistroIngresoRepository::new(&self.connection);
        Ok(RegistroIngresoService::new(&contratistas, &registros)
            .buscar_ingreso_activo_por_gafete(numero)?
            .id)
    }

    pub fn registrar_salida(
        &self,
        id: i64,
        fecha: chrono::NaiveDateTime,
        usuario: i64,
    ) -> Result<(), RegistroIngresoServiceError> {
        let contratistas = SqliteContratistaRepository::new(&self.connection);
        let registros = SqliteRegistroIngresoRepository::new(&self.connection);
        RegistroIngresoService::new(&contratistas, &registros).registrar_salida(id, fecha, usuario)
    }

    pub fn buscar_empresas(
        &self,
        filtro: &FiltroEmpresas,
    ) -> Result<Vec<EmpresaResumen>, EmpresaServiceError> {
        EmpresaConsultaService::new(&SqliteEmpresasQuery::new(&self.connection))
            .buscar_para_tabla(filtro)
    }

    pub fn crear_empresa(&self, nombre: &str) -> Result<i64, EmpresaServiceError> {
        EmpresaService::new(&SqliteEmpresaRepository::new(&self.connection)).crear(nombre)
    }

    pub fn actualizar_empresa(&self, id: i64, nombre: &str) -> Result<(), EmpresaServiceError> {
        EmpresaService::new(&SqliteEmpresaRepository::new(&self.connection)).actualizar(id, nombre)
    }

    pub fn buscar_usuarios(
        &self,
        filtro: &FiltroUsuarios,
    ) -> Result<Vec<UsuarioResumen>, UsuarioServiceError> {
        UsuarioConsultaService::new(&SqliteUsuariosQuery::new(&self.connection))
            .buscar_para_tabla(filtro)
    }

    pub fn crear_usuario(&self, input: CrearUsuarioInput) -> Result<i64, UsuarioServiceError> {
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection)).crear(input)
    }

    pub fn actualizar_usuario(
        &self,
        id: i64,
        input: ActualizarUsuarioInput,
        activo: bool,
    ) -> Result<(), UsuarioServiceError> {
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection))
            .actualizar_administracion(id, input, activo)
    }

    pub fn activar_usuario(&self, id: i64) -> Result<(), UsuarioServiceError> {
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection)).activar(id)
    }

    pub fn desactivar_usuario(&self, id: i64) -> Result<(), UsuarioServiceError> {
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection)).desactivar(id)
    }

    pub fn cambiar_password_usuario(
        &self,
        id: i64,
        password: &str,
    ) -> Result<(), UsuarioServiceError> {
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection))
            .cambiar_password(id, password)
    }
}
