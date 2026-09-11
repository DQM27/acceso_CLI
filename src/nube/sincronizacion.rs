//! Drena la bandeja de salida (`cola_salida`, ver
//! `docs/plan-persistencia-nube.md`) hacia el receptor: por cada fila
//! pendiente, arma el pedido HTTP correspondiente y la marca `enviado` o
//! `fallido` según la respuesta. Una fila fallida no detiene a las demás --
//! se reintenta en la próxima llamada, no bloquea el resto de la cola.
//!
//! Las filas de un mismo tipo (`empresa`, `contratista`, `gafete`,
//! `usuario`, apertura de `ingreso`) se agrupan y se intentan mandar en un
//! solo `POST` con un array -- `PostgREST` hace `upsert` de un array igual
//! que de un objeto suelto. Si el lote entero falla, se cae a mandar esas
//! mismas filas una por una: un `INSERT`/`upsert` de varias filas es
//! atómico en Postgres, así que una sola fila inválida (por ejemplo, un
//! contratista cuya empresa todavía no llegó) tumbaría a todo el lote junto
//! si no se aislara así -- ver hallazgo R-02 de
//! `docs/auditorias/AUDITORIA_RENDIMIENTO_CORE_RUST_2026-09-10.md`. El
//! cierre de un ingreso (`PATCH .../ingresos?...&hora_salida=is.null`)
//! queda afuera del lote a propósito: es un `UPDATE` condicional
//! ("primero en llegar gana"), no un `upsert`, y un `PATCH` con array no
//! aplica una condición distinta por fila.

use std::collections::HashMap;

use rusqlite::{Connection, params};
use serde_json::{Value, json};

use super::cliente::{NubeError, cliente_http};

