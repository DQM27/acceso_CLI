//! Usuarios: consulta y cambio de la contraseña propia.
//!
//! El alta, la edición, la activación y el reseteo de contraseña de OTROS
//! usuarios se hacen sólo desde el panel web (Supabase sólo deja escribir
//! `usuarios` a un administrador del panel); cada equipo recibe el
//! resultado al sincronizar el catálogo.

use rusqlite::{Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::queries::auditoria::SqliteAuditoria;
use crate::database::queries::usuarios::{FiltroUsuarios, SqliteUsuariosQuery, UsuarioResumen};
use crate::database::repositories::usuario_repository::SqliteUsuarioRepository;
use crate::domain::autorizacion::{Operacion, puede_cambiar_password};
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::UsuarioServiceError;
use crate::services::usuario_service::{UsuarioConsultaService, UsuarioService};

use super::{AppCore, verificar_actor_activo};

impl AppCore {
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

    /// Verifica la contraseña actual sin cambiar nada — gate de `/clave` en
    /// la CLI antes de mostrar los campos de contraseña
    /// nueva: verificar primero evita pedirla dos veces sólo para
    /// descartarla al final porque la actual estaba mal.
    pub fn verificar_mi_password(
        &self,
        actor: &UsuarioSesion,
        password: &str,
    ) -> Result<(), UsuarioServiceError> {
        let actor_actual = verificar_actor_activo(&self.connection, actor)?
            .ok_or(UsuarioServiceError::OperacionNoAutorizada)?;
        match crate::services::password::verificar_password(password, &actor_actual.password_hash) {
            Ok(true) => Ok(()),
            Ok(false) => Err(UsuarioServiceError::PasswordActualIncorrecta),
            Err(error) => Err(error.into()),
        }
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
        UsuarioService::new(&SqliteUsuarioRepository::new(&transaction))
            .cambiar_password_propio_auditado(
                actor_actual.id,
                password_actual,
                nueva_password,
                &actor_actual.nombre,
                self.reloj.ahora_utc(),
                &SqliteAuditoria::new(&transaction),
            )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }
}
