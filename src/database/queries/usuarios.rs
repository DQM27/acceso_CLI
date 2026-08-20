use rusqlite::{Connection, Row};

use crate::database::error::DatabaseError;
use crate::database::queries::{
    LIMITE_LISTADO_MAXIMO as LIMITE_MAXIMO, LIMITE_LISTADO_PREDETERMINADO as LIMITE_PREDETERMINADO,
};
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
    fn buscar(&self, filtro: &FiltroUsuarios) -> Result<Vec<UsuarioResumen>, DatabaseError> {
        self.buscar_para_actor(filtro, RolUsuario::Root)
    }

    fn buscar_para_actor(
        &self,
        filtro: &FiltroUsuarios,
        actor: RolUsuario,
    ) -> Result<Vec<UsuarioResumen>, DatabaseError>;
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
    fn buscar_para_actor(
        &self,
        filtro: &FiltroUsuarios,
        actor: RolUsuario,
    ) -> Result<Vec<UsuarioResumen>, DatabaseError> {
        let busqueda = BusquedaTexto::preparar(filtro.texto.as_deref());
        let limite = filtro.limite.clamp(1, LIMITE_MAXIMO) as i64;
        let offset = i64::try_from(filtro.offset).unwrap_or(i64::MAX);
        // Root es invisible para cualquier actor que no sea Root — se omite
        // la condición del todo (no un flag `?1 = 1 OR ...` evaluado en cada
        // fila) para que quede un predicado directo, igual que el resto de
        // los filtros de esta app (`docs/hallazgos-buscador.md`). La tabla
        // `usuarios` es chica y esto no cambia el rendimiento, pero mantiene
        // el mismo criterio en todo el código en vez de una excepción.
        let excluir_root = actor != RolUsuario::Root;
        let (sql, parametros): (String, Vec<rusqlite::types::Value>) = match busqueda.modo {
            1 => {
                let filtro_rol = if excluir_root {
                    "AND rol <> 'ROOT'"
                } else {
                    ""
                };
                (
                    format!(
                        "SELECT id,cedula,nombre,rol,activo FROM usuarios
                         WHERE (PLEGAR(cedula) LIKE PLEGAR(?1) OR PLEGAR(nombre) LIKE PLEGAR(?1))
                           {filtro_rol}
                         ORDER BY CASE WHEN cedula=?2 COLLATE NOCASE THEN 0 ELSE 1 END,
                                  nombre COLLATE NOCASE,id LIMIT ?3 OFFSET ?4"
                    ),
                    vec![
                        busqueda.patron_like.into(),
                        busqueda.texto_literal.into(),
                        limite.into(),
                        offset.into(),
                    ],
                )
            }
            2 => {
                let filtro_rol = if excluir_root {
                    "AND u.rol <> 'ROOT'"
                } else {
                    ""
                };
                (
                    format!(
                        "SELECT u.id,u.cedula,u.nombre,u.rol,u.activo
                         FROM usuarios_fts
                         INNER JOIN usuarios AS u ON u.id=usuarios_fts.rowid
                         WHERE usuarios_fts MATCH ?1
                           {filtro_rol}
                         ORDER BY CASE WHEN u.cedula=?2 COLLATE NOCASE THEN 0 ELSE 1 END,
                                  u.nombre COLLATE NOCASE,u.id LIMIT ?3 OFFSET ?4"
                    ),
                    vec![
                        busqueda.consulta_fts.into(),
                        busqueda.texto_literal.into(),
                        limite.into(),
                        offset.into(),
                    ],
                )
            }
            _ => {
                let filtro_rol = if excluir_root {
                    "WHERE rol <> 'ROOT'"
                } else {
                    ""
                };
                (
                    format!(
                        "SELECT id,cedula,nombre,rol,activo FROM usuarios
                         {filtro_rol}
                         ORDER BY nombre COLLATE NOCASE,id LIMIT ?1 OFFSET ?2"
                    ),
                    vec![limite.into(), offset.into()],
                )
            }
        };
        let mut statement = self.connection.prepare(&sql)?;
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