/// Todo lo que hace falta para hablar con el receptor en nombre de este
/// dispositivo. `apikey` es la clave publicable del proyecto (no un
/// secreto -- ver `get_publishable_keys`), separada del `token` (el JWT que
/// ya identifica a este dispositivo y a su sitio).
pub struct ContextoSincronizacion<'a> {
    pub base_url: &'a str,
    pub apikey: &'a str,
    pub token: &'a str,
    pub dispositivo_id: &'a str,
    pub sitio_id: &'a str,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResumenDrenado {
    pub enviados: u32,
    pub fallidos: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum SincronizacionError {
    #[error("Error de base de datos local: {0}")]
    BaseLocal(#[from] rusqlite::Error),
    #[error(transparent)]
    Red(#[from] NubeError),
    #[error("El receptor respondió {status}: {cuerpo}")]
    RespuestaInesperada { status: u16, cuerpo: String },
    #[error("El receptor mandó una fecha inválida: {0}")]
    FechaInvalida(String),
}

struct FilaCola {
    id: i64,
    entidad: String,
    entidad_uuid: String,
    operacion: String,
    intentos: i64,
}

/// Después de esta cantidad de intentos fallidos seguidos, una fila deja de
/// reintentarse sola y pasa a `estado = 'fallido'` (terminal) -- sin este
/// tope, un dato irremediablemente roto (nunca va a poder mandarse, sea
/// cual sea la razón) se reintentaría cada 5 minutos para siempre, sin que
/// nadie se entere. Con el backoff de abajo, llegar acá lleva más de un día
/// real de reintentos -- no es un umbral que se cruce por una mala racha de
/// conexión.
const INTENTOS_ANTES_DE_FALLO_PERMANENTE: i64 = 20;

/// Envía hasta `limite` filas pendientes, en orden de creación (respetando
/// el backoff de `pendientes`). Nunca devuelve error por una fila
/// individual fallida -- eso queda registrado en la propia fila
/// (`ultimo_error`, y `estado = 'fallido'` sólo tras
/// [`INTENTOS_ANTES_DE_FALLO_PERMANENTE`] intentos); sólo devuelve error si
/// no se pudo ni siquiera leer/actualizar la cola local.
pub fn drenar_cola(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    limite: u32,
) -> Result<ResumenDrenado, SincronizacionError> {
    let cliente = cliente_http();
    let mut resumen = ResumenDrenado::default();

    for grupo in agrupar_por_entidad_y_operacion(pendientes(connection, limite)?) {
        drenar_grupo(&cliente, connection, contexto, grupo, &mut resumen)?;
    }

    Ok(resumen)
}

/// Junta las filas pendientes por `(entidad, operacion)`, preservando el
/// orden en que aparece cada combinación por primera vez -- no hace falta
/// más que eso: como anota el doc-comment de `enviar_empresa`, una fila que
/// depende de otra que todavía no llegó (por ejemplo, un contratista antes
/// que su empresa) simplemente falla por la FK real y el backoff la
/// reintenta sola, sin importar en qué orden se hayan drenado los tipos
/// dentro de esta llamada.
fn agrupar_por_entidad_y_operacion(filas: Vec<FilaCola>) -> Vec<Vec<FilaCola>> {
    let mut grupos: Vec<(String, String, Vec<FilaCola>)> = Vec::new();
    for fila in filas {
        let existente = grupos
            .iter_mut()
            .find(|(entidad, operacion, _)| *entidad == fila.entidad && *operacion == fila.operacion);
        if let Some((_, _, filas_del_grupo)) = existente {
            filas_del_grupo.push(fila);
        } else {
            let entidad = fila.entidad.clone();
            let operacion = fila.operacion.clone();
            grupos.push((entidad, operacion, vec![fila]));
        }
    }
    grupos.into_iter().map(|(_, _, filas)| filas).collect()
}

/// Tabla y parámetro `on_conflict` para mandar un grupo entero en un solo
/// `POST` (array) -- `None` si ese `(entidad, operacion)` no admite lote
/// (ver el doc-comment del módulo: el cierre de ingreso es un `PATCH`
/// condicional, no un `upsert`).
fn destino_lote(entidad: &str, operacion: &str) -> Option<(&'static str, Option<&'static str>)> {
    match (entidad, operacion) {
        ("empresa", _) => Some(("empresas", Some("nombre"))),
        ("contratista", _) => Some(("contratistas", Some("identificacion"))),
        ("gafete", _) => Some(("gafetes", Some("sitio_id,numero"))),
        ("usuario", _) => Some(("usuarios", Some("cedula"))),
        ("ingreso", "cerrar") => None,
        ("ingreso", _) => Some(("ingresos", None)),
        _ => None,
    }
}

/// Arma el cuerpo local de una fila según su `entidad` -- dispatch que sólo
/// usa [`enviar_lote`] para construir varios cuerpos y mandarlos en un
/// array. El camino de una fila sola (`enviar_contratista` y compañía) no
/// pasa por acá: cada uno ya llama directo a su propio
/// `construir_cuerpo_*`.
fn construir_cuerpo(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    entidad: &str,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    match entidad {
        "empresa" => construir_cuerpo_empresa(connection, contexto, uuid),
        "contratista" => construir_cuerpo_contratista(connection, contexto, uuid),
        "gafete" => construir_cuerpo_gafete(connection, contexto, uuid),
        "usuario" => construir_cuerpo_usuario(connection, contexto, uuid),
        "ingreso" => construir_cuerpo_ingreso(connection, contexto, uuid),
        otra => unreachable!("destino_lote ya filtró entidades sin lote (recibido: {otra})"),
    }
}

/// Manda un grupo entero -- todas la misma `(entidad, operacion)` -- en un
/// único `POST` con un array de cuerpos. `destino_lote` decide la tabla y el
/// `on_conflict`; si alguna fila del grupo ni siquiera se puede leer de la
/// base local (por ejemplo, se borró mientras esperaba en la cola), el error
/// sube tal cual y el llamador cae al camino fila por fila, donde esa fila
/// puntual va a fallar de nuevo con el mismo error y quedar registrada sola.
fn enviar_lote(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    tabla: &str,
    on_conflict: Option<&str>,
    entidad: &str,
    grupo: &[FilaCola],
) -> Result<(), SincronizacionError> {
    let mut cuerpos = Vec::with_capacity(grupo.len());
    for fila in grupo {
        cuerpos.push(construir_cuerpo(connection, contexto, entidad, &fila.entidad_uuid)?);
    }

    let url = on_conflict.map_or_else(
        || format!("{}/rest/v1/{tabla}", contexto.base_url),
        |on_conflict| format!("{}/rest/v1/{tabla}?on_conflict={on_conflict}", contexto.base_url),
    );

    let respuesta = cliente
        .post(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpos)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Intenta mandar `grupo` (todas del mismo `(entidad, operacion)`) en un
/// solo lote cuando corresponde; si el lote falla -- de red, o porque
/// Postgres rechazó el `INSERT`/`upsert` completo por una sola fila mala --
/// cae a mandar cada fila por separado, exactamente como si el lote nunca
/// se hubiera intentado. Un grupo de una sola fila nunca pasa por el lote:
/// no hay nada que ahorrar y así ese caso (el más común) queda idéntico al
/// comportamiento de antes.
fn drenar_grupo(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    grupo: Vec<FilaCola>,
    resumen: &mut ResumenDrenado,
) -> Result<(), SincronizacionError> {
    if grupo.len() > 1
        && let Some((tabla, on_conflict)) = destino_lote(&grupo[0].entidad, &grupo[0].operacion)
    {
        let intento_lote = enviar_lote(
            cliente,
            connection,
            contexto,
            tabla,
            on_conflict,
            &grupo[0].entidad,
            &grupo,
        );
        if intento_lote.is_ok() {
            for fila in &grupo {
                marcar(connection, fila.id, "enviado", None)?;
            }
            resumen.enviados += u32::try_from(grupo.len()).unwrap_or(u32::MAX);
            return Ok(());
        }
        // El lote entero falló -- se cae a fila por fila para aislar cuál
        // es la mala en vez de reintentar a todo el grupo junto (ver el
        // doc-comment del módulo).
    }

    for fila in grupo {
        procesar_fila_individual(cliente, connection, contexto, fila, resumen)?;
    }
    Ok(())
}

/// Camino de una fila a la vez -- el único que existía antes del lote, y el
/// que sigue manejando el cierre de ingreso y el fallback tras un lote
/// fallido. Nunca devuelve error por una fila individual fallida -- eso
/// queda registrado en la propia fila (`ultimo_error`, y
/// `estado = 'fallido'` sólo tras [`INTENTOS_ANTES_DE_FALLO_PERMANENTE`]
/// intentos); sólo devuelve error si no se pudo ni siquiera
/// leer/actualizar la cola local.
fn procesar_fila_individual(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    fila: FilaCola,
    resumen: &mut ResumenDrenado,
) -> Result<(), SincronizacionError> {
    let resultado = match (fila.entidad.as_str(), fila.operacion.as_str()) {
        ("empresa", _) => enviar_empresa(cliente, connection, contexto, &fila.entidad_uuid),
        ("contratista", _) => enviar_contratista(cliente, connection, contexto, &fila.entidad_uuid),
        ("gafete", _) => enviar_gafete(cliente, connection, contexto, &fila.entidad_uuid),
        ("usuario", _) => enviar_usuario(cliente, connection, contexto, &fila.entidad_uuid),
        ("ingreso", "cerrar") => {
            enviar_cierre_ingreso(cliente, connection, contexto, &fila.entidad_uuid)
        }
        ("ingreso", _) => enviar_ingreso(cliente, connection, contexto, &fila.entidad_uuid),
        ("movimiento_visita", "cerrar") => {
            enviar_cierre_movimiento_visita(cliente, connection, contexto, &fila.entidad_uuid)
        }
        ("movimiento_visita", _) => {
            enviar_movimiento_visita(cliente, connection, contexto, &fila.entidad_uuid)
        }
        _ => Ok(()),
    };

    match resultado {
        Ok(()) => {
            marcar(connection, fila.id, "enviado", None)?;
            resumen.enviados += 1;
        }
        Err(error) => {
            // "pendiente" de nuevo -- no "fallido" -- para que
            // `pendientes()` la vuelva a considerar más adelante, sujeta
            // al backoff según cuántas veces ya falló.
            let estado = if fila.intentos + 1 >= INTENTOS_ANTES_DE_FALLO_PERMANENTE {
                "fallido"
            } else {
                "pendiente"
            };
            marcar(connection, fila.id, estado, Some(&error.to_string()))?;
            resumen.fallidos += 1;
        }
    }
    Ok(())
}

/// Filas listas para reintentarse ahora: nunca tocadas (`intentos = 0`), o
/// que ya esperaron lo suficiente desde el último intento. La espera crece
/// con cada fallo (15 min, 30 min, 45 min...), tope de un día -- para no
/// mendigar el mismo pedido roto cada 5 minutos para siempre, pero tampoco
/// dejarlo esperando una semana entera. `proximo_intento_en` (columna
/// generada, ver `MIGRACION_27` en `database::schema`) ya trae esa fórmula
/// calculada -- antes era una expresión repetida acá mismo en cada
/// consulta, que `SQLite` no podía resolver con un índice (hallazgo R-06 de
/// `docs/auditorias/AUDITORIA_RENDIMIENTO_CORE_RUST_2026-09-10.md`); ahora
/// es una columna de verdad, con `idx_cola_salida_pendientes` sobre ella.
fn pendientes(connection: &Connection, limite: u32) -> Result<Vec<FilaCola>, SincronizacionError> {
    let mut statement = connection.prepare(
        "
        SELECT id, entidad, entidad_uuid, operacion, intentos FROM cola_salida
        WHERE estado = 'pendiente' AND proximo_intento_en <= datetime('now')
        ORDER BY creado_en
        LIMIT ?1
        ",
    )?;
    let filas = statement
        .query_map(params![limite], |row| {
            Ok(FilaCola {
                id: row.get(0)?,
                entidad: row.get(1)?,
                entidad_uuid: row.get(2)?,
                operacion: row.get(3)?,
                intentos: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(filas)
}

/// Cuántas filas ya agotaron los reintentos automáticos
/// (`estado = 'fallido'`) -- para que la pantalla avise que algo necesita
/// que alguien lo mire, en vez de fallar en silencio para siempre.
pub fn contar_fallos_permanentes(connection: &Connection) -> Result<i64, SincronizacionError> {
    Ok(connection.query_row(
        "SELECT COUNT(*) FROM cola_salida WHERE estado = 'fallido'",
        [],
        |row| row.get(0),
    )?)
}

fn marcar(
    connection: &Connection,
    id: i64,
    estado: &str,
    error: Option<&str>,
) -> Result<(), SincronizacionError> {
    connection.execute(
        "
        UPDATE cola_salida
        SET
            estado = ?1,
            intentos = intentos + 1,
            ultimo_error = ?2,
            actualizado_en = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        WHERE id = ?3
        ",
        params![estado, error, id],
    )?;
    Ok(())
}

fn exigir_2xx(respuesta: reqwest::blocking::Response) -> Result<(), SincronizacionError> {
    if respuesta.status().is_success() {
        return Ok(());
    }
    let status = respuesta.status().as_u16();
    let cuerpo = respuesta.text().unwrap_or_default();
    Err(SincronizacionError::RespuestaInesperada { status, cuerpo })
}

/// `GET` autenticado + deserializar la lista de filas -- compartido por
/// todo lo que trae datos de la nube hacia acá (`recibir_ingresos_abiertos`,
/// `recibir_catalogo_del_sitio`).
fn obtener_json<T: serde::de::DeserializeOwned>(
    cliente: &reqwest::blocking::Client,
    contexto: &ContextoSincronizacion<'_>,
    url: &str,
) -> Result<Vec<T>, SincronizacionError> {
    let respuesta = cliente
        .get(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .send()
        .map_err(NubeError::Red)?;

    if !respuesta.status().is_success() {
        let status = respuesta.status().as_u16();
        let cuerpo = respuesta.text().unwrap_or_default();
        return Err(SincronizacionError::RespuestaInesperada { status, cuerpo });
    }
    let filas = respuesta.json().map_err(NubeError::Red)?;
    Ok(filas)
}

/// Filas por página al paginar con `obtener_json_paginado` -- por debajo
/// del tope de filas por respuesta que Supabase/`PostgREST` impone por
/// defecto (`db-max-rows`, 1000) para que ninguna página sola pueda
/// chocar con ese límite y perder el resto en silencio.
const TAMANO_PAGINA_REMOTA: usize = 500;
const DIAS_TRASLAPE_HISTORIAL: i64 = 7;

/// Igual que `obtener_json`, pero para listas que pueden superar el tope de
/// filas por respuesta de `PostgREST` -- sin esto, un catálogo o un
/// historial que ya cruzó ese tope (típico en el primer sync de un sitio
/// grande, sin `marca_anterior` que acote nada) perdía en silencio todo lo
/// que sobraba: no un error, sólo datos que nunca llegaban. Pagina con
/// `Range` (protocolo estándar de `PostgREST`) hasta que una página vuelve
/// con menos filas que `TAMANO_PAGINA_REMOTA`, señal de que no queda nada
/// más. `url_base` no debe traer su propio `order=` -- esta función agrega
/// uno por `id` (columna presente en todo lo que hoy pagina) para que el
/// orden entre páginas sea estable; sin un orden fijo, dos páginas
/// consecutivas de una tabla que sigue cambiando mientras se pagina
/// podrían saltarse o repetir filas.
fn obtener_json_paginado<T: serde::de::DeserializeOwned>(
    cliente: &reqwest::blocking::Client,
    contexto: &ContextoSincronizacion<'_>,
    url_base: &str,
) -> Result<Vec<T>, SincronizacionError> {
    let mut resultado = Vec::new();
    obtener_json_paginado_con(cliente, contexto, url_base, |pagina: Vec<T>| {
        resultado.extend(pagina);
        Ok(())
    })?;
    Ok(resultado)
}

/// Igual que [`obtener_json_paginado`], pero en vez de acumular todas las
/// páginas en un `Vec` antes de volver, le entrega cada página a
/// `por_pagina` a medida que llega -- pensada para `recibir_historial_del_sitio`,
/// el único llamador cuyo resultado puede llegar a ser grande de verdad (el
/// catálogo de un sitio -- contratistas, gafetes -- está acotado por la
/// plantilla física del sitio; el historial, no). Hallazgo R-03 de
/// `docs/auditorias/AUDITORIA_RENDIMIENTO_CORE_RUST_2026-09-10.md`: sin
/// esto, el primer sync de un sitio con historial grande junta todas las
/// páginas en memoria antes de persistir ninguna. `por_pagina` puede abrir
/// su propia transacción corta y comitearla por página -- eso además
/// acorta cuánto tiempo se retiene el candado de escritura comparado con
/// una única transacción gigante al final, y dos ventajas más: si la red
/// se corta a mitad de la descarga, las páginas ya comiteadas no se pierden
/// (a diferencia de una transacción única, que revierte todo); y las
/// lecturas concurrentes (`GuiState::conexion_secundaria`) sólo se bloquean
/// durante cada transacción corta, no durante toda la descarga.
fn obtener_json_paginado_con<T, F>(
    cliente: &reqwest::blocking::Client,
    contexto: &ContextoSincronizacion<'_>,
    url_base: &str,
    mut por_pagina: F,
) -> Result<(), SincronizacionError>
where
    T: serde::de::DeserializeOwned,
    F: FnMut(Vec<T>) -> Result<(), SincronizacionError>,
{
    let separador = if url_base.contains('?') { '&' } else { '?' };
    let url = format!("{url_base}{separador}order=id.asc");

    let mut desde = 0_usize;
    loop {
        let respuesta = cliente
            .get(&url)
            .header("apikey", contexto.apikey)
            .header("Authorization", format!("Bearer {}", contexto.token))
            .header("Range-Unit", "items")
            .header(
                "Range",
                format!("{desde}-{}", desde + TAMANO_PAGINA_REMOTA - 1),
            )
            .send()
            .map_err(NubeError::Red)?;

        // `is_success()` ya cubre el 206 Partial Content que `PostgREST`
        // devuelve cuando la página pedida no alcanza a cubrir todo lo que
        // hay -- no hace falta distinguirlo de un 200 normal, el criterio
        // de "¿hay más?" de abajo (cuántas filas vinieron) es el mismo.
        if !respuesta.status().is_success() {
            let status = respuesta.status().as_u16();
            let cuerpo = respuesta.text().unwrap_or_default();
            return Err(SincronizacionError::RespuestaInesperada { status, cuerpo });
        }
        let pagina: Vec<T> = respuesta.json().map_err(NubeError::Red)?;
        let recibidas_en_esta_pagina = pagina.len();
        let hay_mas = recibidas_en_esta_pagina == TAMANO_PAGINA_REMOTA;
        por_pagina(pagina)?;
        if !hay_mas {
            break;
        }
        desde += TAMANO_PAGINA_REMOTA;
    }
    Ok(())
}

/// Contratistas (espejo): crear y actualizar se resuelven igual -- un
/// `upsert` (`Prefer: resolution=merge-duplicates`) es idempotente y la
/// versión más nueva siempre termina ganando, así que no hace falta
/// distinguir la operación.
///
/// `empresa_id` manda el UUID real de la empresa en la nube (join contra
/// `empresas.uuid` local) -- puede venir `NULL` si la fila de esa empresa
/// todavía no se drenó (llegó primero el contratista en la cola, algo que
/// no debería pasar en el orden normal de creación, pero no es un error si
/// pasa: el contratista igual se manda, sólo sin el vínculo relacional
/// hasta que la empresa también llegue). `empresa_nombre` se sigue mandando
/// siempre, como snapshot legible sin depender del join.
///
/// `tipo_ingreso`/`fecha_vencimiento_praind`/`es_personal_ruta` viajan
/// también -- sin esto el espejo sólo alcanzaba para mostrar el nombre
/// (pantalla Activos), pero no para que el otro dispositivo del mismo
/// sitio pudiera registrar un ingreso nuevo de este contratista con las
/// reglas de acceso correctas (ver `recibir_catalogo_del_sitio`).
/// Sólo la parte local -- lee `contratistas`/`empresas` y arma el cuerpo que
/// espera el receptor -- sin tocar la red. Separada de [`enviar_contratista`]
/// para que [`enviar_lote`] pueda construir varios cuerpos y mandarlos juntos
/// en un solo `POST` (array), sin duplicar esta consulta.
fn construir_cuerpo_contratista(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let (
        cedula,
        nombre,
        tiene_acceso,
        empresa_nombre,
        empresa_uuid,
        tipo_ingreso,
        fecha_vencimiento_praind,
        es_personal_ruta,
    ): (
        String,
        String,
        i64,
        String,
        Option<String>,
        String,
        Option<String>,
        i64,
    ) = connection.query_row(
        "
        SELECT c.cedula, c.nombre, c.tiene_acceso, e.nombre, e.uuid,
               c.tipo_ingreso, c.fecha_vencimiento_praind, c.es_personal_ruta
        FROM contratistas c
        JOIN empresas e ON e.id = c.empresa_id
        WHERE c.uuid = ?1
        ",
        params![uuid],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        },
    )?;

    Ok(json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "nombre": nombre,
        "identificacion": cedula,
        "empresa_id": empresa_uuid,
        "empresa_nombre": empresa_nombre,
        "activo": tiene_acceso != 0,
        "tipo_ingreso": tipo_ingreso,
        "fecha_vencimiento_praind": fecha_vencimiento_praind,
        "es_personal_ruta": es_personal_ruta != 0,
    }))
}

fn enviar_contratista(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_contratista(connection, contexto, uuid)?;

    // `on_conflict=identificacion`, no el default (la PK `id`) -- sin esto,
    // dos bases locales que nunca compartieron el mismo `uuid` para el
    // mismo contratista (una base vieja o reconstruida, `uuid` local
    // perdido, etc.) generan cada una un `id` propio al azar, y el upsert
    // por PK los acepta como dos personas distintas en vez de fusionarlos
    // -- exactamente el bug reproducido en producción (117 contratistas
    // duplicados). `identificacion` es la cédula real, única de verdad.
    let respuesta = cliente
        .post(format!(
            "{}/rest/v1/contratistas?on_conflict=identificacion",
            contexto.base_url
        ))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Empresas (espejo): mismo criterio de `upsert` que contratistas -- crear
/// y actualizar se resuelven igual, reintentar no duplica nada. Si un
/// contratista de esta empresa se drena antes de que la empresa exista en
/// la nube, esa fila falla por la FK real (`contratistas.empresa_id
/// references empresas`) y el backoff la reintenta sola -- no hace falta
/// forzar el orden acá.
/// Ver el doc-comment de [`construir_cuerpo_contratista`] -- misma idea.
fn construir_cuerpo_empresa(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let (nombre, activo): (String, i64) = connection.query_row(
        "SELECT nombre, activo FROM empresas WHERE uuid = ?1",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    Ok(json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "nombre": nombre,
        "activa": activo != 0,
    }))
}

fn enviar_empresa(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_empresa(connection, contexto, uuid)?;

    // `on_conflict=nombre` -- mismo motivo que `enviar_contratista`: sin
    // esto, dos bases locales sin el mismo `uuid` para la misma empresa
    // (nunca hubo constraint de unicidad remota más que el `id`) generan
    // cada una un `id` propio y el upsert por PK las acepta como empresas
    // distintas en vez de fusionarlas.
    let respuesta = cliente
        .post(format!(
            "{}/rest/v1/empresas?on_conflict=nombre",
            contexto.base_url
        ))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Gafetes (espejo): mismo criterio de `upsert` que contratistas/empresas.
/// Sólo el estado actual (número, estado, a quién se lo debe) -- el
/// historial de incidentes (`gafetes_incidentes`) sigue siendo puramente
/// local, no viaja a la nube. `contratista_deudor_id` manda el UUID real
/// del contratista deudor (`NULL` si el gafete no está `PERDIDO`, o si esa
/// fila del contratista todavía no se drenó -- mismo caso que
/// `empresa_id` en `enviar_contratista`).
/// Ver el doc-comment de [`construir_cuerpo_contratista`] -- misma idea.
fn construir_cuerpo_gafete(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let (numero, estado, deudor_uuid, deudor_nombre): (
        i64,
        String,
        Option<String>,
        Option<String>,
    ) = connection.query_row(
        "
        SELECT g.numero, g.estado, c.uuid, c.nombre
        FROM gafetes g
        LEFT JOIN contratistas c ON c.id = g.contratista_deudor_id
        WHERE g.uuid = ?1
        ",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;

    Ok(json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "numero": numero,
        "estado": estado,
        "contratista_deudor_id": deudor_uuid,
        "contratista_deudor_nombre": deudor_nombre,
    }))
}

fn enviar_gafete(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_gafete(connection, contexto, uuid)?;

    // `on_conflict=sitio_id,numero` -- mismo motivo que
    // `enviar_contratista`/`enviar_empresa`: el número de gafete es único
    // dentro de un sitio aunque el `id` remoto no coincida entre bases.
    let respuesta = cliente
        .post(format!(
            "{}/rest/v1/gafetes?on_conflict=sitio_id,numero",
            contexto.base_url
        ))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Usuarios globales (ROOT/ADMINISTRADOR/OPERADOR, espejo): mismo criterio
/// de `upsert` que contratistas. Sin `password_hash` a propósito -- la
/// nube nunca la recibe (ver `SIN_PASSWORD_LOCAL` en
/// `services/password.rs`): distribuye quién existe y su rol/estado, cada
/// dispositivo fija su propia contraseña local la primera vez que ese
/// operador inicia sesión ahí.
/// Ver el doc-comment de [`construir_cuerpo_contratista`] -- misma idea.
fn construir_cuerpo_usuario(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let (cedula, nombre, rol, activo): (String, String, String, i64) = connection.query_row(
        "SELECT cedula, nombre, rol, activo FROM usuarios WHERE uuid = ?1",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;

    Ok(json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "cedula": cedula,
        "nombre": nombre,
        "rol": rol,
        "activo": activo != 0,
    }))
}

fn enviar_usuario(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_usuario(connection, contexto, uuid)?;

    // `on_conflict=cedula` -- la tabla remota ya tiene `UNIQUE(cedula)`, pero
    // sin decirlo acá el upsert infiere la PK (`id`) como blanco del
    // conflicto: reenviar este usuario con un `uuid` local distinto (mismo
    // caso que contratistas/empresas) violaría esa constraint en vez de
    // fusionarse.
    let respuesta = cliente
        .post(format!(
            "{}/rest/v1/usuarios?on_conflict=cedula",
            contexto.base_url
        ))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Ver el doc-comment de [`construir_cuerpo_contratista`] -- misma idea.
#[allow(clippy::type_complexity)]
fn construir_cuerpo_ingreso(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let (
        contratista_id_local,
        contratista_nombre,
        fecha_hora_ingreso,
        usuario_ingreso_nombre,
        contratista_cedula,
        empresa_nombre,
        tipo_ingreso,
        medio_ingreso,
        gafete_numero,
        resultado_acceso,
        motivo_resultado,
        reglas_version,
        empresa_activa_snapshot,
    ): (
        i64,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<i64>,
        String,
        Option<String>,
        i64,
        bool,
    ) = connection.query_row(
        "
        SELECT contratista_id, contratista_nombre, fecha_hora_ingreso, usuario_ingreso_nombre,
               contratista_cedula, empresa_nombre, tipo_ingreso, medio_ingreso, gafete_numero,
               resultado_acceso, motivo_resultado, reglas_version, empresa_activa_snapshot
        FROM registro_ingresos
        WHERE uuid = ?1
        ",
        params![uuid],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
                row.get(11)?,
                row.get::<_, i64>(12)? != 0,
            ))
        },
    )?;
    let contratista_uuid: String = connection.query_row(
        "SELECT uuid FROM contratistas WHERE id = ?1",
        params![contratista_id_local],
        |row| row.get(0),
    )?;

    Ok(json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_entrada_id": contexto.dispositivo_id,
        "contratista_id": contratista_uuid,
        "contratista_nombre": contratista_nombre,
        "hora_entrada": fecha_hora_ingreso,
        "usuario_entrada_nombre": usuario_ingreso_nombre,
        "contratista_cedula": contratista_cedula,
        "empresa_nombre": empresa_nombre,
        "tipo_ingreso": tipo_ingreso,
        "medio_ingreso": medio_ingreso,
        "gafete_numero": gafete_numero,
        "resultado_acceso": resultado_acceso,
        "motivo_resultado": motivo_resultado,
        "reglas_version": reglas_version,
        "empresa_activa_snapshot": empresa_activa_snapshot,
    }))
}

/// Ingresos (cola), apertura: mismo criterio de `upsert` que contratistas
/// -- reintentar un envío ya recibido no duplica nada.
fn enviar_ingreso(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_ingreso(connection, contexto, uuid)?;

    let respuesta = cliente
        .post(format!("{}/rest/v1/ingresos", contexto.base_url))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Ingresos (cola), cierre: `PATCH .../ingresos?id=eq.<uuid>&hora_salida=is.null`
/// es la regla de "primero en llegar gana" completa -- si el otro
/// dispositivo del mismo sitio ya cerró este ingreso, el filtro no
/// encuentra fila para actualizar (0 filas afectadas, no un error). El
/// objetivo ("que quede cerrado en la nube") ya se cumplió de todas formas,
/// así que no hace falta distinguir ese caso.
fn enviar_cierre_ingreso(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (fecha_hora_salida, usuario_salida_nombre): (Option<String>, Option<String>) = connection
        .query_row(
        "SELECT fecha_hora_salida, usuario_salida_nombre FROM registro_ingresos WHERE uuid = ?1",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let cuerpo = json!({
        "hora_salida": fecha_hora_salida,
        "dispositivo_salida_id": contexto.dispositivo_id,
        "usuario_salida_nombre": usuario_salida_nombre,
    });

    let url = format!(
        "{}/rest/v1/ingresos?id=eq.{uuid}&hora_salida=is.null",
        contexto.base_url
    );

    let respuesta = cliente
        .patch(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Movimientos de visita (cola, alta): mismo armazón que `enviar_ingreso`,
/// pero sin resultado de acceso/PRAIND que mandar. `cita_visitante_id`
/// local es un `INTEGER` (fila de `cita_visitantes`); lo que viaja al
/// servidor es su `uuid` (el `id` real allá) -- misma resolución que
/// `enviar_ingreso` hace con `contratista_id`.
#[allow(clippy::type_complexity)]
fn enviar_movimiento_visita(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (
        cita_visitante_id_local,
        gafete_numero,
        fecha_hora_entrada,
        usuario_entrada_nombre,
        visitante_cedula,
        visitante_nombre,
        empresa,
        anfitrion_nombre,
        motivo,
    ): (
        i64,
        Option<i64>,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = connection.query_row(
        "
        SELECT cita_visitante_id, gafete_numero, fecha_hora_entrada, usuario_entrada_nombre,
               visitante_cedula, visitante_nombre, empresa, anfitrion_nombre, motivo
        FROM movimientos_visita
        WHERE uuid = ?1
        ",
        params![uuid],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
            ))
        },
    )?;
    let cita_visitante_uuid: String = connection.query_row(
        "SELECT uuid FROM cita_visitantes WHERE id = ?1",
        params![cita_visitante_id_local],
        |row| row.get(0),
    )?;

    let cuerpo = json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_entrada_id": contexto.dispositivo_id,
        "cita_visitante_id": cita_visitante_uuid,
        "visitante_cedula": visitante_cedula,
        "visitante_nombre": visitante_nombre,
        "empresa": empresa,
        "anfitrion_nombre": anfitrion_nombre,
        "motivo": motivo,
        "gafete_numero": gafete_numero,
        "hora_entrada": fecha_hora_entrada,
        "usuario_entrada_nombre": usuario_entrada_nombre,
    });

    let respuesta = cliente
        .post(format!("{}/rest/v1/movimientos_visita", contexto.base_url))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Movimientos de visita (cola), cierre: mismo criterio de "primero en
/// llegar gana" que `enviar_cierre_ingreso` -- el filtro `hora_salida=is.null`
/// hace que un cierre que llega tarde (el otro dispositivo del sitio ya lo
/// cerró) no afecte ninguna fila en vez de fallar.
fn enviar_cierre_movimiento_visita(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (fecha_hora_salida, usuario_salida_nombre): (Option<String>, Option<String>) = connection
        .query_row(
        "SELECT fecha_hora_salida, usuario_salida_nombre FROM movimientos_visita WHERE uuid = ?1",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let cuerpo = json!({
        "hora_salida": fecha_hora_salida,
        "dispositivo_salida_id": contexto.dispositivo_id,
        "usuario_salida_nombre": usuario_salida_nombre,
    });

    let url = format!(
        "{}/rest/v1/movimientos_visita?id=eq.{uuid}&hora_salida=is.null",
        contexto.base_url
    );

    let respuesta = cliente
        .patch(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Fila cacheada localmente de un ingreso todavía abierto, creado por el
/// otro dispositivo de este mismo sitio -- ver `ingresos_remotos` en
/// `database::schema` sobre por qué esto no es una fila de
/// `registro_ingresos`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoRemoto {
    pub uuid: String,
    pub contratista_nombre: String,
    pub hora_entrada: String,
    pub usuario_entrada_nombre: Option<String>,
    pub contratista_cedula: Option<String>,
    pub empresa_nombre: Option<String>,
    pub tipo_ingreso: Option<String>,
    pub medio_ingreso: Option<String>,
    pub gafete_numero: Option<i64>,
}

#[derive(serde::Deserialize)]
struct FilaIngresoRemoto {
    id: String,
    contratista_nombre: String,
    hora_entrada: String,
    usuario_entrada_nombre: Option<String>,
    dispositivo_entrada_id: String,
    contratista_cedula: Option<String>,
    empresa_nombre: Option<String>,
    tipo_ingreso: Option<String>,
    medio_ingreso: Option<String>,
    gafete_numero: Option<i64>,
}

#[derive(serde::Deserialize)]
struct FilaCierrePropioRemoto {
    id: String,
    hora_salida: String,
    usuario_salida_nombre: Option<String>,
}

/// Trae cierres que otro dispositivo registró en la nube sobre ingresos
/// que nacieron en esta base local. No usa el repositorio normal de salida:
/// ese camino siempre encola un cambio nuevo, y acá estamos aplicando un
/// hecho ya confirmado por el receptor.
///
/// El pedido a la nube se acota a lo que localmente sigue abierto
/// (`registro_ingresos.fecha_hora_salida IS NULL`) -- antes pedía TODO lo
/// que este dispositivo alguna vez cerró (`dispositivo_entrada_id=eq.<el
/// mío>&hora_salida=not.is.null`, sin ningún otro filtro), una lista que
/// sólo crece con la vida entera del dispositivo y se repetía completa cada
/// ciclo de sync (cada 2 minutos, para siempre) aunque el propio `UPDATE`
/// de más abajo (`WHERE fecha_hora_salida IS NULL`) ya descartaba en
/// silencio todo lo que no fuera nuevo. Con nadie abierto ahora mismo (el
/// caso normal fuera de horas pico) esto ahora ni siquiera pega la llamada.
pub fn recibir_cierres_de_ingresos_propios(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let abiertos_localmente: Vec<String> = {
        let mut statement = connection
            .prepare("SELECT uuid FROM registro_ingresos WHERE fecha_hora_salida IS NULL")?;
        statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?
    };
    if abiertos_localmente.is_empty() {
        return Ok(0);
    }

    let cliente = cliente_http();
    // PostgREST admite `in.(a,b,c)` -- un UUID nunca trae `,`/`)`/espacios,
    // así que unirlos con coma directo es seguro sin escapar nada.
    let lista_uuids = abiertos_localmente.join(",");
    let url = format!(
        "{}/rest/v1/ingresos?id=in.({lista_uuids})&hora_salida=not.is.null\
         &select=id,hora_salida,usuario_salida_nombre",
        contexto.base_url,
    );
    let filas: Vec<FilaCierrePropioRemoto> = obtener_json(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    let mut aplicados = 0_u32;
    for fila in &filas {
        let nombre_salida = fila
            .usuario_salida_nombre
            .as_deref()
            .unwrap_or("Salida registrada en nube");
        // El receptor devuelve el `timestamptz` de Postgres serializado a su
        // manera (milisegundos, offset "+00:00") -- no necesariamente el
        // formato único y reversible que exige `registro_ingresos_salida_utc`.
        // Se reparsea y reformatea acá antes de escribirlo local.
        let hora_salida = crate::tiempo::parsear_utc(&fila.hora_salida)
            .map(crate::tiempo::serializar_utc)
            .map_err(|_| SincronizacionError::FechaInvalida(fila.hora_salida.clone()))?;
        let filas_afectadas = transaction.execute(
            "
            UPDATE registro_ingresos
            SET
                fecha_hora_salida = ?1,
                usuario_salida_id = NULL,
                usuario_salida_nombre = ?2
            WHERE uuid = ?3
              AND fecha_hora_salida IS NULL
            ",
            params![hora_salida, nombre_salida, fila.id],
        )?;
        let filas_afectadas = u32::try_from(filas_afectadas).unwrap_or(u32::MAX);
        aplicados = aplicados.saturating_add(filas_afectadas);
    }
    transaction.commit()?;

    Ok(aplicados)
}

/// Refresca la caché local `ingresos_remotos` con lo que hay abierto ahora
/// mismo en la nube para este sitio -- de *cualquier* dispositivo, ya no
/// sólo "el otro" (`dispositivo_entrada_id=neq.<el mío>` como antes). Ese
/// filtro asumía que un abierto de este dispositivo siempre vive en su
/// `registro_ingresos` local, pero eso se rompe al reinstalar Android: la
/// base local pierde `registro_ingresos` (mismo caso que ya se documentó en
/// `recibir_historial_del_sitio`), y con el filtro viejo esos ingresos
/// desaparecían de la pantalla Activos para siempre -- ni locales (se
/// borraron) ni remotos (el filtro los excluía por ser "propios"). Ahora se
/// trae todo lo abierto del sitio y se descarta explícitamente lo que ya
/// vive en `registro_ingresos` (`existe_localmente` más abajo) -- así el
/// caso normal (nada se perdió) sigue sin duplicar nada, y el caso
/// reinstalado recupera lo suyo por el mismo camino que ya usa para lo
/// ajeno. Reemplaza el contenido entero de la tabla en una sola transacción
/// -- más simple que llevar la cuenta de qué cambió, y la tabla es chica
/// (sólo lo que está abierto ahora).
pub fn recibir_ingresos_abiertos(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<IngresoRemoto>, SincronizacionError> {
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/ingresos?sitio_id=eq.{}&hora_salida=is.null\
         &select=id,contratista_nombre,hora_entrada,usuario_entrada_nombre,dispositivo_entrada_id,\
         contratista_cedula,empresa_nombre,tipo_ingreso,medio_ingreso,gafete_numero",
        contexto.base_url, contexto.sitio_id,
    );
    let filas: Vec<FilaIngresoRemoto> = obtener_json(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "DELETE FROM ingresos_remotos WHERE sitio_id = ?1",
        params![contexto.sitio_id],
    )?;
    let mut remotos = Vec::with_capacity(filas.len());
    for fila in filas {
        // Lo que ya vive en `registro_ingresos` de este dispositivo sigue
        // siendo la fuente de verdad de ahí -- no se duplica en la caché
        // remota. Ver el doc-comment de la función.
        let existe_localmente: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM registro_ingresos WHERE uuid = ?1)",
            params![fila.id],
            |row| row.get(0),
        )?;
        if existe_localmente {
            continue;
        }
        // Mismo motivo que en `recibir_cierres_de_ingresos_propios`: el
        // receptor no devuelve necesariamente el formato único que usa el
        // resto de la app para persistir fechas.
        let hora_entrada = crate::tiempo::parsear_utc(&fila.hora_entrada)
            .map(crate::tiempo::serializar_utc)
            .map_err(|_| SincronizacionError::FechaInvalida(fila.hora_entrada.clone()))?;
        transaction.execute(
            "
            INSERT INTO ingresos_remotos (
                uuid, sitio_id, contratista_nombre, hora_entrada,
                usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en,
                contratista_cedula, empresa_nombre, tipo_ingreso, medio_ingreso, gafete_numero
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?7, ?8, ?9, ?10, ?11)
            ",
            params![
                fila.id,
                contexto.sitio_id,
                fila.contratista_nombre,
                hora_entrada,
                fila.usuario_entrada_nombre,
                fila.dispositivo_entrada_id,
                fila.contratista_cedula,
                fila.empresa_nombre,
                fila.tipo_ingreso,
                fila.medio_ingreso,
                fila.gafete_numero,
            ],
        )?;
        remotos.push(IngresoRemoto {
            uuid: fila.id,
            contratista_nombre: fila.contratista_nombre,
            hora_entrada,
            usuario_entrada_nombre: fila.usuario_entrada_nombre,
            contratista_cedula: fila.contratista_cedula,
            empresa_nombre: fila.empresa_nombre,
            tipo_ingreso: fila.tipo_ingreso,
            medio_ingreso: fila.medio_ingreso,
            gafete_numero: fila.gafete_numero,
        });
    }
    transaction.commit()?;

    Ok(remotos)
}

#[derive(serde::Deserialize)]
struct FilaGafeteOcupado {
    #[allow(dead_code)]
    id: String,
}

#[derive(serde::Deserialize)]
struct FilaUsuarioActivo {
    activo: bool,
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

/// Consulta en vivo -- no la caché local `ingresos_remotos` (que sólo se
/// refresca en cada sync y podría estar desactualizada por minutos) -- si
/// `numero` ya tiene un ingreso abierto en este sitio, creado por *otro*
/// dispositivo. Pensada para llamarse justo antes de confirmar un ingreso
/// nuevo con gafete: dos dispositivos del mismo sitio comparten el mismo
/// rango de gafetes físicos, pero cada uno valida contra su propia base
/// `SQLite` (`idx_registro_ingresos_gafete_activo`), que nunca ve lo que
/// hizo el otro hasta sincronizar -- de ahí que ambos pudieran aceptar el
/// mismo número como activo a la vez. No reserva nada del lado del
/// receptor: sigue existiendo una ventana muy angosta entre esta consulta
/// y que el ingreso realmente se drene a la cola de salida (ver
/// `drenar_cola`), pero cierra el caso normal (no perfectamente
/// simultáneo) que sí se pudo reproducir.
pub fn gafete_ocupado_en_otro_dispositivo(
    contexto: &ContextoSincronizacion<'_>,
    numero: i64,
) -> Result<bool, SincronizacionError> {
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/ingresos?sitio_id=eq.{}&dispositivo_entrada_id=neq.{}&hora_salida=is.null\
         &gafete_numero=eq.{numero}&select=id&limit=1",
        contexto.base_url, contexto.sitio_id, contexto.dispositivo_id,
    );
    let filas: Vec<FilaGafeteOcupado> = obtener_json(&cliente, contexto, &url)?;
    Ok(!filas.is_empty())
}

#[derive(serde::Deserialize)]
struct SitioEmbebido {
    nombre: String,
}

#[derive(serde::Deserialize)]
struct FilaIngresoActivoOtroSitio {
    sitios: Option<SitioEmbebido>,
}

/// `docs/pendientes.md`, "Chequeo cruzado de ingresos abiertos entre
/// sitios": a diferencia de `gafete_ocupado_en_otro_dispositivo` (mismo
/// sitio, otro dispositivo), esto excluye el sitio ACTUAL en vez del
/// dispositivo actual -- lo que se busca es si esta cédula tiene un ingreso
/// abierto en cualquier OTRA unidad operativa. Devuelve el nombre del sitio
/// donde está activo (para el mensaje al operador), o `None` si no hay
/// conflicto. Pensada para llamarse desde `preparar_ingreso`, con el mismo
/// criterio de "mejor esfuerzo, nunca bloqueante si no hay red" que ya usa
/// `usuario_sigue_activo_remoto` en el login -- sin conexión, el registro
/// sigue local (`docs/pendientes.md`: "offline, registrar y alertar luego
/// al sincronizar").
pub fn contratista_activo_en_otro_sitio(
    contexto: &ContextoSincronizacion<'_>,
    cedula: &str,
) -> Result<Option<String>, SincronizacionError> {
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/ingresos?contratista_cedula=eq.{cedula}&sitio_id=neq.{}&hora_salida=is.null\
         &select=sitios(nombre)&limit=1",
        contexto.base_url, contexto.sitio_id,
    );
    let filas: Vec<FilaIngresoActivoOtroSitio> = obtener_json(&cliente, contexto, &url)?;
    Ok(filas.into_iter().next().and_then(|fila| fila.sitios).map(|sitio| sitio.nombre))
}

#[derive(serde::Deserialize)]
struct DispositivoEmbebido {
    tipo: Option<String>,
}

#[derive(serde::Deserialize)]
struct FilaHistorialRemota {
    id: String,
    contratista_cedula: Option<String>,
    contratista_nombre: String,
    empresa_nombre: Option<String>,
    tipo_ingreso: Option<String>,
    medio_ingreso: Option<String>,
    hora_entrada: String,
    hora_salida: Option<String>,
    gafete_numero: Option<i64>,
    usuario_entrada_nombre: Option<String>,
    usuario_salida_nombre: Option<String>,
    resultado_acceso: Option<String>,
    motivo_resultado: Option<String>,
    reglas_version: Option<i64>,
    empresa_activa_snapshot: Option<bool>,
    dispositivo_entrada_id: String,
    dispositivo_salida_id: Option<String>,
    updated_at: String,
    /// `"pc"`/`"mobile"` (`dispositivos.tipo`) -- embebido vía `PostgREST`
    /// (`dispositivo_entrada:dispositivos!ingresos_dispositivo_entrada_id_fkey(tipo)`)
    /// para que la pantalla pueda mostrar de qué tipo de dispositivo vino
    /// un movimiento sin tener que resolver el UUID a mano. Pedido del
    /// usuario tras no poder diferenciar de un vistazo un movimiento de la
    /// PC de uno del celular en Historial.
    dispositivo_entrada: Option<DispositivoEmbebido>,
}

/// Trae a `historial_sitio` todo movimiento (abierto o cerrado) del sitio,
/// de cualquier dispositivo -- decisión explícita del usuario: "es la
/// misma operación vista desde dos dispositivos distintos", no un espejo
/// resumido. Sync incremental, mismo mecanismo que
/// `recibir_catalogo_del_sitio` (`historial_actualizado_hasta` en vez de
/// `catalogo_actualizado_hasta` -- ritmos de sync independientes). `ON
/// CONFLICT` actualiza en vez de insertar de nuevo: un movimiento que
/// nace abierto y se cierra después reaparece con `updated_at` más nuevo,
/// trayendo ya el cierre.
pub fn recibir_historial_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let cliente = cliente_http();

    let marca_anterior: Option<String> = connection.query_row(
        "SELECT historial_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let ahora_remoto_seguro = chrono::Utc::now();
    let marca_consulta =
        marca_historial_para_consulta(marca_anterior.as_deref(), ahora_remoto_seguro);
    let filtro_incremental = marca_consulta
        .as_ref()
        .map(|marca| format!("&updated_at=gt.{}", crate::tiempo::serializar_utc(*marca)))
        .unwrap_or_default();

    // No se excluye el dispositivo actual: tras reinstalar Android, la base
    // local pierde `registro_ingresos`, pero la nube sigue siendo la fuente
    // común del sitio. La UI móvil deduplica por `uuid` cuando el movimiento
    // existe en ambas fuentes.
    let url = format!(
        "{}/rest/v1/ingresos?sitio_id=eq.{}{filtro_incremental}\
         &select=id,contratista_cedula,contratista_nombre,empresa_nombre,tipo_ingreso,\
         medio_ingreso,hora_entrada,hora_salida,gafete_numero,usuario_entrada_nombre,\
         usuario_salida_nombre,resultado_acceso,motivo_resultado,reglas_version,\
         empresa_activa_snapshot,dispositivo_entrada_id,dispositivo_salida_id,updated_at,\
         dispositivo_entrada:dispositivos!ingresos_dispositivo_entrada_id_fkey(tipo)",
        contexto.base_url, contexto.sitio_id,
    );

    // Página por página en vez de acumular todo el historial remoto en un
    // `Vec` antes de tocar la base -- ver el doc-comment de
    // `obtener_json_paginado_con` (hallazgo R-03). `marca_mas_nueva` viaja
    // de página en página: el orden entre páginas es por `id`, no por
    // `updated_at`, así que la marca final tiene que ser el máximo visto en
    // todas, no sólo en la última.
    let mut recibidos_total = 0_u32;
    let mut marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>> = marca_consulta;
    obtener_json_paginado_con(&cliente, contexto, &url, |pagina: Vec<FilaHistorialRemota>| {
        let (recibidos, marca_actualizada) =
            aplicar_pagina_historial(connection, contexto, &pagina, marca_mas_nueva)?;
        recibidos_total += recibidos;
        marca_mas_nueva = marca_actualizada;
        Ok(())
    })?;

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
fn aplicar_pagina_historial(
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
        let Ok(hora_entrada) = crate::tiempo::parsear_utc(&fila.hora_entrada)
            .map(crate::tiempo::serializar_utc)
        else {
            continue;
        };
        let hora_salida = match fila
            .hora_salida
            .as_deref()
            .map(crate::tiempo::parsear_utc)
            .transpose()
        {
            Ok(valor) => valor.map(crate::tiempo::serializar_utc),
            Err(_) => continue,
        };

        transaction.execute(
            "
            INSERT INTO historial_sitio (
                uuid, sitio_id, contratista_cedula, contratista_nombre, empresa_nombre,
                tipo_ingreso, medio_ingreso, hora_entrada, hora_salida, gafete_numero,
                usuario_entrada_nombre, usuario_salida_nombre, resultado_acceso,
                motivo_resultado, reglas_version, empresa_activa_snapshot,
                dispositivo_entrada_id, dispositivo_salida_id, actualizado_en,
                dispositivo_entrada_tipo
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)
            ON CONFLICT(uuid) DO UPDATE SET
                hora_salida = excluded.hora_salida,
                usuario_salida_nombre = excluded.usuario_salida_nombre,
                dispositivo_salida_id = excluded.dispositivo_salida_id,
                resultado_acceso = excluded.resultado_acceso,
                motivo_resultado = excluded.motivo_resultado,
                reglas_version = excluded.reglas_version,
                empresa_activa_snapshot = excluded.empresa_activa_snapshot,
                actualizado_en = excluded.actualizado_en,
                dispositivo_entrada_tipo = excluded.dispositivo_entrada_tipo
            ",
            params![
                fila.id,
                contexto.sitio_id,
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
                ahora,
                fila.dispositivo_entrada
                    .as_ref()
                    .and_then(|d| d.tipo.clone()),
            ],
        )?;
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
            params![crate::tiempo::serializar_utc(marca)],
        )?;
    }
    transaction.commit()?;
    Ok((recibidos, marca_mas_nueva))
}

fn marca_historial_para_consulta(
    marca_anterior: Option<&str>,
    ahora: chrono::DateTime<chrono::Utc>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    let marca = marca_anterior.and_then(|marca| crate::tiempo::parsear_utc(marca).ok())?;
    let base = if marca > ahora { ahora } else { marca };
    Some(base - chrono::Duration::days(DIAS_TRASLAPE_HISTORIAL))
}

#[derive(serde::Deserialize)]
struct FilaHistorialVisitaRemota {
    id: String,
    visitante_cedula: String,
    visitante_nombre: String,
    empresa: Option<String>,
    anfitrion_nombre: Option<String>,
    motivo: Option<String>,
    gafete_numero: Option<i64>,
    hora_entrada: String,
    hora_salida: Option<String>,
    usuario_entrada_nombre: Option<String>,
    usuario_salida_nombre: Option<String>,
    dispositivo_entrada_id: String,
    dispositivo_salida_id: Option<String>,
    updated_at: String,
}

/// Análogo a `FilaHistorialResultado` (contratistas) -- misma resiliencia:
/// una fila con fecha ilegible se omite sin abortar el resto del lote.
enum FilaHistorialVisitaResultado {
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
fn guardar_fila_historial_visita(
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
            ahora,
        ],
    )?;

    Ok(FilaHistorialVisitaResultado::Aplicada {
        actualizado_en: crate::tiempo::parsear_utc(&fila.updated_at).ok(),
    })
}

