use control_acceso::mensajes::{mensaje_nube, mensaje_sincronizacion};
use control_acceso::nube;
use control_acceso::services::autenticacion_service::{UsuarioSesion, verificar_candidato};
use control_acceso::services::error::AutenticacionError;
use tauri::Manager;

use crate::estado::GuiState;

/// Cuánto esperar, como máximo, a que la sincronización previa al login
/// termine antes de seguir con lo que ya haya en local -- ver el
/// doc-comment de `login`. 5s alcanza de sobra para el puñado de usuarios
/// típico de un sitio; una red lenta o caída no debe dejar al punto de acceso sin
/// poder operar.
const ESPERA_MAXIMA_SYNC_LOGIN: std::time::Duration = std::time::Duration::from_secs(5);

#[tauri::command]
pub fn requiere_configuracion_inicial(state: tauri::State<GuiState>) -> Result<bool, String> {
    state
        .core()
        .requiere_configuracion_inicial()
        .map_err(super::mensaje_generico)
}

/// Ya no distingue `sin_password_local` -- `login` resuelve las dos ramas
/// (local y Supabase Auth) del todo lado del backend, la pantalla de login
/// ya no necesita saber cuál de las dos corrió. Ver
/// docs/planes-implementados/plan-autenticacion-supabase-auth.md.
#[derive(serde::Serialize)]
pub struct ErrorLogin {
    pub mensaje: String,
}

impl From<AutenticacionError> for ErrorLogin {
    fn from(error: AutenticacionError) -> Self {
        Self {
            mensaje: control_acceso::mensajes::mensaje_autenticacion(error),
        }
    }
}

impl From<nube::AuthSupabaseError> for ErrorLogin {
    fn from(error: nube::AuthSupabaseError) -> Self {
        Self {
            mensaje: error.to_string(),
        }
    }
}

/// Éxito de `login` -- separado de `UsuarioSesion` (compartido con
/// TUI/mobile) a propósito, sólo desktop necesita decirle a la pantalla
/// que fuerce el cambio de contraseña antes de dejar operar. `false` en la
/// rama local para el ROOT del arranque inicial o cualquier cuenta que ya
/// tenía password local de antes de la migración a Supabase Auth -- esa
/// contraseña ya es la real, no una temporal. Puede ser `true` también en
/// la rama local (`CandidatoAutenticacion::debe_cambiar_password`, ver
/// `intentar_login_local`): un usuario global cuya contraseña TEMPORAL
/// quedó cacheada para operar sin conexión, y que todavía no la cambió,
/// sigue debiendo el cambio aunque el login haya sido local (hallazgo de
/// auditoría 2026-09-24, MV-01/DF-03).
#[derive(serde::Serialize)]
pub struct ResultadoLogin {
    pub sesion: UsuarioSesion,
    pub debe_cambiar_password: bool,
}

