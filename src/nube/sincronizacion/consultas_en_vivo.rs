use super::{ContextoSincronizacion, SincronizacionError, llamar_rpc, obtener_json};
use crate::nube::cliente::{NubeError, cliente_http, cliente_http_verificacion};

#[derive(serde::Deserialize)]
pub(super) struct FilaGafeteOcupado {
    #[allow(dead_code)]
    pub(super) id: String,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaUsuarioActivo {
    pub(super) activo: bool,
}

/// Consulta puntual -- una fila, una columna -- de si `cedula` sigue
/// activa en el catálogo global, sin traer ni tocar nada más. Antes esto
/// se resolvía con una sincronización completa (cola de salida + cierres +
/// ingresos abiertos + catálogo + historial) sólo para confirmar un
/// booleano -- medido como el causante real del retraso perceptible en el
/// login (varios cientos de milisegundos a un par de segundos según el
/// tamaño del sitio), cuando lo único que hace falta acá es esto. `true`
/// si la cédula no existe en el catálogo remoto todavía (usuarios ROOT,
/// que nunca se sincronizan, o un usuario que este dispositivo creó y
/// todavía no llegó a subir) -- no hay nada que decir que esté desactivado
/// si la nube ni siquiera lo conoce.
pub fn usuario_sigue_activo_remoto(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
) -> Result<bool, SincronizacionError> {
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/usuarios?cedula=eq.{cedula}&select=activo&limit=1",
        contexto.base_url,
    );
    let filas: Vec<FilaUsuarioActivo> = obtener_json(&cliente, contexto, &url)?;
    Ok(filas.first().is_none_or(|fila| fila.activo))
}

/// Consulta en vivo -- no una caché local que sólo se refresca en cada sync
/// y podría estar desactualizada por minutos -- si `numero` ya está activo
/// en este sitio del lado de *otro* dispositivo, para cualquier tabla que
/// siga el mismo molde (un gafete numerado + una columna que marca cierre +
/// una columna de dispositivo de apertura). Todas las variantes de
/// "gafete ocupado" (contratista, visita, provisional KOF, proveedor)
/// comparten esta misma regla de negocio -- mismo sitio, no se puede entregar dos veces,
/// hay que vigilar que un dispositivo no dé un gafete que otro ya asignó --
/// así que comparten esta única consulta parametrizada por tabla/columnas
/// en vez de tres copias casi idénticas. Pensada para llamarse justo antes
/// de confirmar la apertura de un ciclo con gafete: cada dispositivo sólo
/// valida contra su propia base `SQLite`, que nunca ve lo que hizo otro
/// hasta sincronizar -- de ahí que dos pudieran aceptar el mismo número
/// como activo a la vez. No reserva nada del lado del receptor: sigue
/// existiendo una ventana muy angosta entre esta consulta y que la
/// apertura realmente se drene a la cola de salida (ver `drenar_cola`),
/// pero cierra el caso normal (no perfectamente simultáneo) que sí se pudo
/// reproducir.
pub(super) fn gafete_ocupado_en_otro_dispositivo_en(
    contexto: &ContextoSincronizacion<'_>,
    tabla: &str,
    columna_dispositivo_apertura: &str,
    columna_cierre: &str,
    numero: i64,
) -> Result<bool, SincronizacionError> {
    let cliente = cliente_http_verificacion();
    let url = format!(
        "{}/rest/v1/{tabla}?sitio_id=eq.{}&{columna_dispositivo_apertura}=neq.{}\
         &{columna_cierre}=is.null&gafete_numero=eq.{numero}&select=id&limit=1",
        contexto.base_url, contexto.sitio_id, contexto.dispositivo_id,
    );
    let filas: Vec<FilaGafeteOcupado> = obtener_json(&cliente, contexto, &url)?;
    Ok(!filas.is_empty())
}

/// Variante contra `ingresos` (gafetes de contratista).
pub fn gafete_ocupado_en_otro_dispositivo(
    contexto: &ContextoSincronizacion<'_>,
    numero: i64,
) -> Result<bool, SincronizacionError> {
    gafete_ocupado_en_otro_dispositivo_en(
        contexto,
        "ingresos",
        "dispositivo_entrada_id",
        "hora_salida",
        numero,
    )
}

/// Variante contra `movimientos_visita` (gafetes de visita).
pub fn gafete_de_visita_ocupado_en_otro_dispositivo(
    contexto: &ContextoSincronizacion<'_>,
    numero: i64,
) -> Result<bool, SincronizacionError> {
    gafete_ocupado_en_otro_dispositivo_en(
        contexto,
        "movimientos_visita",
        "dispositivo_entrada_id",
        "hora_salida",
        numero,
    )
}

