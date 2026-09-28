use rusqlite::{Connection, OptionalExtension, params};

use super::{
    ContextoSincronizacion, SincronizacionError, obtener_json, obtener_json_paginado_con,
    traslape_historial,
};
use crate::nube::cliente::cliente_http;

#[derive(serde::Deserialize)]
pub(super) struct DispositivoEmbebido {
    pub(super) tipo: Option<String>,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaHistorialRemota {
    pub(super) id: String,
    pub(super) contratista_cedula: Option<String>,
    pub(super) contratista_nombre: String,
    pub(super) empresa_nombre: Option<String>,
    pub(super) tipo_ingreso: Option<String>,
    pub(super) medio_ingreso: Option<String>,
    pub(super) hora_entrada: String,
    pub(super) hora_salida: Option<String>,
    pub(super) gafete_numero: Option<i64>,
    pub(super) usuario_entrada_nombre: Option<String>,
    pub(super) usuario_salida_nombre: Option<String>,
    pub(super) resultado_acceso: Option<String>,
    pub(super) motivo_resultado: Option<String>,
    pub(super) reglas_version: Option<i64>,
    pub(super) empresa_activa_snapshot: Option<bool>,
    pub(super) dispositivo_entrada_id: String,
    pub(super) dispositivo_salida_id: Option<String>,
    pub(super) updated_at: String,
    pub(super) placa: Option<String>,
    /// `"pc"`/`"mobile"` (`dispositivos.tipo`) -- embebido vía `PostgREST`
    /// (`dispositivo_entrada:dispositivos!ingresos_dispositivo_entrada_id_fkey(tipo)`)
    /// para que la pantalla pueda mostrar de qué tipo de dispositivo vino
    /// un movimiento sin tener que resolver el UUID a mano. Pedido del
    /// usuario tras no poder diferenciar de un vistazo un movimiento de la
    /// PC de uno del celular en Historial.
    pub(super) dispositivo_entrada: Option<DispositivoEmbebido>,
}

/// Trae a `historial_sitio` todo movimiento (abierto o cerrado) del sitio,
/// de cualquier dispositivo -- decisión explícita del usuario: "es la
/// misma operación vista desde dos dispositivos distintos", no un espejo
/// resumido. Sync incremental, mismo mecanismo que
/// `recibir_catalogo_del_sitio` (`historial_actualizado_hasta` en vez de
/// `catalogo_actualizado_hasta` -- ritmos de sync independientes). `ON
/// CONFLICT` actualiza en vez de insertar de nuevo: un movimiento que
/// nace abierto y se cierra después reaparece con `updated_at` más nuevo,
/// trayendo ya el cierre. `reconciliar`: al abrir la app, compara los
/// últimos 7 días fila por fila y trae sólo lo distinto (ver
/// `filtros_de_historial`); si no, lo cambiado desde la marca.
pub fn recibir_historial_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    reconciliar: bool,
) -> Result<u32, SincronizacionError> {
    let cliente = cliente_http();

    let marca_anterior: Option<String> = connection.query_row(
        "SELECT historial_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let filtros = filtros_de_historial(
        connection,
        contexto,
        "ingresos",
        "historial_sitio",
        marca_anterior.as_deref(),
        reconciliar,
    )?;

    // Página por página en vez de acumular todo el historial remoto en un
    // `Vec` antes de tocar la base -- ver el doc-comment de
    // `obtener_json_paginado_con` (hallazgo R-03). `marca_mas_nueva` viaja
    // de página en página: el orden entre páginas es por `id`, no por
    // `updated_at`, así que la marca final tiene que ser el máximo visto en
    // todas, no sólo en la última. No se excluye el dispositivo actual:
    // tras reinstalar Android, la base local pierde `registro_ingresos`,
    // pero la nube sigue siendo la fuente común del sitio.
    let mut recibidos_total = 0_u32;
    let mut marca_mas_nueva = marca_inicial(marca_anterior.as_deref());
    for filtro in filtros {
        let url = format!(
            "{}/rest/v1/ingresos?sitio_id=eq.{}{filtro}\
             &select=id,contratista_cedula,contratista_nombre,empresa_nombre,tipo_ingreso,\
             medio_ingreso,hora_entrada,hora_salida,gafete_numero,usuario_entrada_nombre,\
             usuario_salida_nombre,resultado_acceso,motivo_resultado,reglas_version,\
             empresa_activa_snapshot,dispositivo_entrada_id,dispositivo_salida_id,updated_at,placa,\
             dispositivo_entrada:dispositivos!ingresos_dispositivo_entrada_id_fkey(tipo)",
            contexto.base_url, contexto.sitio_id,
        );
        obtener_json_paginado_con(
            &cliente,
            contexto,
            &url,
            |pagina: Vec<FilaHistorialRemota>| {
                let (recibidos, marca_actualizada) =
                    aplicar_pagina_historial(connection, contexto, &pagina, marca_mas_nueva)?;
                recibidos_total += recibidos;
                marca_mas_nueva = marca_actualizada;
                Ok(())
            },
        )?;
    }

    Ok(recibidos_total)
}

