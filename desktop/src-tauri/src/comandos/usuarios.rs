use control_acceso::database::queries::usuarios::UsuarioResumen;

use crate::dto::usuarios::{ActualizarUsuarioEntrada, CrearUsuarioEntrada, FiltroUsuariosEntrada};
use crate::estado::GuiState;

#[tauri::command]
pub fn buscar_usuarios(
    filtro: FiltroUsuariosEntrada,
    state: tauri::State<GuiState>,
) -> Result<Vec<UsuarioResumen>, String> {
    let sesion = state.sesion_activa()?;
    state
        .core
        .lock()
        .unwrap()
        .buscar_usuarios(&sesion, &filtro.construir())
        .map_err(control_acceso::mensajes::mensaje_usuario)
}

/// Contraseña en texto plano — a diferencia de la TUI, no hace falta el hilo
/// aparte para Argon2 (Tauri ya despacha este comando en su propio pool).
#[tauri::command]
pub fn crear_usuario(datos: CrearUsuarioEntrada, state: tauri::State<GuiState>) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core
        .lock()
        .unwrap()
        .crear_usuario(&sesion, datos.into())
        .map_err(control_acceso::mensajes::mensaje_usuario)
}

/// No toca la contraseña — eso es `cambiar_password_usuario`, aparte.
#[tauri::command]
pub fn actualizar_usuario(
    id: i64,
    datos: ActualizarUsuarioEntrada,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    let activo = datos.activo;
    state
        .core
        .lock()
        .unwrap()
        .actualizar_usuario(&sesion, id, datos.input(), activo)
        .map_err(control_acceso::mensajes::mensaje_usuario)
}

#[tauri::command]
pub fn cambiar_password_usuario(
    id: i64,
    password: String,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core
        .lock()
        .unwrap()
        .cambiar_password_usuario(&sesion, id, &password)
        .map_err(control_acceso::mensajes::mensaje_usuario)
}