/// Variante contra `prestamos_gafete_provisional` (gafetes provisionales
/// KOF) -- ver `docs/features-futuras/plan-gafetes-provisionales-kof.md`.
pub fn gafete_provisional_ocupado_en_otro_dispositivo(
    contexto: &ContextoSincronizacion<'_>,
    numero: i64,
) -> Result<bool, SincronizacionError> {
    gafete_ocupado_en_otro_dispositivo_en(
        contexto,
        "prestamos_gafete_provisional",
        "dispositivo_entrega_id",
        "hora_devolucion",
        numero,
    )
}

/// Variante contra `ingresos_proveedor` (gafetes de proveedor) -- ver
/// `docs/features-futuras/plan-control-proveedores.md`. Mismo vocabulario
/// entrada/salida que `movimientos_visita` (no entrega/devolución, eso es
/// exclusivo de KOF).
pub fn gafete_de_proveedor_ocupado_en_otro_dispositivo(
    contexto: &ContextoSincronizacion<'_>,
    numero: i64,
) -> Result<bool, SincronizacionError> {
    gafete_ocupado_en_otro_dispositivo_en(
        contexto,
        "ingresos_proveedor",
        "dispositivo_entrada_id",
        "hora_salida",
        numero,
    )
}

/// Gafete de visita en uso por un ingreso por correo del otro dispositivo
/// del sitio -- mismo criterio que
/// [`gafete_de_proveedor_ocupado_en_otro_dispositivo`].
pub fn gafete_de_correo_ocupado_en_otro_dispositivo(
    contexto: &ContextoSincronizacion<'_>,
    numero: i64,
) -> Result<bool, SincronizacionError> {
    gafete_ocupado_en_otro_dispositivo_en(
        contexto,
        "ingresos_correo",
        "dispositivo_entrada_id",
        "hora_salida",
        numero,
    )
}

/// Dónde tiene un contratista un ingreso abierto ahora mismo, según la
/// nube (ver [`contratista_con_ingreso_activo`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoActivoEnLaNube {
    /// `true` si es en este mismo sitio (lo abrió el otro dispositivo).
    pub mismo_sitio: bool,
    pub sitio_nombre: String,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaIngresoActivoCualquierSitio {
    pub(super) sitio_id: String,
    pub(super) sitio_nombre: Option<String>,
}

/// Regla "un contratista no puede tener dos ingresos activos, ni en este
/// sitio ni en otro": pregunta a la nube si esta cédula tiene un ingreso
/// abierto en CUALQUIER sitio. Quien llama propaga el error: si no se puede
/// verificar, no se registra (decisión del dueño, igual que el gafete).
/// Lo abierto por ESTE equipo ya lo frena antes el chequeo local.
///
/// Va por la función `ingreso_activo_de_contratista` y no por
/// `/rest/v1/ingresos`: la RLS sólo le deja leer a cada equipo los ingresos
/// de su sitio, así que la consulta directa nunca veía los de otra unidad y
/// siempre respondía "libre" (visto en staging el 2026-10-03). La función
/// responde sólo si está adentro y dónde, sin abrir el resto.
pub fn contratista_con_ingreso_activo(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
) -> Result<Option<IngresoActivoEnLaNube>, SincronizacionError> {
    activo_segun_funcion(
        contexto,
        "ingreso_activo_de_contratista",
        &serde_json::json!({ "p_cedula": cedula }),
    )
}

/// Un mismo visitante no puede estar adentro dos veces: pregunta a la nube
/// si esta cédula tiene una visita abierta en otra unidad, o en esta pero
/// abierta por el otro equipo, y devuelve el nombre de la unidad (con
/// " (otro equipo de esta unidad)" en el segundo caso). Pensada para
/// `verificar_check_in_visita` (escritorio). Lo abierto por ESTE equipo ya
/// lo frena antes el chequeo local (`CitaServiceError::VisitanteYaEnSitio`),
/// así que una fila de esta unidad que devuelva la nube es del otro equipo.
///
/// Va por la función `visita_activa_de_visitante` y no por
/// `/rest/v1/movimientos_visita`: la RLS sólo deja leer a cada equipo las
/// visitas de su unidad, así que la consulta directa con `sitio_id=neq.`
/// siempre respondía vacío (revisión del 2026-10-04, punto 4).
pub fn visitante_activo_en_otro_sitio(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
) -> Result<Option<String>, SincronizacionError> {
    Ok(activo_segun_funcion(
        contexto,
        "visita_activa_de_visitante",
        &serde_json::json!({ "p_cedula": cedula }),
    )?
    .map(|activo| {
        if activo.mismo_sitio {
            format!("{} (otro equipo de esta unidad)", activo.sitio_nombre)
        } else {
            activo.sitio_nombre
        }
    }))
}

