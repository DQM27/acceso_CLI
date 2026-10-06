//! Lápidas de los registros del otro dispositivo que ESTE equipo cerró a
//! mano (`remotos_cerrados_aca`, `MIGRACION_56`).
//!
//! Carrera que evitan (revisión del 2026-10-04): la recepción de abiertos
//! (`recibir_*_abiertos`) lee la nube y después reemplaza la caché
//! `*_remotos` entera. Si entre esa lectura y ese reemplazo quien opera
//! cerraba a mano un registro del otro equipo (`cerrar_*_remoto`: `PATCH` a
//! la nube y borrar la fila de la caché), el reemplazo la volvía a insertar
//! con los datos viejos y la persona reaparecía "adentro" hasta la
//! siguiente sincronización. Lo mismo con un aviso en vivo atrasado.
//!
//! Ahora el cierre borra la fila y anota la lápida en una sola transacción,
//! y la recepción no inserta un uuid con lápida. La lápida se olvida cuando
//! la nube ya no lo devuelve abierto (cierre confirmado) o, por las dudas,
//! al día.

use std::collections::HashSet;

use rusqlite::{Connection, params};

use super::SincronizacionError;

/// Borra `uuid` de la caché `tabla` y anota la lápida, en una transacción.
/// `tabla` es una de las cachés `*_remotos` (el `CHECK` de la tabla lo
/// garantiza; acá viene siempre de una constante).
pub(super) fn anotar_cierre_remoto(
    connection: &Connection,
    tabla: &'static str,
    uuid: &str,
) -> Result<(), SincronizacionError> {
    let transaction = connection.unchecked_transaction()?;
    transaction.execute(
        &format!("DELETE FROM {tabla} WHERE uuid = ?1"),
        params![uuid],
    )?;
    transaction.execute(
        "INSERT OR REPLACE INTO remotos_cerrados_aca (uuid, tabla, cerrado_en)
         VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
        params![uuid, tabla],
    )?;
    transaction.commit()?;
    Ok(())
}

/// ¿Este equipo cerró a mano este registro remoto (y la nube todavía no lo
/// confirmó)? Entonces no se vuelve a meter en la caché.
pub(super) fn cerrado_aca(
    connection: &Connection,
    uuid: &str,
) -> Result<bool, SincronizacionError> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM remotos_cerrados_aca WHERE uuid = ?1)",
        params![uuid],
        |fila| fila.get(0),
    )?)
}

/// Olvida las lápidas de `tabla` cuyo registro la nube ya no devuelve
/// abierto (`abiertos` es la lista recién leída), y las de más de un día.
pub(super) fn olvidar_cierres_confirmados<'a>(
    connection: &Connection,
    tabla: &'static str,
    abiertos: impl IntoIterator<Item = &'a str>,
) -> Result<(), SincronizacionError> {
    let abiertos: HashSet<&str> = abiertos.into_iter().collect();
    let lapidas: Vec<String> = {
        let mut statement =
            connection.prepare("SELECT uuid FROM remotos_cerrados_aca WHERE tabla = ?1")?;
        statement
            .query_map(params![tabla], |fila| fila.get(0))?
            .collect::<Result<_, _>>()?
    };
    for uuid in lapidas
        .iter()
        .filter(|uuid| !abiertos.contains(uuid.as_str()))
    {
        connection.execute(
            "DELETE FROM remotos_cerrados_aca WHERE uuid = ?1",
            params![uuid],
        )?;
    }
    connection.execute(
        "DELETE FROM remotos_cerrados_aca
         WHERE tabla = ?1 AND cerrado_en < strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 day')",
        params![tabla],
    )?;
    Ok(())
}
