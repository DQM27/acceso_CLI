use chrono::{Duration, NaiveDate};
use rusqlite::{Connection, Row, named_params};

use crate::database::error::DatabaseError;
use crate::database::search::BusquedaTexto;
use crate::domain::acceso::DIAS_ADVERTENCIA_PRAIND;
use crate::models::tipo_ingreso::TipoIngreso;

const LIMITE_PREDETERMINADO: usize = 100;
const LIMITE_MAXIMO: usize = 500;

/// Lectura compuesta lista para presentar sin resolver la empresa por separado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContratistaResumen {
    pub id: i64,
    pub empresa_id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

/// Estado de vencimiento de PRAIND a filtrar. `hoy` viaja con la variante en
/// vez de como campo aparte de `FiltroContratistas` para no dejar un campo
/// "a veces relevante" — sólo importa cuando `praind` es `Some`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiltroPraind {
    /// Vencido: tiene fecha registrada y ya pasó.
    Vencido { hoy: NaiveDate },
    /// Vence dentro de la misma ventana de advertencia que usa
    /// `domain::acceso::verificar_acceso` (30 días).
    ProximoAVencer { hoy: NaiveDate },
    /// Sin fecha de vencimiento registrada (contratistas que no requieren
    /// PRAIND, o a los que les falta el dato).
    SinFecha,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroContratistas {
    pub texto: Option<String>,
    pub empresa_id: Option<i64>,
    /// `None` = todos los tipos; `Some(vec)` filtra a cualquiera de los
    /// listados (como máximo 4, la cantidad de variantes de `TipoIngreso`).
    pub tipos_incluidos: Option<Vec<TipoIngreso>>,
    pub praind: Option<FiltroPraind>,
    pub personal_ruta: Option<bool>,
    pub tiene_acceso: Option<bool>,
    pub limite: usize,
    pub offset: usize,
}

impl Default for FiltroContratistas {
    fn default() -> Self {
        Self {
            texto: None,
            empresa_id: None,
            tipos_incluidos: None,
            praind: None,
            personal_ruta: None,
            tiene_acceso: None,
            limite: LIMITE_PREDETERMINADO,
            offset: 0,
        }
    }
}

pub trait ContratistasQuery {
    fn buscar(&self, filtro: &FiltroContratistas)
    -> Result<Vec<ContratistaResumen>, DatabaseError>;
}

pub struct SqliteContratistasQuery<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteContratistasQuery<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

const CONTRATISTAS_SQL: &str = "
    SELECT
        c.id, c.empresa_id, c.cedula, c.nombre, e.nombre, c.tipo_ingreso,
        c.fecha_vencimiento_praind, c.es_personal_ruta, c.tiene_acceso
    FROM contratistas AS c
    INNER JOIN empresas AS e ON e.id = c.empresa_id
    WHERE (
        :modo_busqueda = 0
        OR (:modo_busqueda = 1 AND (
            c.cedula LIKE :patron COLLATE NOCASE
            OR c.nombre LIKE :patron COLLATE NOCASE
            OR e.nombre LIKE :patron COLLATE NOCASE
        ))
        OR (:modo_busqueda = 2 AND c.id IN (
            SELECT rowid FROM contratistas_fts WHERE contratistas_fts MATCH :consulta_fts
            UNION
            SELECT ct.id FROM empresas_fts
            INNER JOIN contratistas AS ct ON ct.empresa_id = empresas_fts.rowid
            WHERE empresas_fts MATCH :consulta_fts
        ))
    )
    AND (:empresa_id IS NULL OR c.empresa_id = :empresa_id)
    AND (:sin_filtro_tipo OR c.tipo_ingreso IN (:t0, :t1, :t2, :t3))
    AND (
        :praind_modo = 0
        OR (:praind_modo = 1 AND c.fecha_vencimiento_praind IS NOT NULL
            AND c.fecha_vencimiento_praind < :praind_hoy)
        OR (:praind_modo = 2 AND c.fecha_vencimiento_praind IS NOT NULL
            AND c.fecha_vencimiento_praind BETWEEN :praind_hoy AND :praind_limite)
        OR (:praind_modo = 3 AND c.fecha_vencimiento_praind IS NULL)
    )
    AND (:personal_ruta IS NULL OR c.es_personal_ruta = :personal_ruta)
    AND (:tiene_acceso IS NULL OR c.tiene_acceso = :tiene_acceso)
    ORDER BY CASE WHEN c.cedula = :texto_literal COLLATE NOCASE THEN 0 ELSE 1 END,
             c.nombre COLLATE NOCASE, c.id
    LIMIT :limite OFFSET :offset
