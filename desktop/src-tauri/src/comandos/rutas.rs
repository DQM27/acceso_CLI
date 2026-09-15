use control_acceso::mensajes::{mensaje_encargado_ruta, mensaje_ruta, mensaje_vehiculo_ruta};
use control_acceso::models::encargado_ruta::EncargadoRuta;
use control_acceso::models::salida_ruta::{SalidaRuta, SalidaRutaActivaResumen};
use control_acceso::models::vehiculo_ruta::VehiculoRuta;

use crate::dto::rutas::{
    DatosEncargadoRutaEntrada, DatosVehiculoRutaEntrada, SolicitudSalidaRutaEntrada,
};
use crate::estado::GuiState;

// ---- Catálogo: vehículos ----

#[tauri::command]
pub fn listar_vehiculos_ruta(state: tauri::State<GuiState>) -> Result<Vec<VehiculoRuta>, String> {
    state.sesion_activa()?;
    // Sin `mensaje_*` propio a propósito, mismo criterio que
    // `listar_visitas_activas`: es una lectura simple que devuelve
    // `DatabaseError` directo, no un `*ServiceError` de negocio.
    state
        .core()
        .listar_vehiculos_ruta()
        .map_err(|_| "No se pudo cargar la lista de vehículos".to_string())
}

#[tauri::command]
pub fn crear_vehiculo_ruta(
    datos: DatosVehiculoRutaEntrada,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_vehiculo_ruta(&sesion, &datos.construir(0))
        .map_err(mensaje_vehiculo_ruta)
}

#[tauri::command]
pub fn actualizar_vehiculo_ruta(
    id: i64,
    datos: DatosVehiculoRutaEntrada,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .actualizar_vehiculo_ruta(&sesion, &datos.construir(id))
        .map_err(mensaje_vehiculo_ruta)
}

// ---- Catálogo: encargados (personal KOF) ----

#[tauri::command]
pub fn listar_encargados_ruta(state: tauri::State<GuiState>) -> Result<Vec<EncargadoRuta>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_encargados_ruta()
        .map_err(|_| "No se pudo cargar la lista de encargados".to_string())
}

#[tauri::command]
pub fn crear_encargado_ruta(
    datos: DatosEncargadoRutaEntrada,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_encargado_ruta(&sesion, &datos.construir(0))
        .map_err(mensaje_encargado_ruta)
}

#[tauri::command]
pub fn actualizar_encargado_ruta(
    id: i64,
    datos: DatosEncargadoRutaEntrada,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .actualizar_encargado_ruta(&sesion, &datos.construir(id))
        .map_err(mensaje_encargado_ruta)
}

// ---- Operación: salida / retorno ----

#[tauri::command]
pub fn registrar_salida_ruta(
    solicitud: SolicitudSalidaRutaEntrada,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_salida_ruta(&sesion, solicitud.construir())
        .map(|resultado| resultado.salida_id)
        .map_err(mensaje_ruta)
}

#[tauri::command]
pub fn registrar_retorno_ruta(salida_id: i64, state: tauri::State<GuiState>) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_retorno_ruta(&sesion, salida_id)
        .map_err(mensaje_ruta)
}

#[tauri::command]
pub fn listar_rutas_activas(
    state: tauri::State<GuiState>,
) -> Result<Vec<SalidaRutaActivaResumen>, String> {
    state.sesion_activa()?;
    state.core().listar_rutas_activas().map_err(mensaje_ruta)
}

#[tauri::command]
pub fn buscar_salida_ruta(
    id: i64,
    state: tauri::State<GuiState>,
) -> Result<Option<SalidaRuta>, String> {
    state.sesion_activa()?;
    state.core().buscar_salida_ruta(id).map_err(mensaje_ruta)
}
