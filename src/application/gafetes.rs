//! Catálogo de gafetes (`docs/plan-gafetes.md`). **Sin restricción de rol a
//! propósito** — decisión explícita del usuario: cualquier operador con
//! sesión activa gestiona el catálogo completo (alta/baja/perdido/resolver),
//! a diferencia de Empresas/Usuarios. Sólo se exige que el actor siga siendo
//! un usuario activo (`verificar_actor_activo`), mismo mínimo que el resto
//! de `AppCore`.

use rusqlite::{Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::queries::gafetes::{FiltroGafetes, GafeteResumen, SqliteGafetesQuery};
use crate::database::queries::gafetes_incidentes::{
    GafetesIncidentesQuery, IncidenteGafete, SqliteGafetesIncidentes,
};
use crate::database::repositories::cita_repository::{CitaRepository, SqliteCitaRepository};
use crate::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use crate::database::repositories::gafete_repository::SqliteGafeteRepository;
use crate::database::repositories::registro_ingreso_repository::SqliteRegistroIngresoRepository;
use crate::models::gafete::{MotivoResolucionGafete, PortadorGafete, TipoGafete};
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::GafeteServiceError;
use crate::services::gafete_service::{GafeteConsultaService, GafeteService};

use super::{AppCore, verificar_actor_activo};

impl AppCore {
    pub fn buscar_gafetes(
        &self,
        filtro: &FiltroGafetes,
    ) -> Result<Vec<GafeteResumen>, GafeteServiceError> {
        let query = SqliteGafetesQuery::new(&self.connection);
        GafeteConsultaService::new(&query).buscar(filtro)
    }

    /// Historial de incidentes de un gafete puntual (quién lo marcó perdido,
    /// quién lo resolvió y cuándo) — sin actor, mismo criterio sin
    /// restricción que `buscar_gafetes`.
    pub fn historial_gafete(
        &self,
        gafete_id: i64,
    ) -> Result<Vec<IncidenteGafete>, GafeteServiceError> {
        let incidentes = SqliteGafetesIncidentes::new(&self.connection);
        Ok(incidentes.historial(gafete_id)?)
    }

    pub fn crear_gafete(
        &self,
        actor: &UsuarioSesion,
        numero: i64,
        tipo: TipoGafete,
    ) -> Result<i64, GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let id = GafeteService::new(&gafetes).crear_uno(numero, tipo)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(id)
    }

    /// Alta por rango (desde-hasta), para cargar p. ej. 01-25 de una vez —
    /// si un número del rango falla (típicamente duplicado), el rango
    /// completo aborta: la transacción nunca comitea, sin alta parcial.
    pub fn crear_gafetes_rango(
        &self,
        actor: &UsuarioSesion,
        desde: i64,
        hasta: i64,
        tipo: TipoGafete,
    ) -> Result<Vec<i64>, GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let ids = GafeteService::new(&gafetes).crear_rango(desde, hasta, tipo)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(ids)
    }

    pub fn dar_de_baja_gafete(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let registros = SqliteRegistroIngresoRepository::new(&transaction);
        GafeteService::new(&gafetes).dar_de_baja(&registros, id)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    /// Valida que el contratista exista (responsabilidad que ya no vive en
    /// `GafeteService`, que se mantiene genérico -- ver el doc-comment del
    /// servicio) antes de armar el `PortadorGafete::Contratista`.
    pub fn marcar_gafete_perdido_contratista(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        contratista_id: i64,
    ) -> Result<(), GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let contratistas = SqliteContratistaRepository::new(&transaction);
        if contratistas.buscar_por_id(contratista_id)?.is_none() {
            return Err(GafeteServiceError::ContratistaNoEncontrado);
        }
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let incidentes = SqliteGafetesIncidentes::new(&transaction);
        let registros = SqliteRegistroIngresoRepository::new(&transaction);
        GafeteService::new(&gafetes).marcar_perdido(
            &incidentes,
            &registros,
            id,
            PortadorGafete::Contratista(contratista_id),
            actor_actual.id,
            self.reloj.ahora_utc(),
        )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    /// Misma idea que `marcar_gafete_perdido_contratista`, pero valida
    /// existencia contra `CitaRepository` en vez de `ContratistaRepository`
    /// -- `GafeteService` no distingue, sólo recibe el `PortadorGafete` ya
    /// armado.
    pub fn marcar_gafete_perdido_visita(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        cita_visitante_id: i64,
    ) -> Result<(), GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let citas = SqliteCitaRepository::new(&transaction);
        if citas.buscar_visitante_por_id(cita_visitante_id)?.is_none() {
            return Err(GafeteServiceError::VisitaNoEncontrada);
        }
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let incidentes = SqliteGafetesIncidentes::new(&transaction);
        let registros = SqliteRegistroIngresoRepository::new(&transaction);
        GafeteService::new(&gafetes).marcar_perdido(
            &incidentes,
            &registros,
            id,
            PortadorGafete::Visita(cita_visitante_id),
            actor_actual.id,
            self.reloj.ahora_utc(),
        )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    pub fn resolver_gafete(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        motivo: MotivoResolucionGafete,
    ) -> Result<(), GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let incidentes = SqliteGafetesIncidentes::new(&transaction);
        GafeteService::new(&gafetes).resolver(
            &incidentes,
            id,
            motivo,
            actor_actual.id,
            self.reloj.ahora_utc(),
        )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }
}
