use rusqlite::{Connection, params};
use serde_json::json;

use super::{ContextoSincronizacion, SincronizacionError, exigir_2xx, obtener_json};
use crate::nube::cliente::{NubeError, cliente_http};

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
    pub placa: Option<String>,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaIngresoRemoto {
    pub(super) id: String,
    pub(super) contratista_nombre: String,
    pub(super) hora_entrada: String,
    pub(super) usuario_entrada_nombre: Option<String>,
    pub(super) dispositivo_entrada_id: String,
    pub(super) contratista_cedula: Option<String>,
    pub(super) empresa_nombre: Option<String>,
    pub(super) tipo_ingreso: Option<String>,
    pub(super) medio_ingreso: Option<String>,
    pub(super) gafete_numero: Option<i64>,
    pub(super) placa: Option<String>,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaCierrePropioRemoto {
    pub(super) id: String,
    pub(super) hora_salida: String,
    pub(super) usuario_salida_nombre: Option<String>,
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
         contratista_cedula,empresa_nombre,tipo_ingreso,medio_ingreso,gafete_numero,placa",
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
                contratista_cedula, empresa_nombre, tipo_ingreso, medio_ingreso, gafete_numero,
                placa
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?7, ?8, ?9, ?10, ?11, ?12)
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
                fila.placa,
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
            placa: fila.placa,
        });
    }
    transaction.commit()?;

    Ok(remotos)
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