/// Persiste una página de historial remoto en su propia transacción corta,
/// incluida la marca de agua -- así, si la red se corta a mitad de una
/// descarga de varias páginas, las páginas ya procesadas quedan guardadas
/// de verdad (no dependen de que las últimas también lleguen).
///
/// Devuelve cuántas filas de esta página se aplicaron y la marca de agua
/// resultante (el máximo entre `marca_previa` y los `updated_at` de esta
/// página) -- el llamador se la pasa de vuelta como `marca_previa` de la
/// siguiente página.
pub(super) fn aplicar_pagina_historial(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    pagina: &[FilaHistorialRemota],
    marca_previa: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(u32, Option<chrono::DateTime<chrono::Utc>>), SincronizacionError> {
    let transaction = connection.unchecked_transaction()?;
    let mut recibidos = 0_u32;
    let ahora = crate::tiempo::serializar_utc(chrono::Utc::now());
    // La marca de agua del próximo sync incremental es el `updated_at` más
    // nuevo que realmente vino del servidor -- no el reloj de este
    // dispositivo (como quedó mal en la primera versión): si el reloj local
    // está apenas adelantado respecto al de Supabase, una marca tomada de
    // `Utc::now()` queda "en el futuro" para el servidor y cualquier fila
    // con `updated_at` real por debajo de esa marca deja de calificar en
    // `WHERE updated_at > marca` para siempre -- el movimiento desaparece
    // del historial de este dispositivo sin ningún error visible. Sin filas
    // nuevas, la marca no avanza (se vuelve a pedir el mismo rango la
    // próxima vez, que ya sabemos que no trae nada -- preferible a arriesgar
    // perder una fila).
    // Una fila con fecha ilegible no puede tumbar el `?` de acá adentro: eso
    // aborta esta transacción (de una sola página, no de todo el historial)
    // antes del `commit`, y en un dispositivo que necesita traer el
    // historial completo (recién reinstalado, sin marca de agua todavía)
    // una sola fila vieja con un formato raro dejaba SIN historial para
    // siempre -- ni esa fila ni ninguna de las demás, todas las veces que se
    // reintentara, porque el pedido siempre vuelve a traer el sitio entero
    // desde cero. Se omite sólo esa fila (no cuenta para `recibidos` ni para
    // la marca de agua, así que vuelve a pedirse en el próximo sync, pero ya
    // no bloquea al resto).
    let mut marca_mas_nueva = marca_previa;
    for fila in pagina {
        if !guardar_fila_historial(&transaction, contexto.sitio_id, fila, &ahora)? {
            continue;
        }
        recibidos += 1;

        // `updated_at` es una columna de servidor (trigger de Postgres), no
        // texto que alguien tipeó -- a diferencia de `hora_entrada`/
        // `hora_salida` de arriba, no se espera que falle nunca. Si de
        // todos modos fallara, la fila ya se insertó (no se pierde): sólo
        // se evita que esa marca haga avanzar la marca de agua.
        if let Ok(actualizado_en) = crate::tiempo::parsear_utc(&fila.updated_at)
            && marca_mas_nueva.is_none_or(|marca| actualizado_en > marca)
        {
            marca_mas_nueva = Some(actualizado_en);
        }
    }

    if let Some(marca) = marca_mas_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado SET historial_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_marca_utc(marca)],
        )?;
    }
    transaction.commit()?;
    Ok((recibidos, marca_mas_nueva))
}

