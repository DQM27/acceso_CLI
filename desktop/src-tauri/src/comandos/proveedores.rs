use control_acceso::mensajes::{
    mensaje_empresa_proveedor, mensaje_gestion_nube, mensaje_ingreso_proveedor,
    mensaje_sincronizacion,
};
use control_acceso::models::empresa_proveedor::EmpresaProveedor;
use control_acceso::models::registro_ingreso_proveedor::RegistroIngresoProveedorActivoResumen;
use control_acceso::nube;

use crate::dto::proveedores::SolicitudIngresoProveedorEntrada;
use crate::estado::GuiState;

/// Chequeo en vivo (no la caché `ingresos_proveedor_remotos`) contra
/// Supabase de si `numero` ya está activo en este sitio del lado de OTRO
/// dispositivo -- mismo criterio y misma forma que
/// `comandos::ingresos::gafete_libre_en_otro_dispositivo` (contratista):
/// sin secreto guardado no hay con quién chocar, `Ok(true)` ("libre")
/// directo sin tocar la red. Con nube configurada, en cambio, exige estar
/// en línea -- si la consulta falla, el error se propaga en vez de asumir
/// que el gafete está libre. Nunca usa `state.core()` para la parte de red:
/// llamar `AppCore::gafete_de_proveedor_ocupado_en_sitio` directo desde acá
/// retendría el candado compartido durante toda la llamada HTTP, colgando
/// cualquier otro comando que necesite el núcleo mientras tanto -- mismo
/// motivo por el que `gafete_libre_en_otro_dispositivo` tampoco lo usa.
fn gafete_proveedor_libre_en_otro_dispositivo(
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

    let token = state
        .autenticar_con_cache(&secreto)
        .map_err(control_acceso::mensajes::mensaje_nube)?;
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
    let ocupado = nube::gafete_de_proveedor_ocupado_en_otro_dispositivo(&contexto, numero)
        .map_err(mensaje_sincronizacion)?;
    Ok(!ocupado)
}

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
    if !gafete_proveedor_libre_en_otro_dispositivo(&state, datos.gafete_numero)? {
        return Err(format!(
            "El gafete {} ya está en uso en otro dispositivo del sitio",
            datos.gafete_numero
        ));
    }
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
