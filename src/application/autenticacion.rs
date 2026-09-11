//! Arranque (configuración inicial / ROOT inicial) y autenticación.

use crate::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use crate::services::autenticacion_service::{
    AutenticacionService, CandidatoAutenticacion, UsuarioSesion,
};
use crate::services::error::{AutenticacionError, UsuarioServiceError};
use crate::services::usuario_service::{CrearRootInicialInput, UsuarioService};

use super::{AppCore, verificar_actor_activo};

impl AppCore {
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

    /// `false` si `sesion` ya no corresponde a un usuario activo -- por
    /// ejemplo, lo desactivaron en otro dispositivo y esta base recién lo
    /// recibió por sync (ver `nube::AppCore::sincronizar_con_nube`,
    /// `ResumenSincronizacion::sesion_expulsada`). Falla "abierto" (`true`)
    /// ante un error de base de datos -- un glitch transitorio durante un
    /// sync no debería expulsar a nadie por las dudas.
    pub fn sesion_sigue_activa(&self, sesion: &UsuarioSesion) -> bool {
        verificar_actor_activo(&self.connection, sesion).map_or(true, |usuario| usuario.is_some())
    }

    /// Resuelve la cédula sin verificar todavía la contraseña — rápido, sólo `SQLite`. Permite
    /// correr la verificación de Argon2 (lenta) en un hilo aparte sin compartir la conexión.
    pub fn buscar_candidato_autenticacion(
        &self,
        cedula: &str,
    ) -> Result<CandidatoAutenticacion, AutenticacionError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        AutenticacionService::new(&repository).buscar_candidato(cedula)
    }

    /// Ver `AutenticacionService::resolver_identidad_local` -- para cuando
    /// la contraseña ya se verificó contra Supabase Auth
    /// (`nube::auth_supabase::login`), no localmente.
    pub fn resolver_identidad_local(&self, cedula: &str) -> Result<UsuarioSesion, AutenticacionError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        AutenticacionService::new(&repository).resolver_identidad_local(cedula)
    }

    /// Completa el alta de contraseña de un usuario global (Administrador/Operador,
    /// sincronizado por catálogo -- ver `nube::sincronizacion::recibir_catalogo_del_sitio`)
    /// que todavía no inició sesión EN ESTE dispositivo (`AutenticacionError::SinPasswordLocal`).
    /// No exige conocer una contraseña anterior (nunca existió acá) -- sólo la cédula, que la
    /// pantalla de login ya tiene de haber intentado entrar. Deja logueada la sesión directo
    /// en vez de forzar un segundo intento con la contraseña recién fijada.
    pub fn fijar_password_inicial(
        &self,
        cedula: &str,
        nueva_password: &str,
    ) -> Result<UsuarioSesion, UsuarioServiceError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        let usuario = repository
            .buscar_por_cedula(cedula.trim())?
            .ok_or(UsuarioServiceError::UsuarioNoEncontrado)?;
        if !usuario.activo {
            return Err(UsuarioServiceError::UsuarioInactivo);
        }
        if usuario.password_hash != crate::services::password::SIN_PASSWORD_LOCAL {
            return Err(UsuarioServiceError::YaTienePasswordLocal);
        }
        UsuarioService::new(&repository).cambiar_password(usuario.id, nueva_password)?;
        Ok(UsuarioSesion {
            id: usuario.id,
            cedula: usuario.cedula,
            nombre: usuario.nombre,
            rol: usuario.rol,
        })
    }
}
