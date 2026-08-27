use rusqlite::{Connection, Row};

use crate::database::queries::{
    LIMITE_LISTADO_MAXIMO as LIMITE_MAXIMO, LIMITE_LISTADO_PREDETERMINADO as LIMITE_PREDETERMINADO,
};
use crate::database::{error::DatabaseError, search::BusquedaTexto};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct EmpresaResumen {
    pub id: i64,
    pub nombre: String,
    pub contratistas: usize,
    pub activo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroEmpresas {
    pub texto: Option<String>,
    pub limite: usize,
    pub offset: usize,
}

impl Default for FiltroEmpresas {
    fn default() -> Self {
        Self {
            texto: None,
            limite: LIMITE_PREDETERMINADO,
            offset: 0,
        }
    }
}

pub trait EmpresasQuery {
    fn buscar(&self, filtro: &FiltroEmpresas) -> Result<Vec<EmpresaResumen>, DatabaseError>;
}

pub struct SqliteEmpresasQuery<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteEmpresasQuery<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl EmpresasQuery for SqliteEmpresasQuery<'_> {
    fn buscar(&self, filtro: &FiltroEmpresas) -> Result<Vec<EmpresaResumen>, DatabaseError> {
        let busqueda = BusquedaTexto::preparar(filtro.texto.as_deref());
        let limite = filtro.limite.clamp(1, LIMITE_MAXIMO) as i64;
        let offset = i64::try_from(filtro.offset).unwrap_or(i64::MAX);
        let (sql, parametros): (&str, Vec<rusqlite::types::Value>) = match busqueda.modo {
            1 => (
                "SELECT e.id,e.nombre,COUNT(c.id),e.activo FROM empresas e
                 LEFT JOIN contratistas c ON c.empresa_id=e.id
                 WHERE PLEGAR(e.nombre) LIKE PLEGAR(?1)
                 GROUP BY e.id,e.nombre,e.activo ORDER BY e.nombre COLLATE NOCASE,e.id LIMIT ?2 OFFSET ?3",
                vec![busqueda.patron_like.into(), limite.into(), offset.into()],
            ),
            2 => (
                "SELECT e.id,e.nombre,COUNT(c.id),e.activo FROM empresas_fts
                 INNER JOIN empresas e ON e.id=empresas_fts.rowid
                 LEFT JOIN contratistas c ON c.empresa_id=e.id
                 WHERE empresas_fts MATCH ?1
                 GROUP BY e.id,e.nombre,e.activo ORDER BY e.nombre COLLATE NOCASE,e.id LIMIT ?2 OFFSET ?3",
                vec![busqueda.consulta_fts.into(), limite.into(), offset.into()],
            ),
            _ => (
                "SELECT e.id,e.nombre,COUNT(c.id),e.activo FROM empresas e
                 LEFT JOIN contratistas c ON c.empresa_id=e.id
                 GROUP BY e.id,e.nombre,e.activo ORDER BY e.nombre COLLATE NOCASE,e.id LIMIT ?1 OFFSET ?2",
                vec![limite.into(), offset.into()],
            ),
        };
        let mut statement = self.connection.prepare(sql)?;
        Ok(statement
            .query_map(rusqlite::params_from_iter(parametros), convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?)
    }
}

fn convertir_fila(row: &Row<'_>) -> rusqlite::Result<EmpresaResumen> {
    let contratistas: i64 = row.get(2)?;
    Ok(EmpresaResumen {
        id: row.get(0)?,
        nombre: row.get(1)?,
        contratistas: usize::try_from(contratistas).unwrap_or(usize::MAX),
        activo: row.get::<_, i64>(3)? != 0,
    })
}
