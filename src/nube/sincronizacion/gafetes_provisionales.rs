use rusqlite::{Connection, params};
use serde_json::json;

use super::{ContextoSincronizacion, SincronizacionError, exigir_2xx, obtener_json};
use crate::nube::cliente::{NubeError, cliente_http};

/// Lo que este equipo tiene abierto localmente: los ids que se le preguntan
/// a la nube para traer sus cierres (la etapa y `estado_del_sitio`).
pub(super) const ABIERTOS_LOCALES_PRESTAMOS: &str =
    "SELECT uuid FROM prestamos_gafete_provisional WHERE fecha_hora_devolucion IS NULL";

/// Fila cacheada localmente de un préstamo de gafete provisional todavía
/// abierto, entregado por OTRO dispositivo de este mismo sitio -- mismo
/// criterio que [`IngresoProveedorRemoto`], pero contra
/// `prestamos_gafete_provisional_remotos`/`prestamos_gafete_provisional`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrestamoGafeteProvisionalRemoto {
    pub uuid: String,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: String,
    pub gafete_numero: i64,
    pub hora_entrega: String,
    pub usuario_entrega_nombre: String,
}

#[derive(serde::Deserialize)]
pub(in crate::nube) struct FilaPrestamoGafeteProvisionalRemoto {
    pub(super) id: String,
    pub(super) encargado_nombre: String,
    pub(super) encargado_codigo_empleado: String,
    pub(super) gafete_numero: i64,
    pub(super) hora_entrega: String,
    pub(super) usuario_entrega_nombre: String,
}

/// Espejo de [`recibir_ingresos_proveedor_abiertos`], pero contra
/// `prestamos_gafete_provisional` -- misma lógica de "traer todo lo
/// abierto del sitio y descartar lo que ya vive local", mismo reemplazo
/// completo de la caché en una sola transacción.
pub fn recibir_prestamos_gafete_provisional_abiertos(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<Vec<PrestamoGafeteProvisionalRemoto>, SincronizacionError> {
    recibir_prestamos_gafete_provisional_abiertos_con(connection, contexto, None)
}

/// [`recibir_prestamos_gafete_provisional_abiertos`] con las filas ya recibidas en `estado_del_sitio`
/// (`None`: las pide por GET, como siempre).
pub(in crate::nube) fn recibir_prestamos_gafete_provisional_abiertos_con(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    recibidas: Option<Vec<FilaPrestamoGafeteProvisionalRemoto>>,
) -> Result<Vec<PrestamoGafeteProvisionalRemoto>, SincronizacionError> {
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/prestamos_gafete_provisional?sitio_id=eq.{}&hora_devolucion=is.null\
         &select=id,encargado_nombre,encargado_codigo_empleado,gafete_numero,hora_entrega,\
         usuario_entrega_nombre",
        contexto.base_url, contexto.sitio_id,
    );
    let filas: Vec<FilaPrestamoGafeteProvisionalRemoto> = match recibidas {
        Some(filas) => filas,
        None => obtener_json(&cliente, contexto, &url)?,
    };

    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        "DELETE FROM prestamos_gafete_provisional_remotos WHERE sitio_id = ?1",
        params![contexto.sitio_id],
    )?;
    super::cierres_remotos::olvidar_cierres_confirmados(
        &transaction,
        "prestamos_gafete_provisional_remotos",
        filas.iter().map(|fila| fila.id.as_str()),
    )?;
    let mut remotos = Vec::with_capacity(filas.len());
    for fila in filas {
        if let Some(remoto) =
            guardar_prestamo_gafete_provisional_remoto(&transaction, contexto.sitio_id, fila)?
        {
            remotos.push(remoto);
        }
    }
    transaction.commit()?;

    Ok(remotos)
}

/// Guarda en la caché `prestamos_gafete_provisional_remotos` un préstamo
/// abierto por el otro dispositivo del sitio. `None` si ya vive en
/// `prestamos_gafete_provisional` de este equipo o si este equipo acaba de
/// devolverlo a mano (ver `cierres_remotos`). Reemplaza la fila si ya
/// estaba. Compartida por la sincronización y el aviso en vivo
/// (`nube::en_vivo`).
pub(in crate::nube) fn guardar_prestamo_gafete_provisional_remoto(
    connection: &Connection,
    sitio_id: &str,
    fila: FilaPrestamoGafeteProvisionalRemoto,
) -> Result<Option<PrestamoGafeteProvisionalRemoto>, SincronizacionError> {
    let existe_localmente: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM prestamos_gafete_provisional WHERE uuid = ?1)",
        params![fila.id],
        |row| row.get(0),
    )?;
    if existe_localmente || super::cierres_remotos::cerrado_aca(connection, &fila.id)? {
        return Ok(None);
    }
    let hora_entrega = crate::tiempo::parsear_utc(&fila.hora_entrega)
        .map(crate::tiempo::serializar_utc)
        .map_err(|_| SincronizacionError::FechaInvalida(fila.hora_entrega.clone()))?;
    connection.execute(
        "
        INSERT OR REPLACE INTO prestamos_gafete_provisional_remotos (
            uuid, sitio_id, encargado_nombre, encargado_codigo_empleado, gafete_numero,
            hora_entrega, usuario_entrega_nombre, actualizado_en
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        ",
        params![
            fila.id,
            sitio_id,
            fila.encargado_nombre,
            fila.encargado_codigo_empleado,
            fila.gafete_numero,
            hora_entrega,
            fila.usuario_entrega_nombre,
        ],
    )?;
    Ok(Some(PrestamoGafeteProvisionalRemoto {
        uuid: fila.id,
        encargado_nombre: fila.encargado_nombre,
        encargado_codigo_empleado: fila.encargado_codigo_empleado,
        gafete_numero: fila.gafete_numero,
        hora_entrega,
        usuario_entrega_nombre: fila.usuario_entrega_nombre,
    }))
}

