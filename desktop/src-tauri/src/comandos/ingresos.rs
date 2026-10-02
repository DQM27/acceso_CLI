use control_acceso::application::{preparar_ingreso_verificado, registrar_ingreso_verificado};
use control_acceso::database::queries::ingresos::FiltroIngresosActivos;
use control_acceso::mensajes::{
    mensaje_bloqueo_ingreso, mensaje_ingreso, mensaje_ingreso_verificado, mensaje_salida,
};
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::services::registro_ingreso_service::{
    ListaIngresosActivosResumen, PreparacionIngreso, ResultadoRegistroEntrada,
};
use tauri::Manager;

use crate::estado::GuiState;

/// Sin filtro de entrada a propósito: la grilla de Activos filtra en el
/// cliente (columnas de AG Grid) sobre esta misma lista, no repite la
/// consulta contra `SQLite` por cada tecla — a diferencia de Contratistas,
/// que sí filtra en el servidor porque su universo no cabe entero en
/// memoria del lado del webview.
#[tauri::command]
pub fn listar_ingresos_activos(
    state: tauri::State<GuiState>,
) -> Result<ListaIngresosActivosResumen, String> {
    state.sesion_activa()?;
    state
        .core()
        .listar_ingresos_activos(&FiltroIngresosActivos::default())
        .map_err(mensaje_ingreso)
}

/// `PreparacionIngreso` más el motivo de bloqueo ya resuelto por el núcleo
/// (`application::preparar_ingreso_verificado`: reglas locales y, con nube,
/// la verificación en vivo), igual que recibe el móvil: el frontend no
/// decide si se puede continuar ni arma el texto. `None` = se puede
/// continuar.
#[derive(serde::Serialize)]
pub struct PreparacionIngresoConBloqueo {
    #[serde(flatten)]
    preparacion: PreparacionIngreso,
    mensaje_bloqueo: Option<String>,
}

#[tauri::command]
pub async fn preparar_ingreso(
    contratista_id: i64,
    app: tauri::AppHandle,
) -> Result<PreparacionIngresoConBloqueo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<GuiState>();
        let sesion = state.sesion_activa()?;
        let (preparacion, bloqueo) = preparar_ingreso_verificado(
            || state.core(),
            state.nube_del_dispositivo(),
            Some(&sesion),
            contratista_id,
        )
        .map_err(mensaje_ingreso)?;
        Ok(PreparacionIngresoConBloqueo {
            preparacion,
            mensaje_bloqueo: bloqueo.as_ref().map(mensaje_bloqueo_ingreso),
        })
    })
    .await
    .map_err(|error| format!("No se pudo preparar el ingreso: {error}"))?
}

#[tauri::command]
pub fn registrar_ingreso(
    contratista_id: i64,
    medio: MedioIngreso,
    gafete: Option<i64>,
    placa: Option<String>,
    state: tauri::State<GuiState>,
) -> Result<ResultadoRegistroEntrada, String> {
    let sesion = state.sesion_activa()?;
    registrar_ingreso_verificado(
        || state.core(),
        state.nube_del_dispositivo(),
        &sesion,
        contratista_id,
        medio,
        gafete,
        placa,
    )
    .map_err(mensaje_ingreso_verificado)
}

#[tauri::command]
pub fn registrar_salida(id: i64, state: tauri::State<GuiState>) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_salida(&sesion, id)
        .map_err(mensaje_salida)
}
