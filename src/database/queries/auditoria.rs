//! Auditoría genérica de cambios — cubre contratistas, empresas y usuarios
//! en una sola tabla (`auditoria_cambios`) en vez de una tabla por entidad
//! (`auditoria_contratistas`, reemplazada por `MIGRACION_13` en
//! `src/database/schema.rs`). `usuario_nombre`/`entidad_nombre` quedan como
//! snapshot en la propia fila — a diferencia del diseño anterior (JOIN en
//! vivo a `contratistas`/`usuarios`), así una entidad renombrada o dada de
//! baja no le hace perder sentido a una fila de auditoría vieja.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use crate::{
    database::error::DatabaseError,
    tiempo::{parsear_utc, serializar_utc},
};

pub const LIMITE_AUDITORIA_PREDETERMINADO: usize = 50;
pub const LIMITE_AUDITORIA_MAXIMO: usize = 200;

/// Espejo de `TipoIngreso` (`src/models/tipo_ingreso.rs`): codificación
/// canónica en minúsculas para SQLite (`as_str_sql`/`from_str_sql`), único
/// lugar donde vive ese mapeo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum EntidadAuditada {
    Contratista,
    Empresa,
    Usuario,
}

impl EntidadAuditada {
    pub const fn as_str_sql(self) -> &'static str {
        match self {
            Self::Contratista => "contratista",
            Self::Empresa => "empresa",
            Self::Usuario => "usuario",
        }
    }

    pub fn from_str_sql(texto: &str) -> Option<Self> {
        match texto {
            "contratista" => Some(Self::Contratista),
            "empresa" => Some(Self::Empresa),
            "usuario" => Some(Self::Usuario),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CambioAuditado {
    pub id: i64,
    pub fecha_hora: DateTime<Utc>,
    pub usuario_id: i64,
    pub usuario_nombre: String,
    pub entidad: EntidadAuditada,
    pub entidad_id: i64,
    pub entidad_nombre: String,
    pub campo: String,
    pub valor_anterior: Option<String>,
    pub valor_nuevo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroAuditoria {
    pub limite: usize,
    pub offset: usize,
}

impl Default for FiltroAuditoria {
    fn default() -> Self {
        Self {
            limite: LIMITE_AUDITORIA_PREDETERMINADO,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginaAuditoria {
    pub items: Vec<CambioAuditado>,
    pub total: usize,
}

pub trait AuditoriaWriter {
    #[allow(clippy::too_many_arguments)]
    fn registrar_cambio(
        &self,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        usuario_nombre: &str,
        entidad: EntidadAuditada,
        entidad_id: i64,
        entidad_nombre: &str,
        campo: &str,
        valor_anterior: Option<&str>,
        valor_nuevo: Option<&str>,
    ) -> Result<(), DatabaseError>;
}

pub struct SqliteAuditoria<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteAuditoria<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn buscar(&self, filtro: &FiltroAuditoria) -> Result<PaginaAuditoria, DatabaseError> {
        let limite =
            i64::try_from(filtro.limite.clamp(1, LIMITE_AUDITORIA_MAXIMO)).unwrap_or(i64::MAX);
        let offset = i64::try_from(filtro.offset).unwrap_or(i64::MAX);
        let transaction = self.connection.unchecked_transaction()?;
        let total: i64 =
            transaction.query_row("SELECT COUNT(*) FROM auditoria_cambios", [], |row| {
                row.get(0)
            })?;
        let mut statement = transaction.prepare(
            "SELECT id,fecha_hora,usuario_id,usuario_nombre,entidad,entidad_id,
                    entidad_nombre,campo,valor_anterior,valor_nuevo
             FROM auditoria_cambios
             ORDER BY fecha_hora DESC,id DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let items = statement
            .query_map(params![limite, offset], |row| {
                let fecha: String = row.get(1)?;
                let fecha_hora = parsear_utc(&fecha).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                let entidad_texto: String = row.get(4)?;
                let entidad = EntidadAuditada::from_str_sql(&entidad_texto).ok_or_else(|| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        format!("entidad desconocida: {entidad_texto}").into(),
                    )
                })?;
                Ok(CambioAuditado {
                    id: row.get(0)?,
                    fecha_hora,
                    usuario_id: row.get(2)?,
                    usuario_nombre: row.get(3)?,
                    entidad,
                    entidad_id: row.get(5)?,
                    entidad_nombre: row.get(6)?,
                    campo: row.get(7)?,
                    valor_anterior: row.get(8)?,
                    valor_nuevo: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        transaction.commit()?;
        Ok(PaginaAuditoria {
            items,
            total: usize::try_from(total).unwrap_or(usize::MAX),
        })
    }
}

impl AuditoriaWriter for SqliteAuditoria<'_> {
    fn registrar_cambio(
        &self,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        usuario_nombre: &str,
        entidad: EntidadAuditada,
        entidad_id: i64,
        entidad_nombre: &str,
        campo: &str,
        valor_anterior: Option<&str>,
        valor_nuevo: Option<&str>,
    ) -> Result<(), DatabaseError> {
        self.connection.execute(
            "INSERT INTO auditoria_cambios(
                fecha_hora,usuario_id,usuario_nombre,entidad,entidad_id,entidad_nombre,
                campo,valor_anterior,valor_nuevo
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                serializar_utc(fecha_hora),
                usuario_id,
                usuario_nombre,
                entidad.as_str_sql(),
                entidad_id,
                entidad_nombre,
                campo,
                valor_anterior,
                valor_nuevo,
            ],
        )?;
        Ok(())
    }
}