/// Guarda (o actualiza) un movimiento en `historial_sitio`. `false` si la
/// fila trae una fecha ilegible y se omitió (ver `aplicar_pagina_historial`).
/// No toca la marca de agua: la usan tanto la sincronización (que la mueve
/// aparte) como el aviso en vivo (que no debe moverla).
fn guardar_fila_historial(
    transaction: &Connection,
    sitio_id: &str,
    fila: &FilaHistorialRemota,
    ahora: &str,
) -> Result<bool, SincronizacionError> {
    let Ok(hora_entrada) =
        crate::tiempo::parsear_utc(&fila.hora_entrada).map(crate::tiempo::serializar_utc)
    else {
        return Ok(false);
    };
    let hora_salida = match fila
        .hora_salida
        .as_deref()
        .map(crate::tiempo::parsear_utc)
        .transpose()
    {
        Ok(valor) => valor.map(crate::tiempo::serializar_utc),
        Err(_) => return Ok(false),
    };

    transaction.execute(
        "
        INSERT INTO historial_sitio (
            uuid, sitio_id, contratista_cedula, contratista_nombre, empresa_nombre,
            tipo_ingreso, medio_ingreso, hora_entrada, hora_salida, gafete_numero,
            usuario_entrada_nombre, usuario_salida_nombre, resultado_acceso,
            motivo_resultado, reglas_version, empresa_activa_snapshot,
            dispositivo_entrada_id, dispositivo_salida_id, actualizado_en,
            dispositivo_entrada_tipo, placa
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)
        ON CONFLICT(uuid) DO UPDATE SET
            hora_salida = excluded.hora_salida,
            usuario_salida_nombre = excluded.usuario_salida_nombre,
            dispositivo_salida_id = excluded.dispositivo_salida_id,
            resultado_acceso = excluded.resultado_acceso,
            motivo_resultado = excluded.motivo_resultado,
            reglas_version = excluded.reglas_version,
            empresa_activa_snapshot = excluded.empresa_activa_snapshot,
            actualizado_en = excluded.actualizado_en,
            dispositivo_entrada_tipo = COALESCE(
                excluded.dispositivo_entrada_tipo,
                historial_sitio.dispositivo_entrada_tipo
            )
        ",
        params![
            fila.id,
            sitio_id,
            fila.contratista_cedula,
            fila.contratista_nombre,
            fila.empresa_nombre,
            fila.tipo_ingreso,
            fila.medio_ingreso,
            hora_entrada,
            hora_salida,
            fila.gafete_numero,
            fila.usuario_entrada_nombre,
            fila.usuario_salida_nombre,
            fila.resultado_acceso,
            fila.motivo_resultado,
            fila.reglas_version,
            fila.empresa_activa_snapshot,
            fila.dispositivo_entrada_id,
            fila.dispositivo_salida_id,
            actualizado_en_servidor(&fila.updated_at, ahora),
            fila.dispositivo_entrada
                .as_ref()
                .and_then(|d| d.tipo.clone()),
            fila.placa,
        ],
    )?;
    Ok(true)
}

/// Aviso en vivo con la fila de un ingreso (`nube::en_vivo`): la misma fila
/// que traería la sincronización, guardada sin consultar a la nube. Sin
/// `dispositivo_entrada_tipo` (el aviso no trae el `JOIN` con
/// `dispositivos`): una fila nueva queda sin tipo hasta el próximo pulso,
/// que la vuelve a traer completa porque esto no mueve la marca de agua.
/// `false` si la fila no se pudo interpretar.
pub(in crate::nube) fn guardar_historial_en_vivo(
    transaction: &Connection,
    sitio_id: &str,
    registro: serde_json::Value,
) -> Result<bool, SincronizacionError> {
    let Ok(fila) = serde_json::from_value::<FilaHistorialRemota>(registro) else {
        return Ok(false);
    };
    let ahora = crate::tiempo::serializar_utc(chrono::Utc::now());
    guardar_fila_historial(transaction, sitio_id, &fila, &ahora)
}

/// Lo que se guarda en `actualizado_en` de los historiales locales: el
/// `updated_at` del servidor (con microsegundos), no la hora local de
/// escritura -- así la reconciliación al abrir la app puede comparar fila
/// por fila qué cambió (ver `filtros_de_historial`). `ahora` sólo si el
/// servidor mandara una fecha ilegible, que no pasa: esa fila simplemente
/// vuelve a pedirse en la próxima reconciliación.
fn actualizado_en_servidor(updated_at: &str, ahora: &str) -> String {
    crate::tiempo::parsear_utc(updated_at)
        .map_or_else(|_| ahora.to_string(), crate::tiempo::serializar_marca_utc)
}

/// Una fila del índice liviano de la reconciliación: sólo lo necesario
/// para saber si la copia local está al día.
#[derive(serde::Deserialize)]
struct IndiceHistorialRemoto {
    id: String,
    updated_at: String,
}

/// Cuántos ids viajan en cada `id=in.(...)` de la reconciliación (una URL
/// de ~4 KB, lejos de cualquier límite).
const IDS_POR_PEDIDO: usize = 100;

