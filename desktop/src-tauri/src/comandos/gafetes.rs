use control_acceso::database::queries::gafetes::GafeteResumen;
use control_acceso::database::queries::gafetes_incidentes::IncidenteGafete;
use control_acceso::models::gafete::MotivoResolucionGafete;

use crate::dto::gafetes::{FiltroGafetesEntrada, TipoGafeteEntrada};
use crate::estado::GuiState;

/// Catálogo completo de gafetes, sin restricción de rol a propósito (mismo
/// criterio que el núcleo — cualquier operador con sesión gestiona
/// alta/baja/perdido/resolver, a diferencia de Empresas/Usuarios).
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

#[tauri::command(async)]
pub fn dar_de_baja_gafete(id: i64, state: tauri::State<GuiState>) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .dar_de_baja_gafete(&sesion, id)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command(async)]
pub fn marcar_gafete_perdido_contratista(
    id: i64,
    contratista_id: i64,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .marcar_gafete_perdido_contratista(&sesion, id, contratista_id)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command(async)]
pub fn marcar_gafete_perdido_visita(
    id: i64,
    cita_visitante_id: i64,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .marcar_gafete_perdido_visita(&sesion, id, cita_visitante_id)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command(async)]
pub fn marcar_gafete_perdido_provisional_kof(
    id: i64,
    encargado_ruta_id: i64,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .marcar_gafete_perdido_provisional_kof(&sesion, id, encargado_ruta_id)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}

#[tauri::command(async)]
pub fn resolver_gafete(
    id: i64,
    motivo: MotivoResolucionGafete,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .resolver_gafete(&sesion, id, motivo)
        .map_err(control_acceso::mensajes::mensaje_gafete)
}
