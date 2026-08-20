use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::database::backup::{RespaldoError, RespaldoResumen, ResultadoValidacion, TipoRespaldo};
use crate::database::connection::open_database;
use crate::database::error::DatabaseError;
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
use crate::database::repositories::usuario_repository::SqliteUsuarioRepository;
use crate::database::schema::SchemaError;
use crate::models::usuario::RolUsuario;
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
        datos: DatosActualizacionContratista,
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
    /// reloj no haya retrocedido, corre `operar` y confirma. Compartido por
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
    fn en_transaccion_con_reloj_validado<T>(
        &self,
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
        let resultado = operar(&transaction, ahora)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(resultado)
    }

    pub fn registrar_ingreso(
        &self,
        contratista_id: i64,
        medio: crate::models::medio_ingreso::MedioIngreso,
        gafete: Option<i64>,
        usuario_id: i64,
    ) -> Result<ResultadoRegistroEntrada, RegistroIngresoServiceError> {
        self.en_transaccion_con_reloj_validado(|transaction, ahora| {
            let contratistas = SqliteContratistaRepository::new(transaction);
            let registros = SqliteRegistroIngresoRepository::new(transaction);
            RegistroIngresoService::new(&contratistas, &registros).registrar_entrada(
                contratista_id,
                medio,
                gafete,
                usuario_id,
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

    pub fn registrar_salida(
        &self,
        id: i64,
        usuario: i64,
    ) -> Result<(), RegistroIngresoServiceError> {
        self.en_transaccion_con_reloj_validado(|transaction, ahora| {
            let contratistas = SqliteContratistaRepository::new(transaction);
            let registros = SqliteRegistroIngresoRepository::new(transaction);
            RegistroIngresoService::new(&contratistas, &registros)
                .registrar_salida(id, ahora, usuario)
        })
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

    pub fn activar_empresa(&self, id: i64) -> Result<(), EmpresaServiceError> {
        EmpresaService::new(&SqliteEmpresaRepository::new(&self.connection)).activar(id)
    }

    pub fn desactivar_empresa(&self, id: i64) -> Result<(), EmpresaServiceError> {
        EmpresaService::new(&SqliteEmpresaRepository::new(&self.connection)).desactivar(id)
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

    /// Parte barata de crear un usuario (sin Argon2) — permite correr el hash en un hilo
    /// aparte sin bloquear la TUI mientras se valida.
    pub fn validar_datos_para_crear_usuario(
        &self,
        input: &CrearUsuarioInput,
    ) -> Result<(), UsuarioServiceError> {
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection))
            .validar_datos_para_crear(input)
    }

    pub fn crear_usuario_con_hash(
        &self,
        cedula: &str,
        nombre: &str,
        rol: RolUsuario,
        activo: bool,
        password_hash: String,
    ) -> Result<i64, UsuarioServiceError> {
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection)).crear_con_hash(
            cedula,
            nombre,
            rol,
            activo,
            password_hash,
        )
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

    pub fn validar_password_para_cambio(
        &self,
        id: i64,
        password: &str,
    ) -> Result<(), UsuarioServiceError> {
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection))
            .validar_password_para_cambio(id, password)
    }

    pub fn cambiar_password_usuario_con_hash(
        &self,
        id: i64,
        password_hash: &str,
    ) -> Result<(), UsuarioServiceError> {
        UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection))
            .cambiar_password_con_hash(id, password_hash)
    }

    pub fn crear_respaldo(&self, tipo: TipoRespaldo) -> Result<RespaldoResumen, RespaldoError> {
        crate::database::backup::crear_respaldo(
            &self.connection,
            &self.directorio_respaldos(),
            tipo,
        )
    }

    pub fn listar_respaldos(&self) -> Result<Vec<RespaldoResumen>, RespaldoError> {
        crate::database::backup::listar_respaldos(&self.directorio_respaldos())
    }

    pub fn validar_respaldo(&self, ruta: &Path) -> Result<ResultadoValidacion, RespaldoError> {
        crate::database::backup::validar_respaldo(ruta)
    }

    /// Crea el respaldo automático del día si todavía no existe uno — a lo
    /// sumo uno por día calendario en Costa Rica. Es best-effort a
    /// propósito (no devuelve `Result`): a diferencia del respaldo previo a
    /// una migración, éste no es obligatorio, así que un fallo (disco lleno,
    /// permisos) no debe impedir que la app arranque.
    pub fn respaldo_automatico_diario_si_hace_falta(&self) {
        let Ok(listado) = self.listar_respaldos() else {
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
        if self.crear_respaldo(TipoRespaldo::Automatico).is_ok() {
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
    pub fn exportar_respaldo(&self, origen: &Path, destino: &Path) -> std::io::Result<()> {
        std::fs::copy(origen, destino).map(|_| ())
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
