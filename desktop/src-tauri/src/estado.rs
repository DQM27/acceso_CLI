use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use control_acceso::application::AppCore;
use control_acceso::database::connection::abrir_conexion_secundaria_escritura;
use control_acceso::instancia::InstanciaGuard;
use control_acceso::nube::{CacheTokenDispositivo, NubeError, SesionSupabase, TokenDispositivo};
use control_acceso::services::autenticacion_service::UsuarioSesion;
use rusqlite::Connection;
use zeroize::Zeroizing;

/// Sesión de un usuario global contra Supabase Auth (Administrador/Operador,
/// o un ROOT ya sincronizado a otro sitio) -- ver
/// docs/planes-implementados/plan-autenticacion-supabase-auth.md. Distinta del
/// `TokenDispositivo` que cachea `CacheTokenDispositivo` (identidad del
/// DISPOSITIVO ante el receptor): esto es la identidad de
/// la PERSONA. Vive sólo en memoria -- nunca se persiste a disco, así que
/// cerrar la app siempre la pierde y el próximo arranque exige un login
/// real de nuevo contra Supabase, sin importar cuánto quedara del tope de
/// 12h.
struct SesionSupabaseCacheada {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    /// Última vez que se confirmó de verdad contra Supabase (login inicial
    /// o una renovación exitosa) -- la base del tope duro de 12h. Una
    /// renovación en segundo plano la corre para adelante; el login
    /// inicial no es el único momento que cuenta.
    confirmada_en: Instant,
}

/// Tope duro de presencia (ver el plan): aunque el token técnico siga sin
/// vencer, si pasaron 12h desde la última confirmación real contra
/// Supabase, la sesión se da por vencida. Lo aplica el cliente -- no
/// depende de la configuración de expiración del proyecto de Supabase
/// (que es global y afecta también al panel web).
const TOPE_PRESENCIA_SUPABASE: Duration = Duration::from_secs(12 * 60 * 60);

/// Estado administrado por Tauri. Dos mutexes separados porque ningún flujo
/// necesita actualizar sesión y base de datos como una sola operación atómica
/// (ver docs/plan-tauri.md, sección "Estado y sesión").
///
/// Campos privados a propósito: todo acceso pasa por los métodos de abajo,
/// que recuperan el mutex si quedó envenenado por un panic en otro comando
/// en vez de dejar ese panic tumbar en cadena cualquier comando futuro que
/// intente tomar el mismo lock — con `std::sync::Mutex`, un solo panic
/// aislado no debería dejar la app entera inutilizable hasta reiniciarla.
pub struct GuiState {
    core: Mutex<AppCore>,
    sesion: Mutex<Option<UsuarioSesion>>,
    /// Mantiene el candado de instancia vivo mientras dure la app — nunca se
    /// lee, sólo existe para que no se libere antes de tiempo (mismo patrón
    /// que `main.rs` con `_instancia`).
    _instancia: InstanciaGuard,
    /// Campo HERMANO de `core`, nunca protegido por el mismo candado --
    /// ver el doc-comment de `control_acceso::nube::CacheTokenDispositivo`
    /// sobre por qué (varios comandos autentican sin pasar por
    /// `state.core()` a propósito, para no retener ese candado compartido
    /// durante la llamada de red).
    cache_token: CacheTokenDispositivo,
    sesion_supabase: Mutex<Option<SesionSupabaseCacheada>>,
    /// Ruta del archivo de base de datos, resuelta una sola vez al arrancar
    /// (ver `lib.rs::run`) — el núcleo ya no expone `ruta_base_datos()`
    /// (rama `SQLCipher` sin respaldo local), así que `conexion_secundaria`
    /// la necesita guardada acá.
    ruta_base_datos: PathBuf,
    /// Clave de `SQLCipher` ya resuelta al arrancar (ver
    /// `lib.rs::run`/`clave_cifrado.rs`) — `conexion_secundaria` la necesita
    /// para poder leer el mismo archivo cifrado. `Zeroizing` la borra de
    /// memoria cuando la app cierra.
    clave_base_datos: Zeroizing<[u8; 32]>,
}

impl GuiState {
    pub fn new(
        core: AppCore,
        instancia: InstanciaGuard,
        ruta_base_datos: PathBuf,
        clave_base_datos: Zeroizing<[u8; 32]>,
    ) -> Self {
        Self {
            core: Mutex::new(core),
            sesion: Mutex::new(None),
            _instancia: instancia,
            cache_token: CacheTokenDispositivo::new(),
            sesion_supabase: Mutex::new(None),
            ruta_base_datos,
            clave_base_datos,
        }
    }

    /// Reusa el último `TokenDispositivo` mientras siga vigente en vez de
    /// autenticar de cero -- ver `CacheTokenDispositivo`. Reproducido en
    /// producción: "Registrar" con gafete pagaba una autenticación
    /// completa contra la nube en cada registro
    /// (`gafete_libre_en_otro_dispositivo`), aunque el dispositivo ya se
    /// hubiera autenticado segundos antes para sincronizar o loguearse --
    /// se sentía como que la app se colgaba en cada registro.
    pub fn autenticar_con_cache(&self, secreto: &str) -> Result<TokenDispositivo, NubeError> {
        self.cache_token.autenticar_con_cache(secreto)
    }

