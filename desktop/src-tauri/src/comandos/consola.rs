use control_acceso::application::Autocompletado;
use control_acceso::lenguaje_comandos::ContextState;

use crate::estado::GuiState;

/// Puente hacia el mismo lenguaje de comandos de `--comandos` (DEC de
/// `ejecutar_comando` en `src/application/comandos.rs`) — piloto de la
/// consola tipo terminal de la GUI. `resolver` nunca falla con `Err`: los
/// casos inválidos ya vuelven como `ContextState::MensajeError`, así que
/// este comando no necesita mapear errores de dominio, sólo la falta de
/// sesión.
#[tauri::command]
pub fn ejecutar_comando(texto: String, state: tauri::State<GuiState>) -> Result<ContextState, String> {
    let sesion = state.sesion_activa()?;
    Ok(state.core().ejecutar_comando(&sesion, &texto))
}

/// Sugerencias en vivo + autocompletado (Tab) mientras se teclea, sin
/// confirmar todavía — mismo par que ya usa `--comandos` en cada tecla.
#[tauri::command]
pub fn autocompletar_comando(
    texto: String,
    state: tauri::State<GuiState>,
) -> Result<Autocompletado, String> {
    state.sesion_activa()?;
    Ok(state.core().autocompletar_comando(&texto))
}
