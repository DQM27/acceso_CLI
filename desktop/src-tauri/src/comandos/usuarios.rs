use crate::estado::GuiState;

/// `/clave` — cambiar la contraseña de la propia sesión. Crear/editar/
/// buscar usuarios globales u otorgarle/resetearle la contraseña a OTRO
/// usuario desde el escritorio ya no existe -- esa capacidad quedó
/// exclusiva del panel administrativo web (`admin-create-usuario`/
/// `admin-reset-password-usuario`, ver
/// docs/plan-autenticacion-supabase-auth.md). `AppCore::cambiar_mi_password`
/// ya verifica `password_actual` (Argon2) y valida la nueva en un solo
/// paso.
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
