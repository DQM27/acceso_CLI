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
        AutenticacionService::new(&repository).autenticar(cedula, password, self.reloj.ahora_utc())
    }

    /// Ver `AutenticacionService::autenticar_con_estado` -- para un login
    /// LOCAL (mobile: `Nucleo::autenticar`/`autenticar_con_secreto`) que
    /// necesita saber si debe exigir el cambio de contraseña, en vez de
    /// asumir siempre que no (hallazgo de auditoría 2026-09-24,
    /// MV-01/DF-03).
    pub fn autenticar_con_estado(
        &self,
        cedula: &str,
        password: &str,
    ) -> Result<(UsuarioSesion, bool), AutenticacionError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        AutenticacionService::new(&repository).autenticar_con_estado(
            cedula,
            password,
            self.reloj.ahora_utc(),
        )
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
        AutenticacionService::new(&repository).buscar_candidato(cedula, self.reloj.ahora_utc())
    }

    /// Ver `AutenticacionService::resolver_identidad_local` -- para cuando
    /// la contraseña ya se verificó contra Supabase Auth
    /// (`nube::auth_supabase::login`), no localmente.
    pub fn resolver_identidad_local(
        &self,
        cedula: &str,
    ) -> Result<UsuarioSesion, AutenticacionError> {
        let repository = SqliteUsuarioRepository::new(&self.connection);
        AutenticacionService::new(&repository).resolver_identidad_local(cedula)
    }

    /// Cachea localmente un hash real de `password` para `id` -- se llama
    /// tras un login exitoso contra Supabase Auth (`nube::auth_supabase::login`,
    /// ver `login_supabase`/`autenticar_supabase` en desktop/mobile), nunca
    /// con una contraseña sin verificar. Ver `Usuario::password_hash_confirmado_en`
    /// y `docs/decisiones-tecnicas.md` (entrada 2026-09-18): existe para que
    /// un usuario global (Administrador/Operador) pueda seguir operando sin
    /// internet ante un corte, acotado a `TOPE_CACHE_LOCAL_OFFLINE` (24h,
    /// `services::autenticacion_service`) desde este instante -- pasado ese
    /// tope, `AutenticacionService::buscar_candidato` deja de aceptarlo y
    /// hay que volver a loguear online para renovarlo.
    ///
    /// Quien llama debe tratar un error acá como best-effort (mismo criterio
    /// que el resto de la sincronización "mejor esfuerzo" de este crate):
    /// no cachear no debe tumbar un login que de por sí ya fue exitoso
    /// contra Supabase, sólo significa que el próximo corte de internet no
    /// va a tener este atajo disponible para esta cuenta.
    /// `debe_cambiar_password` queda grabado junto al hash
    /// (`Usuario::password_temporal_cacheada`) -- pasar `true` cuando la
    /// contraseña que se está cacheando es una temporal todavía sin
    /// cambiar (login online con `debe_cambiar_password` en `true`) es lo
    /// que le permite a un login sin conexión seguir exigiendo el cambio
    /// en vez de dejarlo pasar (hallazgo de auditoría 2026-09-24,
    /// MV-01/DF-03). Un refresco de caché tras un cambio de contraseña
    /// real (`cambiar_password_supabase`) debe pasar `false`.
    pub fn cachear_password_local(
        &self,
        id: i64,
        password: &str,
        debe_cambiar_password: bool,
    ) -> Result<(), UsuarioServiceError> {
        let hash = crate::services::password::generar_hash(password)?;
        let confirmado_en = crate::tiempo::serializar_utc(self.reloj.ahora_utc());
        let repository = SqliteUsuarioRepository::new(&self.connection);
        repository.actualizar_password_cacheada(
            id,
            &hash,
            &confirmado_en,
            debe_cambiar_password,
        )?;
        Ok(())
    }

}
