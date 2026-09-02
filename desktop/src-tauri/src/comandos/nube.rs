use control_acceso::nube;

use crate::estado::GuiState;

/// Resultado de una sincronización manual, para la pantalla.
#[derive(serde::Serialize)]
pub struct ResumenSincronizacion {
    pub enviados: u32,
    pub fallidos: u32,
    pub remotos_abiertos: u32,
    pub sitio_id: String,
    pub dispositivo_id: String,
    pub tipo: String,
}

/// Espejo de `nube::IngresoRemoto` -- un ingreso abierto por el otro
/// dispositivo del mismo sitio, listo para mostrarse y, si hace falta,
/// cerrarse desde acá.
#[derive(serde::Serialize)]
pub struct IngresoRemoto {
    pub uuid: String,
    pub contratista_nombre: String,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: Option<String>,
}

/// Autentica este dispositivo contra el receptor -- un solo lugar para no
/// repetir "cargar secreto + pedir token" en cada comando de este archivo.
fn autenticar(state: &tauri::State<GuiState>) -> Result<nube::TokenDispositivo, String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .autorizar_gestion_nube(&actor)
        .map_err(|error| error.to_string())?;

    let secreto = nube::credenciales::cargar_secreto()
        .ok_or_else(|| "Todavía no se guardó el secreto de este dispositivo".to_string())?;
    nube::autenticar_dispositivo(nube::BASE_URL, &secreto).map_err(|error| error.to_string())
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
        .guardar_secreto_dispositivo(&actor, None, &secreto)
        .map_err(|error| error.to_string())
}

/// No revela el secreto -- sólo si ya hay uno guardado, para que la
/// pantalla sepa si mostrar el campo para pegarlo o el estado "configurado".
#[tauri::command]
pub fn secreto_dispositivo_guardado(state: tauri::State<GuiState>) -> Result<bool, String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .secreto_dispositivo_guardado(&actor, None)
        .map_err(|error| error.to_string())
}

/// Autentica este dispositivo, drena la bandeja de salida pendiente y
/// refresca la caché de lo que el otro dispositivo del mismo sitio tiene
/// abierto ahora mismo. Igual que `crear_respaldo`: autoriza rápido con el
/// candado compartido, lo suelta, y hace la parte lenta (red) sobre una
/// conexión propia -- ver `GuiState::conexion_secundaria`.
#[tauri::command]
pub fn sincronizar_con_nube(state: tauri::State<GuiState>) -> Result<ResumenSincronizacion, String> {
    let token = autenticar(&state)?;
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::BASE_URL,
        apikey: nube::APIKEY,
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };

    let conexion = state.conexion_secundaria()?;
    let resumen = nube::drenar_cola(&conexion, &contexto, 200).map_err(|error| error.to_string())?;
    let remotos = nube::recibir_ingresos_abiertos(&conexion, &contexto)
        .map_err(|error| error.to_string())?;

    Ok(ResumenSincronizacion {
        enviados: resumen.enviados,
        fallidos: resumen.fallidos,
        remotos_abiertos: u32::try_from(remotos.len()).unwrap_or(u32::MAX),
        sitio_id: token.sitio_id,
        dispositivo_id: token.dispositivo_id,
        tipo: token.tipo,
    })
}

/// Lectura pura de la caché local `ingresos_remotos` -- ya la llenó la
/// última `sincronizar_con_nube`, no hace falta red para mostrarla.
#[tauri::command]
pub fn listar_ingresos_remotos(state: tauri::State<GuiState>) -> Result<Vec<IngresoRemoto>, String> {
    state.sesion_activa()?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, contratista_nombre, hora_entrada, usuario_entrada_nombre
             FROM ingresos_remotos ORDER BY hora_entrada",
        )
        .map_err(|error| error.to_string())?;
    let filas = statement
        .query_map([], |row| {
            Ok(IngresoRemoto {
                uuid: row.get(0)?,
                contratista_nombre: row.get(1)?,
                hora_entrada: row.get(2)?,
                usuario_entrada_nombre: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(filas)
}

/// Cierra, contra la nube, un ingreso abierto por el otro dispositivo del
/// mismo sitio -- ver `nube::cerrar_ingreso_remoto`: nunca toca el
/// historial local, sólo la caché.
#[tauri::command]
pub fn cerrar_ingreso_remoto(uuid: String, state: tauri::State<GuiState>) -> Result<(), String> {
    let actor = state.sesion_activa()?;
    let token = autenticar(&state)?;
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::BASE_URL,
        apikey: nube::APIKEY,
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    let conexion = state.conexion_secundaria()?;
    nube::cerrar_ingreso_remoto(&conexion, &contexto, &uuid, &actor.nombre)
        .map_err(|error| error.to_string())
}
