use std::sync::Mutex;

use control_acceso::application::AppCore;
use control_acceso::instancia::InstanciaGuard;
use control_acceso::services::autenticacion_service::UsuarioSesion;

/// Estado administrado por Tauri. Dos mutexes separados porque ningún flujo
/// necesita actualizar sesión y base de datos como una sola operación atómica
/// (ver docs/plan-tauri.md, sección "Estado y sesión").
pub struct GuiState {
    pub core: Mutex<AppCore>,
    pub sesion: Mutex<Option<UsuarioSesion>>,
    /// Mantiene el candado de instancia vivo mientras dure la app — nunca se
    /// lee, sólo existe para que no se libere antes de tiempo (mismo patrón
    /// que `main.rs` con `_instancia`).
    pub _instancia: InstanciaGuard,
}