/// Devuelve, junto con la sesión, si la contraseña recién verificada era
/// una TEMPORAL todavía cacheada (`CandidatoAutenticacion::debe_cambiar_password`)
/// -- antes de este campo, `login` siempre devolvía `debe_cambiar_password:
/// false` para esta rama, lo que permitía esquivar el cambio obligatorio
/// quedándose sin conexión (hallazgo de auditoría 2026-09-24, MV-01/DF-03).
fn intentar_login_local(
    state: &GuiState,
    cedula: &str,
    password: &str,
) -> Result<(UsuarioSesion, bool), AutenticacionError> {
    let candidato = state.core().buscar_candidato_autenticacion(cedula)?;
    let debe_cambiar_password = candidato.debe_cambiar_password;
    let sesion = verificar_candidato(candidato, password)?;
    Ok((sesion, debe_cambiar_password))
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
///
/// `pub` (el módulo `comandos` no es público fuera del crate, así que esto
/// no amplía nada) -- también la llama
/// `lib.rs::precargar_catalogo_durante_splash` para adelantar esta misma
/// descarga a los 3 segundos del splash (ver ese doc-comment), así el
/// reintento de acá de más abajo casi nunca tiene que esperar los
/// `ESPERA_MAXIMA_SYNC_LOGIN` completos.
pub fn refrescar_catalogo_sin_sesion(state: &GuiState) -> Result<(), String> {
    let secreto = nube::credenciales::cargar_secreto()
        .ok_or_else(|| "Todavía no se guardó el secreto de este dispositivo".to_string())?;
    let token = state.autenticar_con_cache(&secreto).map_err(mensaje_nube)?;
    if let Some(desfase_ms) = token.desfase_reloj_ms {
        state.core().actualizar_desfase_reloj(desfase_ms);
    }
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::base_url(),
        apikey: nube::apikey(),
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
        base_url: nube::base_url(),
        apikey: nube::apikey(),
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
/// bloqueante" (el punto de acceso tiene que poder operar sin internet), así que
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
) -> Result<ResultadoLogin, ErrorLogin> {
    let state = app.state::<GuiState>();

    let (sesion, debe_cambiar_password) = match intentar_login_local(&state, &cedula, &password) {
        Ok(resultado) => resultado,
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
        // El centinela `SIN_PASSWORD_LOCAL` ya no significa "mostrar el
        // formulario de alta" -- significa "este usuario global se
        // autentica contra Supabase Auth, no localmente" (ver
        // docs/planes-implementados/plan-autenticacion-supabase-auth.md). El único camino que
        // sigue siendo 100% local es el ROOT del arranque inicial
        // (`crear_root_inicial`), que nunca cae acá porque nace con un
        // hash real desde el principio.
        Err(AutenticacionError::SinPasswordLocal) => {
            return login_supabase(&app, &cedula, &password).await;
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
    if matches!(chequeo, Ok(Ok(Ok(false)))) {
        return Err(ErrorLogin {
            mensaje: "Este usuario fue desactivado".to_string(),
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

    Ok(ResultadoLogin {
        sesion,
        debe_cambiar_password,
    })
}

/// Login contra Supabase Auth (`nube::auth_supabase::login`) -- ver el
/// comentario de `login` de más arriba y
/// docs/planes-implementados/plan-autenticacion-supabase-auth.md. A diferencia del camino local,
/// necesita red sí o sí: sin ella, no hay forma de verificar la
/// contraseña de un usuario que nunca la fijó en este dispositivo, así
/// que el registro también queda bloqueado hasta que haya conexión (no es
/// el caso "mejor esfuerzo" de otros chequeos remotos de esta app).
async fn login_supabase(
    app: &tauri::AppHandle,
    cedula: &str,
    password: &str,
) -> Result<ResultadoLogin, ErrorLogin> {
    let state = app.state::<GuiState>();
    let cedula = cedula.to_string();
    let password = password.to_string();
    let cedula_supabase = cedula.clone();
    // Se usa más abajo para cachear el login offline (`cachear_password_local`)
    // -- `password` se mueve al `spawn_blocking` de acá abajo, así que hace
    // falta esta copia ANTES de ese `move`.
    let password_para_cache = password.clone();

    let sesion_supabase = tauri::async_runtime::spawn_blocking(move || {
        nube::login(
            nube::base_url(),
            nube::apikey(),
            &cedula_supabase,
            &password,
        )
    })
    .await
    .map_err(|_| ErrorLogin {
        mensaje: "No se pudo completar el login".to_string(),
    })??;

    // La identidad (nombre/rol/activo) ya está local -- llegó por el
    // catálogo sincronizado (`recibir_catalogo_del_sitio`), Supabase Auth
    // sólo confirmó que la contraseña era correcta. Si por algún motivo
    // esta cédula todavía no está en el catálogo local (sitio recién
    // conectado, o esta persona se dio de alta hace apenas un instante),
    // se intenta refrescar el catálogo una vez antes de rendirse -- mismo
    // criterio que la reactivación del camino local, arriba.
    let intento_identidad = state.core().resolver_identidad_local(&cedula);
    let identidad = match intento_identidad {
        Ok(identidad) => identidad,
        Err(AutenticacionError::CredencialesInvalidas | AutenticacionError::UsuarioInactivo) => {
            let manejador = app.clone();
            let _ = tokio::time::timeout(
                ESPERA_MAXIMA_SYNC_LOGIN,
                tauri::async_runtime::spawn_blocking(move || {
                    refrescar_catalogo_sin_sesion(&manejador.state::<GuiState>())
                }),
            )
            .await;
            state.core().resolver_identidad_local(&cedula)?
        }
        Err(otro) => return Err(otro.into()),
    };

    state.iniciar_sesion(identidad.clone());
    state.iniciar_sesion_supabase(sesion_supabase.clone());

    // Best-effort a propósito (ver el doc-comment de `cachear_password_local`):
    // un fallo acá (disco lleno, lo que sea) no debe tumbar un login que ya
    // fue exitoso contra Supabase, sólo deja sin el atajo offline a esta
    // cuenta hasta el próximo login online.
    //
    // Propaga `debe_cambiar_password` al caché a propósito (hallazgo de
    // auditoría 2026-09-24, MV-01/DF-03): si la contraseña que se está
    // cacheando es una temporal todavía sin cambiar, un login sin conexión
    // más adelante debe seguir exigiendo el cambio, no aceptarla como si ya
    // fuera definitiva.
    let id_para_cache = identidad.id;
    let debe_cambiar_password_para_cache = sesion_supabase.debe_cambiar_password;
    let manejador = app.clone();
    match tauri::async_runtime::spawn_blocking(move || {
        manejador.state::<GuiState>().core().cachear_password_local(
            id_para_cache,
            &password_para_cache,
            debe_cambiar_password_para_cache,
        )
    })
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => log::warn!("no se pudo cachear el login offline: {error}"),
        Err(error) => log::warn!("no se pudo lanzar el cacheo de login offline: {error}"),
    }

    let manejador = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = tauri::async_runtime::spawn_blocking(move || {
            crate::comandos::nube::ejecutar_sincronizacion(&manejador.state::<GuiState>())
        })
        .await;
    });

    Ok(ResultadoLogin {
        sesion: identidad,
        debe_cambiar_password: sesion_supabase.debe_cambiar_password,
    })
}

/// Cambio de contraseña obligatorio (`debe_cambiar_password` en `true`
/// tras `login`) o rutinario -- misma llamada, `nube::auth_supabase::cambiar_password`
/// ya revalida `password_actual` con un login real antes de aceptar la
/// nueva, no confía en que la sesión siga abierta.
#[tauri::command]
pub async fn cambiar_password_supabase(
    password_actual: String,
    password_nueva: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let state = app.state::<GuiState>();
    let sesion = state.sesion_activa()?;
    let access_token = state
        .access_token_supabase_vigente()
        .ok_or_else(|| "La sesión venció -- iniciá sesión de nuevo".to_string())?;
    // Se usan más abajo para refrescar el caché de login offline -- `sesion`
    // y `password_nueva` se mueven al `spawn_blocking` de acá abajo.
    let id_para_cache = sesion.id;
    let password_para_cache = password_nueva.clone();

    tauri::async_runtime::spawn_blocking(move || {
        nube::cambiar_password(
            nube::base_url(),
            nube::apikey(),
            &access_token,
            &sesion.cedula,
            &password_actual,
            &password_nueva,
        )
    })
    .await
    .map_err(super::mensaje_generico)?
    .map_err(super::mensaje_generico)?;

    // Best-effort, mismo criterio que en `login_supabase` -- ver el
    // doc-comment de `cachear_password_local`. Refresca el caché con la
    // contraseña NUEVA y una marca de vencimiento fresca: sin esto, el
    // caché seguiría teniendo la contraseña VIEJA hasta el próximo login
    // online, que dejaría de servir apenas cambiara la contraseña.
    // `false`: el cambio ya se confirmó contra Supabase Auth (arriba), así
    // que la contraseña que se está cacheando ahora es la definitiva, no
    // una temporal pendiente de cambio.
    let manejador = app.clone();
    match tauri::async_runtime::spawn_blocking(move || {
        manejador.state::<GuiState>().core().cachear_password_local(
            id_para_cache,
            &password_para_cache,
            false,
        )
    })
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => log::warn!("no se pudo refrescar el cacheo de login offline: {error}"),
        Err(error) => {
            log::warn!("no se pudo lanzar el refresco del cacheo de login offline: {error}");
        }
    }

    Ok(())
}

#[tauri::command]
pub fn cerrar_sesion(state: tauri::State<GuiState>) {
    state.cerrar_sesion();
}