    /// Descarta el `TokenDispositivo` cacheado -- ver
    /// `SincronizacionError::token_dispositivo_vencido`: el receptor lo
    /// rechazó a mitad de una sincronización aunque `autenticar_con_cache`
    /// lo creía vigente (desfase de reloj, o el dispositivo estuvo inactivo
    /// más de lo que el margen de 30s contemplaba). La próxima llamada a
    /// `autenticar_con_cache` pide uno nuevo sin esperar a que este
    /// "`vigente_por`" calculado localmente se cumpla solo.
    pub fn invalidar_token_cacheado(&self) {
        self.cache_token.invalidar();
    }

    /// Acceso al núcleo compartido por todos los comandos.
    pub fn core(&self) -> MutexGuard<'_, AppCore> {
        self.core
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Conexión propia al mismo archivo, independiente de la que vive
    /// dentro de `core` — para comandos cuya consulta puede tardar cientos
    /// de milisegundos o más con datos grandes (exportar/cargar Historial y
    /// Auditoría completos, ver `comandos/historial.rs`/`comandos/auditoria.rs`)
    /// y no deben retener el mutex compartido mientras tanto: aunque el
    /// comando ya corre en el pool de hilos bloqueantes de Tauri (no
    /// congela la ventana), retener `core` sí bloquearía a cualquier OTRO
    /// comando que también lo necesite (mismo hallazgo que motivó el hilo
    /// propio en TUI/CLI para exportar, ver `docs/pendientes.md`). El
    /// candado sólo se toma para leer la ruta del archivo, no durante la
    /// consulta. Escribe (sincronización con la nube, `cerrar_ingreso_remoto`)
    /// -- por eso usa la fábrica central de *escritura*
    /// (`abrir_conexion_secundaria_escritura`), no la de sólo lectura que usan
    /// los hilos de exportación de TUI/CLI.
    pub fn conexion_secundaria(&self) -> Result<Connection, String> {
        abrir_conexion_secundaria_escritura(&self.ruta_base_datos, Some(&self.clave_base_datos))
            .map_err(|error| error.to_string())
    }

    /// Sesión actual o el error que ya usan todos los comandos que la
    /// necesitan — un solo lugar para ese chequeo repetido.
    pub fn sesion_activa(&self) -> Result<UsuarioSesion, String> {
        self.lock_sesion()
            .clone()
            .ok_or_else(|| "No hay una sesión activa".to_string())
    }

    pub fn iniciar_sesion(&self, sesion: UsuarioSesion) {
        *self.lock_sesion() = Some(sesion);
    }

    pub fn cerrar_sesion(&self) {
        *self.lock_sesion() = None;
        *self.lock_sesion_supabase() = None;
    }

    fn lock_sesion(&self) -> MutexGuard<'_, Option<UsuarioSesion>> {
        self.sesion
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Guarda (o reemplaza) la sesión de Supabase Auth -- se llama tanto
    /// en el login inicial como en cada renovación exitosa en segundo
    /// plano, siempre con una marca de tiempo nueva (`confirmada_en`).
    pub fn iniciar_sesion_supabase(&self, sesion: SesionSupabase) {
        *self.lock_sesion_supabase() = Some(SesionSupabaseCacheada {
            access_token: sesion.access_token,
            refresh_token: sesion.refresh_token,
            expires_in: sesion.expires_in,
            confirmada_en: Instant::now(),
        });
    }

    /// Token de acceso vigente para usar como `Authorization: Bearer`, si
    /// lo hay -- `None` si nunca hubo sesión, si el token técnico ya
    /// venció, o si pasó `TOPE_PRESENCIA_SUPABASE` desde la última
    /// confirmación real (aunque el token en sí siga sin vencer).
    pub fn access_token_supabase_vigente(&self) -> Option<String> {
        let guard = self.lock_sesion_supabase();
        let entrada = guard.as_ref()?;
        let vigente_por = Duration::from_secs(entrada.expires_in);
        let vencido = entrada.confirmada_en.elapsed() >= vigente_por
            || entrada.confirmada_en.elapsed() >= TOPE_PRESENCIA_SUPABASE;
        let token = entrada.access_token.clone();
        drop(guard);
        if vencido { None } else { Some(token) }
    }

    /// `refresh_token` actual, para la renovación en segundo plano -- `None`
    /// si nunca hubo sesión de Supabase (usuario logueado localmente, p.
    /// ej. ROOT del arranque inicial) o si ya se cerró sesión.
    pub fn refresh_token_supabase(&self) -> Option<String> {
        self.lock_sesion_supabase()
            .as_ref()
            .map(|entrada| entrada.refresh_token.clone())
    }

    fn lock_sesion_supabase(&self) -> MutexGuard<'_, Option<SesionSupabaseCacheada>> {
        self.sesion_supabase
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
