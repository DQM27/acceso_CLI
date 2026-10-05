use chrono::NaiveDate;
use control_acceso::application::registrar_entrada_visita_verificada;
use control_acceso::mensajes::{mensaje_cita, mensaje_entrada_visita_verificada};
use control_acceso::models::cita::{Cita, CitaVisitante, EstadoCita};
use control_acceso::models::movimiento_visita::MovimientoVisitaActivoResumen;
use control_acceso::nube;
use control_acceso::services::error::CitaServiceError;
use rusqlite::params;
use tauri::Manager;

use crate::comandos::historial::rango_utc;
use crate::estado::GuiState;

/// Mismo tope y mismo criterio de mejor esfuerzo que
/// `ESPERA_MAXIMA_CHEQUEO_OTRO_SITIO` en `comandos/ingresos.rs` -- duplicada
/// a propósito, no generalizada (mismo motivo que el resto de este archivo
/// duplica en vez de compartir con el dominio de contratistas).
const ESPERA_MAXIMA_CHEQUEO_OTRO_SITIO: std::time::Duration = std::time::Duration::from_secs(5);

/// Espejo de `comandos::ingresos::chequear_activo_en_otro_sitio`, pero para
/// visitas -- misma idea: un visitante no puede estar activo en dos sitios
/// a la vez, mismo criterio que un contratista.
fn chequear_visitante_activo_en_otro_sitio(state: &GuiState, cedula: &str) -> Option<String> {
    if !state.nube_vinculada() {
        return None;
    }
    let token = state.autenticar_con_cache().ok()?;
    state.core().aplicar_token(&token);
    let contexto = nube::ContextoSincronizacion {
        base_url: nube::base_url(),
        apikey: nube::apikey(),
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    nube::visitante_activo_en_otro_sitio(&contexto, cedula)
        .ok()
        .flatten()
}

/// DTO de presentación -- `AppCore::verificar_check_in_visita` devuelve una
/// tupla `(Cita, CitaVisitante)`; acá se nombra para que el lado TypeScript
/// tenga campos, no un array posicional.
#[derive(serde::Serialize)]
pub struct PreparacionVisita {
    pub cita: Cita,
    pub visitante: CitaVisitante,
    /// Ver `chequear_visitante_activo_en_otro_sitio` -- mismo criterio que
    /// `PreparacionIngreso::activo_en_otro_sitio` (contratistas): mejor
    /// esfuerzo, `None` también cuando no hubo forma de verificar, no sólo
    /// cuando de verdad no hay conflicto.
    pub activo_en_otro_sitio: Option<String>,
}

/// Por qué no sigue un check-in. `informativo`: la visita existe pero es
/// para otro día (`CitaServiceError::es_informativo`), la pantalla lo
/// muestra como aviso y no como error. `alternativa_por_correo`: no hay
/// cita que valga hoy y nada lo impide, la pantalla ofrece registrarla como
/// autorizada por correo (`CitaServiceError::admite_registro_por_correo`).
#[derive(Debug, serde::Serialize)]
pub struct RechazoVisita {
    mensaje: String,
    informativo: bool,
    alternativa_por_correo: bool,
}

impl From<String> for RechazoVisita {
    fn from(mensaje: String) -> Self {
        Self {
            mensaje,
            informativo: false,
            alternativa_por_correo: false,
        }
    }
}

impl From<CitaServiceError> for RechazoVisita {
    fn from(error: CitaServiceError) -> Self {
        let informativo = error.es_informativo();
        let alternativa_por_correo = error.admite_registro_por_correo();
        Self {
            mensaje: mensaje_cita(error),
            informativo,
            alternativa_por_correo,
        }
    }
}

/// Async por el mismo motivo que `preparar_ingreso`
/// (`comandos/ingresos.rs`): el chequeo local (`AppCore::verificar_check_in_visita`)
/// es instantáneo y siempre corre; el remoto (`activo_en_otro_sitio`, mejor
/// esfuerzo) es el único que necesita `tokio`/tope de tiempo.
#[tauri::command]
pub async fn verificar_check_in_visita(
    cedula: String,
    app: tauri::AppHandle,
) -> Result<PreparacionVisita, RechazoVisita> {
    let state = app.state::<GuiState>();
    state.sesion_activa()?;
    let (cita, visitante) = state.core().verificar_check_in_visita(&cedula)?;

    let visitante_cedula = visitante.cedula.clone();
    let manejador = app.clone();
    let chequeo = tokio::time::timeout(
        ESPERA_MAXIMA_CHEQUEO_OTRO_SITIO,
        tauri::async_runtime::spawn_blocking(move || {
            chequear_visitante_activo_en_otro_sitio(
                &manejador.state::<GuiState>(),
                &visitante_cedula,
            )
        }),
    )
    .await;
    let activo_en_otro_sitio = chequeo.ok().and_then(Result::ok).flatten();

    Ok(PreparacionVisita {
        cita,
        visitante,
        activo_en_otro_sitio,
    })
}

/// Las reglas (gafete libre en el otro equipo, visitante adentro en otra
/// unidad, la cita misma) viven en
/// `application::registrar_entrada_visita_verificada`, la misma que usa el
/// teléfono.
#[tauri::command]
pub fn registrar_entrada_visita(
    cedula: String,
    gafete: Option<i64>,
    // Medio de ingreso: `None` = caminando, la placa = vehículo.
    placa: Option<String>,
    state: tauri::State<GuiState>,
) -> Result<i64, String> {
    let sesion = state.sesion_activa()?;
    registrar_entrada_visita_verificada(
        || state.core(),
        state.nube_del_dispositivo(),
        &sesion,
        &cedula,
        gafete,
        placa,
    )
    .map_err(mensaje_entrada_visita_verificada)
}

#[tauri::command]
pub fn registrar_salida_visita(
    movimiento_id: i64,
    state: tauri::State<GuiState>,
) -> Result<(), String> {
    let sesion = state.sesion_activa()?;
    state
        .core()
        .registrar_salida_visita(&sesion, movimiento_id)
        .map_err(mensaje_cita)
}

#[tauri::command]
pub fn listar_visitas_activas(
    state: tauri::State<GuiState>,
) -> Result<Vec<MovimientoVisitaActivoResumen>, String> {
    state.sesion_activa()?;
    // Sin `mensaje_*` propio a propósito: `AppCore::listar_visitas_activas`
    // devuelve `DatabaseError` directo (es una lectura simple, no pasa por
    // un `*ServiceError` de negocio) -- no exponer su `Display` (interpola
    // el error crudo de `SQLite`), mismo criterio que el resto de mensajes
    // de este módulo.
    state
        .core()
        .listar_visitas_activas()
        .map_err(|_| "No se pudo cargar la lista de visitas activas".to_string())
}

/// Espejo de `historial_visitas_sitio` -- análogo a
/// `comandos::historial::listar_historial_sitio` (contratistas), mismo
/// criterio: sin límite ni filtro de texto, la grilla (AG Grid) filtra del
/// lado del cliente. Sólo tiene sentido en PC -- el celular nunca sincroniza
/// esta caché (`mobile/rust-core/src/lib.rs`), así que ahí siempre estaría
/// vacía; este comando no existe del lado móvil.
#[derive(serde::Serialize)]
pub struct MovimientoHistorialVisitaRemoto {
    pub uuid: String,
    pub cedula: String,
    pub nombre: String,
    pub empresa: Option<String>,
    pub anfitrion_nombre: Option<String>,
    pub motivo: Option<String>,
    pub gafete_numero: Option<i64>,
    /// Medio de ingreso: NULL = caminando (o entrada anterior a anotarlo).
    pub placa: Option<String>,
    pub fecha_hora_entrada: String,
    pub fecha_hora_salida: Option<String>,
    pub usuario_entrada_nombre: Option<String>,
    pub usuario_salida_nombre: Option<String>,
}

#[tauri::command]
pub fn listar_historial_visitas_sitio(
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    state: tauri::State<GuiState>,
) -> Result<Vec<MovimientoHistorialVisitaRemoto>, String> {
    state.sesion_activa()?;
    let (desde_utc, hasta_utc) = rango_utc(desde, hasta).map_err(super::mensaje_generico)?;
    let conexion = state.conexion_secundaria()?;
    let mut statement = conexion
        .prepare(
            "SELECT uuid, visitante_cedula, visitante_nombre, empresa, anfitrion_nombre,
                    motivo, gafete_numero, hora_entrada, hora_salida,
                    usuario_entrada_nombre, usuario_salida_nombre, placa
             FROM historial_visitas_sitio
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
                Ok(MovimientoHistorialVisitaRemoto {
                    uuid: row.get(0)?,
                    cedula: row.get(1)?,
                    nombre: row.get(2)?,
                    empresa: row.get(3)?,
                    anfitrion_nombre: row.get(4)?,
                    motivo: row.get(5)?,
                    gafete_numero: row.get(6)?,
                    fecha_hora_entrada: row.get(7)?,
                    fecha_hora_salida: row.get(8)?,
                    usuario_entrada_nombre: row.get(9)?,
                    usuario_salida_nombre: row.get(10)?,
                    placa: row.get(11)?,
                })
            },
        )
        .map_err(super::mensaje_generico)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(super::mensaje_generico)
}

