use rusqlite::{params, Connection, Row};

use crate::database::error::DatabaseError;
use crate::models::usuario::{RolUsuario, Usuario};

pub trait UsuarioRepository {
    fn crear(
        &self,
        usuario: &Usuario,
    ) -> Result<i64, DatabaseError>;

    fn buscar_por_cedula(
        &self,
        cedula: &str,
    ) -> Result<Option<Usuario>, DatabaseError>;

    fn buscar_por_id(
        &self,
        id: i64,
    ) -> Result<Option<Usuario>, DatabaseError>;

    fn actualizar(
        &self,
        usuario: &Usuario,
    ) -> Result<(), DatabaseError>;

    fn listar(
        &self,
    ) -> Result<Vec<Usuario>, DatabaseError>;
}

pub struct SqliteUsuarioRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteUsuarioRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<Usuario> {
    let rol_texto: String = row.get(4)?;

    let rol = match rol_texto.as_str() {
        "ROOT" => RolUsuario::Root,
        "ADMINISTRADOR" => RolUsuario::Administrador,
        "OPERADOR" => RolUsuario::Operador,

        _ => {
            return Err(
                rusqlite::Error::InvalidColumnType(
                    4,
                    "rol".to_string(),
                    rusqlite::types::Type::Text,
                )
            );
        }
    };

    Ok(Usuario {
        id: row.get(0)?,
        cedula: row.get(1)?,
        nombre: row.get(2)?,
        password_hash: row.get(3)?,
        rol,
        activo: row.get::<_, i64>(5)? != 0,
    })
}

fn rol_a_texto(rol: RolUsuario) -> &'static str {
    match rol {
        RolUsuario::Root => "ROOT",
        RolUsuario::Administrador => "ADMINISTRADOR",
        RolUsuario::Operador => "OPERADOR",
    }
}

impl<'a> UsuarioRepository for SqliteUsuarioRepository<'a> {
    fn crear(
        &self,
        usuario: &Usuario,
    ) -> Result<i64, DatabaseError> {
        self.connection.execute(
            "
            INSERT INTO usuarios (
                cedula,
                nombre,
                password_hash,
                rol,
                activo
            )
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                usuario.cedula,
                usuario.nombre,
                usuario.password_hash,
                rol_a_texto(usuario.rol),
                usuario.activo as i64,
            ],
        )?;

        Ok(self.connection.last_insert_rowid())
    }

    fn buscar_por_cedula(
        &self,
        cedula: &str,
    ) -> Result<Option<Usuario>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                cedula,
                nombre,
                password_hash,
                rol,
                activo
            FROM usuarios
            WHERE cedula = ?1
            ",
        )?;

        match statement.query_row(
            params![cedula],
            convertir_fila,
        ) {
            Ok(usuario) => Ok(Some(usuario)),

            Err(
                rusqlite::Error::QueryReturnedNoRows
            ) => Ok(None),

            Err(error) => Err(
                DatabaseError::from(error)
            ),
        }
    }

    fn buscar_por_id(
        &self,
        id: i64,
    ) -> Result<Option<Usuario>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                cedula,
                nombre,
                password_hash,
                rol,
                activo
            FROM usuarios
            WHERE id = ?1
            ",
        )?;

        match statement.query_row(
            params![id],
            convertir_fila,
        ) {
            Ok(usuario) => Ok(Some(usuario)),

            Err(
                rusqlite::Error::QueryReturnedNoRows
            ) => Ok(None),

            Err(error) => Err(
                DatabaseError::from(error)
            ),
        }
    }

    fn actualizar(
        &self,
        usuario: &Usuario,
    ) -> Result<(), DatabaseError> {
        self.connection.execute(
            "
            UPDATE usuarios
            SET
                cedula = ?1,
                nombre = ?2,
                password_hash = ?3,
                rol = ?4,
                activo = ?5
            WHERE id = ?6
            ",
            params![
                usuario.cedula,
                usuario.nombre,
                usuario.password_hash,
                rol_a_texto(usuario.rol),
                usuario.activo as i64,
                usuario.id,
            ],
        )?;

        Ok(())
    }

    fn listar(
        &self,
    ) -> Result<Vec<Usuario>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                cedula,
                nombre,
                password_hash,
                rol,
                activo
            FROM usuarios
            ORDER BY nombre
            ",
        )?;

        let usuarios = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(usuarios)
    }
}