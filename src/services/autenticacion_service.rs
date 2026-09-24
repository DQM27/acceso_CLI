use chrono::{DateTime, Utc};

use crate::database::repositories::usuario_repository::UsuarioRepository;
use crate::models::usuario::RolUsuario;
use crate::tiempo::parsear_utc;

use super::error::AutenticacionError;
use super::password::{SIN_PASSWORD_LOCAL, verificar_password};

/// Cuánto dura válido un hash cacheado tras un login online exitoso contra
/// Supabase (`Usuario::password_hash_confirmado_en`) -- ver
/// `docs/decisiones-tecnicas.md`, entrada 2026-09-18. Sólo aplica a ese
/// caché: un hash permanente (`password_hash_confirmado_en` en `None` --
/// ROOT, o cualquier cuenta local de antes de la migración a Supabase Auth)
/// nunca vence, sin importar este tope.
const TOPE_CACHE_LOCAL_OFFLINE: chrono::Duration = chrono::Duration::hours(24);

/// Identidad autenticada que puede cruzar hacia aplicación/presentación sin exponer el hash.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct UsuarioSesion {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub rol: RolUsuario,
}

/// Resultado de resolver la cédula (existe, está activo) sin haber verificado todavía la
/// contraseña — permite separar la parte que toca `SQLite` (rápida) de la verificación de
/// Argon2 (lenta), para que esta última pueda correr fuera del hilo de eventos.
#[derive(Clone, PartialEq, Eq)]
pub struct CandidatoAutenticacion {
    pub sesion: UsuarioSesion,
    pub password_hash: String,
    /// Ver `Usuario::password_temporal_cacheada` -- `true` significa que,
    /// si `password` verifica contra `password_hash`, el login todavía
    /// debe exigir un cambio de contraseña antes de dejar operar (mismo
    /// significado que `debe_cambiar_password` en el login online). Quien
    /// llama debe leer este campo ANTES de pasar `self` a
    /// `verificar_candidato`, que lo consume.
    pub debe_cambiar_password: bool,
}

impl std::fmt::Debug for CandidatoAutenticacion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CandidatoAutenticacion")
            .field("sesion", &self.sesion)
            .field("password_hash", &"«redactado»")
            .field("debe_cambiar_password", &self.debe_cambiar_password)
            .finish()
    }
}

pub struct AutenticacionService<'a, R>
where
    R: UsuarioRepository + ?Sized,
{
    usuarios: &'a R,
}

