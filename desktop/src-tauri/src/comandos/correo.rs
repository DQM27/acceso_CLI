//! Ingreso "por correo" (visita autorizada por correo, comodín previo al
//! módulo de Visitas) -- mismo molde que `comandos::proveedores`.

use chrono::NaiveDate;
use control_acceso::application::registrar_ingreso_correo_verificado;
use control_acceso::mensajes::{mensaje_ingreso_correo, mensaje_ingreso_correo_verificado};
use control_acceso::models::registro_ingreso_correo::RegistroIngresoCorreoActivoResumen;
use rusqlite::params;

use crate::comandos::historial::rango_utc;
use crate::dto::correo::SolicitudIngresoCorreoEntrada;
use crate::estado::GuiState;

#[tauri::command]
pub fn registrar_ingreso_correo(
    solicitud: SolicitudIngresoCorreoEntrada,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    registrar_ingreso_correo_verificado(
        || state.core(),
        state.nube_del_dispositivo(),
        &sesion,
        solicitud.construir(),
    )
    .map_err(mensaje_ingreso_correo_verificado)
}

#[tauri::command]
pub fn registrar_salida_correo(id: i64, state: tauri::State<GuiState>) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_salida_correo(&sesion, id)
        .map_err(mensaje_ingreso_correo)
}

#[tauri::command]
pub fn listar_correos_activos(
    state: tauri::State<GuiState>,
) -> Result<Vec<RegistroIngresoCorreoActivoResumen>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_correos_activos()
        .map_err(mensaje_ingreso_correo)
}

/// Espejo de `historial_ingresos_correo_sitio` -- mismo criterio que
/// `HistorialIngresoProveedorRemoto`: sin filtro de texto, la grilla filtra
/// del lado del cliente. Sólo escritorio.
#[derive(serde::Serialize)]
pub struct HistorialIngresoCorreoRemoto {
    pub uuid: String,
    pub cedula: String,
    pub nombre: String,
    pub motivo: Option<String>,
    pub placa: Option<String>,
    pub gafete_numero: Option<i64>,
    pub fecha_hora_ingreso: String,
    pub fecha_hora_salida: Option<String>,
    pub usuario_ingreso_nombre: Option<String>,
    pub usuario_salida_nombre: Option<String>,
}

#[tauri::command]
pub fn listar_historial_ingresos_correo_sitio(
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    state: tauri::State<GuiState>,
) -> Result<Vec<HistorialIngresoCorreoRemoto>, String> {
    state.sesion_activa()?;
    let (desde_utc, hasta_utc) = rango_utc(desde, hasta).map_err(super::mensaje_generico)?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, cedula, nombre, motivo, placa, gafete_numero,
                    hora_entrada, hora_salida, usuario_entrada_nombre, usuario_salida_nombre
             FROM historial_ingresos_correo_sitio
             WHERE hora_entrada >= ?1 AND hora_entrada < ?2
             ORDER BY hora_entrada DESC",
        )
        .map_err(super::mensaje_generico)?;
    statement
        .query_map(
            params![
                control_acceso::tiempo::serializar_utc(desde_utc),
                control_acceso::tiempo::serializar_utc(hasta_utc)
            ],
            |row| {
                Ok(HistorialIngresoCorreoRemoto {
                    uuid: row.get(0)?,
                    cedula: row.get(1)?,
                    nombre: row.get(2)?,
                    motivo: row.get(3)?,
                    placa: row.get(4)?,
                    gafete_numero: row.get(5)?,
                    fecha_hora_ingreso: row.get(6)?,
                    fecha_hora_salida: row.get(7)?,
                    usuario_ingreso_nombre: row.get(8)?,
                    usuario_salida_nombre: row.get(9)?,
                })
            },
        )
        .map_err(super::mensaje_generico)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(super::mensaje_generico)
}