/// Los filtros de URL (uno por pedido) para recibir un historial:
/// - Sin marca guardada (primera vez en este equipo): un pedido sin
///   filtro, todo el historial del sitio.
/// - Incremental (pulso, avisos): lo que cambió desde la marca, menos el
///   traslape corto.
/// - Reconciliación (al abrir la app): en vez de volver a bajar completas
///   las filas de los últimos 7 días, baja sólo su índice (`id`,
///   `updated_at`), lo compara con `actualizado_en` de `tabla_local` (el
///   `updated_at` del servidor, ver `actualizado_en_servidor`) y pide
///   completas sólo las filas que faltan o cambiaron. Si nada difiere, no
///   hay ningún pedido más.
///
/// `tabla_remota`/`tabla_local` son nombres fijos del código, nunca datos.
fn filtros_de_historial(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    tabla_remota: &str,
    tabla_local: &str,
    marca_anterior: Option<&str>,
    reconciliar: bool,
) -> Result<Vec<String>, SincronizacionError> {
    let Some(desde) = marca_historial_para_consulta(
        marca_anterior,
        chrono::Utc::now(),
        traslape_historial(reconciliar),
    ) else {
        return Ok(vec![String::new()]);
    };
    let desde = crate::tiempo::serializar_marca_utc(desde);
    if !reconciliar {
        return Ok(vec![format!("&updated_at=gt.{desde}")]);
    }

    // El índice se compara página por página (N6): no hace falta juntarlo
    // entero en memoria para saber qué filas faltan o cambiaron.
    let mut consulta_local = connection.prepare(&format!(
        "SELECT actualizado_en FROM {tabla_local} WHERE uuid = ?1"
    ))?;
    let mut distintos = Vec::new();
    obtener_json_paginado_con(
        &cliente_http(),
        contexto,
        &format!(
            "{}/rest/v1/{tabla_remota}?sitio_id=eq.{}&updated_at=gt.{desde}&select=id,updated_at",
            contexto.base_url, contexto.sitio_id,
        ),
        |pagina: Vec<IndiceHistorialRemoto>| {
            for fila in pagina {
                let guardado: Option<String> = consulta_local
                    .query_row(params![fila.id], |row| row.get(0))
                    .optional()?;
                let al_dia = guardado
                    .as_deref()
                    .and_then(|valor| crate::tiempo::parsear_utc(valor).ok())
                    .zip(crate::tiempo::parsear_utc(&fila.updated_at).ok())
                    .is_some_and(|(local, remoto)| local == remoto);
                if !al_dia {
                    distintos.push(fila.id);
                }
            }
            Ok(())
        },
    )?;
    Ok(distintos
        .chunks(IDS_POR_PEDIDO)
        .map(|ids| format!("&id=in.({})", ids.join(",")))
        .collect())
}

/// La marca de agua de la que parte una recepción: la guardada, saneada si
/// quedó en el futuro. No la de la consulta (que ya tiene restado el
/// traslape): una página vacía no debe hacer retroceder la marca.
fn marca_inicial(marca_anterior: Option<&str>) -> Option<chrono::DateTime<chrono::Utc>> {
    let marca = crate::tiempo::parsear_utc(marca_anterior?).ok()?;
    Some(marca.min(chrono::Utc::now()))
}

/// La marca desde la que se pide un historial: la guardada menos
/// `traslape` (ver `traslape_historial`), saneada si quedó en el futuro.
pub(super) fn marca_historial_para_consulta(
    marca_anterior: Option<&str>,
    ahora: chrono::DateTime<chrono::Utc>,
    traslape: chrono::Duration,
) -> Option<chrono::DateTime<chrono::Utc>> {
    let marca = marca_anterior.and_then(|marca| crate::tiempo::parsear_utc(marca).ok())?;
    let base = if marca > ahora { ahora } else { marca };
    Some(base - traslape)
}

#[derive(serde::Deserialize)]
pub(super) struct FilaHistorialVisitaRemota {
    pub(super) id: String,
    pub(super) visitante_cedula: String,
    pub(super) visitante_nombre: String,
    pub(super) empresa: Option<String>,
    pub(super) anfitrion_nombre: Option<String>,
    pub(super) motivo: Option<String>,
    pub(super) gafete_numero: Option<i64>,
    pub(super) hora_entrada: String,
    pub(super) hora_salida: Option<String>,
    pub(super) usuario_entrada_nombre: Option<String>,
    pub(super) usuario_salida_nombre: Option<String>,
    pub(super) dispositivo_entrada_id: String,
    pub(super) dispositivo_salida_id: Option<String>,
    pub(super) updated_at: String,
}

