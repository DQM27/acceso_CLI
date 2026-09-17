use control_acceso::mensajes::{mensaje_gafete_provisional, mensaje_gestion_nube, mensaje_nube};
use control_acceso::models::encargado_ruta::EncargadoRuta;
use control_acceso::models::prestamo_gafete_provisional::PrestamoGafeteProvisionalActivoResumen;
use control_acceso::nube;

use crate::estado::GuiState;

/// Mismo criterio y misma forma que
/// `comandos::proveedores::gafete_proveedor_libre_en_otro_dispositivo`: sin
/// secreto guardado no hay con quién chocar (`Ok(true)`, libre); con nube
/// configurada exige estar en línea -- si la consulta falla, el error se
/// propaga en vez de asumir que el gafete está libre. Nunca usa
/// `state.core()` para la parte de red, mismo motivo de siempre (no
/// retener el candado del núcleo durante la llamada HTTP).
fn gafete_provisional_libre_en_otro_dispositivo(
    state: &GuiState,
    numero: i64,
) -> Result<bool, String> {
    let Some(secreto) = nube::credenciales::cargar_secreto() else {
        return Ok(true);
    };
    let actor = state.sesion_activa()?;
    state
        .core()
        .autorizar_uso_nube(&actor)
        .map_err(mensaje_gestion_nube)?;

    let token = state.autenticar_con_cache(&secreto).map_err(mensaje_nube)?;
    if let Some(desfase_ms) = token.desfase_reloj_ms {
        state.core().actualizar_desfase_reloj(desfase_ms);
    }
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::BASE_URL,
        apikey: nube::APIKEY,
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    let ocupado = nube::gafete_provisional_ocupado_en_otro_dispositivo(&contexto, numero)
        .map_err(control_acceso::mensajes::mensaje_sincronizacion)?;
    Ok(!ocupado)
}

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
        .buscar_encargados_ruta(&texto)
        .map_err(|_| "No se pudo buscar en el catálogo de encargados".to_string())
}

#[tauri::command]
pub fn entregar_gafete_provisional(
    encargado_id: i64,
    gafete_numero: i64,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    if !gafete_provisional_libre_en_otro_dispositivo(&state, gafete_numero)? {
        return Err(format!(
            "El gafete {gafete_numero} ya está prestado en otro dispositivo del sitio"
        ));
    }
    state
        .core()
        .entregar_gafete_provisional(&sesion, encargado_id, gafete_numero)
        .map_err(mensaje_gafete_provisional)
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
