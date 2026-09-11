use control_acceso::database::queries::ingresos::FiltroIngresosActivos;
use control_acceso::domain::resultado_acceso::ResultadoAcceso;
use control_acceso::mensajes::{
    mensaje_gestion_nube, mensaje_ingreso, mensaje_nube, mensaje_salida, mensaje_sincronizacion,
};
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::nube;
use control_acceso::services::registro_ingreso_service::{
    ListaIngresosActivosResumen, PreparacionIngreso, ResultadoRegistroEntrada,
};
use tauri::Manager;

use crate::estado::GuiState;

/// Tope para el chequeo cruzado de "¿esta cédula ya está activa en otro
/// sitio?" (`docs/pendientes.md`) -- mismo criterio de mejor esfuerzo que
/// `ESPERA_MAXIMA_SYNC_LOGIN` en `comandos/autenticacion.rs`: con conexión
/// bloquea, sin conexión (o si tarda más de esto) el registro sigue local
/// sin frenar al operador -- el conflicto, si lo hay, se detecta después al
/// sincronizar.
const ESPERA_MAXIMA_CHEQUEO_OTRO_SITIO: std::time::Duration = std::time::Duration::from_secs(5);

/// Best-effort: sin secreto de dispositivo guardado no hay con qué
/// consultar, no bloquea nada. Sólo se llama cuando los chequeos locales de
/// `preparar_ingreso` ya dejaron pasar (sin sentido gastar una vuelta de
/// red para algo que de todos modos ya está bloqueado).
fn chequear_activo_en_otro_sitio(state: &GuiState, cedula: &str) -> Option<String> {
    let secreto = nube::credenciales::cargar_secreto()?;
    let token = state.autenticar_con_cache(&secreto).ok()?;
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
    nube::contratista_activo_en_otro_sitio(&contexto, cedula)
        .ok()
        .flatten()
}

/// Chequeo en vivo -- no la caché local `ingresos_remotos`, que sólo se
/// refresca en cada sync y podría estar desactualizada -- de si `numero` ya
/// está activo en este sitio del lado de OTRO dispositivo. Cada dispositivo
/// sólo valida el gafete contra su propia base `SQLite`
/// (`idx_registro_ingresos_gafete_activo`), que nunca ve lo que hizo el
/// otro hasta sincronizar -- así ambos podían aceptar el mismo número como
/// activo a la vez. Sin secreto guardado (dispositivo sin nube configurada)
/// no hay con quién chocar, se salta sin tocar la red -- `Ok(true)`
/// ("libre") directo. Con nube configurada, en cambio, esto exige estar en
/// línea: si la consulta falla, el error se propaga en vez de asumir que el
/// gafete está libre (decisión explícita del usuario: prefiere bloquear el
/// ingreso a arriesgar el mismo número duplicado otra vez). Igual que
/// `autenticar` (`comandos/nube.rs`): autoriza rápido con el candado
/// compartido y lo suelta antes de la parte lenta (red).
fn gafete_libre_en_otro_dispositivo(state: &GuiState, numero: i64) -> Result<bool, String> {
    let Some(secreto) = nube::credenciales::cargar_secreto() else {
        return Ok(true);
    };
    let actor = state.sesion_activa()?;
    state
        .core()
        .autorizar_uso_nube(&actor)
        .map_err(mensaje_gestion_nube)?;

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
    let ocupado = nube::gafete_ocupado_en_otro_dispositivo(&contexto, numero)
        .map_err(mensaje_sincronizacion)?;
    Ok(!ocupado)
}

/// Sin filtro de entrada a propósito: la grilla de Activos filtra en el
/// cliente (columnas de AG Grid) sobre esta misma lista, no repite la
/// consulta contra `SQLite` por cada tecla — a diferencia de Contratistas,
/// que sí filtra en el servidor porque su universo no cabe entero en
/// memoria del lado del webview.
#[tauri::command]
pub fn listar_ingresos_activos(
    state: tauri::State<GuiState>,
) -> Result<ListaIngresosActivosResumen, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_ingresos_activos(&FiltroIngresosActivos::default())
        .map_err(mensaje_ingreso)
}

/// Dos chequeos de "ya está adentro", uno local (`tiene_ingreso_activo`,
/// instantáneo, siempre corre) y uno remoto (`activo_en_otro_sitio`, mejor
/// esfuerzo, ver `chequear_activo_en_otro_sitio`) -- sólo el segundo
/// necesita `async`/tope de tiempo, por eso el comando entero lo es (mismo
/// motivo que `login` en `comandos/autenticacion.rs`).
#[tauri::command]
pub async fn preparar_ingreso(
    contratista_id: i64,
    app: tauri::AppHandle,
) -> Result<PreparacionIngreso, String> {
    let state = app.state::<GuiState>();
    state.sesion_activa()?;
    let mut preparacion = state
        .core()
        .preparar_ingreso(contratista_id)
        .map_err(mensaje_ingreso)?;

    if !preparacion.tiene_ingreso_activo
        && !matches!(preparacion.resultado_acceso, ResultadoAcceso::Denegado(_))
    {
        let cedula = preparacion.cedula.clone();
        let manejador = app.clone();
        let chequeo = tokio::time::timeout(
            ESPERA_MAXIMA_CHEQUEO_OTRO_SITIO,
            tauri::async_runtime::spawn_blocking(move || {
                chequear_activo_en_otro_sitio(&manejador.state::<GuiState>(), &cedula)
            }),
        )
        .await;
        if let Ok(Ok(sitio)) = chequeo {
            preparacion.activo_en_otro_sitio = sitio;
        }
    }

    Ok(preparacion)
}

#[tauri::command]
pub fn registrar_ingreso(
    contratista_id: i64,
    medio: MedioIngreso,
    gafete: Option<i64>,
    state: tauri::State<GuiState>,
) -> Result<ResultadoRegistroEntrada, String> {
    let sesion = state.sesion_activa()?;
    if let Some(numero) = gafete
        && !gafete_libre_en_otro_dispositivo(&state, numero)?
    {
        return Err(format!(
            "El gafete {numero} ya está en uso en otro dispositivo del sitio"
        ));
    }
    state
        .core()
        .registrar_ingreso(&sesion, contratista_id, medio, gafete)
        .map_err(mensaje_ingreso)
}

#[tauri::command]
pub fn registrar_salida(id: i64, state: tauri::State<GuiState>) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_salida(&sesion, id)
        .map_err(mensaje_salida)
}
