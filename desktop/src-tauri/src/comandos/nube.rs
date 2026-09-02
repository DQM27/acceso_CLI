use control_acceso::mensajes::{mensaje_gestion_nube, mensaje_nube, mensaje_sincronizacion};
use control_acceso::nube;

use crate::estado::GuiState;

/// Resultado de una sincronización, para la pantalla y para el evento que
/// emite la sincronización automática en segundo plano (ver `crate::run`).
#[derive(Debug, Clone, serde::Serialize)]
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
/// repetir "cargar secreto + pedir token" en cada función de este archivo.
fn autenticar(state: &GuiState) -> Result<nube::TokenDispositivo, String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .autorizar_gestion_nube(&actor)
        .map_err(mensaje_gestion_nube)?;

    let secreto = nube::credenciales::cargar_secreto()
        .ok_or_else(|| "Todavía no se guardó el secreto de este dispositivo".to_string())?;
    nube::autenticar_dispositivo(nube::BASE_URL, &secreto).map_err(mensaje_nube)
}

/// Autentica, drena la bandeja de salida pendiente y refresca la caché de
/// lo que el otro dispositivo del mismo sitio tiene abierto ahora mismo.
/// Compartida por el comando manual (`sincronizar_con_nube`) y el
/// disparador automático (`crate::run`) -- misma lógica, dos formas de
/// dispararla. Igual que `crear_respaldo`: autoriza rápido con el candado
/// compartido (dentro de `autenticar`), lo suelta, y hace la parte lenta
/// (red) sobre una conexión propia -- ver `GuiState::conexion_secundaria`.
pub fn ejecutar_sincronizacion(state: &GuiState) -> Result<ResumenSincronizacion, String> {
    let token = autenticar(state)?;
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::BASE_URL,
        apikey: nube::APIKEY,
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };

    let conexion = state.conexion_secundaria()?;
    let resumen = nube::drenar_cola(&conexion, &contexto, 200).map_err(mensaje_sincronizacion)?;
    let remotos = nube::recibir_ingresos_abiertos(&conexion, &contexto)
        .map_err(mensaje_sincronizacion)?;

    Ok(ResumenSincronizacion {
        enviados: resumen.enviados,
        fallidos: resumen.fallidos,
        remotos_abiertos: u32::try_from(remotos.len()).unwrap_or(u32::MAX),
        sitio_id: token.sitio_id,
        dispositivo_id: token.dispositivo_id,
        tipo: token.tipo,
    })
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
        .map_err(mensaje_gestion_nube)
}

/// No revela el secreto -- sólo si ya hay uno guardado, para que la
/// pantalla sepa si mostrar el campo para pegarlo o el estado "configurado".
#[tauri::command]
pub fn secreto_dispositivo_guardado(state: tauri::State<GuiState>) -> Result<bool, String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .secreto_dispositivo_guardado(&actor, None)
        .map_err(mensaje_gestion_nube)
}

#[tauri::command]
pub fn sincronizar_con_nube(state: tauri::State<GuiState>) -> Result<ResumenSincronizacion, String> {
    ejecutar_sincronizacion(&state)
}

/// Cuántas filas de `cola_salida` ya agotaron los reintentos automáticos
/// (ver `INTENTOS_ANTES_DE_FALLO_PERMANENTE`) -- para avisar que algo
/// necesita que alguien lo mire, en vez de fallar en silencio para siempre.
#[tauri::command]
pub fn fallos_permanentes_nube(state: tauri::State<GuiState>) -> Result<i64, String> {
    state.sesion_activa()?;
    let conexion = state.conexion_secundaria()?;
    nube::contar_fallos_permanentes(&conexion).map_err(mensaje_sincronizacion)
}

/// Lectura pura de la caché local `ingresos_remotos` -- ya la llenó la
/// última sincronización (manual o automática), no hace falta red para
/// mostrarla. Errores de `SQLite` crudos acá abajo: no hay un `mensaje_*`
/// para `rusqlite::Error` suelto (siempre viene envuelto en un tipo propio
/// en el resto del núcleo), y este `SELECT` no debería fallar en la
/// práctica.
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
        .map_err(mensaje_sincronizacion)
}
