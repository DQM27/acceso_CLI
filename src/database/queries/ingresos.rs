use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{Connection, Row, named_params};

use crate::database::error::DatabaseError;
use crate::database::search::BusquedaTexto;
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::tipo_ingreso::TipoIngreso;

const LIMITE_ACTIVOS_PREDETERMINADO: usize = 100;
const LIMITE_ACTIVOS_MAXIMO: usize = 500;
const LIMITE_HISTORIAL_PREDETERMINADO: usize = 50;
const LIMITE_HISTORIAL_MAXIMO: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoActivoLectura {
    pub registro_id: i64,
    pub contratista_id: i64,
    pub empresa_id: i64,
    pub cedula: String,
    pub contratista_nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub medio_ingreso: MedioIngreso,
    pub fecha_hora_ingreso: NaiveDateTime,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: String,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroIngresosActivos {
    pub texto: Option<String>,
    pub limite: usize,
    pub offset: usize,
}

impl Default for FiltroIngresosActivos {
    fn default() -> Self {
        Self {
            texto: None,
            limite: LIMITE_ACTIVOS_PREDETERMINADO,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovimientoIngresoResumen {
    pub registro_id: i64,
    pub contratista_id: i64,
    pub cedula: String,
    pub contratista_nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub medio_ingreso: MedioIngreso,
    pub fecha_hora_ingreso: NaiveDateTime,
    pub fecha_hora_salida: Option<NaiveDateTime>,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: String,
    pub usuario_salida_nombre: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoMovimiento {
    Todos,
    Activos,
    Cerrados,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroHistorial {
    /// Límite inferior inclusivo aplicado a `fecha_hora_ingreso`.
    pub desde: NaiveDateTime,
    /// Límite superior exclusivo aplicado a `fecha_hora_ingreso`.
    pub hasta: NaiveDateTime,
    pub texto_persona: Option<String>,
    pub empresa_id: Option<i64>,
    pub tipo_ingreso: Option<TipoIngreso>,
    pub gafete_numero: Option<i64>,
    pub estado: EstadoMovimiento,
    pub limite: usize,
    pub offset: usize,
}

impl FiltroHistorial {
    pub fn nuevo(desde: NaiveDateTime, hasta: NaiveDateTime) -> Self {
        Self {
            desde,
            hasta,
            texto_persona: None,
            empresa_id: None,
            tipo_ingreso: None,
            gafete_numero: None,
            estado: EstadoMovimiento::Todos,
            limite: LIMITE_HISTORIAL_PREDETERMINADO,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginaHistorial {
    pub items: Vec<MovimientoIngresoResumen>,
    pub total: usize,
}

pub trait IngresosQuery {
    fn listar_activos(
        &self,
        filtro: &FiltroIngresosActivos,
    ) -> Result<Vec<IngresoActivoLectura>, DatabaseError>;

    fn buscar_historial(&self, filtro: &FiltroHistorial) -> Result<PaginaHistorial, DatabaseError>;
}

pub struct SqliteIngresosQuery<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteIngresosQuery<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl IngresosQuery for SqliteIngresosQuery<'_> {
    fn listar_activos(
        &self,
        filtro: &FiltroIngresosActivos,
    ) -> Result<Vec<IngresoActivoLectura>, DatabaseError> {
        let busqueda = BusquedaTexto::preparar(filtro.texto.as_deref());
        let limite = filtro.limite.clamp(1, LIMITE_ACTIVOS_MAXIMO) as i64;
        let offset = offset_sql(filtro.offset);

        let (sql, parametros): (&str, Vec<rusqlite::types::Value>) = match busqueda.modo {
            1 => (
                ACTIVOS_CORTO_SQL,
                vec![
                    busqueda.patron_like.into(),
                    busqueda.numero_exacto.into(),
                    limite.into(),
                    offset.into(),
                ],
            ),
            2 => (
                ACTIVOS_FTS_SQL,
                vec![
                    busqueda.consulta_fts.into(),
                    busqueda.numero_exacto.into(),
                    limite.into(),
                    offset.into(),
                ],
            ),
            _ => (ACTIVOS_SIN_FILTRO_SQL, vec![limite.into(), offset.into()]),
        };
        let mut statement = self.connection.prepare(sql)?;
        let items = statement
            .query_map(rusqlite::params_from_iter(parametros), convertir_activo)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }

    fn buscar_historial(&self, filtro: &FiltroHistorial) -> Result<PaginaHistorial, DatabaseError> {
        let busqueda = BusquedaTexto::preparar(filtro.texto_persona.as_deref());
        let tipo = filtro.tipo_ingreso.map(tipo_a_texto);
        let estado = estado_a_texto(filtro.estado);
        let desde = fecha_hora_a_texto(filtro.desde);
        let hasta = fecha_hora_a_texto(filtro.hasta);
        let limite = filtro.limite.clamp(1, LIMITE_HISTORIAL_MAXIMO) as i64;
        let offset = offset_sql(filtro.offset);

        let count_sql = format!("SELECT COUNT(*) {HISTORIAL_FROM_WHERE}");
        let total: i64 = self.connection.query_row(
            &count_sql,
            named_params! {
                ":desde": desde,
                ":hasta": hasta,
                ":modo_busqueda": busqueda.modo,
                ":patron": busqueda.patron_like,
                ":consulta_fts": busqueda.consulta_fts,
                ":empresa_id": filtro.empresa_id,
                ":tipo": tipo,
                ":gafete": filtro.gafete_numero,
                ":estado": estado,
            },
            |row| row.get(0),
        )?;

        let select_sql = format!(
            "SELECT {HISTORIAL_COLUMNAS} {HISTORIAL_FROM_WHERE} \
             ORDER BY r.fecha_hora_ingreso DESC, r.id DESC LIMIT :limite OFFSET :offset"
        );
        let mut statement = self.connection.prepare(&select_sql)?;
        let items = statement
            .query_map(
                named_params! {
                    ":desde": desde,
                    ":hasta": hasta,
                    ":modo_busqueda": busqueda.modo,
                    ":patron": busqueda.patron_like,
                    ":consulta_fts": busqueda.consulta_fts,
                    ":empresa_id": filtro.empresa_id,
                    ":tipo": tipo,
                    ":gafete": filtro.gafete_numero,
                    ":estado": estado,
                    ":limite": limite,
                    ":offset": offset,
                },
                convertir_movimiento,
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PaginaHistorial {
            items,
            total: usize::try_from(total).unwrap_or(usize::MAX),
        })
    }
}

const ACTIVOS_SIN_FILTRO_SQL: &str = "
    SELECT
        r.id, r.contratista_id, r.empresa_id, c.cedula, c.nombre, e.nombre,
        r.tipo_ingreso, r.medio_ingreso, r.fecha_hora_ingreso, r.gafete_numero,
        ui.nombre, c.fecha_vencimiento_praind, c.es_personal_ruta, c.tiene_acceso
    FROM registro_ingresos AS r
    INNER JOIN contratistas AS c ON c.id = r.contratista_id
    INNER JOIN empresas AS e ON e.id = r.empresa_id
    INNER JOIN usuarios AS ui ON ui.id = r.usuario_ingreso_id
    WHERE r.fecha_hora_salida IS NULL
    ORDER BY r.fecha_hora_ingreso DESC, r.id DESC
    LIMIT ?1 OFFSET ?2
";

const ACTIVOS_CORTO_SQL: &str = "
    SELECT
        r.id, r.contratista_id, r.empresa_id, c.cedula, c.nombre, e.nombre,
        r.tipo_ingreso, r.medio_ingreso, r.fecha_hora_ingreso, r.gafete_numero,
        ui.nombre, c.fecha_vencimiento_praind, c.es_personal_ruta, c.tiene_acceso
    FROM registro_ingresos AS r
    INNER JOIN contratistas AS c ON c.id = r.contratista_id
    INNER JOIN empresas AS e ON e.id = r.empresa_id
    INNER JOIN usuarios AS ui ON ui.id = r.usuario_ingreso_id
    WHERE r.fecha_hora_salida IS NULL AND (
        c.cedula LIKE ?1 COLLATE NOCASE OR c.nombre LIKE ?1 COLLATE NOCASE
        OR e.nombre LIKE ?1 COLLATE NOCASE
        OR (?2 IS NOT NULL AND r.gafete_numero = ?2)
    )
    ORDER BY r.fecha_hora_ingreso DESC, r.id DESC
    LIMIT ?3 OFFSET ?4
";

const ACTIVOS_FTS_SQL: &str = "
    WITH contratistas_coincidentes(id) AS (
        SELECT rowid FROM contratistas_fts WHERE contratistas_fts MATCH ?1
        UNION
        SELECT c.id FROM empresas_fts
        INNER JOIN contratistas AS c ON c.empresa_id = empresas_fts.rowid
        WHERE empresas_fts MATCH ?1
    ), registros_coincidentes(id) AS (
        SELECT r.id FROM contratistas_coincidentes
        INNER JOIN registro_ingresos AS r
            ON r.contratista_id = contratistas_coincidentes.id
        WHERE r.fecha_hora_salida IS NULL
        UNION
        SELECT id FROM registro_ingresos
        WHERE ?2 IS NOT NULL AND gafete_numero = ?2 AND fecha_hora_salida IS NULL
    )
    SELECT
        r.id, r.contratista_id, r.empresa_id, c.cedula, c.nombre, e.nombre,
        r.tipo_ingreso, r.medio_ingreso, r.fecha_hora_ingreso, r.gafete_numero,
        ui.nombre, c.fecha_vencimiento_praind, c.es_personal_ruta, c.tiene_acceso
    FROM registros_coincidentes
    INNER JOIN registro_ingresos AS r ON r.id = registros_coincidentes.id
    INNER JOIN contratistas AS c ON c.id = r.contratista_id
    INNER JOIN empresas AS e ON e.id = r.empresa_id
    INNER JOIN usuarios AS ui ON ui.id = r.usuario_ingreso_id
    ORDER BY r.fecha_hora_ingreso DESC, r.id DESC
    LIMIT ?3 OFFSET ?4
";

const HISTORIAL_COLUMNAS: &str = "
    r.id, r.contratista_id, c.cedula, c.nombre, e.nombre, r.tipo_ingreso,
    r.medio_ingreso, r.fecha_hora_ingreso, r.fecha_hora_salida,
    r.gafete_numero, ui.nombre, us.nombre
";

const HISTORIAL_FROM_WHERE: &str = "
    FROM registro_ingresos AS r
    INNER JOIN contratistas AS c ON c.id = r.contratista_id
    INNER JOIN empresas AS e ON e.id = r.empresa_id
    INNER JOIN usuarios AS ui ON ui.id = r.usuario_ingreso_id
    LEFT JOIN usuarios AS us ON us.id = r.usuario_salida_id
    WHERE r.fecha_hora_ingreso >= :desde
      AND r.fecha_hora_ingreso < :hasta
      AND (
          :modo_busqueda = 0
          OR (:modo_busqueda = 1 AND (
              c.cedula LIKE :patron COLLATE NOCASE
              OR c.nombre LIKE :patron COLLATE NOCASE
          ))
          OR (:modo_busqueda = 2 AND c.id IN (
              SELECT rowid FROM contratistas_fts WHERE contratistas_fts MATCH :consulta_fts
          ))
      )
      AND (:empresa_id IS NULL OR r.empresa_id = :empresa_id)
      AND (:tipo IS NULL OR r.tipo_ingreso = :tipo)
      AND (:gafete IS NULL OR r.gafete_numero = :gafete)
      AND (
          :estado = 'TODOS'
          OR (:estado = 'ACTIVOS' AND r.fecha_hora_salida IS NULL)
          OR (:estado = 'CERRADOS' AND r.fecha_hora_salida IS NOT NULL)
      )
";

fn convertir_activo(row: &Row<'_>) -> rusqlite::Result<IngresoActivoLectura> {
    Ok(IngresoActivoLectura {
        registro_id: row.get(0)?,
        contratista_id: row.get(1)?,
        empresa_id: row.get(2)?,
        cedula: row.get(3)?,
        contratista_nombre: row.get(4)?,
        empresa_nombre: row.get(5)?,
        tipo_ingreso: tipo_desde_fila(row, 6)?,
        medio_ingreso: medio_desde_fila(row, 7)?,
        fecha_hora_ingreso: fecha_hora_desde_fila(row, 8)?,
        gafete_numero: row.get(9)?,
        usuario_ingreso_nombre: row.get(10)?,
        fecha_vencimiento_praind: fecha_desde_fila(row, 11)?,
        es_personal_ruta: row.get::<_, i64>(12)? != 0,
        tiene_acceso: row.get::<_, i64>(13)? != 0,
    })
}

fn convertir_movimiento(row: &Row<'_>) -> rusqlite::Result<MovimientoIngresoResumen> {
    Ok(MovimientoIngresoResumen {
        registro_id: row.get(0)?,
        contratista_id: row.get(1)?,
        cedula: row.get(2)?,
        contratista_nombre: row.get(3)?,
        empresa_nombre: row.get(4)?,
        tipo_ingreso: tipo_desde_fila(row, 5)?,
        medio_ingreso: medio_desde_fila(row, 6)?,
        fecha_hora_ingreso: fecha_hora_desde_fila(row, 7)?,
        fecha_hora_salida: fecha_hora_opcional_desde_fila(row, 8)?,
        gafete_numero: row.get(9)?,
        usuario_ingreso_nombre: row.get(10)?,
        usuario_salida_nombre: row.get(11)?,
    })
}

fn tipo_desde_fila(row: &Row<'_>, indice: usize) -> rusqlite::Result<TipoIngreso> {
    let valor: String = row.get(indice)?;
    match valor.as_str() {
        "PRAIND" => Ok(TipoIngreso::Praind),
        "IN_HOUSE" => Ok(TipoIngreso::InHouse),
        "POR_CORREO" => Ok(TipoIngreso::PorCorreo),
        "SWAT" => Ok(TipoIngreso::Swat),
        _ => Err(tipo_invalido(indice, "tipo_ingreso")),
    }
}

fn medio_desde_fila(row: &Row<'_>, indice: usize) -> rusqlite::Result<MedioIngreso> {
    let valor: String = row.get(indice)?;
    match valor.as_str() {
        "CAMINANDO" => Ok(MedioIngreso::Caminando),
        "VEHICULO" => Ok(MedioIngreso::Vehiculo),
        _ => Err(tipo_invalido(indice, "medio_ingreso")),
    }
}

fn fecha_desde_fila(row: &Row<'_>, indice: usize) -> rusqlite::Result<Option<NaiveDate>> {
    let valor: Option<String> = row.get(indice)?;
    valor.map(|fecha| parsear_fecha(&fecha, indice)).transpose()
}

fn fecha_hora_desde_fila(row: &Row<'_>, indice: usize) -> rusqlite::Result<NaiveDateTime> {
    let valor: String = row.get(indice)?;
    parsear_fecha_hora(&valor, indice)
}

fn fecha_hora_opcional_desde_fila(
    row: &Row<'_>,
    indice: usize,
) -> rusqlite::Result<Option<NaiveDateTime>> {
    let valor: Option<String> = row.get(indice)?;
    valor
        .map(|fecha| parsear_fecha_hora(&fecha, indice))
        .transpose()
}

fn parsear_fecha(valor: &str, indice: usize) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(valor, "%Y-%m-%d").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            indice,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn parsear_fecha_hora(valor: &str, indice: usize) -> rusqlite::Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(valor, "%Y-%m-%d %H:%M:%S").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            indice,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn tipo_a_texto(tipo: TipoIngreso) -> &'static str {
    match tipo {
        TipoIngreso::Praind => "PRAIND",
        TipoIngreso::InHouse => "IN_HOUSE",
        TipoIngreso::PorCorreo => "POR_CORREO",
        TipoIngreso::Swat => "SWAT",
    }
}

fn estado_a_texto(estado: EstadoMovimiento) -> &'static str {
    match estado {
        EstadoMovimiento::Todos => "TODOS",
        EstadoMovimiento::Activos => "ACTIVOS",
        EstadoMovimiento::Cerrados => "CERRADOS",
    }
}

fn fecha_hora_a_texto(fecha: NaiveDateTime) -> String {
    fecha.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn offset_sql(offset: usize) -> i64 {
    i64::try_from(offset).unwrap_or(i64::MAX)
}

fn tipo_invalido(indice: usize, nombre: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(indice, nombre.to_owned(), rusqlite::types::Type::Text)
}