impl<'a, R> AutenticacionService<'a, R>
where
    R: UsuarioRepository + ?Sized,
{
    pub fn new(usuarios: &'a R) -> Self {
        Self { usuarios }
    }

    pub fn autenticar(
        &self,
        cedula: &str,
        password: &str,
        ahora: DateTime<Utc>,
    ) -> Result<UsuarioSesion, AutenticacionError> {
        let (sesion, _debe_cambiar_password) = self.autenticar_con_estado(cedula, password, ahora)?;
        Ok(sesion)
    }

    /// Igual que [`Self::autenticar`], pero además devuelve si el hash que
    /// acaba de verificar era una contraseña TEMPORAL todavía cacheada
    /// (`Usuario::password_temporal_cacheada`) -- necesario para que un
    /// login LOCAL (sin pasar por Supabase Auth) siga exigiendo el cambio
    /// de contraseña en vez de devolver siempre `false` como si la
    /// contraseña verificada ya fuera definitiva. Ver
    /// `CandidatoAutenticacion::debe_cambiar_password` y el hallazgo de
    /// auditoría 2026-09-24 (MV-01/DF-03).
    pub fn autenticar_con_estado(
        &self,
        cedula: &str,
        password: &str,
        ahora: DateTime<Utc>,
    ) -> Result<(UsuarioSesion, bool), AutenticacionError> {
        let candidato = self.buscar_candidato(cedula, ahora)?;
        let debe_cambiar_password = candidato.debe_cambiar_password;
        let sesion = verificar_candidato(candidato, password)?;
        Ok((sesion, debe_cambiar_password))
    }

    /// Resuelve la cédula y confirma que el usuario está activo, sin verificar todavía la
    /// contraseña. El llamador decide dónde y cuándo correr `verificar_password` sobre el
    /// hash devuelto (por ejemplo, en un hilo aparte).
    ///
    /// `ahora` viene siempre del reloj corregido del núcleo (`AppCore::reloj`),
    /// nunca leído acá -- este servicio no tiene ni debe tener acceso a un
    /// reloj propio, sigue el mismo criterio que el resto de `services/`
    /// (recibe el tiempo, no lo mide). Se usa únicamente para decidir si un
    /// hash CACHEADO (`Usuario::password_hash_confirmado_en`, ver
    /// `TOPE_CACHE_LOCAL_OFFLINE`) ya venció.
    pub fn buscar_candidato(
        &self,
        cedula: &str,
        ahora: DateTime<Utc>,
    ) -> Result<CandidatoAutenticacion, AutenticacionError> {
        let usuario = self
            .usuarios
            .buscar_por_cedula(cedula.trim())?
            .ok_or(AutenticacionError::CredencialesInvalidas)?;

        if !usuario.activo {
            return Err(AutenticacionError::UsuarioInactivo);
        }

        // Se resuelve ANTES de pedirle la contraseña a quien intenta entrar
        // -- el punto es mandarlo al alta de contraseña apenas escribe la
        // cédula, no después de que también tipeó una contraseña que nunca
        // se iba a poder verificar contra nada.
        if usuario.password_hash == SIN_PASSWORD_LOCAL {
            return Err(AutenticacionError::SinPasswordLocal);
        }

        // Un hash CACHEADO (ver `Usuario::password_hash_confirmado_en`) sólo
        // es válido `TOPE_CACHE_LOCAL_OFFLINE` desde que se confirmó online
        // por última vez -- pasado eso, se trata exactamente igual que
        // `SIN_PASSWORD_LOCAL`: sin camino local usable, quien llama cae al
        // login online de nuevo (ver `login_supabase`/`autenticar_supabase`).
        // Una marca ilegible (dato corrupto, no debería pasar nunca) se
        // trata como vencida -- fallar hacia "pedí red de nuevo" es más
        // seguro que fallar hacia "aceptar sin chequear".
        if let Some(confirmado_en) = usuario.password_hash_confirmado_en.as_deref() {
            let vencido = parsear_utc(confirmado_en)
                .map_or(true, |marca| ahora - marca > TOPE_CACHE_LOCAL_OFFLINE);
            if vencido {
                return Err(AutenticacionError::SinPasswordLocal);
            }
        }

        Ok(CandidatoAutenticacion {
            sesion: UsuarioSesion {
                id: usuario.id,
                cedula: usuario.cedula,
                nombre: usuario.nombre,
                rol: usuario.rol,
            },
            password_hash: usuario.password_hash,
            debe_cambiar_password: usuario.password_temporal_cacheada,
        })
    }

    /// Resuelve identidad/rol/estado local de `cedula` SIN verificar
    /// contraseña -- para cuando esa verificación ya pasó por otro lado
    /// (Supabase Auth, ver `nube::auth_supabase::login` y
    /// docs/planes-implementados/plan-autenticacion-supabase-auth.md). A diferencia de
    /// `buscar_candidato`, acá `SIN_PASSWORD_LOCAL` NO es un error: es
    /// justo el estado esperado de cualquier usuario sincronizado, que ya
    /// no fija contraseña local nunca (ese camino queda reservado sólo
    /// para ROOT del arranque inicial).
    pub fn resolver_identidad_local(
        &self,
        cedula: &str,
    ) -> Result<UsuarioSesion, AutenticacionError> {
        let usuario = self
            .usuarios
            .buscar_por_cedula(cedula.trim())?
            .ok_or(AutenticacionError::CredencialesInvalidas)?;

        if !usuario.activo {
            return Err(AutenticacionError::UsuarioInactivo);
        }

        Ok(UsuarioSesion {
            id: usuario.id,
            cedula: usuario.cedula,
            nombre: usuario.nombre,
            rol: usuario.rol,
        })
    }
}

/// Verifica `password` contra un candidato ya resuelto por `buscar_candidato`.
/// Función libre (no depende de `&self`/repositorio) a propósito: la usa tanto
/// `autenticar` como el hilo aparte que arma la TUI para correr Argon2 sin
/// bloquear la UI — antes ese hilo copiaba a mano el mismo `match` de aquí.
pub fn verificar_candidato(
    candidato: CandidatoAutenticacion,
    password: &str,
) -> Result<UsuarioSesion, AutenticacionError> {
    match verificar_password(password, &candidato.password_hash) {
        Ok(true) => Ok(candidato.sesion),
        Ok(false) => Err(AutenticacionError::CredencialesInvalidas),
        Err(_) => Err(AutenticacionError::HashInvalido),
    }
}
