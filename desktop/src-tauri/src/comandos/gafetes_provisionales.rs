use chrono::NaiveDate;
use control_acceso::application::entregar_gafete_provisional_verificado;
use control_acceso::mensajes::{
    mensaje_entrega_gafete_provisional_verificada, mensaje_gafete_provisional,
};
use control_acceso::models::encargado_ruta::EncargadoRuta;
use control_acceso::models::prestamo_gafete_provisional::PrestamoGafeteProvisionalActivoResumen;
use rusqlite::params;

use crate::comandos::historial::rango_utc;
use crate::estado::GuiState;

/// Buscador por nombre o código de empleado -- mismo catálogo
/// (`encargados_ruta`) que ya usa `PantallaRutas`/`listar_encargados_ruta`,
/// pero acotado en el núcleo en vez de traer el universo completo: esta
/// pantalla no es una grilla AG Grid con filtro del lado del cliente, es un
/// campo de búsqueda como el del buscador de empresas proveedoras.
#[tauri::command]
pub fn buscar_encargados_ruta_provisional(
    texto: String,
    state: tauri::State<GuiState>,
) -> Result<Vec<EncargadoRuta>, String> {
    state.sesion_activa()?;
    state
        .core()
        // Es un selector para prestar un gafete -- un encargado desactivado
        // no es una opción válida (mismo criterio que
        // `EncargadoRutaService::buscar_seleccionables`).
        .buscar_encargados_ruta_seleccionables(&texto)
        .map_err(|_| "No se pudo buscar en el catálogo de encargados".to_string())
}

#[tauri::command]
pub fn entregar_gafete_provisional(
    encargado_id: i64,
    gafete_numero: i64,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    let secreto = state.credencial_nube();
    entregar_gafete_provisional_verificado(
        || state.core(),
        state.nube_del_dispositivo(secreto.as_deref()),
        &sesion,
        encargado_id,
        gafete_numero,
    )
    .map_err(mensaje_entrega_gafete_provisional_verificada)
}

#[tauri::command]
pub fn registrar_devolucion_gafete_provisional(
    id: i64,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_devolucion_gafete_provisional(&sesion, id)
        .map_err(mensaje_gafete_provisional)
}

#[tauri::command]
pub fn listar_gafetes_provisionales_activos(
    state: tauri::State<GuiState>,
) -> Result<Vec<PrestamoGafeteProvisionalActivoResumen>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_gafetes_provisionales_activos()
        .map_err(mensaje_gafete_provisional)
}

/// Espejo de `prestamos_gafete_provisional_historial_sitio` (ver
/// `database::schema`, migración 50) -- préstamos del sitio generados por
/// CUALQUIER dispositivo, incluido éste. Mismo criterio que
/// `listar_historial_ingresos_proveedor_sitio` (`comandos::proveedores`):
/// esta caché ya incluye lo que ESTE dispositivo entregó (vuelve sincronizada
/// desde Supabase), así que la pantalla la usa como única fuente para
/// "Historial" -- sin fusionar con la tabla local, mismo criterio que
/// Proveedores/Visitas (volumen bajo, no amerita esa complejidad extra).
#[derive(serde::Serialize)]
pub struct PrestamoGafeteProvisionalHistorialSitio {
    pub uuid: String,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: String,
    pub gafete_numero: i64,
    pub fecha_hora_entrega: String,
    pub usuario_entrega_nombre: String,
    pub fecha_hora_devolucion: Option<String>,
    pub usuario_devolucion_nombre: Option<String>,
}

#[tauri::command]
pub fn listar_gafetes_provisionales_historial_sitio(
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    state: tauri::State<GuiState>,
) -> Result<Vec<PrestamoGafeteProvisionalHistorialSitio>, String> {
    state.sesion_activa()?;
    // Mismo filtro de período que Historial y el historial de proveedores
    // (selector "Período", ver `rango_utc`) -- `fecha_hora_entrega` ya se
    // guarda normalizada con `serializar_utc`, así que compara como texto.
    let (desde_utc, hasta_utc) = rango_utc(desde, hasta).map_err(super::mensaje_generico)?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, encargado_nombre, encargado_codigo_empleado, gafete_numero,
                    fecha_hora_entrega, usuario_entrega_nombre, fecha_hora_devolucion,
                    usuario_devolucion_nombre
             FROM prestamos_gafete_provisional_historial_sitio
             WHERE fecha_hora_entrega >= ?1 AND fecha_hora_entrega < ?2
             ORDER BY fecha_hora_entrega DESC",
        )
        .map_err(super::mensaje_generico)?;
    let filas = statement
        .query_map(
            params![
                control_acceso::tiempo::serializar_utc(desde_utc),
                control_acceso::tiempo::serializar_utc(hasta_utc)
            ],
            |row| {
                Ok(PrestamoGafeteProvisionalHistorialSitio {
                    uuid: row.get(0)?,
                    encargado_nombre: row.get(1)?,
                    encargado_codigo_empleado: row.get(2)?,
                    gafete_numero: row.get(3)?,
                    fecha_hora_entrega: row.get(4)?,
                    usuario_entrega_nombre: row.get(5)?,
                    fecha_hora_devolucion: row.get(6)?,
                    usuario_devolucion_nombre: row.get(7)?,
                })
            },
        )
        .map_err(super::mensaje_generico)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(super::mensaje_generico)?;
    Ok(filas)
}
