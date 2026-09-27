use rusqlite::{Connection, params};
use serde_json::{Value, json};

use super::{ConflictoGafeteActivo, ContextoSincronizacion, ResumenDrenado, SincronizacionError};
use crate::nube::cliente::{NubeError, cliente_http};

pub(super) struct FilaCola {
    pub(super) id: i64,
    pub(super) entidad: String,
    pub(super) entidad_uuid: String,
    pub(super) operacion: String,
    pub(super) intentos: i64,
}

/// Después de esta cantidad de intentos fallidos seguidos, una fila deja de
/// reintentarse sola y pasa a `estado = 'fallido'` (terminal) -- sin este
/// tope, un dato irremediablemente roto (nunca va a poder mandarse, sea
/// cual sea la razón) se reintentaría cada 5 minutos para siempre, sin que
/// nadie se entere. Con el backoff de abajo, llegar acá lleva más de un día
/// real de reintentos -- no es un umbral que se cruce por una mala racha de
/// conexión.
pub(super) const INTENTOS_ANTES_DE_FALLO_PERMANENTE: i64 = 20;

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
pub(super) fn agrupar_por_entidad_y_operacion(filas: Vec<FilaCola>) -> Vec<Vec<FilaCola>> {
    let mut grupos: Vec<(String, String, Vec<FilaCola>)> = Vec::new();
    for fila in filas {
        let existente = grupos.iter_mut().find(|(entidad, operacion, _)| {
            *entidad == fila.entidad && *operacion == fila.operacion
        });
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
pub(super) fn destino_lote(
    entidad: &str,
    operacion: &str,
) -> Option<(&'static str, Option<&'static str>)> {
    match (entidad, operacion) {
        ("empresa", _) => Some(("empresas", Some("nombre"))),
        ("contratista", _) => Some(("contratistas", Some("identificacion"))),
        ("gafete", _) => Some(("gafetes", Some("sitio_id,numero"))),
        ("usuario", _) => Some(("usuarios", Some("cedula"))),
        ("ingreso" | "salida_ruta", "cerrar") => None,
        ("ingreso", _) => Some(("ingresos", None)),
        ("ruta", _) => Some(("rutas", Some("numero"))),
        ("vehiculo_ruta", _) => Some(("vehiculos_ruta", Some("placa"))),
        ("encargado_ruta", _) => Some(("encargados_ruta", Some("codigo_empleado"))),
        ("salida_ruta", _) => Some(("salidas_ruta", None)),
        ("empresa_proveedor", _) => Some(("empresas_proveedor", Some("nombre"))),
        _ => None,
    }
}

/// Arma el cuerpo local de una fila según su `entidad` -- dispatch que sólo
/// usa [`enviar_lote`] para construir varios cuerpos y mandarlos en un
/// array. El camino de una fila sola (`enviar_contratista` y compañía) no
/// pasa por acá: cada uno ya llama directo a su propio
/// `construir_cuerpo_*`.
pub(super) fn construir_cuerpo(
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
        "ruta" => construir_cuerpo_ruta(connection, contexto, uuid),
        "vehiculo_ruta" => construir_cuerpo_vehiculo_ruta(connection, contexto, uuid),
        "encargado_ruta" => construir_cuerpo_encargado_ruta(connection, contexto, uuid),
        "empresa_proveedor" => construir_cuerpo_empresa_proveedor(connection, contexto, uuid),
        otra => unreachable!("destino_lote ya filtró entidades sin lote (recibido: {otra})"),
    }
}

/// Manda un grupo entero -- todas la misma `(entidad, operacion)` -- en un
/// único `POST` con un array de cuerpos. `destino_lote` decide la tabla y el
/// `on_conflict`; si alguna fila del grupo ni siquiera se puede leer de la
/// base local (por ejemplo, se borró mientras esperaba en la cola), el error
/// sube tal cual y el llamador cae al camino fila por fila, donde esa fila
/// puntual va a fallar de nuevo con el mismo error y quedar registrada sola.
pub(super) fn enviar_lote(
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
        cuerpos.push(construir_cuerpo(
            connection,
            contexto,
            entidad,
            &fila.entidad_uuid,
        )?);
    }

    let url = on_conflict.map_or_else(
        || format!("{}/rest/v1/{tabla}", contexto.base_url),
        |on_conflict| {
            format!(
                "{}/rest/v1/{tabla}?on_conflict={on_conflict}",
                contexto.base_url
            )
        },
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
pub(super) fn drenar_grupo(
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
pub(super) fn procesar_fila_individual(
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
        ("ruta", _) => enviar_ruta(cliente, connection, contexto, &fila.entidad_uuid),
        ("movimiento_visita", "cerrar") => {
            enviar_cierre_movimiento_visita(cliente, connection, contexto, &fila.entidad_uuid)
        }
        ("movimiento_visita", _) => {
            enviar_movimiento_visita(cliente, connection, contexto, &fila.entidad_uuid)
        }
        ("vehiculo_ruta", _) => {
            enviar_vehiculo_ruta(cliente, connection, contexto, &fila.entidad_uuid)
        }
        ("encargado_ruta", _) => {
            enviar_encargado_ruta(cliente, connection, contexto, &fila.entidad_uuid)
        }
        ("salida_ruta", "cerrar") => {
            enviar_cierre_salida_ruta(cliente, connection, contexto, &fila.entidad_uuid)
        }
        ("salida_ruta", _) => enviar_salida_ruta(cliente, connection, contexto, &fila.entidad_uuid),
        ("prestamo_gafete_provisional", "cerrar") => enviar_cierre_prestamo_gafete_provisional(
            cliente,
            connection,
            contexto,
            &fila.entidad_uuid,
        ),
        ("prestamo_gafete_provisional", _) => {
            enviar_prestamo_gafete_provisional(cliente, connection, contexto, &fila.entidad_uuid)
        }
        ("empresa_proveedor", _) => {
            enviar_empresa_proveedor(cliente, connection, contexto, &fila.entidad_uuid)
        }
        ("ingreso_proveedor", "cerrar") => {
            enviar_cierre_ingreso_proveedor(cliente, connection, contexto, &fila.entidad_uuid)
        }
        ("ingreso_proveedor", _) => {
            enviar_ingreso_proveedor(cliente, connection, contexto, &fila.entidad_uuid)
        }
        _ => Ok(()),
    };

    match resultado {
        Ok(()) => {
            marcar(connection, fila.id, "enviado", None)?;
            resumen.enviados += 1;
        }
        Err(error)
            if fila.entidad == "ingreso"
                && fila.operacion != "cerrar"
                && es_conflicto_gafete_activo(&error) =>
        {
            // Fallo permanente por naturaleza -- Postgres ya rechazó este
            // gafete para este sitio del lado de OTRO dispositivo, nunca
            // va a dejar de fallar solo. No tiene sentido esperar los
            // `INTENTOS_ANTES_DE_FALLO_PERMANENTE` reintentos con backoff
            // de hasta un día -- eso es para fallas que sí pueden
            // resolverse solas (red, Postgres caído un rato), esto no.
            log::error!(
                "cola_salida: fila {} (ingreso {}) chocó con un gafete ya activo en otro dispositivo del sitio, queda fallida de inmediato: {error}",
                fila.id,
                fila.entidad_uuid,
            );
            marcar(connection, fila.id, "fallido", Some(&error.to_string()))?;
            resumen.fallidos += 1;
            resumen
                .conflictos_gafete
                .push(construir_conflicto_gafete(connection, &fila.entidad_uuid)?);
        }
        Err(error) => {
            // "pendiente" de nuevo -- no "fallido" -- para que
            // `pendientes()` la vuelva a considerar más adelante, sujeta
            // al backoff según cuántas veces ya falló.
            let agota_reintentos = fila.intentos + 1 >= INTENTOS_ANTES_DE_FALLO_PERMANENTE;
            let estado = if agota_reintentos {
                "fallido"
            } else {
                "pendiente"
            };
            // Antes esto sólo quedaba en `ultimo_error` (columna de la
            // propia fila) -- nadie se enteraba salvo que fuera a mirar la
            // cola a mano. Ver
            // docs/auditorias/plan-qa-buenas-practicas-2026-09-17.md, punto
            // 5.2. El caso "fallido" es el crítico de verdad: esa fila no
            // se va a reintentar más sola, alguien tiene que intervenir.
            if agota_reintentos {
                log::error!(
                    "cola_salida: fila {} ({} {}) agotó sus reintentos y quedó fallida de forma permanente: {error}",
                    fila.id,
                    fila.entidad,
                    fila.entidad_uuid,
                );
            } else {
                log::warn!(
                    "cola_salida: fila {} ({} {}) falló, intento {} de {}: {error}",
                    fila.id,
                    fila.entidad,
                    fila.entidad_uuid,
                    fila.intentos + 1,
                    INTENTOS_ANTES_DE_FALLO_PERMANENTE,
                );
            }
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
/// `docs/auditorias/auditoria-rendimiento-core-rust-2026-09-10.md`); ahora
/// es una columna de verdad, con `idx_cola_salida_pendientes` sobre ella.
pub(super) fn pendientes(
    connection: &Connection,
    limite: u32,
) -> Result<Vec<FilaCola>, SincronizacionError> {
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

pub(super) fn marcar(
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

pub(super) fn exigir_2xx(
    respuesta: reqwest::blocking::Response,
) -> Result<(), SincronizacionError> {
    if respuesta.status().is_success() {
        return Ok(());
    }
    let status = respuesta.status().as_u16();
    let cuerpo = respuesta.text().unwrap_or_default();
    Err(SincronizacionError::RespuestaInesperada { status, cuerpo })
}

/// Reconoce, por nombre, el `409` que Postgres devuelve cuando el índice
/// único `ingresos_gafete_activo_sitio_idx` (ver la migración que lo crea)
/// rechaza un `POST` porque otro dispositivo del mismo sitio ya tiene ese
/// gafete activo. No cualquier `23505` -- sólo ESTE índice, nombrado en el
/// cuerpo de la respuesta de `PostgREST` (`message`), para no confundirlo
/// con una violación de unicidad distinta que el día de mañana pudiera
/// darse por otro motivo.
pub(super) fn es_conflicto_gafete_activo(error: &SincronizacionError) -> bool {
    matches!(
        error,
        SincronizacionError::RespuestaInesperada { status: 409, cuerpo }
            if cuerpo.contains("ingresos_gafete_activo_sitio_idx")
    )
}

/// Arma [`ConflictoGafeteActivo`] con los mismos tres campos que ya lee
/// `construir_cuerpo_ingreso` de la fila local -- nada nuevo. `gafete_numero`
/// se lee como `i64` sin `Option`, sin `unwrap_or_default()`: el único
/// camino que llega hasta acá es un choque de GAFETE, así que si esta fila
/// no tuviera uno, `es_conflicto_gafete_activo` no la habría podido dejar
/// pasar -- si esa premisa alguna vez fallara, mejor que la conversión de
/// `rusqlite` reviente con un error real que fingir un gafete `0`.
pub(super) fn construir_conflicto_gafete(
    connection: &Connection,
    uuid: &str,
) -> Result<ConflictoGafeteActivo, SincronizacionError> {
    let (contratista_nombre, fecha_hora_ingreso, gafete_numero): (String, String, i64) = connection
        .query_row(
            "SELECT contratista_nombre, fecha_hora_ingreso, gafete_numero
             FROM registro_ingresos WHERE uuid = ?1",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    Ok(ConflictoGafeteActivo {
        contratista_nombre,
        gafete_numero,
        fecha_hora_ingreso,
    })
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
pub(super) fn construir_cuerpo_contratista(
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

pub(super) fn enviar_contratista(
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
pub(super) fn construir_cuerpo_empresa(
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

pub(super) fn enviar_empresa(
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
/// Sólo el estado actual (número, tipo, estado, a quién se le asignó la
/// última vez) -- el historial de incidentes (`gafetes_incidentes`) sigue
/// siendo puramente local, no viaja a la nube. `contratista_portador_id`/
/// `visita_portador_id` mandan el UUID real de la fila correspondiente
/// (`NULL` si el gafete no está `PERDIDO`, o si esa fila todavía no se
/// drenó -- mismo caso que `empresa_id` en `enviar_contratista`); a lo sumo
/// una de las dos tiene valor, igual que localmente.
/// Ver el doc-comment de [`construir_cuerpo_contratista`] -- misma idea.
/// Fila cruda para armar el cuerpo remoto de un gafete -- struct en vez de
/// una tupla de 7 elementos (`clippy::type_complexity`).
pub(super) struct FilaGafeteLocal {
    pub(super) numero: i64,
    pub(super) tipo: String,
    pub(super) estado: String,
    pub(super) contratista_portador_uuid: Option<String>,
    pub(super) contratista_portador_nombre: Option<String>,
    pub(super) visita_portador_uuid: Option<String>,
    pub(super) visita_portador_nombre: Option<String>,
}

pub(super) fn construir_cuerpo_gafete(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let fila: FilaGafeteLocal = connection.query_row(
        "
        SELECT g.numero, g.tipo, g.estado, c.uuid, c.nombre, cv.uuid, cv.nombre
        FROM gafetes g
        LEFT JOIN contratistas c ON c.id = g.contratista_portador_id
        LEFT JOIN cita_visitantes cv ON cv.id = g.visita_portador_id
        WHERE g.uuid = ?1
        ",
        params![uuid],
        |row| {
            Ok(FilaGafeteLocal {
                numero: row.get(0)?,
                tipo: row.get(1)?,
                estado: row.get(2)?,
                contratista_portador_uuid: row.get(3)?,
                contratista_portador_nombre: row.get(4)?,
                visita_portador_uuid: row.get(5)?,
                visita_portador_nombre: row.get(6)?,
            })
        },
    )?;

    Ok(json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "numero": fila.numero,
        "tipo": fila.tipo,
        "estado": fila.estado,
        "contratista_portador_id": fila.contratista_portador_uuid,
        "contratista_portador_nombre": fila.contratista_portador_nombre,
        "visita_portador_id": fila.visita_portador_uuid,
        "visita_portador_nombre": fila.visita_portador_nombre,
    }))
}

pub(super) fn enviar_gafete(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_gafete(connection, contexto, uuid)?;

    // `on_conflict=sitio_id,numero,tipo` -- mismo motivo que
    // `enviar_contratista`/`enviar_empresa`: el número de gafete es único
    // dentro de un sitio y un tipo (contratista/visita) aunque el `id`
    // remoto no coincida entre bases.
    let respuesta = cliente
        .post(format!(
            "{}/rest/v1/gafetes?on_conflict=sitio_id,numero,tipo",
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
pub(super) fn construir_cuerpo_usuario(
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

pub(super) fn enviar_usuario(
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
pub(super) fn construir_cuerpo_ingreso(
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
        placa,
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
        Option<String>,
    ) = connection.query_row(
        "
        SELECT contratista_id, contratista_nombre, fecha_hora_ingreso, usuario_ingreso_nombre,
               contratista_cedula, empresa_nombre, tipo_ingreso, medio_ingreso, gafete_numero,
               resultado_acceso, motivo_resultado, reglas_version, empresa_activa_snapshot, placa
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
                row.get(13)?,
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
        "placa": placa,
    }))
}

/// Ingresos (cola), apertura: mismo criterio de `upsert` que contratistas
/// -- reintentar un envío ya recibido no duplica nada.
pub(super) fn enviar_ingreso(
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
pub(super) fn enviar_cierre_ingreso(
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
pub(super) fn enviar_movimiento_visita(
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
pub(super) fn enviar_cierre_movimiento_visita(
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

/// Préstamos de gafete provisional KOF (cola, alta): mismo armazón que
/// `enviar_movimiento_visita`. `encargado_id` local es un `INTEGER` (fila
/// de `encargados_ruta`); lo que viaja al servidor es su `uuid` -- misma
/// resolución que `enviar_movimiento_visita` hace con `cita_visitante_id`.
#[allow(clippy::type_complexity)]
pub(super) fn enviar_prestamo_gafete_provisional(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (
        encargado_id_local,
        encargado_nombre,
        encargado_codigo_empleado,
        gafete_numero,
        fecha_hora_entrega,
        usuario_entrega_nombre,
    ): (i64, String, String, i64, String, String) = connection.query_row(
        "
        SELECT encargado_id, encargado_nombre, encargado_codigo_empleado, gafete_numero,
               fecha_hora_entrega, usuario_entrega_nombre
        FROM prestamos_gafete_provisional
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
            ))
        },
    )?;
    let encargado_uuid: String = connection.query_row(
        "SELECT uuid FROM encargados_ruta WHERE id = ?1",
        params![encargado_id_local],
        |row| row.get(0),
    )?;

    let cuerpo = json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_entrega_id": contexto.dispositivo_id,
        "encargado_id": encargado_uuid,
        "encargado_nombre": encargado_nombre,
        "encargado_codigo_empleado": encargado_codigo_empleado,
        "gafete_numero": gafete_numero,
        "hora_entrega": fecha_hora_entrega,
        "usuario_entrega_nombre": usuario_entrega_nombre,
    });

    let respuesta = cliente
        .post(format!(
            "{}/rest/v1/prestamos_gafete_provisional",
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

/// Préstamos de gafete provisional KOF (cola), cierre: mismo criterio de
/// "primero en llegar gana" que `enviar_cierre_movimiento_visita` -- el
/// filtro `hora_devolucion=is.null` hace que un cierre que llega tarde (el
/// otro dispositivo del sitio ya lo cerró) no afecte ninguna fila en vez de
/// fallar.
pub(super) fn enviar_cierre_prestamo_gafete_provisional(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (fecha_hora_devolucion, usuario_devolucion_nombre): (Option<String>, Option<String>) =
        connection.query_row(
            "SELECT fecha_hora_devolucion, usuario_devolucion_nombre
             FROM prestamos_gafete_provisional WHERE uuid = ?1",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

    let cuerpo = json!({
        "hora_devolucion": fecha_hora_devolucion,
        "dispositivo_devolucion_id": contexto.dispositivo_id,
        "usuario_devolucion_nombre": usuario_devolucion_nombre,
    });

    let url = format!(
        "{}/rest/v1/prestamos_gafete_provisional?id=eq.{uuid}&hora_devolucion=is.null",
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

/// Empresas proveedoras (`docs/features-futuras/plan-control-proveedores.md`,
/// espejo): mismo criterio de `upsert` que `enviar_empresa` -- catálogo
/// separado a propósito, `on_conflict=nombre` por el mismo motivo (sin
/// constraint de unicidad remota más que el `id`).
pub(super) fn construir_cuerpo_empresa_proveedor(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let (nombre, activo): (String, i64) = connection.query_row(
        "SELECT nombre, activo FROM empresas_proveedor WHERE uuid = ?1",
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

pub(super) fn enviar_empresa_proveedor(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_empresa_proveedor(connection, contexto, uuid)?;

    let respuesta = cliente
        .post(format!(
            "{}/rest/v1/empresas_proveedor?on_conflict=nombre",
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

/// Ingresos de proveedor (`docs/features-futuras/plan-control-proveedores.md`,
/// cola, alta): mismo armazón que `enviar_movimiento_visita` -- mismo
/// vocabulario entrada/salida (no entrega/devolución, eso es exclusivo de
/// KOF). `empresa_id` local es un `INTEGER` (fila de `empresas_proveedor`);
/// lo que viaja al servidor es su `uuid` -- misma resolución que
/// `enviar_movimiento_visita` hace con `cita_visitante_id`.
#[allow(clippy::type_complexity)]
pub(super) fn enviar_ingreso_proveedor(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (
        cedula,
        nombre,
        empresa_id_local,
        empresa_nombre,
        placa,
        gafete_numero,
        fecha_hora_ingreso,
        usuario_ingreso_nombre,
    ): (
        String,
        String,
        i64,
        String,
        Option<String>,
        i64,
        String,
        String,
    ) = connection.query_row(
        "
        SELECT cedula, nombre, empresa_id, empresa_nombre, placa, gafete_numero,
               fecha_hora_ingreso, usuario_ingreso_nombre
        FROM registro_ingresos_proveedor
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
            ))
        },
    )?;
    let empresa_uuid: String = connection.query_row(
        "SELECT uuid FROM empresas_proveedor WHERE id = ?1",
        params![empresa_id_local],
        |row| row.get(0),
    )?;

    let cuerpo = json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_entrada_id": contexto.dispositivo_id,
        "cedula": cedula,
        "nombre": nombre,
        "empresa_id": empresa_uuid,
        "empresa_nombre": empresa_nombre,
        "placa": placa,
        "gafete_numero": gafete_numero,
        "hora_entrada": fecha_hora_ingreso,
        "usuario_entrada_nombre": usuario_ingreso_nombre,
    });

    let respuesta = cliente
        .post(format!("{}/rest/v1/ingresos_proveedor", contexto.base_url))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Ingresos de proveedor (cola), cierre: mismo criterio de "primero en
/// llegar gana" que `enviar_cierre_movimiento_visita` -- el filtro
/// `hora_salida=is.null` hace que un cierre que llega tarde (el otro
/// dispositivo del sitio ya lo cerró) no afecte ninguna fila en vez de
/// fallar.
pub(super) fn enviar_cierre_ingreso_proveedor(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (fecha_hora_salida, usuario_salida_nombre): (Option<String>, Option<String>) = connection
        .query_row(
        "SELECT fecha_hora_salida, usuario_salida_nombre
             FROM registro_ingresos_proveedor WHERE uuid = ?1",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let cuerpo = json!({
        "hora_salida": fecha_hora_salida,
        "dispositivo_salida_id": contexto.dispositivo_id,
        "usuario_salida_nombre": usuario_salida_nombre,
    });

    let url = format!(
        "{}/rest/v1/ingresos_proveedor?id=eq.{uuid}&hora_salida=is.null",
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

/// Catálogo de números de ruta (`docs/planes-implementados/plan-control-rutas.md`,
/// pedido explícito del usuario, 2026-09-15) -- a diferencia de
/// vehículos/encargados (globales, `upsert` por clave natural sin
/// `sitio_id`), este catálogo SÍ es por sitio (como `gafetes`): cada fila
/// remota lleva `sitio_id`/`dispositivo_origen_id` y el `on_conflict` es
/// sólo `numero` (la migración de Supabase declara `UNIQUE(numero)` SIN
/// `sitio_id` a propósito -- dos sitios nunca comparten el mismo número
/// de ruta, ver la migración `catalogo_rutas_numeros_validos`).
pub(super) fn construir_cuerpo_ruta(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let (numero, activo): (i64, i64) = connection.query_row(
        "SELECT numero, activo FROM rutas WHERE uuid = ?1",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    Ok(json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "numero": numero,
        "activo": activo != 0,
    }))
}

/// `on_conflict=numero` -- ver el doc-comment de `construir_cuerpo_ruta`:
/// la unicidad remota real es sólo por número, no por `(sitio_id, numero)`
/// como `gafetes`.
pub(super) fn enviar_ruta(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_ruta(connection, contexto, uuid)?;

    let respuesta = cliente
        .post(format!(
            "{}/rest/v1/rutas?on_conflict=numero",
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

/// Control de rutas (`docs/planes-implementados/plan-control-rutas.md`) --
/// mismo espíritu que empresas (catálogo simple, `upsert` por clave
/// natural) para vehículos/encargados, y que
/// `enviar_movimiento_visita`/`enviar_cierre_movimiento_visita` para el
/// ciclo salida/retorno.
pub(super) fn construir_cuerpo_vehiculo_ruta(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let (numero_unidad, placa, activo): (Option<String>, String, i64) = connection.query_row(
        "SELECT numero_unidad, placa, activo FROM vehiculos_ruta WHERE uuid = ?1",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    Ok(json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "numero_unidad": numero_unidad,
        "placa": placa,
        "activo": activo != 0,
    }))
}

/// `on_conflict=placa` -- mismo motivo que `enviar_empresa`: sin esto, dos
/// bases locales sin el mismo `uuid` para el mismo vehículo generan cada
/// una un `id` propio y el upsert por PK las acepta como vehículos
/// distintos en vez de fusionarlas.
pub(super) fn enviar_vehiculo_ruta(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_vehiculo_ruta(connection, contexto, uuid)?;

    let respuesta = cliente
        .post(format!(
            "{}/rest/v1/vehiculos_ruta?on_conflict=placa",
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

pub(super) fn construir_cuerpo_encargado_ruta(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let (codigo_empleado, nombre, cedula, activo): (String, String, Option<String>, i64) =
        connection.query_row(
            "SELECT codigo_empleado, nombre, cedula, activo FROM encargados_ruta WHERE uuid = ?1",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

    Ok(json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "codigo_empleado": codigo_empleado,
        "nombre": nombre,
        "cedula": cedula,
        "activo": activo != 0,
    }))
}

/// `on_conflict=codigo_empleado` -- mismo motivo que `enviar_vehiculo_ruta`.
pub(super) fn enviar_encargado_ruta(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_encargado_ruta(connection, contexto, uuid)?;

    let respuesta = cliente
        .post(format!(
            "{}/rest/v1/encargados_ruta?on_conflict=codigo_empleado",
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

/// Fila cruda para armar el cuerpo remoto de una salida de ruta -- struct
/// en vez de una tupla de 13 elementos (`clippy::type_complexity`).
pub(super) struct FilaSalidaRutaLocal {
    pub(super) vehiculo_id: Option<i64>,
    pub(super) vehiculo_placa: String,
    pub(super) vehiculo_numero_unidad: Option<String>,
    pub(super) encargado_id: Option<i64>,
    pub(super) encargado_nombre: String,
    pub(super) numero_ruta: i64,
    pub(super) sub_numero: i64,
    pub(super) numero_documento: String,
    pub(super) fecha_documento: String,
    pub(super) resultado: String,
    pub(super) motivo_resultado: Option<String>,
    pub(super) fecha_hora_salida: String,
    pub(super) usuario_salida_nombre: String,
}

pub(super) fn construir_cuerpo_salida_ruta(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<Value, SincronizacionError> {
    let fila: FilaSalidaRutaLocal = connection.query_row(
        "SELECT vehiculo_id, vehiculo_placa, vehiculo_numero_unidad, encargado_id,
                encargado_nombre, numero_ruta, sub_numero, numero_documento,
                fecha_documento, resultado, motivo_resultado, fecha_hora_salida,
                usuario_salida_nombre
         FROM salidas_ruta WHERE uuid = ?1",
        params![uuid],
        |row| {
            Ok(FilaSalidaRutaLocal {
                vehiculo_id: row.get(0)?,
                vehiculo_placa: row.get(1)?,
                vehiculo_numero_unidad: row.get(2)?,
                encargado_id: row.get(3)?,
                encargado_nombre: row.get(4)?,
                numero_ruta: row.get(5)?,
                sub_numero: row.get(6)?,
                numero_documento: row.get(7)?,
                fecha_documento: row.get(8)?,
                resultado: row.get(9)?,
                motivo_resultado: row.get(10)?,
                fecha_hora_salida: row.get(11)?,
                usuario_salida_nombre: row.get(12)?,
            })
        },
    )?;

    // El catálogo es consultivo (ver `RutaService`): si hubo match local se
    // manda el uuid real de esa fila; si no, `None` viaja tal cual -- la
    // nube no exige que `vehiculo_id`/`encargado_id` existan (mismas
    // columnas nullable del lado de la migración de Supabase).
    let vehiculo_uuid: Option<String> = fila
        .vehiculo_id
        .map(|id| {
            connection.query_row(
                "SELECT uuid FROM vehiculos_ruta WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
        })
        .transpose()?;
    let encargado_uuid: Option<String> = fila
        .encargado_id
        .map(|id| {
            connection.query_row(
                "SELECT uuid FROM encargados_ruta WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
        })
        .transpose()?;

    Ok(json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_salida_id": contexto.dispositivo_id,
        "vehiculo_id": vehiculo_uuid,
        "vehiculo_placa": fila.vehiculo_placa,
        "vehiculo_numero_unidad": fila.vehiculo_numero_unidad,
        "encargado_id": encargado_uuid,
        "encargado_nombre": fila.encargado_nombre,
        "numero_ruta": fila.numero_ruta,
        "sub_numero": fila.sub_numero,
        "numero_documento": fila.numero_documento,
        "fecha_documento": fila.fecha_documento,
        "resultado": fila.resultado,
        "motivo_resultado": fila.motivo_resultado,
        "hora_salida": fila.fecha_hora_salida,
        "usuario_salida_nombre": fila.usuario_salida_nombre,
    }))
}

pub(super) fn enviar_salida_ruta(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let cuerpo = construir_cuerpo_salida_ruta(connection, contexto, uuid)?;

    let respuesta = cliente
        .post(format!("{}/rest/v1/salidas_ruta", contexto.base_url))
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token))
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
        .json(&cuerpo)
        .send()
        .map_err(NubeError::Red)?;

    exigir_2xx(respuesta)
}

/// Cierre (cola), mismo criterio "primero en llegar gana" que
/// `enviar_cierre_ingreso`/`enviar_cierre_movimiento_visita` -- el filtro
/// `hora_retorno=is.null` hace que un cierre que llega tarde (otro
/// dispositivo ya cerró esta salida) no afecte ninguna fila en vez de
/// fallar.
pub(super) fn enviar_cierre_salida_ruta(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (fecha_hora_retorno, usuario_retorno_nombre): (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT fecha_hora_retorno, usuario_retorno_nombre FROM salidas_ruta WHERE uuid = ?1",
            params![uuid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

    let cuerpo = json!({
        "hora_retorno": fecha_hora_retorno,
        "dispositivo_retorno_id": contexto.dispositivo_id,
        "usuario_retorno_nombre": usuario_retorno_nombre,
    });

    let url = format!(
        "{}/rest/v1/salidas_ruta?id=eq.{uuid}&hora_retorno=is.null",
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
