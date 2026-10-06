use rusqlite::{Connection, params};
use serde_json::json;

use super::{ContextoSincronizacion, SincronizacionError, exigir_2xx, obtener_json};
use crate::nube::cliente::{NubeError, cliente_http};

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
pub(super) struct FilaPrestamoGafeteProvisionalRemoto {
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
    let cliente = cliente_http();
    let url = format!(
        "{}/rest/v1/prestamos_gafete_provisional?sitio_id=eq.{}&hora_devolucion=is.null\
         &select=id,encargado_nombre,encargado_codigo_empleado,gafete_numero,hora_entrega,\
         usuario_entrega_nombre",
        contexto.base_url, contexto.sitio_id,
    );
    let filas: Vec<FilaPrestamoGafeteProvisionalRemoto> = obtener_json(&cliente, contexto, &url)?;

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
        let existe_localmente: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM prestamos_gafete_provisional WHERE uuid = ?1)",
            params![fila.id],
            |row| row.get(0),
        )?;
        // Lo propio no se duplica en la caché; lo que este equipo acaba de
        // cerrar a mano tampoco vuelve (ver `cierres_remotos`).
        if existe_localmente || super::cierres_remotos::cerrado_aca(&transaction, &fila.id)? {
            continue;
        }
        let hora_entrega = crate::tiempo::parsear_utc(&fila.hora_entrega)
            .map(crate::tiempo::serializar_utc)
            .map_err(|_| SincronizacionError::FechaInvalida(fila.hora_entrega.clone()))?;
        transaction.execute(
            "
            INSERT INTO prestamos_gafete_provisional_remotos (
                uuid, sitio_id, encargado_nombre, encargado_codigo_empleado, gafete_numero,
                hora_entrega, usuario_entrega_nombre, actualizado_en
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            ",
            params![
                fila.id,
                contexto.sitio_id,
                fila.encargado_nombre,
                fila.encargado_codigo_empleado,
                fila.gafete_numero,
                hora_entrega,
                fila.usuario_entrega_nombre,
            ],
        )?;
        remotos.push(PrestamoGafeteProvisionalRemoto {
            uuid: fila.id,
            encargado_nombre: fila.encargado_nombre,
            encargado_codigo_empleado: fila.encargado_codigo_empleado,
            gafete_numero: fila.gafete_numero,
            hora_entrega,
            usuario_entrega_nombre: fila.usuario_entrega_nombre,
        });
    }
    transaction.commit()?;

    Ok(remotos)
}

#[derive(serde::Deserialize)]
pub(super) struct FilaDevolucionPropiaRemota {
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
    let abiertos_localmente: Vec<String> = {
        let mut statement = connection.prepare(
            "SELECT uuid FROM prestamos_gafete_provisional WHERE fecha_hora_devolucion IS NULL",
        )?;
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
    let filas: Vec<FilaDevolucionPropiaRemota> = obtener_json(&cliente, contexto, &url)?;

    let transaction = connection.unchecked_transaction()?;
    let mut aplicados = 0_u32;
    for fila in &filas {
        let nombre_devolucion = fila
            .usuario_devolucion_nombre
            .as_deref()
            .unwrap_or("Devolución registrada en nube");
        let hora_devolucion = crate::tiempo::parsear_utc(&fila.hora_devolucion)
            .map(crate::tiempo::serializar_utc)
            .map_err(|_| SincronizacionError::FechaInvalida(fila.hora_devolucion.clone()))?;
        let filas_afectadas = transaction.execute(
            "
            UPDATE prestamos_gafete_provisional
            SET
                fecha_hora_devolucion = ?1,
                usuario_devolucion_id = NULL,
                usuario_devolucion_nombre = ?2
            WHERE uuid = ?3
              AND fecha_hora_devolucion IS NULL
            ",
            params![hora_devolucion, nombre_devolucion, fila.id],
        )?;
        let filas_afectadas = u32::try_from(filas_afectadas).unwrap_or(u32::MAX);
        aplicados = aplicados.saturating_add(filas_afectadas);
    }
    transaction.commit()?;

    Ok(aplicados)
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
