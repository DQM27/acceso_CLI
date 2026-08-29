use control_acceso::services::autenticacion_service::{UsuarioSesion, verificar_candidato};

use crate::estado::GuiState;

#[tauri::command]
pub fn requiere_configuracion_inicial(state: tauri::State<GuiState>) -> Result<bool, String> {
    state
        .core()
        .requiere_configuracion_inicial()
        .map_err(|error| error.to_string())
}

/// Login en dos pasos, igual que la TUI (ver `AutenticacionService`), pero sin
/// el canal `mpsc` que ella necesita: Tauri ya despacha este comando en su
/// propio pool de hilos, así que el cálculo de Argon2 no congela la ventana.
/// Lo que sí se preserva es soltar el lock de `core` ANTES de verificar la
/// contraseña, para no bloquear otros comandos durante el hash — el guard de
/// `state.core()` es un temporal y se libera al terminar esta sentencia.
#[tauri::command]
pub fn login(
    cedula: String,
    password: String,
    state: tauri::State<GuiState>,
) -> Result<UsuarioSesion, String> {
    let candidato = state
        .core()
        .buscar_candidato_autenticacion(&cedula)
        .map_err(control_acceso::mensajes::mensaje_autenticacion)?;
    let sesion = verificar_candidato(candidato, &password)
        .map_err(control_acceso::mensajes::mensaje_autenticacion)?;
    state.iniciar_sesion(sesion.clone());
    Ok(sesion)
}

#[tauri::command]
pub fn cerrar_sesion(state: tauri::State<GuiState>) {
    state.cerrar_sesion();
}