";

impl ContratistasQuery for SqliteContratistasQuery<'_> {
    fn buscar(
        &self,
        filtro: &FiltroContratistas,
    ) -> Result<Vec<ContratistaResumen>, DatabaseError> {
        let busqueda = BusquedaTexto::preparar(filtro.texto.as_deref());
        let limite = filtro.limite.clamp(1, LIMITE_MAXIMO) as i64;
        let offset = filtro.offset as i64;

        let sin_filtro_tipo = filtro.tipos_incluidos.is_none();
        let mut tipos_bind: [Option<&'static str>; 4] = [None; 4];
        if let Some(tipos) = &filtro.tipos_incluidos {
            for (slot, tipo) in tipos_bind.iter_mut().zip(tipos.iter()) {
                *slot = Some(tipo_a_texto(*tipo));
            }
        }
        let [t0, t1, t2, t3] = tipos_bind;

        let formato = |f: NaiveDate| f.format("%Y-%m-%d").to_string();
        let (praind_modo, praind_hoy, praind_limite): (i64, Option<String>, Option<String>) =
            match filtro.praind {
                None => (0, None, None),
                Some(FiltroPraind::Vencido { hoy }) => (1, Some(formato(hoy)), None),
                Some(FiltroPraind::ProximoAVencer { hoy }) => (
                    2,
                    Some(formato(hoy)),
                    Some(formato(hoy + Duration::days(DIAS_ADVERTENCIA_PRAIND))),
                ),
                Some(FiltroPraind::SinFecha) => (3, None, None),
            };

        let mut statement = self.connection.prepare(CONTRATISTAS_SQL)?;
        let resultados = statement
            .query_map(
                named_params! {
                    ":modo_busqueda": busqueda.modo,
                    ":patron": busqueda.patron_like,
                    ":consulta_fts": busqueda.consulta_fts,
                    ":texto_literal": busqueda.texto_literal,
                    ":empresa_id": filtro.empresa_id,
                    ":sin_filtro_tipo": sin_filtro_tipo,
                    ":t0": t0,
                    ":t1": t1,
                    ":t2": t2,
                    ":t3": t3,
                    ":praind_modo": praind_modo,
                    ":praind_hoy": praind_hoy,
                    ":praind_limite": praind_limite,
                    ":personal_ruta": filtro.personal_ruta,
                    ":tiene_acceso": filtro.tiene_acceso,
                    ":limite": limite,
                    ":offset": offset,
                },
                convertir_fila,
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(resultados)
    }
}

fn tipo_a_texto(tipo: TipoIngreso) -> &'static str {
    match tipo {
        TipoIngreso::Praind => "PRAIND",
        TipoIngreso::InHouse => "IN_HOUSE",
        TipoIngreso::PorCorreo => "POR_CORREO",
        TipoIngreso::Swat => "SWAT",
    }
}

fn convertir_fila(row: &Row<'_>) -> rusqlite::Result<ContratistaResumen> {
    let tipo_texto: String = row.get(5)?;
    let tipo_ingreso = match tipo_texto.as_str() {
        "PRAIND" => TipoIngreso::Praind,
        "IN_HOUSE" => TipoIngreso::InHouse,
        "POR_CORREO" => TipoIngreso::PorCorreo,
        "SWAT" => TipoIngreso::Swat,
        _ => return Err(tipo_invalido(5, "tipo_ingreso")),
    };

    let fecha_texto: Option<String> = row.get(6)?;
    let fecha_vencimiento_praind = fecha_texto
        .map(|fecha| {
            NaiveDate::parse_from_str(&fecha, "%Y-%m-%d").map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;

    Ok(ContratistaResumen {
        id: row.get(0)?,
        empresa_id: row.get(1)?,
        cedula: row.get(2)?,
        nombre: row.get(3)?,
        empresa_nombre: row.get(4)?,
        tipo_ingreso,
        fecha_vencimiento_praind,
        es_personal_ruta: row.get::<_, i64>(7)? != 0,
        tiene_acceso: row.get::<_, i64>(8)? != 0,
    })
}

fn tipo_invalido(indice: usize, nombre: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(indice, nombre.to_owned(), rusqlite::types::Type::Text)
}
