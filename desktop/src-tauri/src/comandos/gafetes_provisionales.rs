use control_acceso::mensajes::{mensaje_gafete_provisional, mensaje_gestion_nube, mensaje_nube};
use control_acceso::models::encargado_ruta::EncargadoRuta;
use control_acceso::models::prestamo_gafete_provisional::PrestamoGafeteProvisionalActivoResumen;
use control_acceso::nube;
use rusqlite::params;

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
        base_url: nube::base_url(),
        apikey: nube::apikey(),
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
        // Es un selector para prestar un gafete -- un encargado desactivado
        // no es una opción válida (mismo criterio que
        // `EncargadoRutaService::buscar_seleccionables`).
        .buscar_encargados_ruta_seleccionables(&texto)
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

/// Espejo de `prestamos_gafete_provisional_historial_sitio` (ver
/// `database::schema`, migración 50) -- préstamos del sitio generados por
/// CUALQUIER dispositivo, incluido éste. Mismo criterio que
/// `listar_historial_ingresos_proveedor_sitio` (`comandos::proveedores`):
/// esta caché ya incluye lo que ESTE dispositivo entregó (vuelve sincronizada
/// desde Supabase), así que la pantalla la usa como única fuente para
/// "Historial" -- sin fusionar con la tabla local, mismo criterio que
/// Proveedores/Visitas (volumen bajo, no amerita esa complejidad extra).
#[derive(serde::Serialize)]
pub struct PrestamoGafeteProvisionalHistorialSitio {
    pub uuid: String,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: String,
    pub gafete_numero: i64,
    pub fecha_hora_entrega: String,
    pub usuario_entrega_nombre: String,
    pub fecha_hora_devolucion: Option<String>,
    pub usuario_devolucion_nombre: Option<String>,
}

#[tauri::command]
pub fn listar_gafetes_provisionales_historial_sitio(
    state: tauri::State<GuiState>,
) -> Result<Vec<PrestamoGafeteProvisionalHistorialSitio>, String> {
    state.sesion_activa()?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, encargado_nombre, encargado_codigo_empleado, gafete_numero,
                    fecha_hora_entrega, usuario_entrega_nombre, fecha_hora_devolucion,
                    usuario_devolucion_nombre
             FROM prestamos_gafete_provisional_historial_sitio
             ORDER BY fecha_hora_entrega DESC",
        )
        .map_err(super::mensaje_generico)?;
    let filas = statement
        .query_map(params![], |row| {
            Ok(PrestamoGafeteProvisionalHistorialSitio {
                uuid: row.get(0)?,
                encargado_nombre: row.get(1)?,
                encargado_codigo_empleado: row.get(2)?,
                gafete_numero: row.get(3)?,
                fecha_hora_entrega: row.get(4)?,
                usuario_entrega_nombre: row.get(5)?,
                fecha_hora_devolucion: row.get(6)?,
                usuario_devolucion_nombre: row.get(7)?,
            })
        })
        .map_err(super::mensaje_generico)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(super::mensaje_generico)?;
    Ok(filas)
}
