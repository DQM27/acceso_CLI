use chrono::NaiveDate;
use control_acceso::mensajes::{
    mensaje_empresa_proveedor, mensaje_gestion_nube, mensaje_ingreso_proveedor,
    mensaje_sincronizacion,
};
use control_acceso::models::empresa_proveedor::EmpresaProveedor;
use control_acceso::models::registro_ingreso_proveedor::RegistroIngresoProveedorActivoResumen;
use control_acceso::nube;
use rusqlite::params;

use crate::comandos::historial::rango_utc;
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
        base_url: nube::base_url(),
        apikey: nube::apikey(),
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    let ocupado = nube::gafete_de_proveedor_ocupado_en_otro_dispositivo(&contexto, numero)
        .map_err(mensaje_sincronizacion)?;
    Ok(!ocupado)
}

/// Chequeo cruzado entre sitios -- misma cédula no puede estar activa
/// físicamente en dos sitios a la vez. Mismo criterio y misma forma que
/// `comandos::ingresos::chequear_activo_en_otro_sitio` (contratista):
/// best-effort de verdad -- sin secreto guardado, o si la consulta falla
/// por cualquier motivo (sin red, timeout, receptor caído), `None` y no
/// bloquea nada; el registro sigue local y el conflicto, si existe, se
/// detecta después al sincronizar (`nube::proveedores_con_conflicto_activo`).
/// A diferencia del chequeo de gafete de arriba, acá un fallo de red NUNCA
/// se propaga como error -- es la diferencia deliberada entre "un recurso
/// físico compartido no puede duplicarse" (gafete, si falla la consulta
/// mejor frenar) y "esta alerta es una ayuda, no motivo para trabar a
/// alguien parado en la puerta sin señal".
fn proveedor_activo_en_otro_sitio(state: &GuiState, cedula: &str) -> Option<String> {
    let secreto = nube::credenciales::cargar_secreto()?;
    let token = state.autenticar_con_cache(&secreto).ok()?;
    if let Some(desfase_ms) = token.desfase_reloj_ms {
        state.core().actualizar_desfase_reloj(desfase_ms);
    }
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::base_url(),
        apikey: nube::apikey(),
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    nube::proveedor_activo_en_otro_sitio(&contexto, cedula)
        .ok()
        .flatten()
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
    if let Some(sitio) = proveedor_activo_en_otro_sitio(&state, &datos.cedula) {
        return Err(format!(
            "Esta cédula ya tiene un ingreso de proveedor activo en {sitio}"
        ));
    }
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
