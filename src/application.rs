use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::database::backup::{RespaldoError, RespaldoResumen, ResultadoValidacion, TipoRespaldo};
use crate::database::connection::open_database;
use crate::database::error::DatabaseError;
use crate::database::queries::auditoria_contratistas::{
    FiltroAuditoriaContratistas, PaginaAuditoriaContratistas, SqliteAuditoriaContratistas,
};
use crate::database::queries::contratistas::{
    FiltroContratistas, PaginaContratistas, SqliteContratistasQuery,
};
use crate::database::queries::empresas::{EmpresaResumen, FiltroEmpresas, SqliteEmpresasQuery};
use crate::database::queries::ingresos::{
    FiltroHistorial, FiltroIngresosActivos, PaginaHistorial, SqliteIngresosQuery,
};
use crate::database::queries::usuarios::{FiltroUsuarios, SqliteUsuariosQuery, UsuarioResumen};
use crate::database::repositories::contratista_repository::SqliteContratistaRepository;
use crate::database::repositories::empresa_repository::SqliteEmpresaRepository;
use crate::database::repositories::registro_ingreso_repository::SqliteRegistroIngresoRepository;
use crate::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use crate::database::schema::SchemaError;
use crate::domain::autorizacion::{Operacion, puede_cambiar_password, puede_gestionar_usuario};
use crate::exportacion_historial::{
    ColumnaHistorial, FormatosHistorial, escribir_movimiento, preparar_hoja,
};
use crate::models::usuario::{RolUsuario, Usuario};
use crate::services::autenticacion_service::{
    AutenticacionService, CandidatoAutenticacion, UsuarioSesion,
};
use crate::services::contratista_service::{
    ContratistaConsultaService, ContratistaService, DatosActualizacionContratista, DatosContratista,
};
use crate::services::empresa_service::{EmpresaConsultaService, EmpresaService};
use crate::services::error::{
    AutenticacionError, ContratistaServiceError, EmpresaServiceError, RegistroIngresoServiceError,
    UsuarioServiceError,
};
use crate::services::registro_ingreso_service::{
    ListaIngresosActivosResumen, PreparacionIngreso, RegistroIngresoConsultaService,
    RegistroIngresoService, ResultadoRegistroEntrada,
};
use crate::services::usuario_service::{
    ActualizarUsuarioInput, CrearRootInicialInput, CrearUsuarioInput, UsuarioConsultaService,
    UsuarioService,
};
use crate::tiempo::{Reloj, RelojSistema, fecha_costa_rica};

#[derive(Debug)]
pub enum BootstrapError {
    Database(SchemaError),
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

impl From<SchemaError> for BootstrapError {
    fn from(error: SchemaError) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug)]
pub enum ExportarHistorialError {
    SinColumnas,
    DestinoExiste(PathBuf),
    DirectorioNoExiste(PathBuf),
    DemasiadasFilas(usize),
    Consulta(RegistroIngresoServiceError),
    Xlsx(rust_xlsxwriter::XlsxError),
    Io(std::io::Error),
}

impl std::fmt::Display for ExportarHistorialError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SinColumnas => write!(formatter, "Seleccione al menos una columna"),
            Self::DestinoExiste(ruta) => write!(
                formatter,
                "El archivo ya existe; elija otro nombre: {}",
                ruta.display()
            ),
            Self::DirectorioNoExiste(ruta) => {
                write!(
                    formatter,
                    "La carpeta destino no existe: {}",
                    ruta.display()
                )
            }
            Self::DemasiadasFilas(total) => write!(
                formatter,
                "La exportación tiene {total} filas y supera el límite de una hoja de Excel"
            ),
            Self::Consulta(error) => {
                write!(formatter, "No se pudo consultar el historial: {error}")
            }
            Self::Xlsx(error) => write!(formatter, "No se pudo crear el archivo XLSX: {error}"),
            Self::Io(error) => write!(formatter, "No se pudo guardar la exportación: {error}"),
        }
    }
}

