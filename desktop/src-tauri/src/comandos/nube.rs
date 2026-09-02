use control_acceso::nube;

use crate::estado::GuiState;

/// Resultado de una sincronización manual, para la pantalla.
#[derive(serde::Serialize)]
pub struct ResumenSincronizacion {
    pub enviados: u32,
    pub fallidos: u32,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
}

/// Guarda el secreto de este dispositivo (pegado una sola vez desde el
/// panel de administración). Autoriza y escribe en el mismo paso —
/// escribir un archivo chico es barato, no hace falta soltar el candado
/// como en `sincronizar_con_nube`.
#[tauri::command]
pub fn guardar_secreto_dispositivo(
    secreto: String,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .guardar_secreto_dispositivo(&actor, &secreto)
        .map_err(|error| error.to_string())
}

/// No revela el secreto -- sólo si ya hay uno guardado, para que la
/// pantalla sepa si mostrar el campo para pegarlo o el estado "configurado".
#[tauri::command]
pub fn secreto_dispositivo_guardado(state: tauri::State<GuiState>) -> Result<bool, String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .secreto_dispositivo_guardado(&actor)
        .map_err(|error| error.to_string())
}

/// Autentica este dispositivo y drena la bandeja de salida pendiente. Igual
/// que `crear_respaldo`: autoriza rápido con el candado compartido, lo
/// suelta, y hace la parte lenta (red + reintentos por fila) sobre una
/// conexión propia -- ver `GuiState::conexion_secundaria`.
#[tauri::command]
pub fn sincronizar_con_nube(state: tauri::State<GuiState>) -> Result<ResumenSincronizacion, String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .autorizar_gestion_nube(&actor)
        .map_err(|error| error.to_string())?;

    let secreto = nube::credenciales::cargar_secreto()
        .ok_or_else(|| "Todavía no se guardó el secreto de este dispositivo".to_string())?;
    let token = nube::autenticar_dispositivo(nube::BASE_URL, &secreto)
        .map_err(|error| error.to_string())?;

    let contexto = nube::ContextoSincronizacion {
        base_url: nube::BASE_URL,
        apikey: nube::APIKEY,
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    let conexion = state.conexion_secundaria()?;
    let resumen = nube::drenar_cola(&conexion, &contexto, 200).map_err(|error| error.to_string())?;

    Ok(ResumenSincronizacion {
        enviados: resumen.enviados,
        fallidos: resumen.fallidos,
        sitio_id: token.sitio_id,
        dispositivo_id: token.dispositivo_id,
        tipo: token.tipo,
    })
}
