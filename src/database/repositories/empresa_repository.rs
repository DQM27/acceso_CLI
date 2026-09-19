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

    /// `solo_activos`: mismo criterio que `EmpresaProveedorRepository::listar`
    /// -- el selector de empresa al crear/editar un contratista
    /// (`FormularioContratista.tsx`/`PantallaNuevoContratista.kt`) debe pedir
    /// `true`, una empresa desactivada no es una opción válida para un
    /// contratista nuevo. La pantalla de administración pide `false` para
    /// poder ver y reactivar las inactivas. El filtro vive acá a propósito,
    /// no en cada pantalla.
    fn listar(&self, solo_activos: bool) -> Result<Vec<Empresa>, DatabaseError>;
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

    fn listar(&self, solo_activos: bool) -> Result<Vec<Empresa>, DatabaseError> {
        let filtro_activo = if solo_activos { "WHERE activo = 1" } else { "" };
        let mut statement = self.connection.prepare(&format!(
            "
            SELECT
                id,
                nombre,
                activo
            FROM empresas
            {filtro_activo}
            ORDER BY nombre
            "
        ))?;

        let empresas = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(empresas)
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

    fn nueva(nombre: &str) -> Empresa {
        Empresa {
            id: 0,
            nombre: nombre.to_string(),
            activo: true,
        }
    }

    /// Hallazgo del usuario, 2026-09-18: `UNIQUE(nombre)` sólo bloqueaba un
    /// choque exacto -- "Dos Pinos" y "DOS PINOS" se colaban como dos
    /// empresas distintas. El índice único sobre `PLEGAR(nombre)`
    /// (`MIGRACION_47`) cierra ese hueco.
    #[test]
    fn nombre_duplicado_ignorando_mayusculas_y_diacriticos_viola_unique() {
        let connection = conexion();
        let repo = SqliteEmpresaRepository::new(&connection);
        repo.crear(&nueva("Dos Pinos")).unwrap();

        let error = repo.crear(&nueva("DOS PIÑOS")).unwrap_err();
        assert!(error.es_constraint_unique());
    }

    #[test]
    fn listar_trae_todas_ordenadas_por_nombre() {
        let connection = conexion();
        let repo = SqliteEmpresaRepository::new(&connection);
        repo.crear(&nueva("Dos Pinos")).unwrap();
        repo.crear(&nueva("Maika")).unwrap();

        let empresas = repo.listar(false).unwrap();
        assert_eq!(
            empresas
                .iter()
                .map(|e| e.nombre.as_str())
                .collect::<Vec<_>>(),
            vec!["Dos Pinos", "Maika"]
        );
    }

    #[test]
    fn listar_solo_activos_omite_las_desactivadas() {
        let connection = conexion();
        let repo = SqliteEmpresaRepository::new(&connection);
        repo.crear(&nueva("Dos Pinos")).unwrap();
        let id_maika = repo.crear(&nueva("Maika")).unwrap();
        repo.establecer_activo(id_maika, false).unwrap();

        let empresas = repo.listar(true).unwrap();
        assert_eq!(
            empresas
                .iter()
                .map(|e| e.nombre.as_str())
                .collect::<Vec<_>>(),
            vec!["Dos Pinos"]
        );
        assert_eq!(repo.listar(false).unwrap().len(), 2);
    }
}
