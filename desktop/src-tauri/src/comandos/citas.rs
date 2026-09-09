use chrono::NaiveDate;
use control_acceso::mensajes::mensaje_cita;
use control_acceso::models::cita::{Cita, CitaVisitante};
use control_acceso::models::movimiento_visita::MovimientoVisitaActivoResumen;
use rusqlite::params;

use crate::comandos::historial::rango_utc;
use crate::estado::GuiState;

/// DTO de presentación -- `AppCore::verificar_check_in_visita` devuelve una
/// tupla `(Cita, CitaVisitante)`; acá se nombra para que el lado TypeScript
/// tenga campos, no un array posicional.
#[derive(serde::Serialize)]
pub struct PreparacionVisita {
    pub cita: Cita,
    pub visitante: CitaVisitante,
}

#[tauri::command]
pub fn verificar_check_in_visita(
    cedula: String,
    state: tauri::State<GuiState>,
) -> Result<PreparacionVisita, String> {
    state.sesion_activa()?;
    let (cita, visitante) = state
        .core()
        .verificar_check_in_visita(&cedula)
        .map_err(mensaje_cita)?;
    Ok(PreparacionVisita { cita, visitante })
}

#[tauri::command]
pub fn registrar_entrada_visita(
    cedula: String,
    gafete: Option<i64>,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_entrada_visita(&sesion, &cedula, gafete)
        .map_err(mensaje_cita)
}

#[tauri::command]
pub fn registrar_salida_visita(
    movimiento_id: i64,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_salida_visita(&sesion, movimiento_id)
        .map_err(mensaje_cita)
}

#[tauri::command]
pub fn listar_visitas_activas(
    state: tauri::State<GuiState>,
) -> Result<Vec<MovimientoVisitaActivoResumen>, String> {
    state.sesion_activa()?;
    // Sin `mensaje_*` propio a propósito: `AppCore::listar_visitas_activas`
    // devuelve `DatabaseError` directo (es una lectura simple, no pasa por
    // un `*ServiceError` de negocio) -- no exponer su `Display` (interpola
    // el error crudo de `SQLite`), mismo criterio que el resto de mensajes
    // de este módulo.
    state
        .core()
        .listar_visitas_activas()
        .map_err(|_| "No se pudo cargar la lista de visitas activas".to_string())
}

/// Espejo de `historial_visitas_sitio` -- análogo a
/// `comandos::historial::listar_historial_sitio` (contratistas), mismo
/// criterio: sin límite ni filtro de texto, la grilla (AG Grid) filtra del
/// lado del cliente. Sólo tiene sentido en PC -- el celular nunca sincroniza
/// esta caché (`mobile/rust-core/src/lib.rs`), así que ahí siempre estaría
/// vacía; este comando no existe del lado móvil.
#[derive(serde::Serialize)]
pub struct MovimientoHistorialVisitaRemoto {
    pub uuid: String,
    pub cedula: String,
    pub nombre: String,
    pub empresa: Option<String>,
    pub anfitrion_nombre: Option<String>,
    pub motivo: Option<String>,
    pub gafete_numero: Option<i64>,
    pub fecha_hora_entrada: String,
    pub fecha_hora_salida: Option<String>,
    pub usuario_entrada_nombre: Option<String>,
    pub usuario_salida_nombre: Option<String>,
}

#[tauri::command]
pub fn listar_historial_visitas_sitio(
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    state: tauri::State<GuiState>,
) -> Result<Vec<MovimientoHistorialVisitaRemoto>, String> {
    state.sesion_activa()?;
    let (desde_utc, hasta_utc) = rango_utc(desde, hasta).map_err(|error| error.to_string())?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, visitante_cedula, visitante_nombre, empresa, anfitrion_nombre,
                    motivo, gafete_numero, hora_entrada, hora_salida,
                    usuario_entrada_nombre, usuario_salida_nombre
             FROM historial_visitas_sitio
             WHERE hora_entrada >= ?1 AND hora_entrada < ?2
             ORDER BY hora_entrada DESC",
        )
        .map_err(|error| error.to_string())?;
    statement
        .query_map(
            params![
                control_acceso::tiempo::serializar_utc(desde_utc),
                control_acceso::tiempo::serializar_utc(hasta_utc)
            ],
            |row| {
                Ok(MovimientoHistorialVisitaRemoto {
                    uuid: row.get(0)?,
                    cedula: row.get(1)?,
                    nombre: row.get(2)?,
                    empresa: row.get(3)?,
                    anfitrion_nombre: row.get(4)?,
                    motivo: row.get(5)?,
                    gafete_numero: row.get(6)?,
                    fecha_hora_entrada: row.get(7)?,
                    fecha_hora_salida: row.get(8)?,
                    usuario_entrada_nombre: row.get(9)?,
                    usuario_salida_nombre: row.get(10)?,
                })
            },
        )
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}
