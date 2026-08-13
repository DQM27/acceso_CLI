use rusqlite::{Connection, Row, params};

use crate::database::error::DatabaseError;
use crate::models::usuario::RolUsuario;

const LIMITE_PREDETERMINADO: usize = 100;
const LIMITE_MAXIMO: usize = 500;

/// Contrato de lectura administrativa; nunca contiene material de autenticación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsuarioResumen {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub rol: RolUsuario,
    pub activo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroUsuarios {
    pub texto: Option<String>,
    pub limite: usize,
    pub offset: usize,
}

impl Default for FiltroUsuarios {
    fn default() -> Self {
        Self {
            texto: None,
            limite: LIMITE_PREDETERMINADO,
            offset: 0,
        }
    }
}

pub trait UsuariosQuery {
    fn buscar(&self, filtro: &FiltroUsuarios) -> Result<Vec<UsuarioResumen>, DatabaseError>;
}

pub struct SqliteUsuariosQuery<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteUsuariosQuery<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl UsuariosQuery for SqliteUsuariosQuery<'_> {
    fn buscar(&self, filtro: &FiltroUsuarios) -> Result<Vec<UsuarioResumen>, DatabaseError> {
        let patron = filtro
            .texto
            .as_deref()
            .map(str::trim)
            .filter(|texto| !texto.is_empty())
            .map(|texto| format!("%{texto}%"));
        let limite = filtro.limite.clamp(1, LIMITE_MAXIMO) as i64;
        let offset = i64::try_from(filtro.offset).unwrap_or(i64::MAX);
        let mut statement = self.connection.prepare(
            "
            SELECT id, cedula, nombre, rol, activo
            FROM usuarios
            WHERE ?1 IS NULL
               OR cedula LIKE ?1 COLLATE NOCASE
               OR nombre LIKE ?1 COLLATE NOCASE
            ORDER BY nombre COLLATE NOCASE, id
            LIMIT ?2 OFFSET ?3
            ",
        )?;
        let items = statement
            .query_map(params![patron, limite, offset], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }
}

fn convertir_fila(row: &Row<'_>) -> rusqlite::Result<UsuarioResumen> {
    let rol_texto: String = row.get(3)?;
    let rol = match rol_texto.as_str() {
        "ROOT" => RolUsuario::Root,
        "ADMINISTRADOR" => RolUsuario::Administrador,
        "OPERADOR" => RolUsuario::Operador,
        _ => {
            return Err(rusqlite::Error::InvalidColumnType(
                3,
                "rol".to_owned(),
                rusqlite::types::Type::Text,
            ));
        }
    };
    Ok(UsuarioResumen {
        id: row.get(0)?,
        cedula: row.get(1)?,
        nombre: row.get(2)?,
        rol,
        activo: row.get::<_, i64>(4)? != 0,
    })
}