/// Agenda de visitas programadas -- lectura pura de `citas`/`cita_visitantes`
/// (ya sincronizadas por `nube::recibir_citas_del_sitio`, sin consulta
/// adicional a la nube). Trae toda cita cuya vigencia no haya terminado
/// todavía (`fecha_hasta >= hoy`), vigente o cancelada -- se muestra el
/// estado en vez de ocultar las canceladas, mismo criterio que el filtro
/// por columna del resto de las grillas: dejar que quien mira decida qué
/// ver, no decidirlo de antemano acá.
#[derive(serde::Serialize)]
pub struct AgendaVisitaResumen {
    pub cita_id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa: Option<String>,
    pub placa_vehiculo: Option<String>,
    pub motivo: Option<String>,
    pub anfitrion_nombre: String,
    pub fecha_desde: String,
    pub fecha_hasta: String,
    /// Texto libre tipo "HH:MM", puramente informativo -- ver el
    /// doc-comment de `MIGRACION_33` del núcleo.
    pub hora_estimada: Option<String>,
    pub estado: EstadoCita,
}

#[tauri::command]
pub fn listar_agenda_visitas(
    state: tauri::State<GuiState>,
) -> Result<Vec<AgendaVisitaResumen>, String> {
    state.sesion_activa()?;
    // Sin `mensaje_*` propio, mismo criterio que `listar_visitas_activas`:
    // `AppCore::listar_agenda_visitas` devuelve `DatabaseError` directo, no
    // un `*ServiceError` de negocio -- no exponer su `Display` (interpola el
    // error crudo de `SQLite`).
    let filas = state
        .core()
        .listar_agenda_visitas()
        .map_err(|_| "No se pudo cargar la agenda de visitas".to_string())?;
    Ok(filas
        .into_iter()
        .map(|(cita, visitante)| AgendaVisitaResumen {
            cita_id: cita.id,
            cedula: visitante.cedula,
            nombre: visitante.nombre,
            empresa: visitante.empresa,
            placa_vehiculo: visitante.placa_vehiculo,
            motivo: cita.motivo,
            anfitrion_nombre: cita.anfitrion_nombre,
            fecha_desde: cita.fecha_desde.to_string(),
            fecha_hasta: cita.fecha_hasta.to_string(),
            hora_estimada: cita.hora_estimada,
            estado: cita.estado,
        })
        .collect())
}