/// Trae a `historial_visitas_sitio` todo movimiento de visita (abierto o
/// cerrado) del sitio, de cualquier dispositivo -- mismo mecanismo
/// incremental que `recibir_historial_del_sitio` (marca de agua propia,
/// `historial_visitas_actualizado_hasta`, mismo traslape de
/// `DIAS_TRASLAPE_HISTORIAL` días).
pub fn recibir_historial_visitas_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let cliente = cliente_http();

    let marca_anterior: Option<String> = connection.query_row(
        "SELECT historial_visitas_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let ahora_remoto_seguro = chrono::Utc::now();
    let marca_consulta =
        marca_historial_para_consulta(marca_anterior.as_deref(), ahora_remoto_seguro);
    let filtro_incremental = marca_consulta
        .as_ref()
        .map(|marca| format!("&updated_at=gt.{}", crate::tiempo::serializar_utc(*marca)))
        .unwrap_or_default();

    let url = format!(
        "{}/rest/v1/movimientos_visita?sitio_id=eq.{}{filtro_incremental}\
         &select=id,visitante_cedula,visitante_nombre,empresa,anfitrion_nombre,motivo,\
         gafete_numero,hora_entrada,hora_salida,usuario_entrada_nombre,usuario_salida_nombre,\
         dispositivo_entrada_id,dispositivo_salida_id,updated_at",
        contexto.base_url, contexto.sitio_id,
    );
    let filas: Vec<FilaHistorialVisitaRemota> = obtener_json_paginado(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    let mut recibidos = 0_u32;
    let ahora = crate::tiempo::serializar_utc(chrono::Utc::now());
    let mut marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>> = marca_consulta;
    for fila in &filas {
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
            params![crate::tiempo::serializar_utc(marca)],
        )?;
    }
    transaction.commit()?;
    Ok(recibidos)
}

