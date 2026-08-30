//! Catálogo de gafetes (`docs/plan-gafetes.md`). **Sin restricción de rol a
//! propósito** — decisión explícita del usuario: cualquier operador con
//! sesión activa gestiona el catálogo completo (alta/baja/perdido/resolver),
//! a diferencia de Empresas/Usuarios. Sólo se exige que el actor siga siendo
//! un usuario activo (`verificar_actor_activo`), mismo mínimo que el resto
//! de `AppCore`.

use rusqlite::{Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::queries::gafetes::{FiltroGafetes, GafeteResumen, SqliteGafetesQuery};
use crate::database::queries::gafetes_incidentes::SqliteGafetesIncidentes;
use crate::database::repositories::contratista_repository::SqliteContratistaRepository;
use crate::database::repositories::gafete_repository::SqliteGafeteRepository;
use crate::models::gafete::MotivoResolucionGafete;
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

    pub fn crear_gafete(
        &self,
        actor: &UsuarioSesion,
        numero: i64,
    ) -> Result<i64, GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let contratistas = SqliteContratistaRepository::new(&transaction);
        let id = GafeteService::new(&gafetes, &contratistas).crear_uno(numero)?;
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
    ) -> Result<Vec<i64>, GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let contratistas = SqliteContratistaRepository::new(&transaction);
        let ids = GafeteService::new(&gafetes, &contratistas).crear_rango(desde, hasta)?;
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
        let contratistas = SqliteContratistaRepository::new(&transaction);
        GafeteService::new(&gafetes, &contratistas).dar_de_baja(id)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    pub fn marcar_gafete_perdido(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        contratista_deudor_id: i64,
    ) -> Result<(), GafeteServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(GafeteServiceError::Database)?
            .ok_or(GafeteServiceError::OperacionNoAutorizada)?;
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let contratistas = SqliteContratistaRepository::new(&transaction);
        let incidentes = SqliteGafetesIncidentes::new(&transaction);
        GafeteService::new(&gafetes, &contratistas).marcar_perdido(
            &incidentes,
            id,
            contratista_deudor_id,
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
        let contratistas = SqliteContratistaRepository::new(&transaction);
        let incidentes = SqliteGafetesIncidentes::new(&transaction);
        GafeteService::new(&gafetes, &contratistas).resolver(
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