/// Análogo a `FilaHistorialResultado` (contratistas) -- misma resiliencia:
/// una fila con fecha ilegible se omite sin abortar el resto del lote.
pub(super) enum FilaHistorialVisitaResultado {
    Omitida,
    Aplicada {
        actualizado_en: Option<chrono::DateTime<chrono::Utc>>,
    },
}

/// Una sola fila de `recibir_historial_visitas_del_sitio` -- separada sólo
/// para no pasar el límite de líneas del lint `too_many_lines` de esa
/// función (mismo corte que usa `aplicar_pagina_historial` para
/// contratistas: todo el parseo/`INSERT` de una fila, después el resto del
/// lote).
pub(super) fn guardar_fila_historial_visita(
    transaction: &rusqlite::Transaction<'_>,
    contexto: &ContextoSincronizacion<'_>,
    fila: &FilaHistorialVisitaRemota,
    ahora: &str,
) -> Result<FilaHistorialVisitaResultado, SincronizacionError> {
    let Ok(hora_entrada) =
        crate::tiempo::parsear_utc(&fila.hora_entrada).map(crate::tiempo::serializar_utc)
    else {
        return Ok(FilaHistorialVisitaResultado::Omitida);
    };
    let hora_salida = match fila
        .hora_salida
        .as_deref()
        .map(crate::tiempo::parsear_utc)
        .transpose()
    {
        Ok(valor) => valor.map(crate::tiempo::serializar_utc),
        Err(_) => return Ok(FilaHistorialVisitaResultado::Omitida),
    };

    transaction.execute(
        "
        INSERT INTO historial_visitas_sitio (
            uuid, sitio_id, visitante_cedula, visitante_nombre, empresa,
            anfitrion_nombre, motivo, gafete_numero, hora_entrada, hora_salida,
            usuario_entrada_nombre, usuario_salida_nombre, dispositivo_entrada_id,
            dispositivo_salida_id, actualizado_en
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
        ON CONFLICT(uuid) DO UPDATE SET
            hora_salida = excluded.hora_salida,
            usuario_salida_nombre = excluded.usuario_salida_nombre,
            dispositivo_salida_id = excluded.dispositivo_salida_id,
            actualizado_en = excluded.actualizado_en
        ",
        params![
            fila.id,
            contexto.sitio_id,
            fila.visitante_cedula,
            fila.visitante_nombre,
            fila.empresa,
            fila.anfitrion_nombre,
            fila.motivo,
            fila.gafete_numero,
            hora_entrada,
            hora_salida,
            fila.usuario_entrada_nombre,
            fila.usuario_salida_nombre,
            fila.dispositivo_entrada_id,
            fila.dispositivo_salida_id,
            actualizado_en_servidor(&fila.updated_at, ahora),
        ],
    )?;

    Ok(FilaHistorialVisitaResultado::Aplicada {
        actualizado_en: crate::tiempo::parsear_utc(&fila.updated_at).ok(),
    })
}

/// Trae a `historial_visitas_sitio` todo movimiento de visita (abierto o
/// cerrado) del sitio, de cualquier dispositivo -- mismo mecanismo
/// incremental que `recibir_historial_del_sitio` (marca de agua propia,
/// `historial_visitas_actualizado_hasta`, mismo `reconciliar`).
pub fn recibir_historial_visitas_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    reconciliar: bool,
) -> Result<u32, SincronizacionError> {
    let cliente = cliente_http();

    let marca_anterior: Option<String> = connection.query_row(
        "SELECT historial_visitas_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let filtros = filtros_de_historial(
        connection,
        contexto,
        "movimientos_visita",
        "historial_visitas_sitio",
        marca_anterior.as_deref(),
        reconciliar,
    )?;

    // Página por página, mismo criterio que `recibir_historial_del_sitio`
    // (hallazgo R-03).
    let mut recibidos_total = 0_u32;
    let mut marca_mas_nueva = marca_inicial(marca_anterior.as_deref());
    for filtro in filtros {
        let url = format!(
            "{}/rest/v1/movimientos_visita?sitio_id=eq.{}{filtro}\
             &select=id,visitante_cedula,visitante_nombre,empresa,anfitrion_nombre,motivo,\
             gafete_numero,hora_entrada,hora_salida,usuario_entrada_nombre,usuario_salida_nombre,\
             dispositivo_entrada_id,dispositivo_salida_id,updated_at",
            contexto.base_url, contexto.sitio_id,
        );
        obtener_json_paginado_con(
            &cliente,
            contexto,
            &url,
            |pagina: Vec<FilaHistorialVisitaRemota>| {
                let (recibidos, marca_actualizada) = aplicar_pagina_historial_visitas(
                    connection,
                    contexto,
                    &pagina,
                    marca_mas_nueva,
                )?;
                recibidos_total += recibidos;
                marca_mas_nueva = marca_actualizada;
                Ok(())
            },
        )?;
    }

    Ok(recibidos_total)
}

