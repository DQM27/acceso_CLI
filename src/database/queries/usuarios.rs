use rusqlite::{Connection, Row};

use crate::database::error::DatabaseError;
use crate::database::queries::{LIMITE_LISTADO_MAXIMO as LIMITE_MAXIMO, LIMITE_LISTADO_PREDETERMINADO as LIMITE_PREDETERMINADO};
use crate::database::search::BusquedaTexto;
use crate::models::usuario::RolUsuario;

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
        let busqueda = BusquedaTexto::preparar(filtro.texto.as_deref());
        let limite = filtro.limite.clamp(1, LIMITE_MAXIMO) as i64;
        let offset = i64::try_from(filtro.offset).unwrap_or(i64::MAX);
        let (sql, parametros): (&str, Vec<rusqlite::types::Value>) = match busqueda.modo {
            1 => (
                "SELECT id,cedula,nombre,rol,activo FROM usuarios
                 WHERE PLEGAR(cedula) LIKE PLEGAR(?1) OR PLEGAR(nombre) LIKE PLEGAR(?1)
                 ORDER BY CASE WHEN cedula=?2 COLLATE NOCASE THEN 0 ELSE 1 END,
                          nombre COLLATE NOCASE,id LIMIT ?3 OFFSET ?4",
                vec![
                    busqueda.patron_like.into(),
                    busqueda.texto_literal.into(),
                    limite.into(),
                    offset.into(),
                ],
            ),
            2 => (
                "SELECT u.id,u.cedula,u.nombre,u.rol,u.activo
                 FROM usuarios_fts
                 INNER JOIN usuarios AS u ON u.id=usuarios_fts.rowid
                 WHERE usuarios_fts MATCH ?1
                 ORDER BY CASE WHEN u.cedula=?2 COLLATE NOCASE THEN 0 ELSE 1 END,
                          u.nombre COLLATE NOCASE,u.id LIMIT ?3 OFFSET ?4",
                vec![
                    busqueda.consulta_fts.into(),
                    busqueda.texto_literal.into(),
                    limite.into(),
                    offset.into(),
                ],
            ),
            _ => (
                "SELECT id,cedula,nombre,rol,activo FROM usuarios
                 ORDER BY nombre COLLATE NOCASE,id LIMIT ?1 OFFSET ?2",
                vec![limite.into(), offset.into()],
            ),
        };
        let mut statement = self.connection.prepare(sql)?;
        let items = statement
            .query_map(rusqlite::params_from_iter(parametros), convertir_fila)?
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
