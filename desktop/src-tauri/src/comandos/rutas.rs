use control_acceso::mensajes::{
    mensaje_encargado_ruta, mensaje_ruta, mensaje_ruta_catalogo, mensaje_vehiculo_ruta,
};
use control_acceso::models::encargado_ruta::EncargadoRuta;
use control_acceso::models::ruta::Ruta;
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

/// Para el selector de salida de ruta -- un vehículo desactivado nunca es
/// una opción válida, ver `VehiculoRutaService::listar_seleccionables`.
#[tauri::command]
pub fn listar_vehiculos_ruta_seleccionables(
    state: tauri::State<GuiState>,
) -> Result<Vec<VehiculoRuta>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_vehiculos_ruta_seleccionables()
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

/// Para un selector de wizard (entrega de gafete provisional, salida de
/// ruta) -- un encargado desactivado nunca es una opción válida, ver
/// `EncargadoRutaService::listar_seleccionables`.
#[tauri::command]
pub fn listar_encargados_ruta_seleccionables(
    state: tauri::State<GuiState>,
) -> Result<Vec<EncargadoRuta>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_encargados_ruta_seleccionables()
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

// ---- Catálogo: números de ruta ----

#[tauri::command]
pub fn listar_rutas(state: tauri::State<GuiState>) -> Result<Vec<Ruta>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_rutas()
        .map_err(|_| "No se pudo cargar la lista de rutas".to_string())
}

/// Para el selector de salida de ruta -- una ruta dada de baja nunca es una
/// opción válida, ver `RutaCatalogoService::listar_seleccionables`.
#[tauri::command]
pub fn listar_rutas_seleccionables(state: tauri::State<GuiState>) -> Result<Vec<Ruta>, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_rutas_seleccionables()
        .map_err(|_| "No se pudo cargar la lista de rutas".to_string())
}

#[tauri::command]
pub fn crear_ruta(numero: i64, state: tauri::State<GuiState>) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_ruta(&sesion, numero)
        .map_err(mensaje_ruta_catalogo)
}

#[tauri::command]
pub fn crear_rutas_rango(
    desde: i64,
    hasta: i64,
    state: tauri::State<GuiState>,
) -> Result<Vec<i64>, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_rutas_rango(&sesion, desde, hasta)
        .map_err(mensaje_ruta_catalogo)
}

#[tauri::command]
pub fn dar_de_baja_ruta(id: i64, state: tauri::State<GuiState>) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .dar_de_baja_ruta(&sesion, id)
        .map_err(mensaje_ruta_catalogo)
}

#[tauri::command]
pub fn reactivar_ruta(id: i64, state: tauri::State<GuiState>) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .reactivar_ruta(&sesion, id)
        .map_err(mensaje_ruta_catalogo)
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
