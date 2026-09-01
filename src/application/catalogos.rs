//! Contratistas y Empresas.

use rusqlite::{Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::queries::auditoria::{CambioAuditado, FiltroAuditoria, SqliteAuditoria};
use crate::database::queries::contratistas::{
    FiltroContratistas, PaginaContratistas, SqliteContratistasQuery,
};
use crate::database::queries::empresas::{EmpresaResumen, FiltroEmpresas, SqliteEmpresasQuery};
use crate::database::queries::gafetes_incidentes::{
    GafetesIncidentesQuery, IncidenteGafete, SqliteGafetesIncidentes,
};
use crate::database::repositories::contratista_repository::SqliteContratistaRepository;
use crate::database::repositories::empresa_repository::SqliteEmpresaRepository;
use crate::domain::autorizacion::Operacion;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::contratista_service::{
    ContratistaConsultaService, ContratistaService, DatosActualizacionContratista, DatosContratista,
};
use crate::services::empresa_service::{EmpresaConsultaService, EmpresaService};
use crate::services::error::{ContratistaServiceError, EmpresaServiceError};

use super::{AppCore, CargaCompleta, LIMITE_CARGA_COMPLETA_MAXIMO, verificar_actor_activo};

/// Núcleo de [`AppCore::buscar_auditoria`] sobre una `Connection` cualquiera
/// — mismo motivo que `buscar_historial_con_conexion`
/// (`src/application/historial.rs`): un comando Tauri puede abrir su propia
/// conexión en vez de retener el `Mutex<AppCore>` compartido.
pub fn buscar_auditoria_con_conexion(
    connection: &rusqlite::Connection,
    actor: &UsuarioSesion,
    filtro: &FiltroAuditoria,
) -> Result<crate::database::queries::auditoria::PaginaAuditoria, ContratistaServiceError> {
    let actor_actual = verificar_actor_activo(connection, actor)?
        .ok_or(ContratistaServiceError::OperacionNoAutorizada)?;
    if !actor_actual.rol.puede(Operacion::VerAuditoria) {
        return Err(ContratistaServiceError::OperacionNoAutorizada);
    }
    Ok(SqliteAuditoria::new(connection).buscar(filtro)?)
}

/// Núcleo de [`AppCore::buscar_auditoria_completo`] sobre una `Connection`
/// cualquiera — mismo motivo que [`buscar_auditoria_con_conexion`]: evita
/// retener el núcleo compartido durante los ~750ms que puede tardar esta
/// consulta (medido en la auditoría de las tres capas, `docs/pendientes.md`).
pub fn buscar_auditoria_completo_con_conexion(
    connection: &rusqlite::Connection,
    actor: &UsuarioSesion,
) -> Result<CargaCompleta<CambioAuditado>, ContratistaServiceError> {
    let mut consulta = FiltroAuditoria {
        limite: usize::MAX,
        offset: 0,
    };
    let mut todos = Vec::new();
    let mut total;
    loop {
        let pagina = buscar_auditoria_con_conexion(connection, actor, &consulta)?;
        total = pagina.total;
        if pagina.items.is_empty() {
            break;
        }
        todos.extend(pagina.items);
        if todos.len() >= total || todos.len() >= LIMITE_CARGA_COMPLETA_MAXIMO {
            break;
        }
        consulta.offset = todos.len();
    }
    Ok(CargaCompleta {
        truncado: todos.len() < total,
        items: todos,
    })
}

impl AppCore {
    pub fn buscar_contratistas(
        &self,
        filtro: &FiltroContratistas,
    ) -> Result<PaginaContratistas, ContratistaServiceError> {
        let query = SqliteContratistasQuery::new(&self.connection);
        ContratistaConsultaService::new(&query).buscar_para_tabla(filtro)
    }

    /// Auditoría genérica (contratistas, empresas, usuarios — ver
    /// `EntidadAuditada`, `src/database/queries/auditoria.rs`), no sólo de
    /// contratistas. Sigue gateada por `Operacion::VerAuditoria`: da igual
    /// de qué entidad sea el cambio, es la misma información sensible.
    pub fn buscar_auditoria(
        &self,
        actor: &UsuarioSesion,
        filtro: &FiltroAuditoria,
    ) -> Result<crate::database::queries::auditoria::PaginaAuditoria, ContratistaServiceError> {
        buscar_auditoria_con_conexion(&self.connection, actor, filtro)
    }

    /// Todo el conjunto en un solo `Vec`, no sólo una página — mismo
    /// criterio que `buscar_historial_completo`
    /// (`src/application/historial.rs`) para una interfaz que virtualiza del
    /// lado del cliente (AG Grid) en vez de paginar por su cuenta. A
    /// diferencia de historial, `FiltroAuditoria` no tiene un `corte_id` —
    /// `auditoria_cambios` también es append-only, así que un cambio nuevo
    /// insertado justo mientras se pagina podría, en teoría, correr una fila
    /// entre páginas; caso raro (auditoría no se llena tan rápido como los
    /// ingresos) y no hay mecanismo de corte que reutilizar sin agregarlo
    /// primero a la consulta de abajo. Se corta en
    /// [`LIMITE_CARGA_COMPLETA_MAXIMO`] — a diferencia de Historial, esta
    /// pantalla no tiene selector de rango de fechas, así que es la única
    /// barrera real contra un total que crezca sin límite.
    pub fn buscar_auditoria_completo(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<CargaCompleta<CambioAuditado>, ContratistaServiceError> {
        buscar_auditoria_completo_con_conexion(&self.connection, actor)
    }

    /// Incidentes de gafetes (marcar perdido/resolver, `gafetes_incidentes`)
    /// para la pantalla general de Auditoría — mismo gate que
    /// `buscar_auditoria`, aunque el dato viene de una tabla aparte
    /// (`gafetes_incidentes`, no `auditoria_cambios`): es la misma
    /// información sensible sin importar de qué tabla salga. A diferencia
    /// del historial por gafete puntual (`AppCore::historial_gafete`, sin
    /// restricción), acá sí aplica `Operacion::VerAuditoria`.
    pub fn buscar_auditoria_gafetes(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<Vec<IncidenteGafete>, ContratistaServiceError> {
        let actor_actual = verificar_actor_activo(&self.connection, actor)?
            .ok_or(ContratistaServiceError::OperacionNoAutorizada)?;
        if !actor_actual.rol.puede(Operacion::VerAuditoria) {
            return Err(ContratistaServiceError::OperacionNoAutorizada);
        }
        Ok(SqliteGafetesIncidentes::new(&self.connection).historial_completo()?)
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
        if actual.cedula != datos.cedula.trim()
            && !actor_actual.rol.puede(Operacion::EditarCedulaContratista)
        {
            return Err(ContratistaServiceError::OperacionNoAutorizada);
        }
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
            &actor_actual.nombre,
            self.reloj.ahora_utc(),
            &SqliteAuditoria::new(&transaction),
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
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(EmpresaServiceError::Database)?
            .ok_or(EmpresaServiceError::OperacionNoAutorizada)?;
        EmpresaService::new(&SqliteEmpresaRepository::new(&transaction)).actualizar_auditado(
            id,
            nombre,
            actor_actual.id,
            &actor_actual.nombre,
            self.reloj.ahora_utc(),
            &SqliteAuditoria::new(&transaction),
        )?;
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
        servicio.establecer_activo_auditado(
            id,
            activa,
            actor_actual.id,
            &actor_actual.nombre,
            self.reloj.ahora_utc(),
            &SqliteAuditoria::new(&transaction),
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EmpresaServiceError::Database)
    }
}
