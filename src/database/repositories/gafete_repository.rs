//! Escritura del catálogo de gafetes (`docs/plan-gafetes.md`). Un solo
//! trait para lectura+escritura puntual (a diferencia de Empresas/
//! Contratistas, que separan `*Query` de `*Repository`) porque el catálogo
//! es chico y no hay una proyección cara que justifique separarlos —
//! `GafetesQuery` (`queries/gafetes.rs`) sigue aparte sólo para la lista
//! completa con datos del deudor, que sí es una proyección propia.

use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::gafete::{EstadoGafete, Gafete};

pub trait GafeteRepository {
    fn crear(&self, numero: i64) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<Gafete>, DatabaseError>;

    fn buscar_por_numero(&self, numero: i64) -> Result<Option<Gafete>, DatabaseError>;

    fn dar_de_baja(&self, id: i64) -> Result<(), DatabaseError>;

    fn marcar_perdido(&self, id: i64, contratista_deudor_id: i64) -> Result<(), DatabaseError>;

    fn resolver(&self, id: i64) -> Result<(), DatabaseError>;

    /// Números de los gafetes que un contratista debe actualmente
    /// (`estado = 'PERDIDO'` con `contratista_deudor_id` apuntándolo). Un
    /// `Vec` y no `Option<i64>`: nada impide más de una deuda simultánea.
    fn deuda_de_contratista(&self, contratista_id: i64) -> Result<Vec<i64>, DatabaseError>;
}

pub struct SqliteGafeteRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteGafeteRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<Gafete> {
    let estado_texto: String = row.get(2)?;
    let Some(estado) = EstadoGafete::from_str_sql(&estado_texto) else {
        return Err(rusqlite::Error::InvalidColumnType(
            2,
            "estado".to_string(),
            rusqlite::types::Type::Text,
        ));
    };

    Ok(Gafete {
        id: row.get(0)?,
        numero: row.get(1)?,
        estado,
        contratista_deudor_id: row.get(3)?,
    })
}

const SELECT_GAFETE: &str = "SELECT id, numero, estado, contratista_deudor_id FROM gafetes";

fn encolar_actualizacion(connection: &Connection, id: i64) -> Result<(), DatabaseError> {
    let uuid: Option<String> = connection.query_row(
        "SELECT uuid FROM gafetes WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    if let Some(uuid) = uuid {
        cola_salida::encolar(connection, "gafete", &uuid, "actualizar")?;
    }
    Ok(())
}

impl GafeteRepository for SqliteGafeteRepository<'_> {
    fn crear(&self, numero: i64) -> Result<i64, DatabaseError> {
        let uuid = generar_uuid_v4();
        self.connection.execute(
            "INSERT INTO gafetes (numero, estado, uuid) VALUES (?1, 'DISPONIBLE', ?2)",
            params![numero, uuid],
        )?;

        // Capturado antes de encolar: `last_insert_rowid()` refleja el
        // último INSERT de la conexión, y encolar hace el suyo propio.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "gafete", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<Gafete>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_GAFETE} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(gafete) => Ok(Some(gafete)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_numero(&self, numero: i64) -> Result<Option<Gafete>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_GAFETE} WHERE numero = ?1"))?;
        match statement.query_row(params![numero], convertir_fila) {
            Ok(gafete) => Ok(Some(gafete)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn dar_de_baja(&self, id: i64) -> Result<(), DatabaseError> {
        self.connection.execute(
            "UPDATE gafetes SET estado = 'DE_BAJA' WHERE id = ?1",
            params![id],
        )?;
        encolar_actualizacion(self.connection, id)
    }

    fn marcar_perdido(&self, id: i64, contratista_deudor_id: i64) -> Result<(), DatabaseError> {
        self.connection.execute(
            "UPDATE gafetes SET estado = 'PERDIDO', contratista_deudor_id = ?1 WHERE id = ?2",
            params![contratista_deudor_id, id],
        )?;
        encolar_actualizacion(self.connection, id)
    }

    fn resolver(&self, id: i64) -> Result<(), DatabaseError> {
        self.connection.execute(
            "UPDATE gafetes SET estado = 'DISPONIBLE', contratista_deudor_id = NULL WHERE id = ?1",
            params![id],
        )?;
        encolar_actualizacion(self.connection, id)
    }

    fn deuda_de_contratista(&self, contratista_id: i64) -> Result<Vec<i64>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT numero FROM gafetes
             WHERE contratista_deudor_id = ?1 AND estado = 'PERDIDO'
             ORDER BY numero",
        )?;
        let numeros = statement
            .query_map(params![contratista_id], |row| row.get(0))?
            .collect::<Result<Vec<i64>, _>>()?;
        Ok(numeros)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    #[test]
    fn crear_y_buscar_por_numero_redondea_el_viaje() {
        let connection = conexion();
        let repo = SqliteGafeteRepository::new(&connection);

        let id = repo.crear(5).unwrap();
        let gafete = repo.buscar_por_numero(5).unwrap().unwrap();

        assert_eq!(gafete.id, id);
        assert_eq!(gafete.estado, EstadoGafete::Disponible);
        assert_eq!(gafete.contratista_deudor_id, None);
    }

    #[test]
    fn marcar_perdido_y_resolver_limpian_al_deudor() {
        let connection = conexion();
        connection
            .execute("INSERT INTO empresas (nombre) VALUES ('Acme')", params![])
            .unwrap();
        connection
            .execute(
                "INSERT INTO contratistas (cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso)
                 VALUES ('1', 'Juan', 1, 'PRAIND', 0, 1)",
                params![],
            )
            .unwrap();
        let repo = SqliteGafeteRepository::new(&connection);
        let id = repo.crear(1).unwrap();

        repo.marcar_perdido(id, 1).unwrap();
        let perdido = repo.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(perdido.estado, EstadoGafete::Perdido);
        assert_eq!(perdido.contratista_deudor_id, Some(1));
        assert_eq!(repo.deuda_de_contratista(1).unwrap(), vec![1]);

        repo.resolver(id).unwrap();
        let resuelto = repo.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(resuelto.estado, EstadoGafete::Disponible);
        assert_eq!(resuelto.contratista_deudor_id, None);
        assert!(repo.deuda_de_contratista(1).unwrap().is_empty());
    }

    #[test]
    fn numero_duplicado_viola_unique() {
        let connection = conexion();
        let repo = SqliteGafeteRepository::new(&connection);
        repo.crear(1).unwrap();

        let error = repo.crear(1).unwrap_err();
        assert!(error.es_constraint_unique());
    }
}
