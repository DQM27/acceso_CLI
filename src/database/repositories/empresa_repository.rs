use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::empresa::Empresa;

pub trait EmpresaRepository {
    fn crear(&self, empresa: &Empresa) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<Empresa>, DatabaseError>;

    fn buscar_por_nombre(&self, nombre: &str) -> Result<Option<Empresa>, DatabaseError>;

    fn actualizar(&self, empresa: &Empresa) -> Result<(), DatabaseError>;

    fn establecer_activo(&self, id: i64, activo: bool) -> Result<(), DatabaseError>;

    fn listar(&self) -> Result<Vec<Empresa>, DatabaseError>;
}

pub struct SqliteEmpresaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteEmpresaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    fn encolar_actualizacion(&self, id: i64) -> Result<(), DatabaseError> {
        let uuid: Option<String> = self.connection.query_row(
            "SELECT uuid FROM empresas WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if let Some(uuid) = uuid {
            cola_salida::encolar(self.connection, "empresa", &uuid, "actualizar")?;
        }
        Ok(())
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<Empresa> {
    Ok(Empresa {
        id: row.get(0)?,
        nombre: row.get(1)?,
        activo: row.get::<_, i64>(2)? != 0,
    })
}

impl EmpresaRepository for SqliteEmpresaRepository<'_> {
    fn crear(&self, empresa: &Empresa) -> Result<i64, DatabaseError> {
        let uuid = generar_uuid_v4();

        self.connection.execute(
            "
            INSERT INTO empresas (
                nombre, uuid
            )
            VALUES (?1, ?2)
            ",
            params![empresa.nombre, uuid],
        )?;

        // Capturado antes de encolar: `last_insert_rowid()` refleja el
        // último INSERT de la conexión, y encolar hace el suyo propio.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "empresa", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<Empresa>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                nombre,
                activo
            FROM empresas
            WHERE id = ?1
            ",
        )?;

        match statement.query_row(params![id], convertir_fila) {
            Ok(empresa) => Ok(Some(empresa)),

            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),

            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_nombre(&self, nombre: &str) -> Result<Option<Empresa>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                nombre,
                activo
            FROM empresas
            WHERE nombre = ?1
            ",
        )?;

        match statement.query_row(params![nombre], convertir_fila) {
            Ok(empresa) => Ok(Some(empresa)),

            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),

            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn actualizar(&self, empresa: &Empresa) -> Result<(), DatabaseError> {
        self.connection.execute(
            "
            UPDATE empresas
            SET nombre = ?1
            WHERE id = ?2
            ",
            params![empresa.nombre, empresa.id],
        )?;

        self.encolar_actualizacion(empresa.id)
    }

    fn establecer_activo(&self, id: i64, activo: bool) -> Result<(), DatabaseError> {
        self.connection.execute(
            "UPDATE empresas SET activo = ?1 WHERE id = ?2",
            params![i64::from(activo), id],
        )?;
        self.encolar_actualizacion(id)
    }

    fn listar(&self) -> Result<Vec<Empresa>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                nombre,
                activo
            FROM empresas
            ORDER BY nombre
            ",
        )?;

        let empresas = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(empresas)
    }
}