/// Persiste una página de historial de visitas remoto en su propia
/// transacción corta, incluida la marca de agua -- ver el doc-comment de
/// `aplicar_pagina_historial`, mismo criterio.
pub(super) fn aplicar_pagina_historial_visitas(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    pagina: &[FilaHistorialVisitaRemota],
    marca_previa: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(u32, Option<chrono::DateTime<chrono::Utc>>), SincronizacionError> {
    let transaction = connection.unchecked_transaction()?;
    let mut recibidos = 0_u32;
    let ahora = crate::tiempo::serializar_utc(chrono::Utc::now());
    let mut marca_mas_nueva = marca_previa;
    for fila in pagina {
        let FilaHistorialVisitaResultado::Aplicada { actualizado_en } =
            guardar_fila_historial_visita(&transaction, contexto, fila, &ahora)?
        else {
            continue;
        };
        recibidos += 1;
        if let Some(actualizado_en) = actualizado_en
            && marca_mas_nueva.is_none_or(|marca| actualizado_en > marca)
        {
            marca_mas_nueva = Some(actualizado_en);
        }
    }

    if let Some(marca) = marca_mas_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado SET historial_visitas_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_marca_utc(marca)],
        )?;
    }
    transaction.commit()?;
    Ok((recibidos, marca_mas_nueva))
}

#[derive(serde::Deserialize)]
pub(super) struct FilaHistorialIngresoProveedorRemota {
    pub(super) id: String,
    pub(super) cedula: String,
    pub(super) nombre: String,
    pub(super) empresa_nombre: Option<String>,
    pub(super) placa: Option<String>,
    pub(super) gafete_numero: Option<i64>,
    pub(super) hora_entrada: String,
    pub(super) hora_salida: Option<String>,
    pub(super) usuario_entrada_nombre: Option<String>,
    pub(super) usuario_salida_nombre: Option<String>,
    pub(super) dispositivo_entrada_id: String,
    pub(super) dispositivo_salida_id: Option<String>,
    pub(super) updated_at: String,
}

/// Análogo a `FilaHistorialVisitaResultado`.
pub(super) enum FilaHistorialIngresoProveedorResultado {
    Omitida,
    Aplicada {
        actualizado_en: Option<chrono::DateTime<chrono::Utc>>,
    },
}

