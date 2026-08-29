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
        .core()
        .buscar_usuarios(&sesion, &filtro.construir())
        .map_err(control_acceso::mensajes::mensaje_usuario)
}

/// Contraseña en texto plano — a diferencia de la TUI, no hace falta el hilo
/// aparte para Argon2 (Tauri ya despacha este comando en su propio pool).
#[tauri::command]
pub fn crear_usuario(datos: CrearUsuarioEntrada, state: tauri::State<GuiState>) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
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
        .core()
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
        .core()
        .cambiar_password_usuario(&sesion, id, &password)
        .map_err(control_acceso::mensajes::mensaje_usuario)
}

/// `/clave` en la consola — cambiar la contraseña de la propia sesión.
/// `AppCore::cambiar_mi_password` ya verifica `password_actual` (Argon2) y
/// valida la nueva en un solo paso, así que no hace falta el
/// `verificar_mi_password` aparte que sí usa la TUI para no pedir la
/// contraseña nueva dos veces por si la actual estaba mal — acá alcanza con
/// un solo formulario con ambos campos.
#[tauri::command]
pub fn cambiar_mi_password(
    password_actual: String,
    nueva_password: String,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .cambiar_mi_password(&sesion, &password_actual, &nueva_password)
        .map_err(control_acceso::mensajes::mensaje_usuario)
}
