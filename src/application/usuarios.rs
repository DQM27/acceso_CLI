//! Usuarios: alta/edición, activación, y las 3 contraseñas (crear, reset
//! administrativo, cambio propio).

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::queries::auditoria::SqliteAuditoria;
use crate::database::queries::usuarios::{FiltroUsuarios, SqliteUsuariosQuery, UsuarioResumen};
use crate::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use crate::domain::autorizacion::{Operacion, puede_cambiar_password, puede_gestionar_usuario};
use crate::models::usuario::{RolUsuario, Usuario};
use crate::services::autenticacion_service::{CandidatoAutenticacion, UsuarioSesion};
use crate::services::error::UsuarioServiceError;
use crate::services::usuario_service::{
    ActualizarUsuarioInput, CrearUsuarioInput, UsuarioConsultaService, UsuarioService,
};

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

    /// Usuarios ROOT activos — usado por el flujo de recuperación `--reset-root`
    /// (main.rs) para saber a cuál restablecer cuando hay más de uno.
    pub fn listar_roots_activos(&self) -> Result<Vec<Usuario>, UsuarioServiceError> {
        let usuarios =
            UsuarioService::new(&SqliteUsuarioRepository::new(&self.connection)).listar()?;
        Ok(usuarios
            .into_iter()
            .filter(|usuario| usuario.rol == RolUsuario::Root && usuario.activo)
            .collect())
    }

    /// Camino de recuperación fuera de la TUI (`--reset-root` en main.rs), pensado para
    /// cuando el ROOT olvidó su contraseña y no hay otro admin/root con sesión para
    /// restablecérsela. A propósito no pasa por `verificar_gestion_usuario` como el
    /// resto de `cambiar_password_usuario_*`: no hay actor logueado, porque este
    /// flujo existe justo para cuando nadie puede loguearse. Su única barrera es
    /// tener acceso al ejecutable y al archivo de la base de datos — quien tiene eso
    /// ya podría manipular el `.sqlite` directamente, así que esto no baja el nivel
    /// de seguridad real, sólo evita tener que calcular un hash Argon2 a mano.
    pub fn resetear_password_root(
        &self,
        id: i64,
        nueva_password: &str,
    ) -> Result<(), UsuarioServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let usuario = SqliteUsuarioRepository::new(&transaction)
            .buscar_por_id(id)?
            .ok_or(UsuarioServiceError::UsuarioNoEncontrado)?;
        if usuario.rol != RolUsuario::Root || !usuario.activo {
            return Err(UsuarioServiceError::UsuarioNoEncontrado);
        }
        UsuarioService::new(&SqliteUsuarioRepository::new(&transaction))
            .cambiar_password(id, nueva_password)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
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
            .actualizar_administracion_auditada(
                id,
                input,
                activo,
                actor_actual.id,
                &actor_actual.nombre,
                self.reloj.ahora_utc(),
                &SqliteAuditoria::new(&transaction),
            )?;
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
        let actor_actual = verificar_gestion_usuario(&transaction, actor, id, false)?;
        UsuarioService::new(&SqliteUsuarioRepository::new(&transaction))
            .cambiar_password_auditado(
                id,
                password,
                actor_actual.id,
                &actor_actual.nombre,
                self.reloj.ahora_utc(),
                &SqliteAuditoria::new(&transaction),
            )?;
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
        let actor_actual = verificar_gestion_usuario(&transaction, actor, id, false)?;
        UsuarioService::new(&SqliteUsuarioRepository::new(&transaction))
            .cambiar_password_con_hash_auditado(
                id,
                password_hash,
                actor_actual.id,
                &actor_actual.nombre,
                self.reloj.ahora_utc(),
                &SqliteAuditoria::new(&transaction),
            )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
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
            .cambiar_password_con_hash_auditado(
                actor_actual.id,
                nuevo_hash,
                actor_actual.id,
                &actor_actual.nombre,
                self.reloj.ahora_utc(),
                &SqliteAuditoria::new(&transaction),
            )?;
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
        let actor_actual = verificar_gestion_usuario(&transaction, actor, id, true)?;
        let repositorio = SqliteUsuarioRepository::new(&transaction);
        let servicio = UsuarioService::new(&repositorio);
        servicio.establecer_activo_auditado(
            id,
            activo,
            actor_actual.id,
            &actor_actual.nombre,
            self.reloj.ahora_utc(),
            &SqliteAuditoria::new(&transaction),
        )?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(())
    }
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

/// Autoriza contra el rol y estado que existen ahora en `SQLite`. El rol del
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