/// Nombre del anfitrión, embebido vía `PostgREST`
/// (`anfitrion:anfitriones!citas_anfitrion_correo_fkey(nombre)`) -- la
/// política de `anfitriones` que deja leerlo desde un dispositivo del sitio
/// vive en la migración `autoriza_lectura_anfitrion_por_dispositivo_del_sitio`.
/// `Option` porque un embed que RLS filtra vuelve `null`, no un error --
/// nunca debería pasar dado que esa política ya existe, pero no hay forma
/// de que el tipo lo garantice.
#[derive(serde::Deserialize)]
struct AnfitrionEmbebido {
    nombre: String,
}

#[derive(serde::Deserialize)]
struct FilaCitaVisitanteRemota {
    id: String,
    cedula: String,
    nombre: String,
    empresa: Option<String>,
    placa_vehiculo: Option<String>,
}

#[derive(serde::Deserialize)]
struct FilaCitaRemota {
    id: String,
    motivo: Option<String>,
    // `date` de Postgres, no `timestamptz` -- PostgREST ya lo manda como
    // "YYYY-MM-DD" sin hora, mismo formato que espera `citas.fecha_desde`/
    // `fecha_hasta` local -- a diferencia de `hora_entrada`/`updated_at` en
    // otras filas remotas, esto no necesita reparsear/reformatear.
    fecha_desde: String,
    fecha_hasta: String,
    /// Texto libre tipo "HH:MM", puramente informativo -- ver el
    /// doc-comment de `MIGRACION_33` del lado local.
    hora_estimada: Option<String>,
    anfitrion_correo: String,
    anfitrion: Option<AnfitrionEmbebido>,
    estado: String,
    updated_at: String,
    // Embebido en la misma consulta (`cita_visitantes(...)`) -- un solo
    // viaje de red trae la cita completa con su grupo, en vez de una
    // consulta aparte por cada una.
    cita_visitantes: Vec<FilaCitaVisitanteRemota>,
}

/// Guarda una cita y su grupo de visitantes; devuelve el `updated_at`
/// parseado si se aplicó, o `None` si se omitió por una fecha ilegible --
/// mismo criterio de resiliencia que `aplicar_pagina_historial`: una fila mala
/// no puede abortar la transacción entera y dejar sin citas a un
/// dispositivo que necesita traerlas todas (recién reinstalado, sin marca
/// de agua todavía).
fn guardar_cita_remota(
    transaction: &rusqlite::Transaction<'_>,
    fila: &FilaCitaRemota,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, SincronizacionError> {
    let Ok(actualizado_en) = crate::tiempo::parsear_utc(&fila.updated_at) else {
        return Ok(None);
    };

    let anfitrion_nombre = fila
        .anfitrion
        .as_ref()
        .map_or("—", |anfitrion| anfitrion.nombre.as_str());

    transaction.execute(
        "
        INSERT INTO citas (
            uuid, motivo, fecha_desde, fecha_hasta, hora_estimada, anfitrion_nombre,
            anfitrion_correo, estado, creado_en
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(uuid) DO UPDATE SET
            motivo = excluded.motivo,
            fecha_desde = excluded.fecha_desde,
            fecha_hasta = excluded.fecha_hasta,
            hora_estimada = excluded.hora_estimada,
            anfitrion_nombre = excluded.anfitrion_nombre,
            anfitrion_correo = excluded.anfitrion_correo,
            estado = excluded.estado
        ",
        params![
            fila.id,
            fila.motivo,
            fila.fecha_desde,
            fila.fecha_hasta,
            fila.hora_estimada,
            anfitrion_nombre,
            fila.anfitrion_correo,
            fila.estado,
            crate::tiempo::serializar_utc(actualizado_en),
        ],
    )?;

    let cita_id_local: i64 = transaction.query_row(
        "SELECT id FROM citas WHERE uuid = ?1",
        params![fila.id],
        |row| row.get(0),
    )?;

    for visitante in &fila.cita_visitantes {
        transaction.execute(
            "
            INSERT INTO cita_visitantes (uuid, cita_id, cedula, nombre, empresa, placa_vehiculo)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(uuid) DO UPDATE SET
                cita_id = excluded.cita_id,
                cedula = excluded.cedula,
                nombre = excluded.nombre,
                empresa = excluded.empresa,
                placa_vehiculo = excluded.placa_vehiculo
            ",
            params![
                visitante.id,
                cita_id_local,
                visitante.cedula,
                visitante.nombre,
                visitante.empresa,
                visitante.placa_vehiculo,
            ],
        )?;
    }

    Ok(Some(actualizado_en))
}

/// Trae a `citas`/`cita_visitantes` las citas que aplican a este sitio --
/// mismo mecanismo incremental que `recibir_historial_del_sitio`
/// (`citas_actualizado_hasta`, columna propia, ritmo de sync independiente),
/// pero sin filtro explícito de `sitio_id` en la URL: a diferencia de
/// `ingresos`/`gafetes`, una cita no tiene una columna de sitio directa
/// (vive en `cita_sitios`, el puente muchos-a-muchos) -- la política RLS
/// "leer citas propias, del sitio, o admin" ya resuelve ese filtro del lado
/// del servidor, agregarlo acá sería repetir la misma pregunta dos veces.
pub fn recibir_citas_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let cliente = cliente_http();

    let marca_anterior: Option<String> = connection.query_row(
        "SELECT citas_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let filtro_incremental = marca_anterior
        .as_deref()
        .map(|marca| format!("&updated_at=gt.{marca}"))
        .unwrap_or_default();

    let url = format!(
        "{}/rest/v1/citas?select=id,motivo,fecha_desde,fecha_hasta,hora_estimada,\
         anfitrion_correo,estado,updated_at,\
         anfitrion:anfitriones!citas_anfitrion_correo_fkey(nombre),\
         cita_visitantes(id,cedula,nombre,empresa,placa_vehiculo){filtro_incremental}",
        contexto.base_url,
    );
    let filas: Vec<FilaCitaRemota> = obtener_json_paginado(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    let mut recibidas = 0_u32;
    let mut marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>> = marca_anterior
        .as_deref()
        .and_then(|marca| crate::tiempo::parsear_utc(marca).ok());
    for fila in &filas {
        let Some(actualizado_en) = guardar_cita_remota(&transaction, fila)? else {
            continue;
        };
        recibidas += 1;
        if marca_mas_nueva.is_none_or(|marca| actualizado_en > marca) {
            marca_mas_nueva = Some(actualizado_en);
        }
    }

    if let Some(marca) = marca_mas_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado SET citas_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_utc(marca)],
        )?;
    }
    transaction.commit()?;
    Ok(recibidas)
}

/// Cuántas filas se aplicaron localmente al traer el catálogo del sitio --
/// para que la pantalla pueda avisar "3 contratistas nuevos" sin devolver
/// las filas enteras.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResumenCatalogo {
    pub empresas_recibidas: u32,
    pub contratistas_recibidos: u32,
    pub usuarios_recibidos: u32,
    pub gafetes_recibidos: u32,
}

#[derive(serde::Deserialize)]
struct FilaEmpresaRemota {
    id: String,
    nombre: String,
    activa: bool,
    updated_at: String,
}

/// Sin `password_hash` -- nunca viaja, ver el doc-comment de
/// `enviar_usuario`. `rol` puede ser 'ROOT'/'ADMINISTRADOR'/'OPERADOR' (ver
/// migración `permite_root_en_usuarios_globales`).
#[derive(serde::Deserialize)]
struct FilaUsuarioRemota {
    id: String,
    cedula: String,
    nombre: String,
    rol: String,
    activo: bool,
    updated_at: String,
}

#[derive(serde::Deserialize)]
struct FilaContratistaRemota {
    id: String,
    nombre: String,
    identificacion: Option<String>,
    empresa_id: Option<String>,
    empresa_nombre: Option<String>,
    activo: bool,
    tipo_ingreso: Option<String>,
    fecha_vencimiento_praind: Option<String>,
    es_personal_ruta: Option<bool>,
    updated_at: String,
}

/// A diferencia de empresas/contratistas/usuarios (globales), gafetes SÍ
/// se filtra por `sitio_id` -- cada sitio tiene su propio catálogo físico
/// de gafetes, no tiene sentido que uno global exista en todos.
#[derive(serde::Deserialize)]
struct FilaGafeteRemota {
    id: String,
    numero: i64,
    estado: String,
    contratista_deudor_id: Option<String>,
    contratista_deudor_nombre: Option<String>,
    updated_at: String,
}

/// Trae de la nube las empresas y contratistas de *este mismo sitio* que
/// este dispositivo todavía no tiene localmente -- el "pull" que le
/// faltaba al espejo (hasta ahora sólo empujaba: local → nube, nunca al
/// revés). Usa la misma política RLS que ya existe, sin tocarla, así que
/// sólo trae lo del propio sitio -- esto no es el seed global entre
/// sitios (`docs/plan-persistencia-nube.md`, diferido), sólo lo que el
/// otro dispositivo de este sitio ya empujó.
///
/// Mismo patrón `ON CONFLICT` que usa el archivo de seed
/// (`contratistas_base_final_limpia_v15.sql`): si ya existe localmente una
/// fila con el mismo nombre/cédula (la creó este dispositivo, o un import
/// anterior), la actualiza y le completa el `uuid` en vez de duplicarla --
/// `COALESCE(tabla.uuid, excluded.uuid)` nunca pisa un `uuid` que ya tenía.
///
/// Una fila remota sin `identificacion`/`tipo_ingreso` (contratista creado
/// antes de que el espejo mandara estos campos) se salta -- ambos son
/// `NOT NULL` en la tabla local, y en una app de control de acceso no se
/// inventan datos de clasificación para completar el hueco.
/// Resultado de la descarga (sin tocar la base todavía) -- separado de
/// `recibir_catalogo_del_sitio` sólo para que esa función no crezca más allá
/// del límite de líneas del lint `too_many_lines`; el corte natural ya
/// existía en el código (todo el HTTP primero, todo el `INSERT` después).
struct CatalogoRemotoDescargado {
    empresas: Vec<FilaEmpresaRemota>,
    contratistas: Vec<FilaContratistaRemota>,
    usuarios: Vec<FilaUsuarioRemota>,
    gafetes: Vec<FilaGafeteRemota>,
    marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>>,
}