/// Regla "un proveedor no puede estar adentro dos veces, ni en este sitio ni
/// en otro": pregunta a la nube si esta cédula tiene un ingreso de proveedor
/// abierto en CUALQUIER sitio. Quien llama decide qué hacer con el error:
/// sin conexión se registra con las reglas locales y un rechazo de la nube
/// frena (ver `application::con_nube`).
///
/// Va por la función `ingreso_proveedor_activo` y no por
/// `/rest/v1/ingresos_proveedor`: la RLS sólo deja leer a cada equipo lo de
/// su sitio, así que la consulta directa nunca veía otra unidad.
pub fn proveedor_con_ingreso_activo(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
) -> Result<Option<IngresoActivoEnLaNube>, SincronizacionError> {
    activo_segun_funcion(
        contexto,
        "ingreso_proveedor_activo",
        &serde_json::json!({ "p_cedula": cedula }),
    )
}

/// Misma regla que [`proveedor_con_ingreso_activo`], para el ingreso por
/// correo (función `ingreso_correo_activo`).
pub fn correo_con_ingreso_activo(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
) -> Result<Option<IngresoActivoEnLaNube>, SincronizacionError> {
    activo_segun_funcion(
        contexto,
        "ingreso_correo_activo",
        &serde_json::json!({ "p_cedula": cedula }),
    )
}

/// Regla "un encargado no puede tener dos gafetes provisionales KOF a la
/// vez, en ningún sitio": pregunta a la nube si el encargado con este código
/// de empleado tiene un préstamo sin devolver (función
/// `prestamo_provisional_activo_de_encargado`). El código de empleado, y no
/// el id, es lo que identifica al encargado entre equipos.
pub fn encargado_con_prestamo_provisional_activo(
    contexto: &ContextoSincronizacion<'_>,
    codigo_empleado: &str,
) -> Result<Option<IngresoActivoEnLaNube>, SincronizacionError> {
    activo_segun_funcion(
        contexto,
        "prestamo_provisional_activo_de_encargado",
        &serde_json::json!({ "p_codigo_empleado": codigo_empleado }),
    )
}

/// Común a las funciones `... returns table (sitio_id, sitio_nombre)` que
/// responden dónde está activa una persona (a lo sumo una fila).
fn activo_segun_funcion(
    contexto: &ContextoSincronizacion<'_>,
    funcion: &str,
    cuerpo: &serde_json::Value,
) -> Result<Option<IngresoActivoEnLaNube>, SincronizacionError> {
    let cliente = cliente_http_verificacion();
    let filas: Vec<FilaIngresoActivoCualquierSitio> =
        llamar_rpc(&cliente, contexto, funcion, cuerpo)?;
    Ok(filas
        .into_iter()
        .next()
        .map(|fila| activo_desde_fila(contexto, fila)))
}

fn activo_desde_fila(
    contexto: &ContextoSincronizacion<'_>,
    fila: FilaIngresoActivoCualquierSitio,
) -> IngresoActivoEnLaNube {
    IngresoActivoEnLaNube {
        mismo_sitio: fila.sitio_id == contexto.sitio_id,
        sitio_nombre: fila
            .sitio_nombre
            .unwrap_or_else(|| "otro sitio".to_string()),
    }
}

/// Por qué otra vía (y dónde) está adentro una persona, según la nube (ver
/// [`persona_adentro_por_otra_via`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdentroPorOtraViaEnLaNube {
    pub via: crate::models::via_ingreso::ViaIngreso,
    pub sitio_nombre: String,
    pub mismo_sitio: bool,
}

#[derive(serde::Deserialize)]
struct FilaAdentroPorOtraVia {
    via: String,
    sitio_id: String,
    sitio_nombre: Option<String>,
}

