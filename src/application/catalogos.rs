//! Contratistas y Empresas.

use rusqlite::{Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::queries::auditoria_contratistas::{
    FiltroAuditoriaContratistas, PaginaAuditoriaContratistas, SqliteAuditoriaContratistas,
};
use crate::database::queries::contratistas::{
    FiltroContratistas, PaginaContratistas, SqliteContratistasQuery,
};
use crate::database::queries::empresas::{EmpresaResumen, FiltroEmpresas, SqliteEmpresasQuery};
use crate::database::repositories::contratista_repository::SqliteContratistaRepository;
use crate::database::repositories::empresa_repository::SqliteEmpresaRepository;
use crate::domain::autorizacion::Operacion;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::contratista_service::{
    ContratistaConsultaService, ContratistaService, DatosActualizacionContratista, DatosContratista,
};
use crate::services::empresa_service::{EmpresaConsultaService, EmpresaService};
use crate::services::error::{ContratistaServiceError, EmpresaServiceError};

use super::{AppCore, verificar_actor_activo};

impl AppCore {
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
}
