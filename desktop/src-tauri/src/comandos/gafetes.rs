use control_acceso::database::queries::gafetes::GafeteResumen;
use control_acceso::database::queries::gafetes_incidentes::IncidenteGafete;
use control_acceso::domain::gafete::{LecturaConteo, ResultadoConteo};
use control_acceso::models::gafete::PortadorGafete;

use crate::dto::gafetes::{FiltroGafetesEntrada, TipoGafeteEntrada};
use crate::estado::GuiState;

/// Catálogo completo de gafetes, sin restricción de rol a propósito (mismo
/// criterio que el núcleo — cualquier operador con sesión gestiona alta,
/// perdido, pago, recuperar y conteo, a diferencia de Empresas/Usuarios).
#[tauri::command(async)]
pub fn buscar_gafetes(
    filtro: FiltroGafetesEntrada,
    state: tauri::State<GuiState>,
) -> Result<Vec<GafeteResumen>, String> {
    state.sesion_activa()?;
    state
        .core()
        .buscar_gafetes(&filtro.construir())
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

/// Quién tiene abierto el gafete `numero`, de cualquier clase (contratista,
/// proveedor, visita por correo o KOF), este equipo y el otro dispositivo del
/// sitio. Un número puede tener varios: cada conjunto de gafetes numera por su
/// cuenta. La interfaz sólo muestra la respuesta y, si hay más de uno, pide
/// elegir (ver `SalidaModal`). `DatabaseError` directo, sin variante de
/// negocio: el detalle va al registro técnico (`mensaje_generico`).
#[tauri::command(async)]
pub fn tenedores_del_gafete(
    numero: i64,
    state: tauri::State<GuiState>,
) -> Result<Vec<control_acceso::application::TenedorGafete>, String> {
    let actor = state.sesion_activa()?;
    state
        .core()
        .tenedores_del_gafete(&actor, numero)
        .map_err(control_acceso::mensajes::mensaje_generico)
}

/// Historial de un gafete puntual — el núcleo no gatea esta lectura por
/// actor (mismo criterio que `buscar_gafetes`), pero la GUI igual exige
/// sesión activa, por convención (ver `comandos/mod.rs`).
#[tauri::command(async)]
pub fn historial_gafete(
    id: i64,
    state: tauri::State<GuiState>,
) -> Result<Vec<IncidenteGafete>, String> {
    state.sesion_activa()?;
    state
        .core()
        .historial_gafete(id)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command(async)]
pub fn crear_gafete(
    numero: i64,
    tipo: TipoGafeteEntrada,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_gafete(&sesion, numero, tipo.into())
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command(async)]
pub fn crear_gafetes_rango(
    desde: i64,
    hasta: i64,
    tipo: TipoGafeteEntrada,
    state: tauri::State<GuiState>,
) -> Result<Vec<i64>, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .crear_gafetes_rango(&sesion, desde, hasta, tipo.into())
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

/// A quién se le asignó el gafete perdido, si se sabe. Espejo de
/// `PortadorGafete` sin proveedor (no tiene catálogo de personas).
#[derive(serde::Deserialize, Clone, Copy)]
#[serde(tag = "tipo", content = "id", rename_all = "snake_case")]
pub enum ResponsableGafeteEntrada {
    Contratista(i64),
    Visita(i64),
    ProvisionalKof(i64),
}

impl From<ResponsableGafeteEntrada> for PortadorGafete {
    fn from(responsable: ResponsableGafeteEntrada) -> Self {
        match responsable {
            ResponsableGafeteEntrada::Contratista(id) => Self::Contratista(id),
            ResponsableGafeteEntrada::Visita(id) => Self::Visita(id),
            ResponsableGafeteEntrada::ProvisionalKof(id) => Self::ProvisionalKof(id),
        }
    }
}

#[tauri::command(async)]
pub fn reportar_gafete_perdido(
    id: i64,
    responsable: Option<ResponsableGafeteEntrada>,
    detalle: Option<String>,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .reportar_gafete_perdido(&sesion, id, responsable.map(Into::into), detalle.as_deref())
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command(async)]
pub fn registrar_pago_gafete(
    id: i64,
    detalle: Option<String>,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_pago_gafete(&sesion, id, detalle.as_deref())
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command(async)]
pub fn recuperar_gafete(
    id: i64,
    detalle: String,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .recuperar_gafete(&sesion, id, &detalle)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

/// Conteo físico: qué es un número en el momento de escribirlo.
#[tauri::command(async)]
pub fn leer_numero_conteo_gafetes(
    tipo: TipoGafeteEntrada,
    numero: i64,
    state: tauri::State<GuiState>,
) -> Result<LecturaConteo, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .leer_numero_conteo_gafetes(&sesion, tipo.into(), numero)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

/// Conteo físico: lo escrito contra lo que debería haber en la mesa.
#[tauri::command(async)]
pub fn comprobar_conteo_gafetes(
    tipo: TipoGafeteEntrada,
    numeros: Vec<i64>,
    state: tauri::State<GuiState>,
) -> Result<ResultadoConteo, String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .comprobar_conteo_gafetes(&sesion, tipo.into(), &numeros)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}
