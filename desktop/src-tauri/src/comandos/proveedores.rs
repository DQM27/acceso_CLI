use chrono::NaiveDate;
use control_acceso::application::registrar_ingreso_proveedor_verificado;
use control_acceso::mensajes::{
    mensaje_empresa_proveedor, mensaje_ingreso_proveedor, mensaje_ingreso_proveedor_verificado,
};
use control_acceso::models::empresa_proveedor::EmpresaProveedor;
use control_acceso::models::registro_ingreso_proveedor::RegistroIngresoProveedorActivoResumen;
use rusqlite::params;

use crate::comandos::historial::rango_utc;
use crate::dto::proveedores::SolicitudIngresoProveedorEntrada;
use crate::estado::GuiState;

// ---- Catálogo: empresas proveedoras ----

#[tauri::command]
pub fn listar_empresas_proveedor(
    state: tauri::State<GuiState>,
) -> Result<Vec<EmpresaProveedor>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_empresas_proveedor()
        .map_err(|_| "No se pudo cargar la lista de empresas".to_string())
}

/// Para un selector de wizard (elegir empresa para un ingreso nuevo) -- una
/// empresa desactivada nunca es una opción válida, ver
/// `EmpresaProveedorService::listar_seleccionables`.
#[tauri::command]
pub fn listar_empresas_proveedor_seleccionables(
    state: tauri::State<GuiState>,
) -> Result<Vec<EmpresaProveedor>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_empresas_proveedor_seleccionables()
        .map_err(|_| "No se pudo cargar la lista de empresas".to_string())
}

#[tauri::command]
pub fn buscar_empresas_proveedor(
    texto: String,
    state: tauri::State<GuiState>,
) -> Result<Vec<EmpresaProveedor>, String> {
    state.sesion_activa()?;
    state
        .core()
        .buscar_empresas_proveedor(&texto)
        .map_err(|_| "No se pudo cargar la lista de empresas".to_string())
}

#[tauri::command]
pub fn crear_empresa_proveedor(
    nombre: String,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_empresa_proveedor(&sesion, &nombre)
        .map_err(mensaje_empresa_proveedor)
}

#[tauri::command]
pub fn establecer_empresa_proveedor_activa(
    id: i64,
    activa: bool,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    let core = state.core();
    let resultado = if activa {
        core.activar_empresa_proveedor(&sesion, id)
    } else {
        core.desactivar_empresa_proveedor(&sesion, id)
    };
    resultado.map_err(mensaje_empresa_proveedor)
}

// ---- Operación: ingreso / salida ----

#[tauri::command]
pub fn registrar_ingreso_proveedor(
    solicitud: SolicitudIngresoProveedorEntrada,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    registrar_ingreso_proveedor_verificado(
        || state.core(),
        state.nube_del_dispositivo(),
        &sesion,
        solicitud.construir(),
    )
    .map_err(mensaje_ingreso_proveedor_verificado)
}

#[tauri::command]
pub fn registrar_salida_proveedor(id: i64, state: tauri::State<GuiState>) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_salida_proveedor(&sesion, id)
        .map_err(mensaje_ingreso_proveedor)
}

#[tauri::command]
pub fn listar_proveedores_activos(
    state: tauri::State<GuiState>,
) -> Result<Vec<RegistroIngresoProveedorActivoResumen>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_proveedores_activos()
        .map_err(mensaje_ingreso_proveedor)
}

// ---- Historial (exclusivo de escritorio) ----

/// Espejo de `historial_ingresos_proveedor_sitio` -- análogo a
/// `comandos::citas::MovimientoHistorialVisitaRemoto`, mismo criterio: sin
/// límite ni filtro de texto, la grilla (AG Grid) filtra del lado del
/// cliente. Sólo tiene sentido en PC -- el celular nunca sincroniza esta
/// caché (ver el doc-comment de `MIGRACION_43` en `database::schema`), así
/// que ahí siempre estaría vacía; este comando no existe del lado móvil.
#[derive(serde::Serialize)]
pub struct HistorialIngresoProveedorRemoto {
    pub uuid: String,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: Option<String>,
    pub placa: Option<String>,
    pub gafete_numero: Option<i64>,
    pub fecha_hora_ingreso: String,
    pub fecha_hora_salida: Option<String>,
    pub usuario_ingreso_nombre: Option<String>,
    pub usuario_salida_nombre: Option<String>,
}

#[tauri::command]
pub fn listar_historial_ingresos_proveedor_sitio(
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    state: tauri::State<GuiState>,
) -> Result<Vec<HistorialIngresoProveedorRemoto>, String> {
    state.sesion_activa()?;
    let (desde_utc, hasta_utc) = rango_utc(desde, hasta).map_err(super::mensaje_generico)?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, cedula, nombre, empresa_nombre, placa, gafete_numero,
                    hora_entrada, hora_salida, usuario_entrada_nombre, usuario_salida_nombre
             FROM historial_ingresos_proveedor_sitio
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
                Ok(HistorialIngresoProveedorRemoto {
                    uuid: row.get(0)?,
                    cedula: row.get(1)?,
                    nombre: row.get(2)?,
                    empresa_nombre: row.get(3)?,
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
