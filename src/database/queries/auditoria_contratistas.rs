use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use crate::{
    database::error::DatabaseError,
    tiempo::{parsear_utc, serializar_utc},
};

pub const LIMITE_AUDITORIA_PREDETERMINADO: usize = 50;
pub const LIMITE_AUDITORIA_MAXIMO: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CambioContratistaAuditado {
    pub id: i64,
    pub fecha_hora: DateTime<Utc>,
    pub usuario_id: i64,
    pub usuario_nombre: String,
    pub contratista_id: i64,
    pub contratista_nombre: String,
    pub campo: String,
    pub valor_anterior: Option<String>,
    pub valor_nuevo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroAuditoriaContratistas {
    pub limite: usize,
    pub offset: usize,
}

impl Default for FiltroAuditoriaContratistas {
    fn default() -> Self {
        Self {
            limite: LIMITE_AUDITORIA_PREDETERMINADO,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginaAuditoriaContratistas {
    pub items: Vec<CambioContratistaAuditado>,
    pub total: usize,
}

pub trait AuditoriaContratistasWriter {
    fn registrar_cambio(
        &self,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        contratista_id: i64,
        campo: &str,
        valor_anterior: Option<&str>,
        valor_nuevo: Option<&str>,
    ) -> Result<(), DatabaseError>;
}

pub struct SqliteAuditoriaContratistas<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteAuditoriaContratistas<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn buscar(
        &self,
        filtro: &FiltroAuditoriaContratistas,
    ) -> Result<PaginaAuditoriaContratistas, DatabaseError> {
        let limite = filtro.limite.clamp(1, LIMITE_AUDITORIA_MAXIMO) as i64;
        let offset = i64::try_from(filtro.offset).unwrap_or(i64::MAX);
        let transaction = self.connection.unchecked_transaction()?;
        let total: i64 =
            transaction.query_row("SELECT COUNT(*) FROM auditoria_contratistas", [], |row| {
                row.get(0)
            })?;
        let mut statement = transaction.prepare(
            "SELECT a.id,a.fecha_hora,a.usuario_id,u.nombre,
                    a.contratista_id,c.nombre,a.campo,a.valor_anterior,a.valor_nuevo
             FROM auditoria_contratistas a
             JOIN usuarios u ON u.id=a.usuario_id
             JOIN contratistas c ON c.id=a.contratista_id
             ORDER BY a.fecha_hora DESC,a.id DESC
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
                Ok(CambioContratistaAuditado {
                    id: row.get(0)?,
                    fecha_hora,
                    usuario_id: row.get(2)?,
                    usuario_nombre: row.get(3)?,
                    contratista_id: row.get(4)?,
                    contratista_nombre: row.get(5)?,
                    campo: row.get(6)?,
                    valor_anterior: row.get(7)?,
                    valor_nuevo: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        transaction.commit()?;
        Ok(PaginaAuditoriaContratistas {
            items,
            total: usize::try_from(total).unwrap_or(usize::MAX),
        })
    }
}

impl AuditoriaContratistasWriter for SqliteAuditoriaContratistas<'_> {
    fn registrar_cambio(
        &self,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        contratista_id: i64,
        campo: &str,
        valor_anterior: Option<&str>,
        valor_nuevo: Option<&str>,
    ) -> Result<(), DatabaseError> {
        self.connection.execute(
            "INSERT INTO auditoria_contratistas(
                fecha_hora,usuario_id,contratista_id,campo,valor_anterior,valor_nuevo
             ) VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                serializar_utc(fecha_hora),
                usuario_id,
                contratista_id,
                campo,
                valor_anterior,
                valor_nuevo,
            ],
        )?;
        Ok(())
    }
}
