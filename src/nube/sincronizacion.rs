//! Drena la bandeja de salida (`cola_salida`, ver
//! `docs/plan-persistencia-nube.md`) hacia el receptor: por cada fila
//! pendiente, arma el pedido HTTP correspondiente y la marca `enviado` o
//! `fallido` según la respuesta. Una fila fallida no detiene a las demás --
//! se reintenta en la próxima llamada, no bloquea el resto de la cola.

use rusqlite::{Connection, params};
use serde_json::json;

use super::cliente::NubeError;

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
    let cliente = reqwest::blocking::Client::new();
    let mut resumen = ResumenDrenado::default();

    for fila in pendientes(connection, limite)? {
        let resultado = match (fila.entidad.as_str(), fila.operacion.as_str()) {
            ("empresa", _) => enviar_empresa(&cliente, connection, contexto, &fila.entidad_uuid),
            ("contratista", _) => {
                enviar_contratista(&cliente, connection, contexto, &fila.entidad_uuid)
            }
            ("gafete", _) => enviar_gafete(&cliente, connection, contexto, &fila.entidad_uuid),
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

    let respuesta = cliente
        .post(format!("{}/rest/v1/contratistas", contexto.base_url))
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

    let respuesta = cliente
        .post(format!("{}/rest/v1/empresas", contexto.base_url))
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

    let respuesta = cliente
        .post(format!("{}/rest/v1/gafetes", contexto.base_url))
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
    ) = connection.query_row(
        "
        SELECT contratista_id, contratista_nombre, fecha_hora_ingreso, usuario_ingreso_nombre,
               contratista_cedula, empresa_nombre, tipo_ingreso, medio_ingreso, gafete_numero
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
}

#[derive(serde::Deserialize)]
struct FilaIngresoRemoto {
    id: String,
    contratista_nombre: String,
    hora_entrada: String,
    usuario_entrada_nombre: Option<String>,
    dispositivo_entrada_id: String,
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
    let cliente = reqwest::blocking::Client::new();
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
    let cliente = reqwest::blocking::Client::new();
    let url = format!(
        "{}/rest/v1/ingresos?sitio_id=eq.{}&dispositivo_entrada_id=neq.{}&hora_salida=is.null\
         &select=id,contratista_nombre,hora_entrada,usuario_entrada_nombre,dispositivo_entrada_id",
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
                usuario_entrada_nombre, dispositivo_entrada_id, actualizado_en
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            ",
            params![
                fila.id,
                contexto.sitio_id,
                fila.contratista_nombre,
                hora_entrada,
                fila.usuario_entrada_nombre,
                fila.dispositivo_entrada_id,
            ],
        )?;
        remotos.push(IngresoRemoto {
            uuid: fila.id,
            contratista_nombre: fila.contratista_nombre,
            hora_entrada,
            usuario_entrada_nombre: fila.usuario_entrada_nombre,
        });
    }
    transaction.commit()?;

    Ok(remotos)
}

/// Cuántas filas se aplicaron localmente al traer el catálogo del sitio --
/// para que la pantalla pueda avisar "3 contratistas nuevos" sin devolver
/// las filas enteras.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResumenCatalogo {
    pub empresas_recibidas: u32,
    pub contratistas_recibidos: u32,
}

#[derive(serde::Deserialize)]
struct FilaEmpresaRemota {
    id: String,
    nombre: String,
    activa: bool,
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
pub fn recibir_catalogo_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<ResumenCatalogo, SincronizacionError> {
    let cliente = reqwest::blocking::Client::new();
    // Sin `sitio_id=eq...` a propósito -- contratistas y empresas son
    // globales (ver docs/plan-panel-administrativo-web.md, "Modelo de
    // datos"): si a un contratista se le niega el acceso en un sitio, tiene
    // que quedar negado en TODOS. El nombre de la función quedó del modelo
    // viejo (un solo sitio por dispositivo); lo que trae ahora es el
    // catálogo global completo, no "del sitio" de `contexto`.
    let empresas: Vec<FilaEmpresaRemota> = obtener_json(
        &cliente,
        contexto,
        &format!("{}/rest/v1/empresas?select=id,nombre,activa", contexto.base_url),
    )?;
    let contratistas: Vec<FilaContratistaRemota> = obtener_json(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/contratistas?select=id,nombre,identificacion,empresa_id,empresa_nombre,\
             activo,tipo_ingreso,fecha_vencimiento_praind,es_personal_ruta",
            contexto.base_url
        ),
    )?;

