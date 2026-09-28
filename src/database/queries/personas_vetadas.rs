//! ¿Esta cédula tiene un veto vigente? Consulta la copia local de
//! `personas_vetadas` (la baja la sincronización y la actualiza el aviso en
//! vivo), así que responde sin internet. Ver
//! `docs/features-futuras/plan-veto-por-persona.md`.

use rusqlite::Connection;

use crate::database::error::DatabaseError;
use crate::domain::cedula::Cedula;

/// Compara en forma única (`domain::cedula`): el veto vale sin importar
/// cómo se escriba la cédula. Una cédula que no se puede normalizar se
/// busca tal cual (nunca coincide con un veto, que siempre está en forma
/// única).
pub fn esta_vetada(connection: &Connection, cedula: &str) -> Result<bool, DatabaseError> {
    let cedula =
        Cedula::normalizar(cedula).map_or_else(|_| cedula.to_string(), Cedula::into_string);
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM personas_vetadas WHERE cedula = ?1 AND vigente = 1)",
        [cedula],
        |fila| fila.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    #[test]
    fn solo_bloquea_el_veto_vigente_en_cualquier_formato() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO personas_vetadas (uuid, cedula, vigente, actualizado_en) VALUES
                    ('v1', '112340567', 1, '2026-09-28T10:00:00Z'),
                    ('v2', '155812345678', 0, '2026-09-28T10:00:00Z');",
            )
            .unwrap();

        assert!(esta_vetada(&connection, "01-1234-0567").unwrap());
        assert!(esta_vetada(&connection, "112340567").unwrap());
        assert!(
            !esta_vetada(&connection, "155812345678").unwrap(),
            "levantado"
        );
        assert!(!esta_vetada(&connection, "999999999").unwrap());
    }

    #[test]
    fn la_consulta_usa_el_indice_de_vigentes() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let detalle: String = connection
            .query_row(
                "EXPLAIN QUERY PLAN SELECT 1 FROM personas_vetadas WHERE cedula = ?1 AND vigente = 1",
                ["112340567"],
                |fila| fila.get(3),
            )
            .unwrap();
        assert!(
            detalle.contains("idx_personas_vetadas_cedula_vigente"),
            "{detalle}"
        );
    }
}