/// Trae empresas/contratistas/usuarios/gafetes, cada uno incremental según
/// su propia marca -- ver los comentarios que tenían estas mismas consultas
/// en `recibir_catalogo_del_sitio` antes del corte.
fn descargar_catalogo_remoto(
    contexto: &ContextoSincronizacion<'_>,
    marca_anterior: Option<&str>,
    marca_gafetes_anterior: Option<&str>,
) -> Result<CatalogoRemotoDescargado, SincronizacionError> {
    let cliente = cliente_http();
    let filtro_incremental = marca_anterior
        .map(|marca| format!("&updated_at=gt.{marca}"))
        .unwrap_or_default();

    // Sin `sitio_id=eq...` a propósito -- contratistas y empresas son
    // globales (ver docs/plan-panel-administrativo-web.md, "Modelo de
    // datos"): si a un contratista se le niega el acceso en un sitio, tiene
    // que quedar negado en TODOS. El nombre de la función quedó del modelo
    // viejo (un solo sitio por dispositivo); lo que trae ahora es el
    // catálogo global completo, no "del sitio" de `contexto`.
    let empresas: Vec<FilaEmpresaRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/empresas?select=id,nombre,activa,updated_at{filtro_incremental}",
            contexto.base_url
        ),
    )?;
    let contratistas: Vec<FilaContratistaRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/contratistas?select=id,nombre,identificacion,empresa_id,empresa_nombre,\
             activo,tipo_ingreso,fecha_vencimiento_praind,es_personal_ruta,updated_at{filtro_incremental}",
            contexto.base_url
        ),
    )?;
    let usuarios: Vec<FilaUsuarioRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/usuarios?select=id,cedula,nombre,rol,activo,updated_at{filtro_incremental}",
            contexto.base_url
        ),
    )?;
    // Incremental con marca PROPIA (`gafetes_actualizado_hasta`, no la misma
    // `marca_anterior` de arriba) -- antes se descargaba completo cada vez,
    // justamente para no depender de un cursor compartido que podía ser
    // anterior a que gafetes se sumara al pull. Una columna propia (nace en
    // `NULL`) resuelve eso sin ayuda: el primer sync de cualquier
    // dispositivo siempre baja todo. La otra razón de bajar todo siempre
    // -- reintentar gafetes PERDIDOS cuyo deudor todavía no resolvía
    // localmente -- la resuelve `guardar_gafetes`, capando cuánto puede
    // avanzar esta marca. Con `sitio_id=eq...` a diferencia de las tres de
    // arriba -- ver comentario de `FilaGafeteRemota`.
    let filtro_incremental_gafetes = marca_gafetes_anterior
        .map(|marca| format!("&updated_at=gt.{marca}"))
        .unwrap_or_default();
    let gafetes: Vec<FilaGafeteRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/gafetes?sitio_id=eq.{}&select=id,numero,estado,contratista_deudor_id,\
             contratista_deudor_nombre,updated_at{filtro_incremental_gafetes}",
            contexto.base_url, contexto.sitio_id
        ),
    )?;

    // Máximo `updated_at` real entre empresas/contratistas/usuarios --
    // gafetes lleva su propia marca por separado (`guardar_gafetes` la
    // calcula después, capada por lo que haya quedado pendiente de
    // resolver, ver su doc-comment). Sin filas nuevas, la marca no avanza
    // -- preferible repetir la misma consulta (ya sabemos que no trae
    // nada) a arriesgar perder una fila por un reloj local desviado.
    let mut marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>> =
        marca_anterior.and_then(|marca| crate::tiempo::parsear_utc(marca).ok());
    for actualizado_en in empresas
        .iter()
        .map(|f| &f.updated_at)
        .chain(contratistas.iter().map(|f| &f.updated_at))
        .chain(usuarios.iter().map(|f| &f.updated_at))
    {
        let actualizado_en = crate::tiempo::parsear_utc(actualizado_en)
            .map_err(|_| SincronizacionError::FechaInvalida(actualizado_en.clone()))?;
        if marca_mas_nueva.is_none_or(|marca| actualizado_en > marca) {
            marca_mas_nueva = Some(actualizado_en);
        }
    }

    Ok(CatalogoRemotoDescargado {
        empresas,
        contratistas,
        usuarios,
        gafetes,
        marca_mas_nueva,
    })
}

fn guardar_empresas(
    transaction: &rusqlite::Transaction<'_>,
    empresas: &[FilaEmpresaRemota],
) -> Result<u32, SincronizacionError> {
    let mut recibidas = 0;
    for empresa in empresas {
        transaction.execute(
            "
            INSERT INTO empresas (nombre, activo, uuid) VALUES (?1, ?2, ?3)
            ON CONFLICT(nombre) DO UPDATE SET
                activo = excluded.activo,
                uuid = COALESCE(empresas.uuid, excluded.uuid)
            ",
            params![empresa.nombre, empresa.activa, empresa.id],
        )?;
        recibidas += 1;
    }
    Ok(recibidas)
}

fn guardar_contratistas(
    transaction: &rusqlite::Transaction<'_>,
    contratistas: &[FilaContratistaRemota],
) -> Result<u32, SincronizacionError> {
    // Un solo `SELECT` de todas las empresas locales antes del lote, en vez
    // de hasta dos por cada contratista remoto (`resolver_empresa_local`
    // anterior) -- con un catálogo grande eran miles de consultas
    // individuales secuenciales sólo para resolver el vínculo. `empresas`
    // ya está completa en este punto (`guardar_empresas` corrió antes, ver
    // `recibir_catalogo_del_sitio`) y esta función nunca la modifica, así
    // que el índice no se desactualiza durante el resto del lote.
    let indice_empresas = indexar_empresas(transaction)?;

    let mut recibidos = 0;
    for contratista in contratistas {
        let (Some(cedula), Some(tipo_ingreso), Some(es_personal_ruta)) = (
            contratista.identificacion.as_deref(),
            contratista.tipo_ingreso.as_deref(),
            contratista.es_personal_ruta,
        ) else {
            continue;
        };
        let empresa_id_local = indice_empresas.resolver(
            contratista.empresa_id.as_deref(),
            contratista.empresa_nombre.as_deref(),
        );
        let Some(empresa_id_local) = empresa_id_local else {
            continue;
        };

        transaction.execute(
            "
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, fecha_vencimiento_praind,
                es_personal_ruta, tiene_acceso, uuid
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(cedula) DO UPDATE SET
                nombre = excluded.nombre,
                empresa_id = excluded.empresa_id,
                tipo_ingreso = excluded.tipo_ingreso,
                fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
                es_personal_ruta = excluded.es_personal_ruta,
                tiene_acceso = excluded.tiene_acceso,
                uuid = COALESCE(contratistas.uuid, excluded.uuid)
            ",
            params![
                cedula,
                contratista.nombre,
                empresa_id_local,
                tipo_ingreso,
                contratista.fecha_vencimiento_praind,
                es_personal_ruta,
                contratista.activo,
                contratista.id,
            ],
        )?;
        recibidos += 1;
    }
    Ok(recibidos)
}

fn guardar_usuarios(
    transaction: &rusqlite::Transaction<'_>,
    usuarios: &[FilaUsuarioRemota],
) -> Result<u32, SincronizacionError> {
    let mut recibidos = 0;
    for usuario in usuarios {
        // `password_hash` queda AFUERA del `DO UPDATE SET` a propósito --
        // si esta cédula ya existe local (con una contraseña real, fijada
        // en este mismo dispositivo alguna vez), el `ON CONFLICT` no debe
        // tocarla nunca. Sólo un usuario recién llegado (el `INSERT` de la
        // primera vez) arranca con el centinela `SIN_PASSWORD_LOCAL`, que
        // `AutenticacionService::buscar_candidato` reconoce para mandar al
        // alta de contraseña en vez de "credenciales inválidas".
        transaction.execute(
            "
            INSERT INTO usuarios (cedula, nombre, rol, activo, password_hash, uuid)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(cedula) DO UPDATE SET
                nombre = excluded.nombre,
                rol = excluded.rol,
                activo = excluded.activo,
                uuid = COALESCE(usuarios.uuid, excluded.uuid)
            ",
            params![
                usuario.cedula,
                usuario.nombre,
                usuario.rol,
                usuario.activo,
                crate::services::password::SIN_PASSWORD_LOCAL,
                usuario.id,
            ],
        )?;
        recibidos += 1;
    }
    Ok(recibidos)
}

/// Devuelve cuántos gafetes se guardaron y hasta qué `updated_at` es seguro
/// avanzar `gafetes_actualizado_hasta` -- las dos cosas separadas porque no
/// necesariamente coinciden: un gafete PERDIDO cuyo deudor todavía no
/// resuelve localmente NO se guarda (violaría el `CHECK` de la tabla,
/// `estado = 'PERDIDO' AND contratista_deudor_id IS NOT NULL`), pero tiene
/// que seguir pidiéndose en el próximo sync hasta que resuelva -- si la
/// marca avanzara igual hasta su `updated_at`, ese gafete quedaría afuera
/// del filtro incremental (`updated_at=gt.marca`) para siempre y nunca más
/// se sabría de su deuda. Mientras quede alguno pendiente, la marca
/// simplemente no avanza nada este ciclo (`None`, el llamador deja
/// `gafetes_actualizado_hasta` como estaba) -- todo lo demás sí se guarda
/// igual (no tiene sentido demorar gafetes que sí resuelven sólo porque
/// otro, sin relación, sigue pendiente), sólo la marca de agua espera. Es
/// una regla deliberadamente conservadora (podría, en teoría, avanzar
/// parcialmente hasta el más viejo de los pendientes) a cambio de quedar
/// simple y obviamente correcta -- con un catálogo de gafetes chico
/// (acotado al físico de un sitio, no crece con la cantidad de
/// contratistas) el costo de repetir la consulta un ciclo más mientras algo
/// sigue pendiente es insignificante.
fn guardar_gafetes(
    transaction: &rusqlite::Transaction<'_>,
    gafetes: &[FilaGafeteRemota],
) -> Result<(u32, Option<chrono::DateTime<chrono::Utc>>), SincronizacionError> {
    // Mismo motivo que `indice_empresas` en `guardar_contratistas`: un solo
    // `SELECT` de todos los contratistas locales antes del lote, en vez de
    // hasta dos por cada gafete PERDIDO (`resolver_contratista_local`
    // anterior). `guardar_contratistas` ya corrió antes en el mismo
    // `recibir_catalogo_del_sitio` y esta función no modifica
    // `contratistas`, así que el índice se mantiene válido todo el lote.
    let indice_contratistas = indexar_contratistas(transaction)?;

    let mut recibidos = 0_u32;
    let mut marca_maxima_aplicada: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut quedo_pendiente = false;
    for gafete in gafetes {
        let actualizado_en = crate::tiempo::parsear_utc(&gafete.updated_at)
            .map_err(|_| SincronizacionError::FechaInvalida(gafete.updated_at.clone()))?;

        // Un gafete PERDIDO sin deudor resoluble localmente violaría el
        // `CHECK` de la tabla -- se salta por ahora, mismo criterio que un
        // contratista remoto incompleto: se autorresuelve solo en un sync
        // posterior, en cuanto ese contratista también llegue acá (ver el
        // doc-comment de la función sobre `quedo_pendiente`).
        let deudor_id_local = if gafete.estado == "PERDIDO" {
            let Some(id) = indice_contratistas.resolver(
                gafete.contratista_deudor_id.as_deref(),
                gafete.contratista_deudor_nombre.as_deref(),
            ) else {
                quedo_pendiente = true;
                continue;
            };
            Some(id)
        } else {
            None
        };

        transaction.execute(
            "
            INSERT INTO gafetes (numero, estado, contratista_deudor_id, uuid)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(numero) DO UPDATE SET
                estado = excluded.estado,
                contratista_deudor_id = excluded.contratista_deudor_id,
                uuid = COALESCE(gafetes.uuid, excluded.uuid)
            ",
            params![gafete.numero, gafete.estado, deudor_id_local, gafete.id],
        )?;
        recibidos += 1;
        if marca_maxima_aplicada.is_none_or(|marca| actualizado_en > marca) {
            marca_maxima_aplicada = Some(actualizado_en);
        }
    }

    let marca_segura = if quedo_pendiente {
        None
    } else {
        marca_maxima_aplicada
    };
    Ok((recibidos, marca_segura))
}

pub fn recibir_catalogo_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<ResumenCatalogo, SincronizacionError> {
    // Sync incremental (MIGRACION_23): sin esto, cada ciclo (cada 2 minutos,
    // para siempre) traía las tres tablas COMPLETAS aunque nada hubiera
    // cambiado. `marca_anterior` es NULL la primera vez (sembrado inicial,
    // sin filtro -- hay que traer todo lo que ya existe). La marca nueva se
    // calcula a partir del `updated_at` real que devolvió el servidor -- no
    // del reloj de este dispositivo: un reloj local apenas adelantado
    // respecto al del servidor dejaba la marca "en el futuro", y cualquier
    // fila con `updated_at` real por debajo quedaba fuera de
    // `WHERE updated_at > marca` para siempre, sin ningún error visible
    // (bug real, reproducido en producción).
    let marca_anterior: Option<String> = connection.query_row(
        "SELECT catalogo_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    // Marca propia de gafetes (MIGRACION_27) -- ver el doc-comment de
    // `guardar_gafetes` sobre por qué no comparte `marca_anterior`.
    let marca_gafetes_anterior: Option<String> = connection.query_row(
        "SELECT gafetes_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let descarga = descargar_catalogo_remoto(
        contexto,
        marca_anterior.as_deref(),
        marca_gafetes_anterior.as_deref(),
    )?;

    let transaction = connection.unchecked_transaction()?;
    // Orden explícito (no evaluación implícita de un literal de struct):
    // `guardar_contratistas` necesita que `guardar_empresas` ya haya
    // corrido (resuelve `empresa_id` local), y `guardar_gafetes` necesita
    // que `guardar_contratistas` ya haya corrido (resuelve
    // `contratista_deudor_id` local) -- ver sus propios comentarios.
    let empresas_recibidas = guardar_empresas(&transaction, &descarga.empresas)?;
    let contratistas_recibidos = guardar_contratistas(&transaction, &descarga.contratistas)?;
    let usuarios_recibidos = guardar_usuarios(&transaction, &descarga.usuarios)?;
    let (gafetes_recibidos, marca_gafetes_nueva) =
        guardar_gafetes(&transaction, &descarga.gafetes)?;
    let resumen = ResumenCatalogo {
        empresas_recibidas,
        contratistas_recibidos,
        usuarios_recibidos,
        gafetes_recibidos,
    };

    if let Some(marca) = descarga.marca_mas_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado SET catalogo_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_utc(marca)],
        )?;
    }
    if let Some(marca) = marca_gafetes_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado SET gafetes_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_utc(marca)],
        )?;
    }

    transaction.commit()?;
    Ok(resumen)
}

/// Índice en memoria de una tabla local por `uuid` y por `nombre`,
/// construido con un solo `SELECT` antes de recorrer un lote remoto -- ver
/// `indexar_empresas`/`indexar_contratistas`. Reemplaza a
/// `resolver_empresa_local`/`resolver_contratista_local`, que antes hacían
/// hasta dos `SELECT` individuales POR FILA del lote (miles de round-trips
/// secuenciales para un catálogo grande).
struct IndiceLocal {
    por_uuid: HashMap<String, i64>,
    por_nombre: HashMap<String, i64>,
}

impl IndiceLocal {
    /// Primero por `uuid` (el vínculo real), y si todavía no llegó acá por
    /// ese camino, por nombre (mismo respaldo que ya usaban las funciones
    /// que esto reemplaza). `nombre` no es único en ninguna de las dos
    /// tablas que usan esto -- misma ambigüedad que ya tenía el `SELECT
    /// ... WHERE nombre = ?1` original (sin `ORDER BY`, ya devolvía una
    /// fila arbitraria entre varias); acá el índice simplemente se queda
    /// con la última que recorrió `indexar_empresas`/`indexar_contratistas`.
    fn resolver(&self, uuid: Option<&str>, nombre: Option<&str>) -> Option<i64> {
        uuid.and_then(|uuid| self.por_uuid.get(uuid).copied())
            .or_else(|| nombre.and_then(|nombre| self.por_nombre.get(nombre).copied()))
    }
}

