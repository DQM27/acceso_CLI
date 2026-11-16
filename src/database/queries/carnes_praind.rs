//! Carnés PRAIND en la base local (migración 64): la ruta del carné de cada
//! contratista y los carnés ya abiertos en este equipo.
//!
//! Sólo se guarda lo que alguien abrió, para verlo al instante y sin
//! internet; nunca se bajan todos. Va dentro de la base cifrada, no como
//! archivos sueltos. La ruta incluye el SHA-256 del archivo, así que un
//! carné guardado nunca queda desactualizado.

use rusqlite::{Connection, OptionalExtension, params};

/// Tope de carnés guardados (~1,2 MB cada uno). Pasado el tope se borran
/// los que hace más tiempo nadie abre.
pub const MAXIMO_GUARDADOS: i64 = 200;

/// Lo que la pantalla de detalles necesita saber del carné de un contratista.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarneDeContratista {
    /// Id del contratista en la nube; `None` si todavía no se sincronizó.
    pub uuid: Option<String>,
    /// Ruta del carné vigente; `None` si no tiene.
    pub clave: Option<String>,
    /// Si el carné ya está guardado en este equipo.
    pub guardado: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarneGuardado {
    pub tipo_mime: String,
    pub bytes: Vec<u8>,
}

pub fn carne_de_contratista(
    connection: &Connection,
    contratista_id: i64,
) -> rusqlite::Result<Option<CarneDeContratista>> {
    connection
        .query_row(
            "SELECT c.uuid, c.praind_foto,
                    EXISTS (SELECT 1 FROM carnes_praind_guardados g WHERE g.clave = c.praind_foto)
             FROM contratistas c WHERE c.id = ?1",
            params![contratista_id],
            |fila| {
                Ok(CarneDeContratista {
                    uuid: fila.get(0)?,
                    clave: fila.get(1)?,
                    guardado: fila.get(2)?,
                })
            },
        )
        .optional()
}

/// Devuelve el carné guardado y anota que se volvió a abrir.
pub fn abrir_guardado(
    connection: &Connection,
    clave: &str,
    ahora: &str,
) -> rusqlite::Result<Option<CarneGuardado>> {
    let carne = connection
        .query_row(
            "SELECT tipo_mime, bytes FROM carnes_praind_guardados WHERE clave = ?1",
            params![clave],
            |fila| {
                Ok(CarneGuardado {
                    tipo_mime: fila.get(0)?,
                    bytes: fila.get(1)?,
                })
            },
        )
        .optional()?;
    if carne.is_some() {
        connection.execute(
            "UPDATE carnes_praind_guardados SET abierto_en = ?2 WHERE clave = ?1",
            params![clave, ahora],
        )?;
    }
    Ok(carne)
}

/// Guarda un carné recién bajado. Borra los carnés anteriores del mismo
/// contratista (misma carpeta de la ruta) y, pasado el tope, los que hace
/// más tiempo nadie abre.
pub fn guardar(
    connection: &Connection,
    clave: &str,
    tipo_mime: &str,
    bytes: &[u8],
    ahora: &str,
) -> rusqlite::Result<()> {
    let transaction = connection.unchecked_transaction()?;
    if let Some((carpeta, _)) = clave.rsplit_once('/') {
        transaction.execute(
            "DELETE FROM carnes_praind_guardados
             WHERE substr(clave, 1, length(?1) + 1) = ?1 || '/' AND clave <> ?2",
            params![carpeta, clave],
        )?;
    }
    transaction.execute(
        "INSERT INTO carnes_praind_guardados (clave, tipo_mime, bytes, abierto_en)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(clave) DO UPDATE SET abierto_en = excluded.abierto_en",
        params![clave, tipo_mime, bytes, ahora],
    )?;
    transaction.execute(
        "DELETE FROM carnes_praind_guardados WHERE clave IN (
             SELECT clave FROM carnes_praind_guardados
             ORDER BY abierto_en DESC, clave
             LIMIT -1 OFFSET ?1
         )",
        params![MAXIMO_GUARDADOS],
    )?;
    transaction.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    fn base() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    fn cuantos(connection: &Connection) -> i64 {
        connection
            .query_row("SELECT count(*) FROM carnes_praind_guardados", [], |f| {
                f.get(0)
            })
            .unwrap()
    }

    #[test]
    fn guarda_abre_y_reemplaza_el_carne_de_un_contratista() {
        let connection = base();
        let vieja = "contratistas/u1/aaa.png";
        let nueva = "contratistas/u1/bbb.jpg";
        guardar(
            &connection,
            vieja,
            "image/png",
            &[1, 2, 3],
            "2026-11-15T10:00:00Z",
        )
        .unwrap();
        guardar(
            &connection,
            "contratistas/u2/ccc.png",
            "image/png",
            &[9],
            "2026-11-15T10:00:00Z",
        )
        .unwrap();

        let abierto = abrir_guardado(&connection, vieja, "2026-11-15T11:00:00Z")
            .unwrap()
            .unwrap();
        assert_eq!(abierto.bytes, vec![1, 2, 3]);
        assert_eq!(abierto.tipo_mime, "image/png");

        guardar(
            &connection,
            nueva,
            "image/jpeg",
            &[4],
            "2026-11-15T12:00:00Z",
        )
        .unwrap();
        assert!(abrir_guardado(&connection, vieja, "x").unwrap().is_none());
        assert!(abrir_guardado(&connection, nueva, "x").unwrap().is_some());
        // El de otro contratista no se toca.
        assert!(
            abrir_guardado(&connection, "contratistas/u2/ccc.png", "x")
                .unwrap()
                .is_some()
        );
        assert!(
            abrir_guardado(&connection, "contratistas/nadie/x.png", "x")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn pasado_el_tope_borra_los_menos_usados() {
        let connection = base();
        for i in 0..MAXIMO_GUARDADOS + 5 {
            let clave = format!("contratistas/c{i}/x.png");
            let ahora = format!("2026-11-15T10:{:02}:{:02}Z", i / 60, i % 60);
            guardar(&connection, &clave, "image/png", &[1], &ahora).unwrap();
        }
        assert_eq!(cuantos(&connection), MAXIMO_GUARDADOS);
        assert!(
            abrir_guardado(&connection, "contratistas/c0/x.png", "x")
                .unwrap()
                .is_none()
        );
        let ultima = format!("contratistas/c{}/x.png", MAXIMO_GUARDADOS + 4);
        assert!(abrir_guardado(&connection, &ultima, "x").unwrap().is_some());
    }
}
