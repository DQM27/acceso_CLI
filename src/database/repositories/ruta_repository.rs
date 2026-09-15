use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::ruta::Ruta;

pub trait RutaRepository {
    fn crear(&self, numero: i64) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<Ruta>, DatabaseError>;

    fn buscar_por_numero(&self, numero: i64) -> Result<Option<Ruta>, DatabaseError>;

    fn actualizar(&self, ruta: &Ruta) -> Result<(), DatabaseError>;

    fn listar(&self) -> Result<Vec<Ruta>, DatabaseError>;
}

pub struct SqliteRutaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteRutaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    fn encolar_actualizacion(&self, id: i64) -> Result<(), DatabaseError> {
        let uuid: Option<String> = self.connection.query_row(
            "SELECT uuid FROM rutas WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if let Some(uuid) = uuid {
            cola_salida::encolar(self.connection, "ruta", &uuid, "actualizar")?;
        }
        Ok(())
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<Ruta> {
    Ok(Ruta {
        id: row.get(0)?,
        numero: row.get(1)?,
        activo: row.get::<_, i64>(2)? != 0,
    })
}

const SELECT_RUTA: &str = "SELECT id, numero, activo FROM rutas";

impl RutaRepository for SqliteRutaRepository<'_> {
    fn crear(&self, numero: i64) -> Result<i64, DatabaseError> {
        let uuid = generar_uuid_v4();

        self.connection.execute(
            "INSERT INTO rutas (numero, activo, uuid) VALUES (?1, 1, ?2)",
            params![numero, uuid],
        )?;

        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "ruta", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<Ruta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_RUTA} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(ruta) => Ok(Some(ruta)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_numero(&self, numero: i64) -> Result<Option<Ruta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_RUTA} WHERE numero = ?1"))?;
        match statement.query_row(params![numero], convertir_fila) {
            Ok(ruta) => Ok(Some(ruta)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn actualizar(&self, ruta: &Ruta) -> Result<(), DatabaseError> {
        self.connection.execute(
            "UPDATE rutas SET numero = ?1, activo = ?2 WHERE id = ?3",
            params![ruta.numero, i64::from(ruta.activo), ruta.id],
        )?;

        self.encolar_actualizacion(ruta.id)
    }

    fn listar(&self) -> Result<Vec<Ruta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_RUTA} ORDER BY numero"))?;
        let rutas = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rutas)
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
        let repo = SqliteRutaRepository::new(&connection);

        let id = repo.crear(79).unwrap();
        let ruta = repo.buscar_por_numero(79).unwrap().unwrap();

        assert_eq!(ruta.id, id);
        assert_eq!(ruta.numero, 79);
        assert!(ruta.activo);
    }

    #[test]
    fn numero_duplicado_falla() {
        let connection = conexion();
        let repo = SqliteRutaRepository::new(&connection);
        repo.crear(79).unwrap();

        let error = repo.crear(79).unwrap_err();
        assert!(error.es_constraint_unique());
    }

    #[test]
    fn actualizar_desactiva_la_ruta() {
        let connection = conexion();
        let repo = SqliteRutaRepository::new(&connection);
        let id = repo.crear(79).unwrap();
        let mut ruta = repo.buscar_por_id(id).unwrap().unwrap();

        ruta.activo = false;
        repo.actualizar(&ruta).unwrap();

        assert!(!repo.buscar_por_id(id).unwrap().unwrap().activo);
    }

    #[test]
    fn listar_ordena_por_numero() {
        let connection = conexion();
        let repo = SqliteRutaRepository::new(&connection);
        repo.crear(120).unwrap();
        repo.crear(79).unwrap();

        let rutas = repo.listar().unwrap();

        assert_eq!(
            rutas.iter().map(|r| r.numero).collect::<Vec<_>>(),
            vec![79, 120]
        );
    }
}
