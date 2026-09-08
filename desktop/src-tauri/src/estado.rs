use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use control_acceso::application::{AppCore, BootstrapError};
use control_acceso::database::backup::{RespaldoError, TipoRespaldo};
use control_acceso::instancia::InstanciaGuard;
use control_acceso::nube::{self, NubeError, TokenDispositivo};
use control_acceso::services::autenticacion_service::UsuarioSesion;
use rusqlite::Connection;

/// Ver `GuiState::autenticar_con_cache` -- último `TokenDispositivo`
/// obtenido mientras siga vigente, para no autenticar de cero en cada
/// comando que necesita hablar con la nube (login, registrar un ingreso
/// con gafete, etc.). Duplica la idea de `application::nube::TokenCacheado`
/// (interno a `AppCore`, usado por móvil) en vez de reutilizarla porque acá
/// varios comandos autentican SIN pasar por `AppCore`/`state.core()` a
/// propósito -- retener el candado compartido durante la llamada de red es
/// justo lo que esos comandos evitan (ver doc-comment de `GuiState::core`).
struct TokenCacheado {
    secreto: String,
    token: TokenDispositivo,
    obtenido_en: Instant,
}

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
    token_nube_cacheado: Mutex<Option<TokenCacheado>>,
}

impl GuiState {
    pub fn new(core: AppCore, instancia: InstanciaGuard) -> Self {
        Self {
            core: Mutex::new(core),
            sesion: Mutex::new(None),
            _instancia: instancia,
            token_nube_cacheado: Mutex::new(None),
        }
    }

    /// Reusa el último `TokenDispositivo` mientras siga vigente en vez de
    /// autenticar de cero -- ver `TokenCacheado`. Reproducido en producción:
    /// "Registrar" con gafete pagaba una autenticación completa contra la
    /// nube en cada registro (`gafete_libre_en_otro_dispositivo`), aunque
    /// el dispositivo ya se hubiera autenticado segundos antes para
    /// sincronizar o loguearse -- se sentía como que la app se colgaba en
    /// cada registro. Margen de 30s antes del vencimiento real para no
    /// arrancar una operación con un token que puede vencer a mitad de
    /// camino. Un acierto de caché no vuelve a medir el desfase de reloj
    /// (`desfase_reloj_ms` queda en `None`) -- no hace falta remedirlo en
    /// cada llamada, sólo cuando de verdad se habla con el receptor.
    pub fn autenticar_con_cache(&self, secreto: &str) -> Result<TokenDispositivo, NubeError> {
        const MARGEN_EXPIRACION: Duration = Duration::from_secs(30);

        {
            let cache = self
                .token_nube_cacheado
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entrada) = cache.as_ref() {
                let vigente_por =
                    Duration::from_secs(entrada.token.expires_in).saturating_sub(MARGEN_EXPIRACION);
                if entrada.secreto == secreto && entrada.obtenido_en.elapsed() < vigente_por {
                    let mut token = entrada.token.clone();
                    token.desfase_reloj_ms = None;
                    return Ok(token);
                }
            }
        }

        let token = nube::autenticar_dispositivo(nube::BASE_URL, secreto, None)?;
        *self
            .token_nube_cacheado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(TokenCacheado {
            secreto: secreto.to_string(),
            token: token.clone(),
            obtenido_en: Instant::now(),
        });
        Ok(token)
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
    /// consulta.
    pub fn conexion_secundaria(&self) -> Result<Connection, String> {
        let ruta_base_datos = self.core().ruta_base_datos().to_path_buf();
        let conexion = Connection::open(&ruta_base_datos).map_err(|error| error.to_string())?;
        conexion
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| error.to_string())?;
        Ok(conexion)
    }

    /// Restaura `ruta_candidata` como base activa (ver
    /// `database::backup::restaurar_respaldo` y, del lado de la TUI,
    /// `tui/app/actions/admin.rs` — mismo flujo, sin poder reiniciar el
    /// proceso como hace `main.rs`, porque acá `AppCore` vive dentro de un
    /// `Mutex` administrado por Tauri para toda la vida de la app).
    ///
    /// Orden, con el mismo candado tomado de principio a fin para que
    /// ningún otro comando pueda tomar una conexión a mitad de este
    /// intercambio: 1) crea un respaldo `PreRestauracion` de la base activa
    /// usando la conexión todavía viva (autoriza igual que cualquier otra
    /// operación de respaldos); 2) cierra esa conexión — obligatorio antes
    /// de reemplazar el archivo, la función del núcleo lo exige
    /// explícitamente — reemplazándola por un `AppCore` en memoria
    /// (`Connection::open_in_memory`) que sólo existe mientras dura el
    /// intercambio de archivos; 3) reemplaza el archivo; 4) abre un
    /// `AppCore` nuevo sobre el archivo ya restaurado. Al terminar (éxito o
    /// fallo) cierra la sesión — la base activa cambió de identidad, igual
    /// que la TUI fuerza un login nuevo tras restaurar.
    pub fn restaurar_respaldo(
        &self,
        actor: &UsuarioSesion,
        ruta_candidata: &Path,
    ) -> Result<(), RespaldoError> {
        let mut guard = self
            .core
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.crear_respaldo(actor, TipoRespaldo::PreRestauracion)?;
        let ruta_activa = guard.ruta_base_datos().to_path_buf();

        let anterior = std::mem::replace(&mut *guard, AppCore::new(Connection::open_in_memory()?));
        let ruta_cerrada = anterior.cerrar();
        debug_assert_eq!(ruta_cerrada, ruta_activa);

        let resultado =
            control_acceso::database::backup::restaurar_respaldo(ruta_candidata, &ruta_activa);
        match AppCore::abrir(&ruta_activa) {
            Ok(core) => *guard = core,
            Err(BootstrapError::Database(error)) => {
                drop(guard);
                self.cerrar_sesion();
                // Si `restaurar_respaldo` ya había fallado, ese error explica
                // mejor qué pasó (`RollbackFallido` trae guía de recuperación)
                // que uno genérico de "no se pudo reabrir" — se prioriza ese
                // en vez de pisarlo con el de esta apertura.
                return Err(resultado.err().unwrap_or_else(|| error.into()));
            }
        }
        drop(guard);
        self.cerrar_sesion();
        resultado
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
    }

    fn lock_sesion(&self) -> MutexGuard<'_, Option<UsuarioSesion>> {
        self.sesion
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