impl std::error::Error for ExportarHistorialError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Consulta(error) => Some(error),
            Self::Xlsx(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<RegistroIngresoServiceError> for ExportarHistorialError {
    fn from(error: RegistroIngresoServiceError) -> Self {
        Self::Consulta(error)
    }
}

impl From<rust_xlsxwriter::XlsxError> for ExportarHistorialError {
    fn from(error: rust_xlsxwriter::XlsxError) -> Self {
        Self::Xlsx(error)
    }
}

impl From<std::io::Error> for ExportarHistorialError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Fachada de aplicación y propietario único de la conexión SQLite.
pub struct AppCore {
    connection: Connection,
    reloj: Arc<dyn Reloj>,
    /// Ruta del archivo activo, sólo conocida cuando se abre con [`AppCore::abrir`].
    /// Se usa exclusivamente para ubicar el directorio de respaldos junto a la base.
    ruta_base_datos: PathBuf,
}

impl AppCore {
    /// Construye un `AppCore` sin ruta de archivo asociada (pensado para SQLite en
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

    /// Resuelve la cédula sin verificar todavía la contraseña — rápido, sólo SQLite. Permite
    /// correr la verificación de Argon2 (lenta) en un hilo aparte sin compartir la conexión.
    pub fn buscar_candidato_autenticacion(
        &self,
        cedula: &str,
    ) -> Result<CandidatoAutenticacion, AutenticacionError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        AutenticacionService::new(&repository).buscar_candidato(cedula)
    }

    pub fn buscar_contratistas(
        &self,
        filtro: &FiltroContratistas,
    ) -> Result<PaginaContratistas, ContratistaServiceError> {
        let query = SqliteContratistasQuery::new(&self.connection);
        ContratistaConsultaService::new(&query).buscar_para_tabla(filtro)
    }

    pub fn buscar_auditoria_contratistas(
        &self,
        actor: &UsuarioSesion,
        filtro: &FiltroAuditoriaContratistas,
    ) -> Result<PaginaAuditoriaContratistas, ContratistaServiceError> {
        let actor_actual = verificar_actor_activo(&self.connection, actor)?
            .ok_or(ContratistaServiceError::OperacionNoAutorizada)?;
        if !actor_actual.rol.puede(Operacion::VerAuditoria) {
            return Err(ContratistaServiceError::OperacionNoAutorizada);
        }
        Ok(SqliteAuditoriaContratistas::new(&self.connection).buscar(filtro)?)
    }

    pub fn crear_contratista(
        &self,
        actor: &UsuarioSesion,
        datos: DatosContratista,
    ) -> Result<i64, ContratistaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(ContratistaServiceError::Database)?
            .ok_or(ContratistaServiceError::OperacionNoAutorizada)?;
        ContratistaService::new(
            &SqliteContratistaRepository::new(&transaction),
            &SqliteEmpresaRepository::new(&transaction),
        )
        .crear(datos)
        .and_then(|id| {
            transaction
                .commit()
                .map_err(DatabaseError::from)
                .map_err(ContratistaServiceError::Database)?;
            Ok(id)
        })
    }

    pub fn actualizar_contratista(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        datos: DatosActualizacionContratista,
    ) -> Result<(), ContratistaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(ContratistaServiceError::Database)?
            .ok_or(ContratistaServiceError::OperacionNoAutorizada)?;
        let contratistas = SqliteContratistaRepository::new(&transaction);
        let empresas = SqliteEmpresaRepository::new(&transaction);
        let servicio = ContratistaService::new(&contratistas, &empresas);
        let actual = servicio.buscar_por_id(id)?;
        if actual.tiene_acceso != datos.tiene_acceso
            && !actor_actual
                .rol
                .puede(Operacion::ActivarDesactivarContratista)
        {
            return Err(ContratistaServiceError::OperacionNoAutorizada);
        }
        servicio.actualizar_auditado(
            id,
            datos,
            actor_actual.id,
            self.reloj.ahora_utc(),
            &SqliteAuditoriaContratistas::new(&transaction),
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(ContratistaServiceError::Database)
    }

    pub fn listar_empresas(
        &self,
    ) -> Result<Vec<crate::models::empresa::Empresa>, EmpresaServiceError> {
        EmpresaService::new(&SqliteEmpresaRepository::new(&self.connection)).listar()
    }

    pub fn preparar_ingreso(
        &self,
        contratista_id: i64,
    ) -> Result<PreparacionIngreso, RegistroIngresoServiceError> {
        let contratistas = SqliteContratistaRepository::new(&self.connection);
        let empresas = SqliteEmpresaRepository::new(&self.connection);
        let registros = SqliteRegistroIngresoRepository::new(&self.connection);
        RegistroIngresoService::new(&contratistas, &registros).preparar_ingreso(
            &empresas,
            contratista_id,
            fecha_costa_rica(self.reloj.ahora_utc()),
        )
    }

    /// Abre una transacción `Immediate` (el bloqueo se adquiere antes de la
    /// primera lectura definitiva, así los repositorios creados sobre ella
    /// validan e insertan contra el mismo estado de SQLite), valida que el
    /// reloj no haya retrocedido, confirma que `usuario_id` sigue siendo un
    /// operador activo, corre `operar` y confirma. Compartido por
    /// `registrar_ingreso` y `registrar_salida` — antes cada uno repetía este
    /// mismo armazón letra por letra.
    ///
    /// La validación del reloj se queda aquí, en `AppCore`, y no en
    /// `RegistroIngresoService` (que declara `RelojRetrocedido` pero no lo
    /// genera) **a propósito**: es una comprobación de sanidad de todo el
    /// sistema (¿el reloj de la máquina retrocedió respecto al último
    /// movimiento conocido, sin importar de qué contratista?), no una regla
    /// de negocio de una entrada/salida puntual. Moverla al servicio se
    /// intentó y se revirtió — rompía tests de integración que llaman al
    /// servicio directo con datos de prueba cuyos tiempos no representan un
    /// reloj real avanzando (`tests/flujo_integracion.rs`).
    ///
    /// La verificación de operador activo vive aquí por el mismo motivo:
    /// `registrar_entrada`/`registrar_salida` recibían el `usuario_id` como
    /// un entero crudo, sin comprobar que existiera ni que siguiera activo —
    /// la FK de SQLite sólo exige que exista, no que esté activo. Una
    /// sesión de TUI ya iniciada podía seguir registrando movimientos
    /// después de que un administrador desactivara esa cuenta
    /// (`docs/auditoria-dominio-2026-08-20.md`, hallazgo #2). Se revisa
    /// dentro de esta misma transacción `Immediate`, no antes, para que una
    /// desactivación concurrente no pueda colarse entre el chequeo y la
    /// escritura.
    fn en_transaccion_con_reloj_validado<T>(
        &self,
        actor: &UsuarioSesion,
        operar: impl FnOnce(
            &Transaction<'_>,
            chrono::DateTime<chrono::Utc>,
        ) -> Result<T, RegistroIngresoServiceError>,
    ) -> Result<T, RegistroIngresoServiceError> {
        let ahora = self.reloj.ahora_utc();
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        validar_reloj(&transaction, ahora)?;
        verificar_operador_activo(&transaction, actor)?;
        let resultado = operar(&transaction, ahora)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(resultado)
    }

    pub fn registrar_ingreso(
        &self,
        actor: &UsuarioSesion,
        contratista_id: i64,
        medio: crate::models::medio_ingreso::MedioIngreso,
        gafete: Option<i64>,
    ) -> Result<ResultadoRegistroEntrada, RegistroIngresoServiceError> {
        self.en_transaccion_con_reloj_validado(actor, |transaction, ahora| {
            let contratistas = SqliteContratistaRepository::new(transaction);
            let registros = SqliteRegistroIngresoRepository::new(transaction);
            RegistroIngresoService::new(&contratistas, &registros).registrar_entrada(
                contratista_id,
                medio,
                gafete,
                actor.id,
                ahora,
            )
        })
    }

    pub fn listar_ingresos_activos(
        &self,
        filtro: &FiltroIngresosActivos,
    ) -> Result<ListaIngresosActivosResumen, RegistroIngresoServiceError> {
        RegistroIngresoConsultaService::new(&SqliteIngresosQuery::new(&self.connection))
            .listar_activos(filtro, fecha_costa_rica(self.reloj.ahora_utc()))
    }

    pub fn buscar_historial(
        &self,
        filtro: &FiltroHistorial,
    ) -> Result<PaginaHistorial, RegistroIngresoServiceError> {
        RegistroIngresoConsultaService::new(&SqliteIngresosQuery::new(&self.connection))
            .buscar_historial(filtro)
    }

    /// Exporta todo el conjunto filtrado que representa la pantalla, no sólo
    /// su página actual. Se conserva `corte_id`, por lo que ingresos creados
    /// después de cargar Historial no aparecen inesperadamente en el XLSX.
    pub fn exportar_historial(
        &self,
        filtro: &FiltroHistorial,
        columnas: &[ColumnaHistorial],
        destino: &Path,
    ) -> Result<usize, ExportarHistorialError> {
        const MAX_FILAS_DATOS_XLSX: usize = 1_048_575;

        if columnas.is_empty() {
            return Err(ExportarHistorialError::SinColumnas);
        }
        if destino.exists() {
            return Err(ExportarHistorialError::DestinoExiste(destino.to_owned()));
        }
        let directorio = destino
            .parent()
            .filter(|ruta| !ruta.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if !directorio.is_dir() {
            return Err(ExportarHistorialError::DirectorioNoExiste(
                directorio.to_owned(),
            ));
        }

        let mut libro = rust_xlsxwriter::Workbook::new();
        let mut exportados = 0usize;
        {
            let hoja = libro.add_worksheet_with_constant_memory();
            preparar_hoja(hoja, columnas)?;
            let formatos = FormatosHistorial::default();

            let mut consulta = filtro.clone();
            consulta.offset = 0;
            // La consulta limita internamente cada página a 200 filas. El
            // exportador las consume por lotes para no retener todo en RAM.
            consulta.limite = usize::MAX;
            loop {
                let pagina = self.buscar_historial(&consulta)?;
                if pagina.total > MAX_FILAS_DATOS_XLSX {
                    return Err(ExportarHistorialError::DemasiadasFilas(pagina.total));
                }
                consulta.corte_id = Some(pagina.corte_id);
                if pagina.items.is_empty() {
                    break;
                }
                for movimiento in &pagina.items {
                    let fila = u32::try_from(exportados + 1).unwrap_or(u32::MAX);
                    escribir_movimiento(hoja, fila, columnas, movimiento, &formatos)?;
                    exportados += 1;
                }
                if exportados >= pagina.total {
                    break;
                }
                consulta.offset = exportados;
            }

            let ultima_columna = u16::try_from(columnas.len() - 1).unwrap_or(u16::MAX);
            hoja.autofilter(
                0,
                0,
                u32::try_from(exportados).unwrap_or(u32::MAX),
                ultima_columna,
            )?;
        }

        // Se escribe junto al destino y sólo se publica al finalizar. Así un
        // error no deja un XLSX parcial y nunca se reemplaza otro archivo.
        let temporal = tempfile::Builder::new()
            .prefix(".historial-")
            .suffix(".xlsx")
            .tempfile_in(directorio)?
            .into_temp_path();
        libro.save(&temporal)?;
        temporal.persist_noclobber(destino).map_err(|error| {
            if error.error.kind() == std::io::ErrorKind::AlreadyExists {
                ExportarHistorialError::DestinoExiste(destino.to_owned())
            } else {
                ExportarHistorialError::Io(error.error)
            }
        })?;
        Ok(exportados)
    }

    pub fn registrar_salida(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), RegistroIngresoServiceError> {
        self.en_transaccion_con_reloj_validado(actor, |transaction, ahora| {
            let contratistas = SqliteContratistaRepository::new(transaction);
            let registros = SqliteRegistroIngresoRepository::new(transaction);
            RegistroIngresoService::new(&contratistas, &registros)
                .registrar_salida(id, ahora, actor.id)
        })
    }

    pub fn buscar_empresas(
        &self,
        filtro: &FiltroEmpresas,
    ) -> Result<Vec<EmpresaResumen>, EmpresaServiceError> {
        EmpresaConsultaService::new(&SqliteEmpresasQuery::new(&self.connection))
            .buscar_para_tabla(filtro)
    }

    pub fn crear_empresa(
        &self,
        actor: &UsuarioSesion,
        nombre: &str,
    ) -> Result<i64, EmpresaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(EmpresaServiceError::Database)?
            .ok_or(EmpresaServiceError::OperacionNoAutorizada)?;
        let id = EmpresaService::new(&SqliteEmpresaRepository::new(&transaction)).crear(nombre)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EmpresaServiceError::Database)?;
        Ok(id)
    }

    pub fn actualizar_empresa(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        nombre: &str,
    ) -> Result<(), EmpresaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(EmpresaServiceError::Database)?
            .ok_or(EmpresaServiceError::OperacionNoAutorizada)?;
        EmpresaService::new(&SqliteEmpresaRepository::new(&transaction)).actualizar(id, nombre)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EmpresaServiceError::Database)
    }

    pub fn activar_empresa(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), EmpresaServiceError> {
        self.establecer_empresa_activa(actor, id, true)
    }

    pub fn desactivar_empresa(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), EmpresaServiceError> {
        self.establecer_empresa_activa(actor, id, false)
    }

    fn establecer_empresa_activa(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        activa: bool,
    ) -> Result<(), EmpresaServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(EmpresaServiceError::Database)?
            .ok_or(EmpresaServiceError::OperacionNoAutorizada)?;
        if !actor_actual.rol.puede(Operacion::ActivarDesactivarEmpresa) {
            return Err(EmpresaServiceError::OperacionNoAutorizada);
        }
        let repositorio = SqliteEmpresaRepository::new(&transaction);
        let servicio = EmpresaService::new(&repositorio);
        if activa {
            servicio.activar(id)?;
        } else {
            servicio.desactivar(id)?;
        }
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EmpresaServiceError::Database)
    }

    pub fn buscar_usuarios(
        &self,
        actor: &UsuarioSesion,
        filtro: &FiltroUsuarios,
    ) -> Result<Vec<UsuarioResumen>, UsuarioServiceError> {
        let actor_actual = verificar_actor_activo(&self.connection, actor)?
            .ok_or(UsuarioServiceError::OperacionNoAutorizada)?;
        if !actor_actual.rol.puede(Operacion::GestionarUsuarios) {
            return Err(UsuarioServiceError::OperacionNoAutorizada);
        }
        UsuarioConsultaService::new(&SqliteUsuariosQuery::new(&self.connection))
            .buscar_para_tabla_como(filtro, actor_actual.rol)
    }

    pub fn crear_usuario(
        &self,
        actor: &UsuarioSesion,
        input: CrearUsuarioInput,
    ) -> Result<i64, UsuarioServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_creacion_usuario(&transaction, actor, input.rol)?;
        let id = UsuarioService::new(&SqliteUsuarioRepository::new(&transaction)).crear(input)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(id)
    }

    /// Parte barata de crear un usuario (sin Argon2) — permite correr el hash en un hilo
    /// aparte sin bloquear la TUI mientras se valida.
    pub fn validar_datos_para_crear_usuario(
        &self,
        actor: &UsuarioSesion,
        input: &CrearUsuarioInput,
    ) -> Result<(), UsuarioServiceError> {
        verificar_creacion_usuario(&self.connection, actor, input.rol)?;
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection))
            .validar_datos_para_crear(input)
    }

    pub fn crear_usuario_con_hash(
        &self,
        actor: &UsuarioSesion,
        cedula: &str,
        nombre: &str,
        rol: RolUsuario,
        activo: bool,
        password_hash: String,
    ) -> Result<i64, UsuarioServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_creacion_usuario(&transaction, actor, rol)?;
        let id = UsuarioService::new(&SqliteUsuarioRepository::new(&transaction)).crear_con_hash(
            cedula,
            nombre,
            rol,
            activo,
            password_hash,
        )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(id)
    }

    pub fn actualizar_usuario(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        input: ActualizarUsuarioInput,
        activo: bool,
    ) -> Result<(), UsuarioServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_gestion_usuario(&transaction, actor, id, true)?;
        if !puede_gestionar_usuario(actor_actual.rol, input.rol) {
            return Err(UsuarioServiceError::OperacionNoAutorizada);
        }
        UsuarioService::new(&SqliteUsuarioRepository::new(&transaction))
            .actualizar_administracion(id, input, activo)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    pub fn activar_usuario(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), UsuarioServiceError> {
        self.establecer_usuario_activo(actor, id, true)
    }

    pub fn desactivar_usuario(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), UsuarioServiceError> {
        self.establecer_usuario_activo(actor, id, false)
    }

    pub fn cambiar_password_usuario(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        password: &str,
    ) -> Result<(), UsuarioServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_gestion_usuario(&transaction, actor, id, false)?;
        UsuarioService::new(&SqliteUsuarioRepository::new(&transaction))
            .cambiar_password(id, password)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    pub fn validar_password_para_cambio(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        password: &str,
    ) -> Result<(), UsuarioServiceError> {
        verificar_gestion_usuario(&self.connection, actor, id, false)?;
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection))
            .validar_password_para_cambio(id, password)
    }

    pub fn cambiar_password_usuario_con_hash(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        password_hash: &str,
    ) -> Result<(), UsuarioServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_gestion_usuario(&transaction, actor, id, false)?;
        UsuarioService::new(&SqliteUsuarioRepository::new(&transaction))
            .cambiar_password_con_hash(id, password_hash)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    pub fn cambiar_mi_password(
        &self,
        actor: &UsuarioSesion,
        password_actual: &str,
        nueva_password: &str,
    ) -> Result<(), UsuarioServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)?
            .ok_or(UsuarioServiceError::OperacionNoAutorizada)?;
        if !puede_cambiar_password(
            actor_actual.id,
            actor_actual.rol,
            actor_actual.id,
            actor_actual.rol,
        ) {
            return Err(UsuarioServiceError::OperacionNoAutorizada);
        }
        UsuarioService::new(&SqliteUsuarioRepository::new(&transaction)).cambiar_password_propio(
            actor_actual.id,
            password_actual,
            nueva_password,
        )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    /// Resuelve el hash vigente y valida la contraseña nueva sin ejecutar
    /// Argon2. La TUI usa el candidato devuelto en un hilo aparte.
    pub fn preparar_cambio_password_propio(
        &self,
        actor: &UsuarioSesion,
        nueva_password: &str,
    ) -> Result<CandidatoAutenticacion, UsuarioServiceError> {
        let actor_actual = verificar_actor_activo(&self.connection, actor)?
            .ok_or(UsuarioServiceError::OperacionNoAutorizada)?;
        let repositorio = SqliteUsuarioRepository::new(&self.connection);
        UsuarioService::new(&repositorio)
            .validar_password_para_cambio(actor_actual.id, nueva_password)?;
        Ok(CandidatoAutenticacion {
            sesion: UsuarioSesion {
                id: actor_actual.id,
                cedula: actor_actual.cedula,
                nombre: actor_actual.nombre,
                rol: actor_actual.rol,
            },
            password_hash: actor_actual.password_hash,
        })
    }

    pub fn cambiar_mi_password_con_hash(
        &self,
        actor: &UsuarioSesion,
        hash_actual_verificado: &str,
        nuevo_hash: &str,
    ) -> Result<(), UsuarioServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)?
            .ok_or(UsuarioServiceError::OperacionNoAutorizada)?;
        if actor_actual.password_hash != hash_actual_verificado {
            return Err(UsuarioServiceError::PasswordActualIncorrecta);
        }
        UsuarioService::new(&SqliteUsuarioRepository::new(&transaction))
            .cambiar_password_con_hash(actor_actual.id, nuevo_hash)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    fn establecer_usuario_activo(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        activo: bool,
    ) -> Result<(), UsuarioServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_gestion_usuario(&transaction, actor, id, true)?;
        let repositorio = SqliteUsuarioRepository::new(&transaction);
        let servicio = UsuarioService::new(&repositorio);
        if activo {
            servicio.activar(id)?;
        } else {
            servicio.desactivar(id)?;
        }
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    pub fn crear_respaldo(
        &self,
        actor: &UsuarioSesion,
        tipo: TipoRespaldo,
    ) -> Result<RespaldoResumen, RespaldoError> {
        self.autorizar_respaldo(actor)?;
        self.crear_respaldo_sistema(tipo)
    }

    fn crear_respaldo_sistema(&self, tipo: TipoRespaldo) -> Result<RespaldoResumen, RespaldoError> {
        crate::database::backup::crear_respaldo(
            &self.connection,
            &self.directorio_respaldos(),
            tipo,
        )
    }

    pub fn listar_respaldos(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<Vec<RespaldoResumen>, RespaldoError> {
        self.autorizar_respaldo(actor)?;
        self.listar_respaldos_sistema()
    }

    fn listar_respaldos_sistema(&self) -> Result<Vec<RespaldoResumen>, RespaldoError> {
        crate::database::backup::listar_respaldos(&self.directorio_respaldos())
    }

    pub fn validar_respaldo(
        &self,
        actor: &UsuarioSesion,
        ruta: &Path,
    ) -> Result<ResultadoValidacion, RespaldoError> {
        self.autorizar_respaldo(actor)?;
        crate::database::backup::validar_respaldo(ruta)
    }

    /// Crea el respaldo automático del día si todavía no existe uno — a lo
    /// sumo uno por día calendario en Costa Rica. Es best-effort a
    /// propósito (no devuelve `Result`): a diferencia del respaldo previo a
    /// una migración, éste no es obligatorio, así que un fallo (disco lleno,
    /// permisos) no debe impedir que la app arranque.
    pub fn respaldo_automatico_diario_si_hace_falta(&self) {
        let Ok(listado) = self.listar_respaldos_sistema() else {
            return;
        };
        let hoy = crate::tiempo::fecha_costa_rica(self.reloj.ahora_utc());
        let ya_existe_hoy = listado.iter().any(|respaldo| {
            respaldo.tipo == TipoRespaldo::Automatico
                && crate::tiempo::fecha_costa_rica(respaldo.creado_en) == hoy
        });
        if ya_existe_hoy {
            return;
        }
        if self
            .crear_respaldo_sistema(TipoRespaldo::Automatico)
            .is_ok()
        {
            let _ = crate::database::backup::aplicar_retencion(
                &self.directorio_respaldos(),
                TipoRespaldo::Automatico,
                crate::database::backup::RETENCION_AUTOMATICOS,
            );
        }
    }

    /// El archivo interno ya fue validado por `crear_respaldo`; exportarlo es una
    /// copia simple a la ruta que indique el operador, sin volver a pasar por el
    /// motor de respaldo.
    pub fn exportar_respaldo(
        &self,
        actor: &UsuarioSesion,
        origen: &Path,
        destino: &Path,
    ) -> Result<(), RespaldoError> {
        self.autorizar_respaldo(actor)?;
        std::fs::copy(origen, destino)
            .map(|_| ())
            .map_err(Into::into)
    }

    fn autorizar_respaldo(&self, actor: &UsuarioSesion) -> Result<(), RespaldoError> {
        let usuario = verificar_actor_activo(&self.connection, actor)
            .map_err(|error| match error {
                DatabaseError::Sqlite(error) => RespaldoError::Sqlite(error),
                _ => RespaldoError::OperacionNoAutorizada,
            })?
            .ok_or(RespaldoError::OperacionNoAutorizada)?;
        if !usuario.rol.puede(Operacion::GestionarRespaldos) {
            return Err(RespaldoError::OperacionNoAutorizada);
        }
        Ok(())
    }

    fn directorio_respaldos(&self) -> PathBuf {
        self.ruta_base_datos
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups")
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

/// Comprobación de sanidad de todo el sistema, no una regla de negocio de
/// `RegistroIngresoService` (ver el comentario de
/// `en_transaccion_con_reloj_validado`). Sin SQL propio —
/// `ultimo_instante_movimiento` vive en `database::queries::ingresos`, junto
/// al resto del acceso a `registro_ingresos`.
fn validar_reloj(
    connection: &Connection,
    ahora: chrono::DateTime<chrono::Utc>,
) -> Result<(), RegistroIngresoServiceError> {
    let Some(ultima) = crate::database::queries::ingresos::ultimo_instante_movimiento(connection)?
    else {
        return Ok(());
    };
    if ahora < ultima {
        return Err(RegistroIngresoServiceError::RelojRetrocedido);
    }
    Ok(())
}

/// Comprobación de sanidad del actor, mismo criterio que `validar_reloj`: no
/// es una regla de negocio de una entrada/salida puntual, es "¿quién dice
/// que está registrando esto sigue siendo un operador real?". La FK de
/// `usuario_ingreso_id`/`usuario_salida_id` en SQLite sólo exige que el ID
/// exista — nunca que la cuenta siga activa.
fn verificar_operador_activo(
    connection: &Connection,
    actor: &UsuarioSesion,
) -> Result<(), RegistroIngresoServiceError> {
    if verificar_actor_activo(connection, actor)?.is_some() {
        Ok(())
    } else {
        Err(RegistroIngresoServiceError::OperadorNoAutorizado)
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

fn verificar_creacion_usuario(
    connection: &Connection,
    actor: &UsuarioSesion,
    objetivo: RolUsuario,
) -> Result<Usuario, UsuarioServiceError> {
    let actor_actual = verificar_actor_activo(connection, actor)?
        .ok_or(UsuarioServiceError::OperacionNoAutorizada)?;
    if !puede_gestionar_usuario(actor_actual.rol, objetivo) {
        return Err(UsuarioServiceError::OperacionNoAutorizada);
    }
    Ok(actor_actual)
}

/// Autoriza contra el rol y estado que existen ahora en SQLite. El rol del
/// snapshot de sesión nunca decide permisos: una degradación surte efecto en
/// la siguiente operación sin necesidad de reiniciar la TUI.
fn verificar_gestion_usuario(
    connection: &Connection,
    actor: &UsuarioSesion,
    objetivo_id: i64,
    permitir_mismo_usuario: bool,
) -> Result<Usuario, UsuarioServiceError> {
    let actor_actual = verificar_actor_activo(connection, actor)?
        .ok_or(UsuarioServiceError::OperacionNoAutorizada)?;
    let objetivo = SqliteUsuarioRepository::new(connection)
        .buscar_por_id(objetivo_id)?
        .ok_or(UsuarioServiceError::UsuarioNoEncontrado)?;
    if (!permitir_mismo_usuario && actor_actual.id == objetivo.id)
        || !puede_gestionar_usuario(actor_actual.rol, objetivo.rol)
    {
        return Err(UsuarioServiceError::OperacionNoAutorizada);
    }
    Ok(actor_actual)
}