#[derive(serde::Deserialize)]
pub(in crate::nube) struct FilaDevolucionPropiaRemota {
    pub(super) id: String,
    pub(super) hora_devolucion: String,
    pub(super) usuario_devolucion_nombre: Option<String>,
}

/// Espejo de [`recibir_cierres_de_ingresos_propios_proveedor`], pero para
/// devoluciones de préstamos de gafete provisional que ESTE dispositivo
/// entregó y OTRO dispositivo devolvió.
pub fn recibir_devoluciones_propias_gafete_provisional(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<u32, SincronizacionError> {
    recibir_devoluciones_propias_gafete_provisional_con(connection, contexto, None)
}

/// [`recibir_devoluciones_propias_gafete_provisional`] con las filas ya recibidas en `estado_del_sitio`
/// (`None`: las pide por GET, como siempre).
pub(in crate::nube) fn recibir_devoluciones_propias_gafete_provisional_con(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    recibidas: Option<Vec<FilaDevolucionPropiaRemota>>,
) -> Result<u32, SincronizacionError> {
    let abiertos_localmente: Vec<String> = {
        let mut statement = connection.prepare(ABIERTOS_LOCALES_PRESTAMOS)?;
        statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?
    };
    if abiertos_localmente.is_empty() {
        return Ok(0);
    }

    let cliente = cliente_http();
    let lista_uuids = abiertos_localmente.join(",");
    let url = format!(
        "{}/rest/v1/prestamos_gafete_provisional?id=in.({lista_uuids})&hora_devolucion=not.is.null\
         &select=id,hora_devolucion,usuario_devolucion_nombre",
        contexto.base_url,
    );
    let filas: Vec<FilaDevolucionPropiaRemota> = match recibidas {
        Some(filas) => filas,
        None => obtener_json(&cliente, contexto, &url)?,
    };

    let transaction = connection.unchecked_transaction()?;
    let mut aplicados = 0_u32;
    for fila in &filas {
        if aplicar_devolucion_de_prestamo_propio(
            &transaction,
            &fila.id,
            &fila.hora_devolucion,
            fila.usuario_devolucion_nombre.as_deref(),
        )? {
            aplicados = aplicados.saturating_add(1);
        }
    }
    transaction.commit()?;

    Ok(aplicados)
}

/// Registra acá la devolución de un préstamo propio que devolvió OTRO
/// dispositivo. `false` si no había nada que cerrar. Compartida por la
/// sincronización y el aviso en vivo (`nube::en_vivo`).
pub(in crate::nube) fn aplicar_devolucion_de_prestamo_propio(
    connection: &Connection,
    id: &str,
    hora_devolucion: &str,
    usuario_devolucion_nombre: Option<&str>,
) -> Result<bool, SincronizacionError> {
    let nombre_devolucion = usuario_devolucion_nombre.unwrap_or("Devolución registrada en nube");
    let hora_devolucion = crate::tiempo::parsear_utc(hora_devolucion)
        .map(crate::tiempo::serializar_utc)
        .map_err(|_| SincronizacionError::FechaInvalida(hora_devolucion.to_string()))?;
    let filas_afectadas = connection.execute(
        "
        UPDATE prestamos_gafete_provisional
        SET
            fecha_hora_devolucion = ?1,
            usuario_devolucion_id = NULL,
            usuario_devolucion_nombre = ?2
        WHERE uuid = ?3
          AND fecha_hora_devolucion IS NULL
        ",
        params![hora_devolucion, nombre_devolucion, id],
    )?;
    Ok(filas_afectadas > 0)
}

/// Espejo de [`cerrar_ingreso_proveedor_remoto`], pero para registrar la
/// devolución de un préstamo de gafete provisional que OTRO dispositivo
/// entregó. `hora` como en [`cerrar_ingreso_remoto`]: la del reloj
/// corregido de quien llama.
pub fn cerrar_prestamo_gafete_provisional_remoto(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    uuid: &str,
    usuario_devolucion_nombre: &str,
    hora: chrono::DateTime<chrono::Utc>,
) -> Result<(), SincronizacionError> {
    let cliente = cliente_http();
    let cuerpo = json!({
        "hora_devolucion": crate::tiempo::serializar_utc(hora),
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

    exigir_2xx(respuesta)?;

    // Borra de la caché y anota la lápida juntos: una recepción que leyó la
    // nube antes del cierre no la vuelve a meter (ver `cierres_remotos`).
    super::cierres_remotos::anotar_cierre_remoto(
        connection,
        "prestamos_gafete_provisional_remotos",
        uuid,
    )
}
