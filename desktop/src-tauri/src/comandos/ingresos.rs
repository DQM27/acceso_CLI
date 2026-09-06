use control_acceso::database::queries::ingresos::FiltroIngresosActivos;
use control_acceso::mensajes::{
    mensaje_gestion_nube, mensaje_ingreso, mensaje_nube, mensaje_salida, mensaje_sincronizacion,
};
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::nube;
use control_acceso::services::registro_ingreso_service::{
    ListaIngresosActivosResumen, PreparacionIngreso, ResultadoRegistroEntrada,
};

use crate::estado::GuiState;

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

#[tauri::command]
pub fn preparar_ingreso(
    contratista_id: i64,
    state: tauri::State<GuiState>,
) -> Result<PreparacionIngreso, String> {
    state.sesion_activa()?;
    state
        .core()
        .preparar_ingreso(contratista_id)
        .map_err(mensaje_ingreso)
}

#[tauri::command]
pub fn registrar_ingreso(
    contratista_id: i64,
    medio: MedioIngreso,
    gafete: Option<i64>,
    state: tauri::State<GuiState>,
) -> Result<ResultadoRegistroEntrada, String> {
    let sesion = state.sesion_activa()?;
    if let Some(numero) = gafete {
        if !gafete_libre_en_otro_dispositivo(&state, numero)? {
            return Err(format!(
                "El gafete {numero} ya está en uso en otro dispositivo del sitio"
            ));
        }
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
