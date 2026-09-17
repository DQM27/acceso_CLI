use control_acceso::mensajes::{mensaje_empresa_proveedor, mensaje_ingreso_proveedor};
use control_acceso::models::empresa_proveedor::EmpresaProveedor;
use control_acceso::models::registro_ingreso_proveedor::RegistroIngresoProveedorActivoResumen;

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
    let datos = solicitud.construir();
    state
        .core()
        .registrar_ingreso_proveedor(
            &sesion,
            &datos.cedula,
            &datos.nombre,
            datos.empresa_id,
            datos.placa,
            datos.gafete_numero,
        )
        .map_err(mensaje_ingreso_proveedor)
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
