use rusqlite::{params, Connection, Row};

use crate::database::error::DatabaseError;
use crate::models::empresa::Empresa;

pub trait EmpresaRepository {
    fn crear(
        &self,
        empresa: &Empresa,
    ) -> Result<i64, DatabaseError>;

    fn buscar_por_id(
        &self,
        id: i64,
    ) -> Result<Option<Empresa>, DatabaseError>;

    fn listar(
        &self,
    ) -> Result<Vec<Empresa>, DatabaseError>;
}

pub struct SqliteEmpresaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteEmpresaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(
    row: &Row,
) -> rusqlite::Result<Empresa> {
    Ok(Empresa {
        id: row.get(0)?,
        nombre: row.get(1)?,
    })
}

impl<'a> EmpresaRepository
    for SqliteEmpresaRepository<'a>
{
    fn crear(
        &self,
        empresa: &Empresa,
    ) -> Result<i64, DatabaseError> {
        self.connection.execute(
            "
            INSERT INTO empresas (
                nombre
            )
            VALUES (?1)
            ",
            params![empresa.nombre],
        )?;

        Ok(self.connection.last_insert_rowid())
    }

    fn buscar_por_id(
        &self,
        id: i64,
    ) -> Result<Option<Empresa>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                nombre
            FROM empresas
            WHERE id = ?1
            ",
        )?;

        match statement.query_row(
            params![id],
            convertir_fila,
        ) {
            Ok(empresa) => Ok(Some(empresa)),

            Err(
                rusqlite::Error::QueryReturnedNoRows
            ) => Ok(None),

            Err(error) => Err(
                DatabaseError::from(error)
            ),
        }
    }

    fn listar(
        &self,
    ) -> Result<Vec<Empresa>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                nombre
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