/// Regla "una persona no puede estar adentro por dos vías a la vez"
/// (contratista, proveedor, por correo): pregunta a la nube si esta cédula
/// está adentro por una vía distinta de `via`, en cualquier unidad o equipo
/// (función `persona_adentro_por_otra_via`, que compara la cédula
/// normalizada). Quien llama decide qué hacer con el error, con el mismo
/// criterio que la regla de la misma vía (ver `application::con_nube`).
pub fn persona_adentro_por_otra_via(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
    via: crate::models::via_ingreso::ViaIngreso,
) -> Result<Option<AdentroPorOtraViaEnLaNube>, SincronizacionError> {
    use crate::models::via_ingreso::ViaIngreso;

    let via_texto = match via {
        ViaIngreso::Contratista => "CONTRATISTA",
        ViaIngreso::Proveedor => "PROVEEDOR",
        ViaIngreso::PorCorreo => "POR_CORREO",
    };
    let filas: Vec<FilaAdentroPorOtraVia> = llamar_rpc(
        &cliente_http_verificacion(),
        contexto,
        "persona_adentro_por_otra_via",
        &serde_json::json!({ "p_cedula": cedula, "p_via": via_texto }),
    )?;
    Ok(filas
        .into_iter()
        .find_map(|fila| otra_via_desde_fila(contexto, fila)))
}

fn otra_via_desde_fila(
    contexto: &ContextoSincronizacion<'_>,
    fila: FilaAdentroPorOtraVia,
) -> Option<AdentroPorOtraViaEnLaNube> {
    use crate::models::via_ingreso::ViaIngreso;

    let via = match fila.via.as_str() {
        "CONTRATISTA" => ViaIngreso::Contratista,
        "PROVEEDOR" => ViaIngreso::Proveedor,
        "POR_CORREO" => ViaIngreso::PorCorreo,
        _ => return None,
    };
    Some(AdentroPorOtraViaEnLaNube {
        via,
        mismo_sitio: fila.sitio_id == contexto.sitio_id,
        sitio_nombre: fila
            .sitio_nombre
            .unwrap_or_else(|| "otro sitio".to_string()),
    })
}

/// Qué entrada se verifica en [`verificar_entrada`].
#[derive(Debug, Clone, Copy)]
pub enum EntradaAVerificar {
    Contratista,
    Proveedor,
    PorCorreo,
    /// Préstamo de gafete provisional: la "cédula" es el código de empleado
    /// del encargado, y no hay "otra vía".
    Kof,
}

/// Las tres respuestas de la verificación final de una entrada (ver
/// [`verificar_entrada`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificacionEntradaEnLaNube {
    pub misma_via: Option<IngresoActivoEnLaNube>,
    pub otra_via: Option<AdentroPorOtraViaEnLaNube>,
    pub gafete_ocupado: bool,
}

#[derive(serde::Deserialize)]
struct FilaVerificacionEntrada {
    misma_via: Option<FilaIngresoActivoCualquierSitio>,
    otra_via: Option<FilaAdentroPorOtraVia>,
    gafete_ocupado: bool,
}

/// La verificación final de una entrada en un solo llamado (función
/// `verificar_entrada`): adentro por la misma vía en cualquier unidad
/// ([`contratista_con_ingreso_activo`] y sus pares), adentro por otra vía
/// ([`persona_adentro_por_otra_via`]) y gafete en uso por el otro equipo
/// ([`gafete_ocupado_en_otro_dispositivo`] y sus pares). La función de la nube
/// llama a esas mismas reglas. `None` si la nube todavía no la tiene (404):
/// quien llama hace las consultas sueltas, como antes.
pub fn verificar_entrada(
    contexto: &ContextoSincronizacion<'_>,
    entrada: EntradaAVerificar,
    cedula: &str,
    gafete: Option<i64>,
) -> Result<Option<VerificacionEntradaEnLaNube>, SincronizacionError> {
    let via = match entrada {
        EntradaAVerificar::Contratista => "CONTRATISTA",
        EntradaAVerificar::Proveedor => "PROVEEDOR",
        EntradaAVerificar::PorCorreo => "POR_CORREO",
        EntradaAVerificar::Kof => "KOF",
    };
    let respuesta = cliente_http_verificacion()
        .post(format!(
            "{}/rest/v1/rpc/verificar_entrada",
            contexto.base_url
        ))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .json(&serde_json::json!({ "p_via": via, "p_cedula": cedula, "p_gafete": gafete }))
        .send()
        .map_err(NubeError::Red)?;
    let status = respuesta.status().as_u16();
    if status == 404 {
        return Ok(None);
    }
    if !respuesta.status().is_success() {
        let cuerpo = respuesta.text().unwrap_or_default();
        return Err(SincronizacionError::RespuestaInesperada { status, cuerpo });
    }
    let fila: FilaVerificacionEntrada = respuesta.json().map_err(NubeError::Red)?;
    Ok(Some(VerificacionEntradaEnLaNube {
        misma_via: fila.misma_via.map(|f| activo_desde_fila(contexto, f)),
        otra_via: fila.otra_via.and_then(|f| otra_via_desde_fila(contexto, f)),
        gafete_ocupado: fila.gafete_ocupado,
    }))
}