/// Espejo de `guardar_fila_historial_visita`, pero contra
/// `historial_ingresos_proveedor_sitio`.
pub(super) fn guardar_fila_historial_ingreso_proveedor(
    transaction: &rusqlite::Transaction<'_>,
    contexto: &ContextoSincronizacion<'_>,
    fila: &FilaHistorialIngresoProveedorRemota,
    ahora: &str,
) -> Result<FilaHistorialIngresoProveedorResultado, SincronizacionError> {
    let Ok(hora_entrada) =
        crate::tiempo::parsear_utc(&fila.hora_entrada).map(crate::tiempo::serializar_utc)
    else {
        return Ok(FilaHistorialIngresoProveedorResultado::Omitida);
    };
    let hora_salida = match fila
        .hora_salida
        .as_deref()
        .map(crate::tiempo::parsear_utc)
        .transpose()
    {
        Ok(valor) => valor.map(crate::tiempo::serializar_utc),
        Err(_) => return Ok(FilaHistorialIngresoProveedorResultado::Omitida),
    };

    transaction.execute(
        "
        INSERT INTO historial_ingresos_proveedor_sitio (
            uuid, sitio_id, cedula, nombre, empresa_nombre, placa, gafete_numero,
            hora_entrada, hora_salida, usuario_entrada_nombre, usuario_salida_nombre,
            dispositivo_entrada_id, dispositivo_salida_id, actualizado_en
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
        ON CONFLICT(uuid) DO UPDATE SET
            hora_salida = excluded.hora_salida,
            usuario_salida_nombre = excluded.usuario_salida_nombre,
            dispositivo_salida_id = excluded.dispositivo_salida_id,
            actualizado_en = excluded.actualizado_en
        ",
        params![
            fila.id,
            contexto.sitio_id,
            fila.cedula,
            fila.nombre,
            fila.empresa_nombre,
            fila.placa,
            fila.gafete_numero,
            hora_entrada,
            hora_salida,
            fila.usuario_entrada_nombre,
            fila.usuario_salida_nombre,
            fila.dispositivo_entrada_id,
            fila.dispositivo_salida_id,
            actualizado_en_servidor(&fila.updated_at, ahora),
        ],
    )?;

    Ok(FilaHistorialIngresoProveedorResultado::Aplicada {
        actualizado_en: crate::tiempo::parsear_utc(&fila.updated_at).ok(),
    })
}

/// Espejo de `recibir_historial_visitas_del_sitio`, pero contra
/// `ingresos_proveedor` -- mismo mecanismo incremental (marca de agua
/// propia, `historial_ingresos_proveedor_actualizado_hasta`, mismo
/// `reconciliar`). Sólo tiene sentido
/// llamarla en escritorio -- ver el doc-comment de `MIGRACION_43`.
pub fn recibir_historial_ingresos_proveedor_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    reconciliar: bool,
) -> Result<u32, SincronizacionError> {
    let cliente = cliente_http();

    let marca_anterior: Option<String> = connection.query_row(
        "SELECT historial_ingresos_proveedor_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let filtros = filtros_de_historial(
        connection,
        contexto,
        "ingresos_proveedor",
        "historial_ingresos_proveedor_sitio",
        marca_anterior.as_deref(),
        reconciliar,
    )?;

    let mut recibidos_total = 0_u32;
    let mut marca_mas_nueva = marca_inicial(marca_anterior.as_deref());
    for filtro in filtros {
        let url = format!(
            "{}/rest/v1/ingresos_proveedor?sitio_id=eq.{}{filtro}\
             &select=id,cedula,nombre,empresa_nombre,placa,gafete_numero,hora_entrada,hora_salida,\
             usuario_entrada_nombre,usuario_salida_nombre,dispositivo_entrada_id,\
             dispositivo_salida_id,updated_at",
            contexto.base_url, contexto.sitio_id,
        );
        obtener_json_paginado_con(
            &cliente,
            contexto,
            &url,
            |pagina: Vec<FilaHistorialIngresoProveedorRemota>| {
                let (recibidos, marca_actualizada) = aplicar_pagina_historial_ingresos_proveedor(
                    connection,
                    contexto,
                    &pagina,
                    marca_mas_nueva,
                )?;
                recibidos_total += recibidos;
                marca_mas_nueva = marca_actualizada;
                Ok(())
            },
        )?;
    }

    Ok(recibidos_total)
}

/// Espejo de `aplicar_pagina_historial_visitas`.
pub(super) fn aplicar_pagina_historial_ingresos_proveedor(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    pagina: &[FilaHistorialIngresoProveedorRemota],
    marca_previa: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(u32, Option<chrono::DateTime<chrono::Utc>>), SincronizacionError> {
    let transaction = connection.unchecked_transaction()?;
    let mut recibidos = 0_u32;
    let ahora = crate::tiempo::serializar_utc(chrono::Utc::now());
    let mut marca_mas_nueva = marca_previa;
    for fila in pagina {
        let FilaHistorialIngresoProveedorResultado::Aplicada { actualizado_en } =
            guardar_fila_historial_ingreso_proveedor(&transaction, contexto, fila, &ahora)?
        else {
            continue;
        };
        recibidos += 1;
        if let Some(actualizado_en) = actualizado_en
            && marca_mas_nueva.is_none_or(|marca| actualizado_en > marca)
        {
            marca_mas_nueva = Some(actualizado_en);
        }
    }

    if let Some(marca) = marca_mas_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado SET historial_ingresos_proveedor_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_marca_utc(marca)],
        )?;
    }
    transaction.commit()?;
    Ok((recibidos, marca_mas_nueva))
}

#[derive(serde::Deserialize)]
pub(super) struct FilaHistorialGafeteProvisionalRemota {
    pub(super) id: String,
    pub(super) encargado_nombre: String,
    pub(super) encargado_codigo_empleado: String,
    pub(super) gafete_numero: i64,
    pub(super) hora_entrega: String,
    pub(super) usuario_entrega_nombre: String,
    pub(super) hora_devolucion: Option<String>,
    pub(super) usuario_devolucion_nombre: Option<String>,
    pub(super) updated_at: String,
}

