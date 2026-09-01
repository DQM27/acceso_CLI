use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use control_acceso::application::{AppCore, BootstrapError};
use control_acceso::database::backup::{RespaldoError, TipoRespaldo};
use control_acceso::instancia::InstanciaGuard;
use control_acceso::services::autenticacion_service::UsuarioSesion;
use rusqlite::Connection;

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
}

impl GuiState {
    pub fn new(core: AppCore, instancia: InstanciaGuard) -> Self {
        Self {
            core: Mutex::new(core),
            sesion: Mutex::new(None),
            _instancia: instancia,
        }
    }

    /// Acceso al núcleo compartido por todos los comandos.
    pub fn core(&self) -> MutexGuard<'_, AppCore> {
        self.core
            .lock()
            .unwrap_or_else(|envenenado| envenenado.into_inner())
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
            .unwrap_or_else(|envenenado| envenenado.into_inner());
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
            .unwrap_or_else(|envenenado| envenenado.into_inner())
    }
}
