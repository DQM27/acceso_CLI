use control_acceso::services::autenticacion_service::{UsuarioSesion, verificar_candidato};
use control_acceso::services::error::AutenticacionError;
use tauri::Manager;

use crate::estado::GuiState;

/// Cuánto esperar, como máximo, a que la sincronización previa al login
/// termine antes de seguir con lo que ya haya en local -- ver el
/// doc-comment de `login`. 5s alcanza de sobra para el puñado de usuarios
/// típico de un sitio; una red lenta o caída no debe dejar a la garita sin
/// poder operar.
const ESPERA_MAXIMA_SYNC_LOGIN: std::time::Duration = std::time::Duration::from_secs(5);

#[tauri::command]
pub fn requiere_configuracion_inicial(state: tauri::State<GuiState>) -> Result<bool, String> {
    state
        .core()
        .requiere_configuracion_inicial()
        .map_err(|error| error.to_string())
}

/// Distinto de un `String` plano a propósito: la pantalla de login necesita
/// diferenciar "cédula/contraseña incorrecta" (error de verdad) de "este
/// usuario global todavía no fijó contraseña en este dispositivo" (no es un
/// error del usuario, hay que mostrarle el formulario para fijarla) sin
/// depender de comparar el texto exacto del mensaje.
#[derive(serde::Serialize)]
pub struct ErrorLogin {
    pub mensaje: String,
    pub sin_password_local: bool,
}

impl From<AutenticacionError> for ErrorLogin {
    fn from(error: AutenticacionError) -> Self {
        let sin_password_local = matches!(error, AutenticacionError::SinPasswordLocal);
        Self {
            mensaje: control_acceso::mensajes::mensaje_autenticacion(error),
            sin_password_local,
        }
    }
}

/// Login en dos pasos, igual que la TUI (ver `AutenticacionService`), pero sin
/// el canal `mpsc` que ella necesita: Tauri ya despacha este comando en su
/// propio pool de hilos, así que el cálculo de Argon2 no congela la ventana.
/// Lo que sí se preserva es soltar el lock de `core` ANTES de verificar la
/// contraseña, para no bloquear otros comandos durante el hash — el guard de
/// `state.core()` es un temporal y se libera al terminar esta sentencia.
///
/// Después de validar localmente, intenta una sincronización rápida (con
/// tope de tiempo) para que una baja/desactivación reciente en otro
/// dispositivo se refleje antes de dejarlo entrar -- decisión explícita:
/// "por seguridad, pero nunca bloqueante" (la garita tiene que poder operar
/// sin internet). Sin red o si tarda más de `ESPERA_MAXIMA_SYNC_LOGIN`,
/// sigue con lo que ya validó local -- la sincronización sigue corriendo
/// en segundo plano igual y, si de verdad estaba desactivado, la próxima
/// vez que traiga señal `ejecutar_sincronizacion` lo expulsa solo (ver
/// `ResumenSincronizacion::sesion_expulsada`).
#[tauri::command]
pub async fn login(
    cedula: String,
    password: String,
    app: tauri::AppHandle,
) -> Result<UsuarioSesion, ErrorLogin> {
    let state = app.state::<GuiState>();
    let candidato = state.core().buscar_candidato_autenticacion(&cedula)?;
    let sesion = verificar_candidato(candidato, &password)?;
    state.iniciar_sesion(sesion.clone());

    let manejador = app.clone();
    let _ = tokio::time::timeout(
        ESPERA_MAXIMA_SYNC_LOGIN,
        tauri::async_runtime::spawn_blocking(move || {
            crate::comandos::nube::ejecutar_sincronizacion(&manejador.state::<GuiState>())
        }),
    )
    .await;

    // `ejecutar_sincronizacion` ya se encarga de cerrar la sesión sola si
    // la sincronización (si llegó a completarse a tiempo) trajo la baja de
    // este usuario -- acá sólo hace falta confirmar si sigue existiendo.
    state.sesion_activa().map_err(|_| ErrorLogin {
        mensaje: "Este usuario fue desactivado".to_string(),
        sin_password_local: false,
    })
}

/// Completa el alta de contraseña de un usuario global que la pantalla de
/// login detectó vía `ErrorLogin::sin_password_local` -- ver
/// `AppCore::fijar_password_inicial`. Deja la sesión iniciada directo,
/// como si hubiera sido un login exitoso (que en los hechos, lo es).
#[tauri::command]
pub fn fijar_password_inicial(
    cedula: String,
    nueva_password: String,
    state: tauri::State<GuiState>,
) -> Result<UsuarioSesion, String> {
    let sesion = state
        .core()
        .fijar_password_inicial(&cedula, &nueva_password)
        .map_err(control_acceso::mensajes::mensaje_usuario)?;
    state.iniciar_sesion(sesion.clone());
    Ok(sesion)
}

#[tauri::command]
pub fn cerrar_sesion(state: tauri::State<GuiState>) {
    state.cerrar_sesion();
}
