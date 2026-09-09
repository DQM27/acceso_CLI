//! Drena la bandeja de salida (`cola_salida`, ver
//! `docs/plan-persistencia-nube.md`) hacia el receptor: por cada fila
//! pendiente, arma el pedido HTTP correspondiente y la marca `enviado` o
//! `fallido` según la respuesta. Una fila fallida no detiene a las demás --
//! se reintenta en la próxima llamada, no bloquea el resto de la cola.

use std::collections::HashMap;

use rusqlite::{Connection, params};
use serde_json::json;

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

    for fila in pendientes(connection, limite)? {
        let resultado = match (fila.entidad.as_str(), fila.operacion.as_str()) {
            ("empresa", _) => enviar_empresa(&cliente, connection, contexto, &fila.entidad_uuid),
            ("contratista", _) => {
                enviar_contratista(&cliente, connection, contexto, &fila.entidad_uuid)
            }
            ("gafete", _) => enviar_gafete(&cliente, connection, contexto, &fila.entidad_uuid),
            ("usuario", _) => enviar_usuario(&cliente, connection, contexto, &fila.entidad_uuid),
            ("ingreso", "cerrar") => {
                enviar_cierre_ingreso(&cliente, connection, contexto, &fila.entidad_uuid)
            }
            ("ingreso", _) => enviar_ingreso(&cliente, connection, contexto, &fila.entidad_uuid),
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
    }

    Ok(resumen)
}