    let transaction = connection.unchecked_transaction()?;
    let mut resumen = ResumenCatalogo::default();

    for empresa in &empresas {
        transaction.execute(
            "
            INSERT INTO empresas (nombre, activo, uuid) VALUES (?1, ?2, ?3)
            ON CONFLICT(nombre) DO UPDATE SET
                activo = excluded.activo,
                uuid = COALESCE(empresas.uuid, excluded.uuid)
            ",
            params![empresa.nombre, empresa.activa, empresa.id],
        )?;
        resumen.empresas_recibidas += 1;
    }

    for contratista in &contratistas {
        let (Some(cedula), Some(tipo_ingreso), Some(es_personal_ruta)) = (
            contratista.identificacion.as_deref(),
            contratista.tipo_ingreso.as_deref(),
            contratista.es_personal_ruta,
        ) else {
            continue;
        };
        let empresa_id_local = resolver_empresa_local(
            &transaction,
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
        resumen.contratistas_recibidos += 1;
    }

    transaction.commit()?;
    Ok(resumen)
}

/// Resuelve el `id` local de la empresa de un contratista remoto: primero
/// por `uuid` (el vínculo real), y si esa empresa todavía no llegó acá por
/// ese camino, por nombre (mismo respaldo que ya usa el lado de envío,
/// `enviar_contratista`).
fn resolver_empresa_local(
    transaction: &rusqlite::Transaction<'_>,
    empresa_uuid: Option<&str>,
    empresa_nombre: Option<&str>,
) -> Option<i64> {
    empresa_uuid
        .and_then(|uuid| {
            transaction
                .query_row(
                    "SELECT id FROM empresas WHERE uuid = ?1",
                    params![uuid],
                    |row| row.get(0),
                )
                .ok()
        })
        .or_else(|| {
            empresa_nombre.and_then(|nombre| {
                transaction
                    .query_row(
                        "SELECT id FROM empresas WHERE nombre = ?1",
                        params![nombre],
                        |row| row.get(0),
                    )
                    .ok()
            })
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
    let cliente = reqwest::blocking::Client::new();
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
        // Formato real que devuelve PostgREST para un `timestamptz`
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
    fn recibe_catalogo_del_sitio_y_lo_guarda_local() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let base_url = servidor_de_respuestas(vec![
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-empresa-remota\",\"nombre\":\"Empresa Remota\",\"activa\":true}]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-contratista-remoto\",\"nombre\":\"Persona Remota\",\
             \"identificacion\":\"1-1111\",\"empresa_id\":\"uuid-empresa-remota\",\
             \"empresa_nombre\":\"Empresa Remota\",\"activo\":true,\"tipo_ingreso\":\"SWAT\",\
             \"fecha_vencimiento_praind\":null,\"es_personal_ruta\":false}]",
        ]);

        let resumen = recibir_catalogo_del_sitio(&connection, &contexto(&base_url)).unwrap();

        assert_eq!(
            resumen,
            ResumenCatalogo {
                empresas_recibidas: 1,
                contratistas_recibidos: 1
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
             [{\"id\":\"uuid-empresa-remota\",\"nombre\":\"Empresa Remota\",\"activa\":true}]",
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n\
             [{\"id\":\"uuid-contratista-remoto\",\"nombre\":\"Persona Remota\",\
             \"identificacion\":\"1-1111\",\"empresa_id\":\"uuid-empresa-remota\",\
             \"empresa_nombre\":\"Empresa Remota\",\"activo\":true,\"tipo_ingreso\":\"SWAT\",\
             \"fecha_vencimiento_praind\":null,\"es_personal_ruta\":false}]",
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
             \"es_personal_ruta\":null}]",
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
