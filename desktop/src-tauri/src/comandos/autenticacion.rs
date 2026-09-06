use control_acceso::mensajes::{mensaje_nube, mensaje_sincronizacion};
use control_acceso::nube;
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

fn intentar_login_local(
    state: &GuiState,
    cedula: &str,
    password: &str,
) -> Result<UsuarioSesion, AutenticacionError> {
    let candidato = state.core().buscar_candidato_autenticacion(cedula)?;
    verificar_candidato(candidato, password)
}

/// Trae sólo el catálogo (usuarios/contratistas/empresas/gafetes), sin
/// sesión ni autorización de por medio -- a diferencia de
/// `ejecutar_sincronizacion`/`nube::autenticar`, que exigen una sesión ya
/// abierta (`autorizar_uso_nube`). Hace falta un camino sin esa exigencia
/// para el caso "reactivaron a este usuario en otro dispositivo, y este
/// todavía lo tiene marcado inactivo local" (ver `login`): en ese momento
/// todavía no hay ninguna sesión válida que autorice nada, es justo lo que
/// se está tratando de determinar. La identidad de la nube es del
/// dispositivo (el secreto), no del usuario que intenta entrar, así que no
/// hace falta una.
fn refrescar_catalogo_sin_sesion(state: &GuiState) -> Result<(), String> {
    let secreto = nube::credenciales::cargar_secreto()
        .ok_or_else(|| "Todavía no se guardó el secreto de este dispositivo".to_string())?;
    let token = state.autenticar_con_cache(&secreto).map_err(mensaje_nube)?;
    if let Some(desfase_ms) = token.desfase_reloj_ms {
        state.core().actualizar_desfase_reloj(desfase_ms);
    }
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::BASE_URL,
        apikey: nube::APIKEY,
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    let conexion = state.conexion_secundaria()?;
    nube::recibir_catalogo_del_sitio(&conexion, &contexto).map_err(mensaje_sincronizacion)?;
    Ok(())
}

/// Confirma en vivo si `cedula` sigue activa, sin sincronizar nada más --
/// una fila, una columna. Reemplaza a la sincronización completa que este
/// chequeo hacía antes: medida como la causa real del retraso perceptible
/// al loguearse (varios cientos de milisegundos a un par de segundos según
/// el tamaño del sitio), cuando lo único que hace falta acá es esto. La
/// sincronización completa (cola, catálogo, historial...) sigue
/// corriendo, pero en segundo plano -- ver `login`.
fn usuario_sigue_activo_remoto(state: &GuiState, cedula: &str) -> Result<bool, String> {
    let secreto = nube::credenciales::cargar_secreto()
        .ok_or_else(|| "Todavía no se guardó el secreto de este dispositivo".to_string())?;
    let token = state.autenticar_con_cache(&secreto).map_err(mensaje_nube)?;
    if let Some(desfase_ms) = token.desfase_reloj_ms {
        state.core().actualizar_desfase_reloj(desfase_ms);
    }
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::BASE_URL,
        apikey: nube::APIKEY,
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    nube::usuario_sigue_activo_remoto(&contexto, cedula).map_err(mensaje_sincronizacion)
}

/// Login en dos pasos, igual que la TUI (ver `AutenticacionService`), pero sin
/// el canal `mpsc` que ella necesita: Tauri ya despacha este comando en su
/// propio pool de hilos, así que el cálculo de Argon2 no congela la ventana.
/// Lo que sí se preserva es soltar el lock de `core` ANTES de verificar la
/// contraseña, para no bloquear otros comandos durante el hash — el guard de
/// `state.core()` es un temporal y se libera al terminar esta sentencia.
///
/// Dos chequeos contra la nube, uno para cada dirección de un cambio de
/// estado remoto -- decisión explícita: "por seguridad, pero nunca
/// bloqueante" (la garita tiene que poder operar sin internet), así que
/// los dos son best-effort con `ESPERA_MAXIMA_SYNC_LOGIN` de tope:
///
/// 1. **Reactivación**: si el chequeo local dice "inactivo"
///    (`AutenticacionError::UsuarioInactivo`), puede ser que a este
///    usuario lo hayan reactivado en otro dispositivo y esta base todavía
///    no se enteró -- antes de rendirse, refresca sólo el catálogo
///    (`refrescar_catalogo_sin_sesion`, sin sesión) y reintenta el login
///    local una vez más. Sin esto, una reactivación remota nunca se podía
///    reflejar acá: el login fallaba en el chequeo local ANTES de llegar a
///    sincronizar nada.
/// 2. **Baja**: tras un login local exitoso, confirma en vivo que la
///    cédula sigue activa (`usuario_sigue_activo_remoto` -- una fila, una
///    columna, no una sincronización completa: eso era lo que hacía sentir
///    el login lento). La sincronización completa (cola, catálogo,
///    historial...) igual se dispara, pero **en segundo plano**, sin que
///    el login espere por ella -- se sigue beneficiando el resto de la
///    sesión sin agregarle ni un milisegundo a la espera de entrar.
///
/// En ambos casos, sin red o si tarda más del tope, sigue con lo que ya
/// haya en local -- si de verdad cambió algo, la sincronización de fondo
/// (o la próxima periódica/Realtime/login) lo termina reflejando.
#[tauri::command]
pub async fn login(
    cedula: String,
    password: String,
    app: tauri::AppHandle,
) -> Result<UsuarioSesion, ErrorLogin> {
    let state = app.state::<GuiState>();

    let sesion = match intentar_login_local(&state, &cedula, &password) {
        Ok(sesion) => sesion,
        Err(AutenticacionError::UsuarioInactivo) => {
            let manejador = app.clone();
            let _ = tokio::time::timeout(
                ESPERA_MAXIMA_SYNC_LOGIN,
                tauri::async_runtime::spawn_blocking(move || {
                    refrescar_catalogo_sin_sesion(&manejador.state::<GuiState>())
                }),
            )
            .await;
            intentar_login_local(&state, &cedula, &password)?
        }
        Err(otro) => return Err(otro.into()),
    };

    let cedula_chequeo = sesion.cedula.clone();
    let manejador = app.clone();
    let chequeo = tokio::time::timeout(
        ESPERA_MAXIMA_SYNC_LOGIN,
        tauri::async_runtime::spawn_blocking(move || {
            usuario_sigue_activo_remoto(&manejador.state::<GuiState>(), &cedula_chequeo)
        }),
    )
    .await;
    // Sólo el `Ok(Ok(Ok(false)))` explícito (respondió a tiempo, sin error,
    // y dijo que no) rechaza el login -- cualquier otra combinación (sin
    // red, tardó, o dijo que sí) sigue adelante con lo que ya validó local.
    if let Ok(Ok(Ok(false))) = chequeo {
        return Err(ErrorLogin {
            mensaje: "Este usuario fue desactivado".to_string(),
            sin_password_local: false,
        });
    }

    state.iniciar_sesion(sesion.clone());

    // Sincronización completa en segundo plano, sin bloquear la respuesta
    // de este comando -- `ejecutar_sincronizacion` ya cierra la sesión
    // sola si de todos modos encuentra una baja (`ResumenSincronizacion::sesion_expulsada`),
    // así que no perder esta corrida no debilita la protección, sólo la
    // vuelve un poco menos inmediata en el caso raro de que el chequeo
    // rápido de arriba haya dicho que sí pero algo más haya cambiado justo
    // en el medio.
    let manejador = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = tauri::async_runtime::spawn_blocking(move || {
            crate::comandos::nube::ejecutar_sincronizacion(&manejador.state::<GuiState>())
        })
        .await;
    });

    Ok(sesion)
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