/// Trae a `prestamos_gafete_provisional_historial_sitio` todo préstamo
/// (activo o devuelto) del sitio, de cualquier dispositivo -- mismo
/// criterio que `recibir_historial_del_sitio`: "es la misma operación
/// vista desde dos dispositivos distintos", no una versión resumida.
/// Falencia detectada por el usuario 2026-09-21: la pantalla de escritorio
/// no tenía ninguna vista de historial de gafetes provisionales, sólo
/// "Activos" -- un préstamo que OTRO dispositivo entregó Y devolvió nunca
/// quedaba guardado localmente (sólo pasaba por la caché
/// `prestamos_gafete_provisional_remotos` mientras estaba abierto).
///
/// `obtener_json` (no `obtener_json_paginado_con`) y la marca de agua SIN
/// ventana de traslape (no `marca_historial_para_consulta`) a propósito --
/// a diferencia del historial de ingresos (que sí tuvo ese bug real de
/// desfase de reloj perdiendo filas para siempre), este flujo es de
/// volumen muy bajo ("uno o dos olvidos por turno", ver el doc-comment que
/// tenía `GafetesProvisionales.tsx` antes de esta feature) -- mismo
/// criterio que el resto del catálogo del sitio (contratistas, gafetes,
/// rutas, citas), que tampoco usa esa ventana.
pub fn recibir_historial_gafetes_provisionales_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let cliente = cliente_http();

    let marca_anterior: Option<String> = connection.query_row(
        "SELECT gafetes_provisionales_historial_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let filtro_incremental = marca_anterior
        .as_deref()
        .map(|marca| format!("&updated_at=gt.{marca}"))
        .unwrap_or_default();

    let url = format!(
        "{}/rest/v1/prestamos_gafete_provisional?sitio_id=eq.{}{filtro_incremental}\
         &select=id,encargado_nombre,encargado_codigo_empleado,gafete_numero,hora_entrega,\
         usuario_entrega_nombre,hora_devolucion,usuario_devolucion_nombre,updated_at",
        contexto.base_url, contexto.sitio_id,
    );
    let filas: Vec<FilaHistorialGafeteProvisionalRemota> = obtener_json(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    let mut recibidos = 0_u32;
    let ahora_texto = crate::tiempo::serializar_utc(chrono::Utc::now());
    // Igual que `aplicar_pagina_citas`: una fila con fecha ilegible se omite
    // sin abortar el resto (no cuenta para `recibidos` ni la marca de agua,
    // vuelve a pedirse en el próximo sync).
    let mut marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>> = marca_anterior
        .as_deref()
        .and_then(|marca| crate::tiempo::parsear_utc(marca).ok());
    for fila in &filas {
        let Ok(fecha_hora_entrega) =
            crate::tiempo::parsear_utc(&fila.hora_entrega).map(crate::tiempo::serializar_utc)
        else {
            continue;
        };
        let fecha_hora_devolucion = match fila
            .hora_devolucion
            .as_deref()
            .map(crate::tiempo::parsear_utc)
            .transpose()
        {
            Ok(valor) => valor.map(crate::tiempo::serializar_utc),
            Err(_) => continue,
        };
        let Ok(actualizado) = crate::tiempo::parsear_utc(&fila.updated_at) else {
            continue;
        };

        transaction.execute(
            "
            INSERT INTO prestamos_gafete_provisional_historial_sitio (
                uuid, sitio_id, encargado_nombre, encargado_codigo_empleado, gafete_numero,
                fecha_hora_entrega, usuario_entrega_nombre, fecha_hora_devolucion,
                usuario_devolucion_nombre, actualizado_en
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
            ON CONFLICT(uuid) DO UPDATE SET
                fecha_hora_devolucion = excluded.fecha_hora_devolucion,
                usuario_devolucion_nombre = excluded.usuario_devolucion_nombre,
                actualizado_en = excluded.actualizado_en
            ",
            params![
                fila.id,
                contexto.sitio_id,
                fila.encargado_nombre,
                fila.encargado_codigo_empleado,
                fila.gafete_numero,
                fecha_hora_entrega,
                fila.usuario_entrega_nombre,
                fecha_hora_devolucion,
                fila.usuario_devolucion_nombre,
                ahora_texto,
            ],
        )?;
        recibidos += 1;
        marca_mas_nueva = Some(marca_mas_nueva.map_or(actualizado, |marca| marca.max(actualizado)));
    }

    if let Some(marca) = marca_mas_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado
             SET gafetes_provisionales_historial_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_marca_utc(marca)],
        )?;
    }
    transaction.commit()?;

    Ok(recibidos)
}