fn indexar_empresas(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<IndiceLocal, SincronizacionError> {
    let mut statement = transaction.prepare("SELECT id, uuid, nombre FROM empresas")?;
    let filas = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut por_uuid = HashMap::new();
    let mut por_nombre = HashMap::new();
    for fila in filas {
        let (id, uuid, nombre) = fila?;
        if let Some(uuid) = uuid {
            por_uuid.insert(uuid, id);
        }
        por_nombre.insert(nombre, id);
    }
    Ok(IndiceLocal {
        por_uuid,
        por_nombre,
    })
}

fn indexar_contratistas(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<IndiceLocal, SincronizacionError> {
    let mut statement = transaction.prepare("SELECT id, uuid, nombre FROM contratistas")?;
    let filas = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut por_uuid = HashMap::new();
    let mut por_nombre = HashMap::new();
    for fila in filas {
        let (id, uuid, nombre) = fila?;
        if let Some(uuid) = uuid {
            por_uuid.insert(uuid, id);
        }
        por_nombre.insert(nombre, id);
    }
    Ok(IndiceLocal {
        por_uuid,
        por_nombre,
    })
}

/// Cierra, directo contra la nube, un ingreso que abrió el otro
/// dispositivo del mismo sitio (mismo `PATCH` condicional que
/// `enviar_cierre_ingreso` -- "primero en llegar gana"). Nunca toca
/// `registro_ingresos` local, sólo la caché `ingresos_remotos`: este
/// ingreso no es -- y nunca fue -- del historial de este dispositivo.
pub fn cerrar_ingreso_remoto(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
    usuario_salida_nombre: &str,
) -> Result<(), SincronizacionError> {
    let cliente = cliente_http();
    let cuerpo = json!({
        "hora_salida": crate::tiempo::serializar_utc(chrono::Utc::now()),
        "dispositivo_salida_id": contexto.dispositivo_id,
        "usuario_salida_nombre": usuario_salida_nombre,
    });

    let url = format!(
        "{}/rest/v1/ingresos?id=eq.{uuid}&hora_salida=is.null",
        contexto.base_url
    );
    let respuesta = cliente
        .patch(url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)?;

    connection.execute(
        "DELETE FROM ingresos_remotos WHERE uuid = ?1",
        params![uuid],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };

    use crate::database::schema::initialize_database;

    use super::*;

    fn servidor_de_una_respuesta(respuesta: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
        let direccion = listener.local_addr().expect("dirección local");
        thread::spawn(move || {
            let Ok((mut conexion, _)) = listener.accept() else {
                return;
            };
            conexion
                .set_read_timeout(Some(Duration::from_millis(200)))
                .expect("set_read_timeout");
            let mut buffer = [0_u8; 4096];
            loop {
                match conexion.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(_leidos) => {}
                }
            }
            let _ = conexion.write_all(respuesta.as_bytes());
            let _ = conexion.flush();
        });
        format!("http://{direccion}")
    }

    /// Como `servidor_de_una_respuesta`, pero para pruebas que disparan más
    /// de un `GET` (`recibir_catalogo_del_sitio` pide primero empresas y
    /// luego contratistas) -- una respuesta por conexión aceptada, en
    /// orden. Cada respuesta debe traer `Connection: close` para que el
    /// cliente abra una conexión nueva en el siguiente pedido.
    fn servidor_de_respuestas(respuestas: Vec<&'static str>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
        let direccion = listener.local_addr().expect("dirección local");
        thread::spawn(move || {
            for respuesta in respuestas {
                let Ok((mut conexion, _)) = listener.accept() else {
                    return;
                };
                conexion
                    .set_read_timeout(Some(Duration::from_millis(200)))
                    .expect("set_read_timeout");
                let mut buffer = [0_u8; 4096];
                loop {
                    match conexion.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(_leidos) => {}
                    }
                }
                let _ = conexion.write_all(respuesta.as_bytes());
                let _ = conexion.flush();
            }
        });
        format!("http://{direccion}")
    }

    /// Prueba directa del hallazgo R-06: antes, `pendientes()` filtraba con
    /// una expresión (`datetime(actualizado_en, ...)`) que SQLite no podía
    /// resolver con ningún índice -- cada `drenar_cola` escaneaba toda
    /// `cola_salida` pendiente. Con `proximo_intento_en` como columna
    /// generada e indexada (`MIGRACION_27`), el plan de consulta real de
    /// `pendientes()` debe usar `idx_cola_salida_pendientes` en vez de un
    /// `SCAN cola_salida`.
    #[test]
    fn la_consulta_de_pendientes_usa_el_indice_no_un_escaneo_completo() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();

        let plan: String = connection
            .query_row(
                "EXPLAIN QUERY PLAN
                 SELECT id, entidad, entidad_uuid, operacion, intentos FROM cola_salida
                 WHERE estado = 'pendiente' AND proximo_intento_en <= datetime('now')
                 ORDER BY creado_en
                 LIMIT 200",
                [],
                |row| row.get::<_, String>(3),
            )
            .unwrap();

        assert!(
            plan.contains("idx_cola_salida_pendientes"),
            "esperaba que el plan usara el índice, se obtuvo: {plan}"
        );
        assert!(
            !plan.to_uppercase().contains("SCAN"),
            "esperaba una búsqueda por índice, no un escaneo completo: {plan}"
        );
    }

    fn conexion_con_contratista() -> (Connection, String) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute("INSERT INTO empresas (nombre) VALUES ('Brisas')", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO contratistas (
                    cedula, nombre, empresa_id, tipo_ingreso,
                    es_personal_ruta, tiene_acceso, uuid
                ) VALUES ('1-2345', 'Persona de prueba', 1, 'SWAT', 0, 1, 'uuid-contratista')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO cola_salida (
                    entidad, entidad_uuid, operacion, creado_en, actualizado_en
                ) VALUES ('contratista', 'uuid-contratista', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        (connection, "uuid-contratista".to_string())
    }

    fn contexto(base_url: &str) -> ContextoSincronizacion<'_> {
        ContextoSincronizacion {
            base_url,
            apikey: "clave-de-prueba",
            token: "token-de-prueba",
            dispositivo_id: "dispositivo-1",
            sitio_id: "sitio-1",
        }
    }

    #[test]
    fn envia_una_empresa_pendiente_y_la_marca_enviada() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO empresas (nombre, activo, uuid)
                 VALUES ('Brisas', 1, 'uuid-empresa')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO cola_salida (
                    entidad, entidad_uuid, operacion, creado_en, actualizado_en
                ) VALUES ('empresa', 'uuid-empresa', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        );

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(
            resumen,
            ResumenDrenado {
                enviados: 1,
                fallidos: 0
            }
        );
        let estado: String = connection
            .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
            .unwrap();
        assert_eq!(estado, "enviado");
    }

    #[test]
    fn envia_un_gafete_pendiente_y_lo_marca_enviado() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO gafetes (numero, estado, uuid)
                 VALUES (5, 'DISPONIBLE', 'uuid-gafete')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO cola_salida (
                    entidad, entidad_uuid, operacion, creado_en, actualizado_en
                ) VALUES ('gafete', 'uuid-gafete', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        );

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(
            resumen,
            ResumenDrenado {
                enviados: 1,
                fallidos: 0
            }
        );
        let estado: String = connection
            .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
            .unwrap();
        assert_eq!(estado, "enviado");
    }

    #[test]
    fn envia_un_contratista_pendiente_y_lo_marca_enviado() {
        let (connection, _uuid) = conexion_con_contratista();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        );

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(
            resumen,
            ResumenDrenado {
                enviados: 1,
                fallidos: 0
            }
        );
        let estado: String = connection
            .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
            .unwrap();
        assert_eq!(estado, "enviado");
    }

    /// Tres empresas nuevas -- sin relación entre sí, así que ninguna
    /// depende de que otra fila del lote se haya aplicado antes.
    fn conexion_con_tres_empresas() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        for n in 1..=3 {
            connection
                .execute(
                    &format!(
                        "INSERT INTO empresas (nombre, activo, uuid)
                         VALUES ('Empresa {n}', 1, 'uuid-empresa-{n}')"
                    ),
                    [],
                )
                .unwrap();
            connection
                .execute(
                    &format!(
                        "INSERT INTO cola_salida (
                            entidad, entidad_uuid, operacion, creado_en, actualizado_en
                        ) VALUES ('empresa', 'uuid-empresa-{n}', 'crear', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
                    ),
                    [],
                )
                .unwrap();
        }
        connection
    }

    #[test]
    fn agrupa_varias_filas_del_mismo_tipo_y_las_manda_en_un_solo_lote() {
        let connection = conexion_con_tres_empresas();
        // Una sola respuesta -- si `drenar_cola` mandara una petición por
        // fila (comportamiento viejo), la segunda y tercera empresa se
        // quedarían sin servidor que les conteste y la prueba fallaría por
        // timeout/error de red en vez de pasar.
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        );

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(
            resumen,
            ResumenDrenado {
                enviados: 3,
                fallidos: 0
            }
        );
        let enviadas: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM cola_salida WHERE estado = 'enviado'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(enviadas, 3);
    }

    #[test]
    fn si_el_lote_completo_falla_cae_a_mandar_cada_fila_por_separado() {
        let connection = conexion_con_tres_empresas();
        // Primera respuesta (al intento de lote): error -- simula que
        // Postgres rechazó el array entero por una sola fila mala. El
        // fallback reintenta las tres filas por separado, pero acá sólo se
        // preparan dos respuestas más a propósito: la prueba verifica que
        // esas dos se marcan `enviado` igual, y que a la tercera (sin
        // respuesta esperándola, como un error de red real) no se la
        // pierde ni se la cuenta como enviada -- queda `pendiente` para el
        // próximo intento.
        let base_url = servidor_de_respuestas(vec![
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"error\":\"fila invalida\"}",
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        ]);

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        // El lote falló una vez (no cuenta como fallo por fila), y las tres
        // filas individuales se reintentaron dentro de la misma llamada:
        // dos con éxito. La tercera no tiene respuesta preparada -- se
        // queda pendiente, igual que pasaría con un error de red real.
        assert_eq!(resumen.enviados, 2);
        let pendientes: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM cola_salida WHERE estado = 'pendiente'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(pendientes, 1, "la tercera fila queda para reintentar, no perdida ni duplicada");
    }

    #[test]
    fn error_del_receptor_deja_la_fila_pendiente_para_reintentar_con_el_motivo_guardado() {
        let (connection, _uuid) = conexion_con_contratista();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"boom\"}",
        );

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(
            resumen,
            ResumenDrenado {
                enviados: 0,
                fallidos: 1
            }
        );
        let (estado, intentos, ultimo_error): (String, i64, Option<String>) = connection
            .query_row(
                "SELECT estado, intentos, ultimo_error FROM cola_salida",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        // "pendiente", no "fallido" -- un solo fallo todavía se reintenta
        // solo (con backoff), no es un fallo permanente.
        assert_eq!(estado, "pendiente");
        assert_eq!(intentos, 1);
        assert!(ultimo_error.unwrap().contains("500"));
    }

    #[test]
    fn una_fila_recien_fallida_no_se_reintenta_de_inmediato_por_el_backoff() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO cola_salida (
                    entidad, entidad_uuid, operacion, estado, intentos,
                    creado_en, actualizado_en
                ) VALUES (
                    'contratista', 'uuid-x', 'crear', 'pendiente', 1,
                    '2026-01-01T00:00:00Z', strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                )",
                [],
            )
            .unwrap();
        // Nunca levanta un servidor: si `pendientes()` la trajera igual, la
        // conexión fallaría y el test lo detectaría por el resumen.
        let resumen = drenar_cola(&connection, &contexto("http://127.0.0.1:1"), 10).unwrap();

        assert_eq!(
            resumen,
            ResumenDrenado {
                enviados: 0,
                fallidos: 0
            }
        );
    }

    #[test]
    fn tras_agotar_los_reintentos_la_fila_queda_fallida_de_forma_permanente() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO cola_salida (
                    entidad, entidad_uuid, operacion, estado, intentos,
                    creado_en, actualizado_en
                ) VALUES (
                    'contratista', 'uuid-x', 'crear', 'pendiente',
                    ?1, '2026-01-01T00:00:00Z', '2020-01-01T00:00:00Z'
                )",
                params![INTENTOS_ANTES_DE_FALLO_PERMANENTE - 1],
            )
            .unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\
             Connection: close\r\n\r\n{\"error\":\"boom\"}",
        );

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(
            resumen,
            ResumenDrenado {
                enviados: 0,
                fallidos: 1
            }
        );
        let estado: String = connection
            .query_row("SELECT estado FROM cola_salida", [], |row| row.get(0))
            .unwrap();
        assert_eq!(estado, "fallido");
        assert_eq!(contar_fallos_permanentes(&connection).unwrap(), 1);
    }

    /// Cita + visitante + movimiento de visita completo (con snapshot),
    /// listo para encolar -- devuelve el `uuid` del movimiento.
    fn conexion_con_movimiento_de_visita(fecha_hora_salida: Option<&str>) -> (Connection, String) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
                 VALUES ('1', 'Guardia', 'h', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO citas (uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                    anfitrion_correo, estado, creado_en)
                 VALUES ('uuid-cita', '2026-01-01', '2026-01-02', 'Anfitrión',
                    'anfitrion@ejemplo.com', 'VIGENTE', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO cita_visitantes (uuid, cita_id, cedula, nombre)
                 VALUES ('uuid-visitante', 1, '1-2345', 'Persona Visitante')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO movimientos_visita (
                    uuid, cita_visitante_id, gafete_numero, fecha_hora_entrada,
                    fecha_hora_salida, usuario_entrada_id, usuario_entrada_nombre,
                    visitante_cedula, visitante_nombre, empresa, anfitrion_nombre, motivo
                ) VALUES (
                    'uuid-movimiento', 1, 12, '2026-01-01T08:00:00Z',
                    ?1, 1, 'Guardia',
                    '1-2345', 'Persona Visitante', 'Brisas', 'Anfitrión', 'Auditoría'
                )",
                params![fecha_hora_salida],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO cola_salida (
                    entidad, entidad_uuid, operacion, creado_en, actualizado_en
                ) VALUES (
                    'movimiento_visita', 'uuid-movimiento', ?1,
                    '2026-01-01T08:00:00Z', '2026-01-01T08:00:00Z'
                )",
                params![if fecha_hora_salida.is_some() {
                    "cerrar"
                } else {
                    "crear"
                }],
            )
            .unwrap();
        (connection, "uuid-movimiento".to_string())
    }

    #[test]
    fn envia_la_apertura_de_un_movimiento_de_visita() {
        let (connection, _) = conexion_con_movimiento_de_visita(None);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let servidor = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut pedido = Vec::new();
            let mut buffer = [0; 4096];
            while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                let leidos = socket.read(&mut buffer).unwrap();
                assert!(leidos > 0);
                pedido.extend_from_slice(&buffer[..leidos]);
            }
            let pedido = String::from_utf8(pedido).unwrap();
            assert!(pedido.starts_with("POST /rest/v1/movimientos_visita "));
            let cuerpo = "[]";
            write!(
                socket,
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
                cuerpo.len()
            )
            .unwrap();
        });

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(
            resumen,
            ResumenDrenado {
                enviados: 1,
                fallidos: 0
            }
        );
        servidor.join().unwrap();
    }

    #[test]
    fn envia_el_cierre_de_un_movimiento_de_visita() {
        let (connection, _) = conexion_con_movimiento_de_visita(Some("2026-01-01T09:00:00Z"));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let servidor = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut pedido = Vec::new();
            let mut buffer = [0; 4096];
            while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                let leidos = socket.read(&mut buffer).unwrap();
                assert!(leidos > 0);
                pedido.extend_from_slice(&buffer[..leidos]);
            }
            let pedido = String::from_utf8(pedido).unwrap();
            assert!(pedido.starts_with(
                "PATCH /rest/v1/movimientos_visita?id=eq.uuid-movimiento&hora_salida=is.null "
            ));
            write!(
                socket,
                "HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
            )
            .unwrap();
        });

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(
            resumen,
            ResumenDrenado {
                enviados: 1,
                fallidos: 0
            }
        );
        servidor.join().unwrap();
    }

    #[test]
    fn envia_la_apertura_de_un_ingreso() {
        let (connection, contratista_uuid) = conexion_con_contratista();
        connection.execute("DELETE FROM cola_salida", []).unwrap();
        connection
            .execute("INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo) VALUES ('1', 'Op', 'h', 'OPERADOR', 1)", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO registro_ingresos (
                    contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso, tipo_ingreso,
                    usuario_ingreso_id, contratista_cedula, contratista_nombre, empresa_nombre,
                    usuario_ingreso_nombre, es_personal_ruta, tiene_acceso, resultado_acceso,
                    reglas_version, uuid
                ) VALUES (
                    1, 1, '2026-01-01T08:00:00Z', 'CAMINANDO', 'SWAT',
                    1, '1-2345', 'Persona de prueba', 'Brisas',
                    'Op', 0, 1, 'PERMITIDO', 1, 'uuid-ingreso'
                )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO cola_salida (
                    entidad, entidad_uuid, operacion, creado_en, actualizado_en
                ) VALUES ('ingreso', 'uuid-ingreso', 'crear', '2026-01-01T08:00:00Z', '2026-01-01T08:00:00Z')",
                [],
            )
            .unwrap();
        let _ = &contratista_uuid;
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        );

        let resumen = drenar_cola(&connection, &contexto(&base_url), 10).unwrap();

        assert_eq!(
            resumen,
            ResumenDrenado {
                enviados: 1,
                fallidos: 0
            }
        );
    }

    #[test]
    fn recibe_ingresos_abiertos_del_otro_dispositivo_y_los_cachea() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-remoto\",\"contratista_nombre\":\"Persona Remota\",\
             \"hora_entrada\":\"2026-01-01T08:00:00Z\",\"usuario_entrada_nombre\":\"Op PC\",\
             \"dispositivo_entrada_id\":\"otro-dispositivo\"}]",
        );

        let recibidos = recibir_ingresos_abiertos(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(recibidos.len(), 1);
        assert_eq!(recibidos[0].uuid, "uuid-remoto");
        assert_eq!(recibidos[0].contratista_nombre, "Persona Remota");
        let cacheados: i64 = connection
            .query_row("SELECT COUNT(*) FROM ingresos_remotos", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(cacheados, 1);
    }

    #[test]
    fn recibir_reemplaza_la_cache_del_sitio_por_completo() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO ingresos_remotos (
                    uuid, sitio_id, contratista_nombre, hora_entrada,
                    usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en
                ) VALUES (
                    'ya-cerrado', 'sitio-1', 'Otra Persona', '2026-01-01T07:00:00Z',
                    NULL, 'otro-dispositivo', '2026-01-01T07:00:00Z'
                )",
                [],
            )
            .unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        );

        recibir_ingresos_abiertos(&connection, &contexto(&base_url)).unwrap();

        let cacheados: i64 = connection
            .query_row("SELECT COUNT(*) FROM ingresos_remotos", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            cacheados, 0,
            "lo que ya no viene en la respuesta se borra de la caché"
        );
    }

    #[test]
    fn recibe_historial_del_sitio_y_guarda_el_tipo_de_dispositivo_embebido() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"mov-1\",\"contratista_nombre\":\"Persona Remota\",\
             \"hora_entrada\":\"2026-01-01T08:00:00Z\",\
             \"dispositivo_entrada_id\":\"otro-dispositivo\",\
             \"updated_at\":\"2026-01-01T08:00:05Z\",\
             \"dispositivo_entrada\":{\"tipo\":\"mobile\"}}]",
        );

        let recibidos = recibir_historial_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(recibidos, 1);
        let tipo: Option<String> = connection
            .query_row(
                "SELECT dispositivo_entrada_tipo FROM historial_sitio WHERE uuid = 'mov-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tipo.as_deref(), Some("mobile"));
    }

    #[test]
    fn recibir_historial_del_sitio_incluye_movimientos_del_dispositivo_actual() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let servidor = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut pedido = Vec::new();
            let mut buffer = [0; 4096];
            while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                let leidos = socket.read(&mut buffer).unwrap();
                assert!(leidos > 0);
                pedido.extend_from_slice(&buffer[..leidos]);
            }
            let pedido = String::from_utf8(pedido).unwrap();
            assert!(pedido.contains("/ingresos?sitio_id=eq.sitio-1"));
            assert!(
                !pedido.contains("dispositivo_entrada_id=neq."),
                "una instalación nueva debe poder repoblar lo que antes generó este mismo dispositivo"
            );
            let cuerpo = "[]";
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
                cuerpo.len()
            )
            .unwrap();
        });

        recibir_historial_del_sitio(&connection, &contexto(&base_url)).unwrap();
        servidor.join().unwrap();
    }

    #[test]
    fn recibir_historial_paginado_persiste_todas_las_paginas_y_la_marca_de_agua_es_el_maximo_global() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();

        // Página 1: exactamente TAMANO_PAGINA_REMOTA filas -- señal de que
        // hay una página más. `updated_at` sube con cada fila, hasta
        // 2026-01-01T00:08:19Z en la última (fila 499, +499 segundos).
        let mut filas_pagina_1 = String::new();
        for i in 0..TAMANO_PAGINA_REMOTA {
            if i > 0 {
                filas_pagina_1.push(',');
            }
            let minutos = i / 60;
            let segundos = i % 60;
            filas_pagina_1.push_str(&format!(
                "{{\"id\":\"mov-{i:04}\",\"contratista_nombre\":\"Persona {i}\",\
                 \"hora_entrada\":\"2026-01-01T00:00:00Z\",\
                 \"dispositivo_entrada_id\":\"dispositivo-1\",\
                 \"updated_at\":\"2026-01-01T00:{minutos:02}:{segundos:02}Z\"}}"
            ));
        }
        let cuerpo_pagina_1 = format!("[{filas_pagina_1}]");

        // Página 2: una sola fila (menos que TAMANO_PAGINA_REMOTA, así que
        // es la última) con un `updated_at` MÁS VIEJO que el máximo de la
        // página 1 -- las páginas vienen ordenadas por `id`, no por
        // `updated_at`, así que esto puede pasar en la práctica. Si la
        // marca de agua final quedara en el valor de la última página en
        // vez del máximo global, esta prueba lo detecta.
        let cuerpo_pagina_2 =
            "[{\"id\":\"mov-pagina-2\",\"contratista_nombre\":\"Persona tardía\",\
             \"hora_entrada\":\"2026-01-01T00:00:00Z\",\
             \"dispositivo_entrada_id\":\"dispositivo-1\",\
             \"updated_at\":\"2026-01-01T00:00:01Z\"}]"
                .to_string();

        let respuesta_pagina_1 = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo_pagina_1}",
                cuerpo_pagina_1.len()
            )
            .into_boxed_str(),
        ) as &'static str;
        let respuesta_pagina_2 = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo_pagina_2}",
                cuerpo_pagina_2.len()
            )
            .into_boxed_str(),
        ) as &'static str;
        let base_url =
            servidor_de_respuestas(vec![respuesta_pagina_1, respuesta_pagina_2]);

        let recibidos = recibir_historial_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(recibidos, TAMANO_PAGINA_REMOTA as u32 + 1);
        let guardadas: i64 = connection
            .query_row("SELECT COUNT(*) FROM historial_sitio", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            guardadas,
            TAMANO_PAGINA_REMOTA as i64 + 1,
            "las filas de ambas páginas quedan persistidas, no sólo la última"
        );
        let marca: String = connection
            .query_row(
                "SELECT historial_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            marca, "2026-01-01T00:08:19Z",
            "la marca de agua es el máximo de TODAS las páginas, no el de la última"
        );
    }

    #[test]
    fn marca_historial_para_consulta_retrocede_una_semana() {
        let ahora = crate::tiempo::parsear_utc("2026-09-09T12:00:00Z").unwrap();

        let marca = marca_historial_para_consulta(Some("2026-09-09T10:00:00Z"), ahora);

        assert_eq!(
            marca,
            Some(crate::tiempo::parsear_utc("2026-09-02T10:00:00Z").unwrap())
        );
    }

    #[test]
    fn marca_historial_para_consulta_sanea_marcas_en_futuro() {
        let ahora = crate::tiempo::parsear_utc("2026-09-09T12:00:00Z").unwrap();

        let marca = marca_historial_para_consulta(Some("2026-12-01T00:00:00Z"), ahora);

        assert_eq!(
            marca,
            Some(crate::tiempo::parsear_utc("2026-09-02T12:00:00Z").unwrap())
        );
    }

    #[test]
    fn recibe_historial_del_sitio_sin_dispositivo_embebido_no_falla() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"mov-2\",\"contratista_nombre\":\"Persona Remota\",\
             \"hora_entrada\":\"2026-01-01T08:00:00Z\",\
             \"dispositivo_entrada_id\":\"otro-dispositivo\",\
             \"updated_at\":\"2026-01-01T08:00:05Z\"}]",
        );

        let recibidos = recibir_historial_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(recibidos, 1);
        let tipo: Option<String> = connection
            .query_row(
                "SELECT dispositivo_entrada_tipo FROM historial_sitio WHERE uuid = 'mov-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tipo, None, "sin embed, queda NULL en vez de fallar");
    }

    #[test]
    fn recibe_una_cita_con_su_grupo_de_visitantes_y_el_nombre_del_anfitrion_embebido() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"cita-1\",\"motivo\":\"Auditoría\",\
             \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-12\",\
             \"anfitrion_correo\":\"kof@brisas.com\",\
             \"anfitrion\":{\"nombre\":\"Persona Anfitriona\"},\
             \"estado\":\"VIGENTE\",\"updated_at\":\"2026-09-09T08:00:00Z\",\
             \"cita_visitantes\":[\
             {\"id\":\"visitante-1\",\"cedula\":\"1-1111\",\"nombre\":\"Visitante Uno\",\
             \"empresa\":null,\"placa_vehiculo\":null}]}]",
        );

        let recibidas = recibir_citas_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(recibidas, 1);
        let (anfitrion_nombre, estado): (String, String) = connection
            .query_row(
                "SELECT anfitrion_nombre, estado FROM citas WHERE uuid = 'cita-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(anfitrion_nombre, "Persona Anfitriona");
        assert_eq!(estado, "VIGENTE");
        let cedula: String = connection
            .query_row(
                "SELECT cedula FROM cita_visitantes WHERE uuid = 'visitante-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cedula, "1-1111");
    }

    #[test]
    fn recibe_la_hora_estimada_de_una_cita_y_la_admite_ausente() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"cita-hora\",\"motivo\":null,\
             \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-10\",\
             \"hora_estimada\":\"10:00:00\",\
             \"anfitrion_correo\":\"kof@brisas.com\",\"anfitrion\":null,\
             \"estado\":\"VIGENTE\",\"updated_at\":\"2026-09-09T08:00:00Z\",\
             \"cita_visitantes\":[]},\
             {\"id\":\"cita-sin-hora\",\"motivo\":null,\
             \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-10\",\
             \"anfitrion_correo\":\"kof@brisas.com\",\"anfitrion\":null,\
             \"estado\":\"VIGENTE\",\"updated_at\":\"2026-09-09T08:00:01Z\",\
             \"cita_visitantes\":[]}]",
        );

        let recibidas = recibir_citas_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(recibidas, 2);
        let hora_estimada: Option<String> = connection
            .query_row(
                "SELECT hora_estimada FROM citas WHERE uuid = 'cita-hora'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(hora_estimada.as_deref(), Some("10:00:00"));
        let sin_hora: Option<String> = connection
            .query_row(
                "SELECT hora_estimada FROM citas WHERE uuid = 'cita-sin-hora'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            sin_hora, None,
            "campo ausente en el JSON no debe fallar, sólo queda NULL"
        );
    }

    #[test]
    fn recibe_cita_sin_anfitrion_embebido_no_falla() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"cita-2\",\"motivo\":null,\
             \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-10\",\
             \"anfitrion_correo\":\"kof@brisas.com\",\"anfitrion\":null,\
             \"estado\":\"VIGENTE\",\"updated_at\":\"2026-09-09T08:00:00Z\",\
             \"cita_visitantes\":[]}]",
        );

        let recibidas = recibir_citas_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(recibidas, 1);
        let anfitrion_nombre: String = connection
            .query_row(
                "SELECT anfitrion_nombre FROM citas WHERE uuid = 'cita-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            anfitrion_nombre, "—",
            "sin embed (RLS lo filtró o no aplica), queda un placeholder en vez de fallar"
        );
    }

    #[test]
    fn una_fila_de_cita_con_fecha_ilegible_se_omite_sin_abortar_las_demas() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"cita-mala\",\"motivo\":null,\
             \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-10\",\
             \"anfitrion_correo\":\"kof@brisas.com\",\"anfitrion\":null,\
             \"estado\":\"VIGENTE\",\"updated_at\":\"no-es-una-fecha\",\
             \"cita_visitantes\":[]},\
             {\"id\":\"cita-buena\",\"motivo\":null,\
             \"fecha_desde\":\"2026-09-10\",\"fecha_hasta\":\"2026-09-10\",\
             \"anfitrion_correo\":\"kof@brisas.com\",\"anfitrion\":null,\
             \"estado\":\"VIGENTE\",\"updated_at\":\"2026-09-09T08:00:00Z\",\
             \"cita_visitantes\":[]}]",
        );

        let recibidas = recibir_citas_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(recibidas, 1, "la fila con updated_at ilegible no cuenta");
        let total: i64 = connection
            .query_row("SELECT COUNT(*) FROM citas", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 1, "solo se guardó la fila válida");
    }

    #[test]
    fn segunda_sincronizacion_de_citas_pide_solo_lo_actualizado_desde_la_marca_previa() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "UPDATE sincronizacion_estado SET citas_actualizado_hasta = '2026-09-09T08:00:00Z' WHERE id = 1",
                [],
            )
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let servidor = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut pedido = Vec::new();
            let mut buffer = [0; 4096];
            while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                let leidos = socket.read(&mut buffer).unwrap();
                assert!(leidos > 0);
                pedido.extend_from_slice(&buffer[..leidos]);
            }
            let pedido = String::from_utf8(pedido).unwrap();
            assert!(
                pedido.contains("updated_at=gt.2026-09-09T08%3A00%3A00Z")
                    || pedido.contains("updated_at=gt.2026-09-09T08:00:00Z")
            );
            assert!(
                !pedido.contains("sitio_id="),
                "sin filtro explícito de sitio -- RLS ya lo resuelve del lado del servidor"
            );
            let cuerpo = "[]";
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
                cuerpo.len()
            )
            .unwrap();
        });

        recibir_citas_del_sitio(&connection, &contexto(&base_url)).unwrap();
        servidor.join().unwrap();
    }

    #[test]
    fn recibe_el_historial_de_visitas_del_sitio_y_lo_guarda_local() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"mov-visita-1\",\"visitante_cedula\":\"1-1111\",\
             \"visitante_nombre\":\"Visitante Remoto\",\"empresa\":\"Brisas\",\
             \"anfitrion_nombre\":\"Anfitrión\",\"motivo\":\"Auditoría\",\
             \"gafete_numero\":9,\"hora_entrada\":\"2026-01-01T08:00:00Z\",\
             \"hora_salida\":null,\"usuario_entrada_nombre\":\"Guardia\",\
             \"usuario_salida_nombre\":null,\"dispositivo_entrada_id\":\"otro-dispositivo\",\
             \"dispositivo_salida_id\":null,\"updated_at\":\"2026-01-01T08:00:05Z\"}]",
        );

        let recibidos =
            recibir_historial_visitas_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(recibidos, 1);
        let (cedula, nombre, empresa, anfitrion): (String, String, Option<String>, Option<String>) =
            connection
                .query_row(
                    "SELECT visitante_cedula, visitante_nombre, empresa, anfitrion_nombre
                 FROM historial_visitas_sitio WHERE uuid = 'mov-visita-1'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .unwrap();
        assert_eq!(cedula, "1-1111");
        assert_eq!(nombre, "Visitante Remoto");
        assert_eq!(empresa.as_deref(), Some("Brisas"));
        assert_eq!(anfitrion.as_deref(), Some("Anfitrión"));
    }

    #[test]
    fn una_fila_de_historial_de_visitas_con_fecha_ilegible_se_omite_sin_abortar_las_demas() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"mov-mala\",\"visitante_cedula\":\"1-1111\",\"visitante_nombre\":\"X\",\
             \"empresa\":null,\"anfitrion_nombre\":null,\"motivo\":null,\"gafete_numero\":null,\
             \"hora_entrada\":\"no-es-una-fecha\",\"hora_salida\":null,\
             \"usuario_entrada_nombre\":null,\"usuario_salida_nombre\":null,\
             \"dispositivo_entrada_id\":\"otro-dispositivo\",\"dispositivo_salida_id\":null,\
             \"updated_at\":\"2026-01-01T08:00:00Z\"},\
             {\"id\":\"mov-buena\",\"visitante_cedula\":\"1-2222\",\"visitante_nombre\":\"Y\",\
             \"empresa\":null,\"anfitrion_nombre\":null,\"motivo\":null,\"gafete_numero\":null,\
             \"hora_entrada\":\"2026-01-01T08:00:00Z\",\"hora_salida\":null,\
             \"usuario_entrada_nombre\":null,\"usuario_salida_nombre\":null,\
             \"dispositivo_entrada_id\":\"otro-dispositivo\",\"dispositivo_salida_id\":null,\
             \"updated_at\":\"2026-01-01T08:00:05Z\"}]",
        );

        let recibidos =
            recibir_historial_visitas_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(recibidos, 1, "la fila con hora_entrada ilegible no cuenta");
        let total: i64 = connection
            .query_row("SELECT COUNT(*) FROM historial_visitas_sitio", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(total, 1, "solo se guardó la fila válida");
    }

    #[test]
    fn segunda_sincronizacion_de_historial_de_visitas_pide_solo_lo_actualizado_desde_la_marca_previa()
     {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "UPDATE sincronizacion_estado SET historial_visitas_actualizado_hasta = '2026-09-09T08:00:00Z' WHERE id = 1",
                [],
            )
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let servidor = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut pedido = Vec::new();
            let mut buffer = [0; 4096];
            while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                let leidos = socket.read(&mut buffer).unwrap();
                assert!(leidos > 0);
                pedido.extend_from_slice(&buffer[..leidos]);
            }
            let pedido = String::from_utf8(pedido).unwrap();
            assert!(pedido.contains("/movimientos_visita?sitio_id=eq.sitio-1"));
            let cuerpo = "[]";
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
                cuerpo.len()
            )
            .unwrap();
        });

        recibir_historial_visitas_del_sitio(&connection, &contexto(&base_url)).unwrap();
        servidor.join().unwrap();
    }

    #[test]
    fn usuario_sigue_activo_remoto_lee_el_booleano_de_una_sola_fila() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"activo\":false}]",
        );

        let activo = usuario_sigue_activo_remoto(&contexto(&base_url), "999999999").unwrap();

        assert!(!activo);
    }

    #[test]
    fn usuario_sigue_activo_remoto_sin_fila_asume_activo() {
        // Un usuario que este dispositivo creó y todavía no subió (o que
        // subió hace un instante y el receptor todavía no lo ve) -- la nube
        // no tiene nada que decir de él, no hay motivo para expulsarlo por
        // eso.
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        );

        let activo = usuario_sigue_activo_remoto(&contexto(&base_url), "ROOT1").unwrap();

        assert!(activo);
    }

    #[test]
    fn contratista_activo_en_otro_sitio_excluye_el_sitio_actual_en_la_url() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let servidor = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut pedido = Vec::new();
            let mut buffer = [0; 4096];
            while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                let leidos = socket.read(&mut buffer).unwrap();
                assert!(leidos > 0);
                pedido.extend_from_slice(&buffer[..leidos]);
            }
            let pedido = String::from_utf8(pedido).unwrap();
            assert!(pedido.contains("contratista_cedula=eq.2001"));
            assert!(pedido.contains("sitio_id=neq.sitio-1"));
            assert!(pedido.contains("hora_salida=is.null"));
            let cuerpo = "[{\"sitios\":{\"nombre\":\"Cartago\"}}]";
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}",
                cuerpo.len()
            )
            .unwrap();
        });

        let sitio = contratista_activo_en_otro_sitio(&contexto(&base_url), "2001").unwrap();

        assert_eq!(sitio, Some("Cartago".to_string()));
        servidor.join().unwrap();
    }

    #[test]
    fn contratista_activo_en_otro_sitio_sin_conflicto_devuelve_none() {
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        );

        let sitio = contratista_activo_en_otro_sitio(&contexto(&base_url), "2001").unwrap();

        assert_eq!(sitio, None);
    }

    #[test]
    fn recibe_cierres_de_ingresos_propios_sin_reencolar() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "
                INSERT INTO empresas (id, nombre, uuid) VALUES (1, 'Brisas', 'uuid-empresa');
                INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
                INSERT INTO contratistas (
                    id, cedula, nombre, empresa_id, tipo_ingreso,
                    es_personal_ruta, tiene_acceso, uuid
                ) VALUES (1, '2001', 'Persona', 1, 'SWAT', 0, 1, 'uuid-contratista');
                INSERT INTO registro_ingresos (
                    id, contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso,
                    tipo_ingreso, gafete_numero, usuario_ingreso_id, contratista_cedula,
                    contratista_nombre, empresa_nombre, usuario_ingreso_nombre,
                    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso,
                    resultado_acceso, motivo_resultado, reglas_version,
                    empresa_activa_snapshot, uuid
                ) VALUES (
                    1, 1, 1, '2026-01-01T08:00:00Z', 'CAMINANDO',
                    'SWAT', NULL, 1, '2001', 'Persona', 'Brisas', 'Operador',
                    NULL, 0, 1, 'PERMITIDO', NULL, 1, 1, 'uuid-ingreso'
                );
                ",
            )
            .unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-ingreso\",\"hora_salida\":\"2026-01-01T10:00:00Z\",\
             \"usuario_salida_nombre\":\"Operador remoto\"}]",
        );

        let aplicados =
            recibir_cierres_de_ingresos_propios(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(aplicados, 1);
        let (salida, usuario_id, usuario_nombre): (String, Option<i64>, String) = connection
            .query_row(
                "SELECT fecha_hora_salida, usuario_salida_id, usuario_salida_nombre
                 FROM registro_ingresos WHERE uuid = 'uuid-ingreso'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(salida, "2026-01-01T10:00:00Z");
        assert_eq!(usuario_id, None);
        assert_eq!(usuario_nombre, "Operador remoto");
        let reencolados: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM cola_salida
                 WHERE entidad = 'ingreso' AND entidad_uuid = 'uuid-ingreso'
                   AND operacion = 'cerrar'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(reencolados, 0);
    }

    #[test]
    fn sin_nada_abierto_localmente_no_llama_a_la_nube() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        // Base vacía -- ni una fila en `registro_ingresos`. Puerto sin nada
        // escuchando: si la función igual intentara pegarle a la red, esto
        // fallaría con un error de conexión en vez de devolver `Ok(0)`.
        let contexto = contexto("http://127.0.0.1:1");

        let aplicados = recibir_cierres_de_ingresos_propios(&connection, &contexto).unwrap();

        assert_eq!(aplicados, 0);
    }

    #[test]
    fn normaliza_la_fecha_de_salida_que_devuelve_postgrest_antes_de_guardarla() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "
                INSERT INTO empresas (id, nombre, uuid) VALUES (1, 'Brisas', 'uuid-empresa');
                INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
                INSERT INTO contratistas (
                    id, cedula, nombre, empresa_id, tipo_ingreso,
                    es_personal_ruta, tiene_acceso, uuid
                ) VALUES (1, '2001', 'Persona', 1, 'SWAT', 0, 1, 'uuid-contratista');
                INSERT INTO registro_ingresos (
                    id, contratista_id, empresa_id, fecha_hora_ingreso, medio_ingreso,
                    tipo_ingreso, gafete_numero, usuario_ingreso_id, contratista_cedula,
                    contratista_nombre, empresa_nombre, usuario_ingreso_nombre,
                    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso,
                    resultado_acceso, motivo_resultado, reglas_version,
                    empresa_activa_snapshot, uuid
                ) VALUES (
                    1, 1, 1, '2026-01-01T08:00:00Z', 'CAMINANDO',
                    'SWAT', NULL, 1, '2001', 'Persona', 'Brisas', 'Operador',
                    NULL, 0, 1, 'PERMITIDO', NULL, 1, 1, 'uuid-ingreso'
                );
                ",
            )
            .unwrap();
        // Formato real que devuelve `PostgREST` para un `timestamptz`
        // (fracción de segundo + offset "+00:00", no el "...Z" sin fracción
        // que exige `registro_ingresos_salida_utc`) -- este caso rompía la
        // sincronización en vivo aunque los tests con formato ya-canónico
        // pasaran.
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-ingreso\",\"hora_salida\":\"2026-01-01T10:00:00.123456+00:00\",\
             \"usuario_salida_nombre\":\"Operador remoto\"}]",
        );

        let aplicados =
            recibir_cierres_de_ingresos_propios(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(aplicados, 1);
        let salida: String = connection
            .query_row(
                "SELECT fecha_hora_salida FROM registro_ingresos WHERE uuid = 'uuid-ingreso'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(salida, "2026-01-01T10:00:00Z");
    }

    #[test]
    fn cierra_un_ingreso_remoto_y_lo_saca_de_la_cache() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO ingresos_remotos (
                    uuid, sitio_id, contratista_nombre, hora_entrada,
                    usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en
                ) VALUES (
                    'uuid-remoto', 'sitio-1', 'Persona Remota', '2026-01-01T08:00:00Z',
                    'Op PC', 'otro-dispositivo', '2026-01-01T08:00:00Z'
                )",
                [],
            )
            .unwrap();
        let base_url = servidor_de_una_respuesta(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        );

        cerrar_ingreso_remoto(
            &connection,
            &contexto(&base_url),
            "uuid-remoto",
            "Op Celular",
        )
        .unwrap();

        let cacheados: i64 = connection
            .query_row("SELECT COUNT(*) FROM ingresos_remotos", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(cacheados, 0);
    }

    fn fila_gafete(
        id: &str,
        numero: i64,
        estado: &str,
        deudor_id: Option<&str>,
        actualizado: &str,
    ) -> FilaGafeteRemota {
        FilaGafeteRemota {
            id: id.to_string(),
            numero,
            estado: estado.to_string(),
            contratista_deudor_id: deudor_id.map(str::to_string),
            contratista_deudor_nombre: None,
            updated_at: actualizado.to_string(),
        }
    }

    #[test]
    fn guardar_gafetes_marca_de_agua_avanza_al_mas_nuevo_cuando_nada_queda_pendiente() {
        let (connection, uuid_contratista) = conexion_con_contratista();
        let transaction = connection.unchecked_transaction().unwrap();
        let gafetes = vec![
            fila_gafete("g1", 1, "DISPONIBLE", None, "2026-01-01T00:00:00Z"),
            fila_gafete(
                "g2",
                2,
                "PERDIDO",
                Some(&uuid_contratista),
                "2026-01-02T00:00:00Z",
            ),
        ];

        let (recibidos, marca) = guardar_gafetes(&transaction, &gafetes).unwrap();

        assert_eq!(recibidos, 2);
        assert_eq!(
            marca.map(crate::tiempo::serializar_utc).as_deref(),
            Some("2026-01-02T00:00:00Z")
        );
    }

    #[test]
    fn guardar_gafetes_perdido_sin_deudor_resoluble_se_omite_y_no_avanza_la_marca() {
        let (connection, uuid_contratista) = conexion_con_contratista();
        let transaction = connection.unchecked_transaction().unwrap();
        let gafetes = vec![
            // Este sí resuelve y se guarda -- pero no debe hacer avanzar la
            // marca, porque el de abajo (más nuevo) se queda pendiente.
            fila_gafete(
                "g1",
                1,
                "PERDIDO",
                Some(&uuid_contratista),
                "2026-01-01T00:00:00Z",
            ),
            // Deudor que no existe localmente todavía -- se salta (violaría
            // el CHECK de la tabla) en vez de fallar toda la sincronización.
            fila_gafete(
                "g2",
                2,
                "PERDIDO",
                Some("uuid-contratista-que-no-llego-todavia"),
                "2026-01-02T00:00:00Z",
            ),
        ];

        let (recibidos, marca) = guardar_gafetes(&transaction, &gafetes).unwrap();

        // g1 se guardó (violaría el CHECK si no) pero la marca de agua no
        // avanza nada este ciclo -- si avanzara hasta el `updated_at` de g1
        // (o más), el próximo sync (`updated_at=gt.marca`) ya no volvería a
        // pedir g2, y su deuda quedaría sin resolver para siempre aunque el
        // contratista faltante llegue después.
        assert_eq!(recibidos, 1);
        let guardado: i64 = transaction
            .query_row("SELECT COUNT(*) FROM gafetes WHERE numero = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(guardado, 1);
        let pendiente: i64 = transaction
            .query_row("SELECT COUNT(*) FROM gafetes WHERE numero = 2", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(pendiente, 0);
        assert_eq!(marca, None);
    }

    #[test]
    fn recibe_estado_de_gafete_desde_nube_aunque_el_cursor_local_ya_exista() {
        let (connection, _) = conexion_con_contratista();
        connection
            .execute(
                "INSERT INTO gafetes (numero, estado) VALUES (26, 'DISPONIBLE')",
                [],
            )
            .unwrap();
        connection.execute(
            "UPDATE sincronizacion_estado SET catalogo_actualizado_hasta = '2099-01-01T00:00:00Z'", [],
        ).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let servidor = thread::spawn(move || {
            for paso in 0..4 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut pedido = Vec::new();
                let mut buffer = [0; 4096];
                while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                    let leidos = socket.read(&mut buffer).unwrap();
                    assert!(leidos > 0);
                    pedido.extend_from_slice(&buffer[..leidos]);
                }
                let cuerpo = if paso == 3 {
                    let pedido = String::from_utf8(pedido).unwrap();
                    assert!(pedido.contains("/gafetes?sitio_id=eq.sitio-1"));
                    // `updated_at` viaja en el `select=` (se necesita para la
                    // marca propia de gafetes), pero acá lo que importa es
                    // que NO haya filtro `updated_at=gt.` -- `gafetes_actualizado_hasta`
                    // nunca se tocó en este test (sólo `catalogo_actualizado_hasta`,
                    // la de arriba), así que su propio cursor sigue en NULL y
                    // pide todo, sin importar qué tan adelantado esté el otro.
                    assert!(
                        !pedido.contains("updated_at=gt."),
                        "el cursor de gafetes no debe heredar el de catálogo"
                    );
                    r#"[{"id":"gafete-remoto","numero":26,"estado":"PERDIDO","contratista_deudor_id":"uuid-contratista","contratista_deudor_nombre":null,"updated_at":"2026-01-01T00:00:00Z"}]"#
                } else {
                    "[]"
                };
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}", cuerpo.len()).unwrap();
            }
        });
        let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();
        servidor.join().unwrap();
        assert_eq!(resumen.gafetes_recibidos, 1);
        let estado: (String, i64) = connection
            .query_row(
                "SELECT estado, contratista_deudor_id FROM gafetes WHERE numero=26",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(estado, ("PERDIDO".to_string(), 1));
    }

    #[test]
    fn segundo_sync_de_catalogo_pide_gafetes_con_su_propia_marca_guardada() {
        let (connection, _) = conexion_con_contratista();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let servidor = thread::spawn(move || {
            // Dos syncs completos = 8 pedidos (empresas/contratistas/
            // usuarios/gafetes, dos veces). Sólo el de gafetes trae algo,
            // para que la marca de agua realmente tenga algo que guardar.
            for paso in 0..8 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut pedido = Vec::new();
                let mut buffer = [0; 4096];
                while !pedido.windows(4).any(|w| w == b"\r\n\r\n") {
                    let leidos = socket.read(&mut buffer).unwrap();
                    assert!(leidos > 0);
                    pedido.extend_from_slice(&buffer[..leidos]);
                }
                let cuerpo = if paso == 3 {
                    let pedido = String::from_utf8(pedido).unwrap();
                    assert!(
                        !pedido.contains("updated_at=gt."),
                        "primer sync: sin marca todavía, tiene que pedir todo"
                    );
                    r#"[{"id":"gafete-remoto","numero":26,"estado":"DISPONIBLE","contratista_deudor_id":null,"contratista_deudor_nombre":null,"updated_at":"2026-01-05T00:00:00Z"}]"#
                } else if paso == 7 {
                    let pedido = String::from_utf8(pedido).unwrap();
                    assert!(
                        pedido.contains("updated_at=gt.2026-01-05T00%3A00%3A00Z")
                            || pedido.contains("updated_at=gt.2026-01-05T00:00:00Z"),
                        "segundo sync: tiene que arrastrar la marca que dejó el primero -- pedido real: {pedido}"
                    );
                    "[]"
                } else {
                    "[]"
                };
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{cuerpo}", cuerpo.len()).unwrap();
            }
        });

        recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();
        let marca_guardada: Option<String> = connection
            .query_row(
                "SELECT gafetes_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(marca_guardada.as_deref(), Some("2026-01-05T00:00:00Z"));

        recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();
        servidor.join().unwrap();
    }

    #[test]
    fn recibe_catalogo_del_sitio_y_lo_guarda_local() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_respuestas(vec![
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-empresa-remota\",\"nombre\":\"Empresa Remota\",\"activa\":true,\
             \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-contratista-remoto\",\"nombre\":\"Persona Remota\",\
             \"identificacion\":\"1-1111\",\"empresa_id\":\"uuid-empresa-remota\",\
             \"empresa_nombre\":\"Empresa Remota\",\"activo\":true,\"tipo_ingreso\":\"SWAT\",\
             \"fecha_vencimiento_praind\":null,\"es_personal_ruta\":false,\
             \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        ]);

        let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(
            resumen,
            ResumenCatalogo {
                empresas_recibidas: 1,
                contratistas_recibidos: 1,
                usuarios_recibidos: 0,
                gafetes_recibidos: 0,
            }
        );
        let (nombre_empresa, uuid_empresa): (String, Option<String>) = connection
            .query_row("SELECT nombre, uuid FROM empresas", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(nombre_empresa, "Empresa Remota");
        assert_eq!(uuid_empresa.as_deref(), Some("uuid-empresa-remota"));

        let (cedula, tipo_ingreso, uuid_contratista): (String, String, Option<String>) = connection
            .query_row(
                "SELECT cedula, tipo_ingreso, uuid FROM contratistas",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(cedula, "1-1111");
        assert_eq!(tipo_ingreso, "SWAT");
        assert_eq!(uuid_contratista.as_deref(), Some("uuid-contratista-remoto"));
    }

    #[test]
    fn recibir_catalogo_fusiona_con_una_fila_local_existente_sin_duplicarla() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO empresas (nombre) VALUES ('Empresa Remota')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO contratistas (
                    cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
                ) VALUES ('1-1111', 'Persona Local', 1, 'SWAT', 0, 1)",
                [],
            )
            .unwrap();
        let base_url = servidor_de_respuestas(vec![
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-empresa-remota\",\"nombre\":\"Empresa Remota\",\"activa\":true,\
             \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-contratista-remoto\",\"nombre\":\"Persona Remota\",\
             \"identificacion\":\"1-1111\",\"empresa_id\":\"uuid-empresa-remota\",\
             \"empresa_nombre\":\"Empresa Remota\",\"activo\":true,\"tipo_ingreso\":\"SWAT\",\
             \"fecha_vencimiento_praind\":null,\"es_personal_ruta\":false,\
             \"updated_at\":\"2026-01-01T00:00:00Z\"}]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        ]);

        recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

        let total_empresas: i64 = connection
            .query_row("SELECT COUNT(*) FROM empresas", [], |row| row.get(0))
            .unwrap();
        let total_contratistas: i64 = connection
            .query_row("SELECT COUNT(*) FROM contratistas", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            total_empresas, 1,
            "no duplica la empresa que ya tenía por nombre"
        );
        assert_eq!(
            total_contratistas, 1,
            "no duplica el contratista que ya tenía por cédula"
        );
        let (nombre_final, uuid_final): (String, Option<String>) = connection
            .query_row("SELECT nombre, uuid FROM contratistas", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(
            nombre_final, "Persona Remota",
            "la fila local se actualiza con lo remoto"
        );
        assert_eq!(
            uuid_final.as_deref(),
            Some("uuid-contratista-remoto"),
            "le completa el uuid"
        );
    }

    #[test]
    fn recibir_catalogo_salta_un_contratista_remoto_sin_identificacion_o_tipo_ingreso() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_respuestas(vec![
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-incompleto\",\"nombre\":\"Persona Incompleta\",\
             \"identificacion\":null,\"empresa_id\":null,\"empresa_nombre\":null,\
             \"activo\":true,\"tipo_ingreso\":null,\"fecha_vencimiento_praind\":null,\
             \"es_personal_ruta\":null,\"updated_at\":\"2026-01-01T00:00:00Z\"}]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[]",
        ]);

        let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(resumen.contratistas_recibidos, 0);
        let total: i64 = connection
            .query_row("SELECT COUNT(*) FROM contratistas", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            total, 0,
            "una fila sin datos suficientes para las reglas de acceso no se inventa"
        );
    }
}
