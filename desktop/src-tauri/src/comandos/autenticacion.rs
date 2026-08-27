use control_acceso::services::autenticacion_service::{UsuarioSesion, verificar_candidato};

use crate::estado::GuiState;

#[tauri::command]
pub fn requiere_configuracion_inicial(state: tauri::State<GuiState>) -> Result<bool, String> {
    state
        .core
        .lock()
        .unwrap()
        .requiere_configuracion_inicial()
        .map_err(|error| error.to_string())
}

/// Login en dos pasos, igual que la TUI (ver `AutenticacionService`), pero sin
/// el canal `mpsc` que ella necesita: Tauri ya despacha este comando en su
/// propio pool de hilos, así que el cálculo de Argon2 no congela la ventana.
/// Lo que sí se preserva es soltar el lock de `core` ANTES de verificar la
/// contraseña, para no bloquear otros comandos durante el hash.
#[tauri::command]
pub fn login(
    cedula: String,
    password: String,
    state: tauri::State<GuiState>,
) -> Result<UsuarioSesion, String> {
    let candidato = {
        let core = state.core.lock().unwrap();
        core.buscar_candidato_autenticacion(&cedula)
            .map_err(|error| error.to_string())?
    };
    let sesion = verificar_candidato(candidato, &password).map_err(|error| error.to_string())?;
    *state.sesion.lock().unwrap() = Some(sesion.clone());
    Ok(sesion)
}

#[tauri::command]
pub fn cerrar_sesion(state: tauri::State<GuiState>) {
    *state.sesion.lock().unwrap() = None;
}