/// Filas listas para reintentarse ahora: nunca tocadas (`intentos = 0`), o
/// que ya esperaron lo suficiente desde el último intento. La espera crece
/// con cada fallo (15 min, 30 min, 45 min...), tope de un día -- para no
/// mendigar el mismo pedido roto cada 5 minutos para siempre, pero tampoco
/// dejarlo esperando una semana entera.
fn pendientes(connection: &Connection, limite: u32) -> Result<Vec<FilaCola>, SincronizacionError> {
    let mut statement = connection.prepare(
        "
        SELECT id, entidad, entidad_uuid, operacion, intentos FROM cola_salida
        WHERE estado = 'pendiente'
          AND (
            intentos = 0
            OR datetime(actualizado_en, '+' || MIN(intentos * 15, 1440) || ' minutes')
               <= datetime('now')
          )
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
    let separador = if url_base.contains('?') { '&' } else { '?' };
    let url = format!("{url_base}{separador}order=id.asc");

    let mut resultado = Vec::new();
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
        resultado.extend(pagina);
        if recibidas_en_esta_pagina < TAMANO_PAGINA_REMOTA {
            break;
        }
        desde += TAMANO_PAGINA_REMOTA;
    }
    Ok(resultado)
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
fn enviar_contratista(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
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

    let cuerpo = json!({
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
    });

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
fn enviar_empresa(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (nombre, activo): (String, i64) = connection.query_row(
        "SELECT nombre, activo FROM empresas WHERE uuid = ?1",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let cuerpo = json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "nombre": nombre,
        "activa": activo != 0,
    });

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
fn enviar_gafete(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
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

    let cuerpo = json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "numero": numero,
        "estado": estado,
        "contratista_deudor_id": deudor_uuid,
        "contratista_deudor_nombre": deudor_nombre,
    });

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
fn enviar_usuario(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let (cedula, nombre, rol, activo): (String, String, String, i64) = connection.query_row(
        "SELECT cedula, nombre, rol, activo FROM usuarios WHERE uuid = ?1",
        params![uuid],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;

    let cuerpo = json!({
        "id": uuid,
        "sitio_id": contexto.sitio_id,
        "dispositivo_origen_id": contexto.dispositivo_id,
        "cedula": cedula,
        "nombre": nombre,
        "rol": rol,
        "activo": activo != 0,
    });

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

/// Ingresos (cola), apertura: mismo criterio de `upsert` que contratistas
/// -- reintentar un envío ya recibido no duplica nada.
#[allow(clippy::type_complexity)]
fn enviar_ingreso(
    cliente: &reqwest::blocking::Client,
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
) -> Result<(), SincronizacionError> {
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

    let cuerpo = json!({
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
    });

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
pub fn recibir_cierres_de_ingresos_propios(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/ingresos?sitio_id=eq.{}&dispositivo_entrada_id=eq.{}\
         &hora_salida=not.is.null&select=id,hora_salida,usuario_salida_nombre",
        contexto.base_url, contexto.sitio_id, contexto.dispositivo_id,
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
/// mismo en la nube para este sitio, creado por *otro* dispositivo
/// (`dispositivo_entrada_id=neq.<el mío>`). Reemplaza el contenido entero
/// de la tabla en una sola transacción -- más simple que llevar la cuenta
/// de qué cambió, y la tabla es chica (sólo lo que está abierto ahora).
pub fn recibir_ingresos_abiertos(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<IngresoRemoto>, SincronizacionError> {
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/ingresos?sitio_id=eq.{}&dispositivo_entrada_id=neq.{}&hora_salida=is.null\
         &select=id,contratista_nombre,hora_entrada,usuario_entrada_nombre,dispositivo_entrada_id,\
         contratista_cedula,empresa_nombre,tipo_ingreso,medio_ingreso,gafete_numero",
        contexto.base_url, contexto.sitio_id, contexto.dispositivo_id,
    );
    let filas: Vec<FilaIngresoRemoto> = obtener_json(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "DELETE FROM ingresos_remotos WHERE sitio_id = ?1",
        params![contexto.sitio_id],
    )?;
    let mut remotos = Vec::with_capacity(filas.len());
    for fila in filas {
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
    let filtro_incremental = marca_anterior
        .as_deref()
        .map(|marca| format!("&updated_at=gt.{marca}"))
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
    let filas: Vec<FilaHistorialRemota> = obtener_json_paginado(&cliente, contexto, &url)?;

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
    let mut marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>> = marca_anterior
        .as_deref()
        .and_then(|marca| crate::tiempo::parsear_utc(marca).ok());
    for fila in &filas {
        let hora_entrada = crate::tiempo::parsear_utc(&fila.hora_entrada)
            .map(crate::tiempo::serializar_utc)
            .map_err(|_| SincronizacionError::FechaInvalida(fila.hora_entrada.clone()))?;
        let hora_salida = fila
            .hora_salida
            .as_deref()
            .map(crate::tiempo::parsear_utc)
            .transpose()
            .map_err(|_| {
                SincronizacionError::FechaInvalida(fila.hora_salida.clone().unwrap_or_default())
            })?
            .map(crate::tiempo::serializar_utc);

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

        let actualizado_en = crate::tiempo::parsear_utc(&fila.updated_at)
            .map_err(|_| SincronizacionError::FechaInvalida(fila.updated_at.clone()))?;
        if marca_mas_nueva.is_none_or(|marca| actualizado_en > marca) {
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
    Ok(recibidos)
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

/// Trae empresas/contratistas/usuarios (incremental, filtrados por
/// `marca_anterior`) y gafetes (completo) -- ver los comentarios que tenían
/// estas mismas consultas en `recibir_catalogo_del_sitio` antes del corte.
fn descargar_catalogo_remoto(
    contexto: &ContextoSincronizacion<'_>,
    marca_anterior: Option<&str>,
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
    // El catálogo de gafetes es pequeño y se descarga completo: el cursor
    // compartido puede ser anterior a la incorporación de gafetes al pull.
    // También permite reintentar deudores que todavía no se pudieron resolver.
    // Con `sitio_id=eq...` a diferencia de las tres de arriba -- ver
    // comentario de `FilaGafeteRemota`.
    let gafetes: Vec<FilaGafeteRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/gafetes?sitio_id=eq.{}&select=id,numero,estado,contratista_deudor_id,\
             contratista_deudor_nombre",
            contexto.base_url, contexto.sitio_id
        ),
    )?;

    // Máximo `updated_at` real entre las tres tablas incrementales (gafetes
    // no participa, se descarga completo cada vez -- ver su comentario más
    // arriba). Sin filas nuevas, la marca no avanza -- preferible repetir la
    // misma consulta (ya sabemos que no trae nada) a arriesgar perder una
    // fila por un reloj local desviado.
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

fn guardar_gafetes(
    transaction: &rusqlite::Transaction<'_>,
    gafetes: &[FilaGafeteRemota],
) -> Result<u32, SincronizacionError> {
    // Mismo motivo que `indice_empresas` en `guardar_contratistas`: un solo
    // `SELECT` de todos los contratistas locales antes del lote, en vez de
    // hasta dos por cada gafete PERDIDO (`resolver_contratista_local`
    // anterior). `guardar_contratistas` ya corrió antes en el mismo
    // `recibir_catalogo_del_sitio` y esta función no modifica
    // `contratistas`, así que el índice se mantiene válido todo el lote.
    let indice_contratistas = indexar_contratistas(transaction)?;

    let mut recibidos = 0;
    for gafete in gafetes {
        // Un gafete PERDIDO sin deudor resoluble localmente violaría el
        // `CHECK` de la tabla (`estado = 'PERDIDO' AND contratista_deudor_id
        // IS NOT NULL`) -- se salta por ahora, mismo criterio que un
        // contratista remoto incompleto: se autorresuelve solo en un sync
        // posterior, en cuanto ese contratista también llegue acá.
        let deudor_id_local = if gafete.estado == "PERDIDO" {
            let Some(id) = indice_contratistas.resolver(
                gafete.contratista_deudor_id.as_deref(),
                gafete.contratista_deudor_nombre.as_deref(),
            ) else {
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
    }
    Ok(recibidos)
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
    let descarga = descargar_catalogo_remoto(contexto, marca_anterior.as_deref())?;

    let transaction = connection.unchecked_transaction()?;
    let resumen = ResumenCatalogo {
        empresas_recibidas: guardar_empresas(&transaction, &descarga.empresas)?,
        contratistas_recibidos: guardar_contratistas(&transaction, &descarga.contratistas)?,
        usuarios_recibidos: guardar_usuarios(&transaction, &descarga.usuarios)?,
        gafetes_recibidos: guardar_gafetes(&transaction, &descarga.gafetes)?,
    };

    if let Some(marca) = descarga.marca_mas_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado SET catalogo_actualizado_hasta = ?1 WHERE id = 1",
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
                    assert!(
                        !pedido.contains("updated_at"),
                        "el cursor no debe omitir gafetes antiguos"
                    );
                    r#"[{"id":"gafete-remoto","numero":26,"estado":"PERDIDO","contratista_deudor_id":"uuid-contratista","contratista_deudor_nombre":null}]"#
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
