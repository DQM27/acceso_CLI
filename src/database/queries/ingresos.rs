use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, Row, Transaction, TransactionBehavior, named_params};

use crate::database::error::DatabaseError;
use crate::database::search::BusquedaTexto;
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::registro_ingreso::{MotivoResultadoIngreso, ResultadoIngresoRegistrado};
use crate::models::tipo_ingreso::TipoIngreso;
use crate::tiempo::{parsear_utc, serializar_utc};

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
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: String,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FiltroIngresosActivos {
    pub texto: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListaIngresosActivosLectura {
    pub items: Vec<IngresoActivoLectura>,
    pub total: usize,
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
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub fecha_hora_salida: Option<DateTime<Utc>>,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: String,
    pub usuario_salida_nombre: Option<String>,
    pub resultado_acceso: ResultadoIngresoRegistrado,
    pub motivo_resultado: Option<MotivoResultadoIngreso>,
    pub reglas_version: i64,
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
    pub desde: DateTime<Utc>,
    /// Límite superior exclusivo aplicado a `fecha_hora_ingreso`.
    pub hasta: DateTime<Utc>,
    pub texto_persona: Option<String>,
    pub empresa_id: Option<i64>,
    pub tipo_ingreso: Option<TipoIngreso>,
    pub gafete_numero: Option<i64>,
    pub estado: EstadoMovimiento,
    pub limite: usize,
    pub offset: usize,
    /// ID máximo visible en esta navegación. Excluye ingresos creados después de
    /// cargar la primera página para que las páginas no se desplacen.
    pub corte_id: Option<i64>,
}

impl FiltroHistorial {
    pub fn nuevo(desde: DateTime<Utc>, hasta: DateTime<Utc>) -> Self {
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
            corte_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginaHistorial {
    pub items: Vec<MovimientoIngresoResumen>,
    pub total: usize,
    pub corte_id: i64,
}

pub trait IngresosQuery {
    fn listar_activos(
        &self,
        filtro: &FiltroIngresosActivos,
    ) -> Result<ListaIngresosActivosLectura, DatabaseError>;

    fn buscar_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<IngresoActivoLectura>, DatabaseError>;

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
    ) -> Result<ListaIngresosActivosLectura, DatabaseError> {
        let busqueda = BusquedaTexto::preparar(filtro.texto.as_deref());
        let total: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM registro_ingresos WHERE fecha_hora_salida IS NULL",
            [],
            |row| row.get(0),
        )?;

        let (sql, parametros): (&str, Vec<rusqlite::types::Value>) = match busqueda.modo {
            1 => (
                ACTIVOS_CORTO_SQL,
                vec![busqueda.patron_like.into(), busqueda.numero_exacto.into()],
            ),
            2 => (
                ACTIVOS_FTS_SQL,
                vec![busqueda.consulta_fts.into(), busqueda.numero_exacto.into()],
            ),
            _ => (ACTIVOS_SIN_FILTRO_SQL, vec![]),
        };
        let mut statement = self.connection.prepare(sql)?;
        let items = statement
            .query_map(rusqlite::params_from_iter(parametros), convertir_activo)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ListaIngresosActivosLectura {
            items,
            total: usize::try_from(total).unwrap_or(usize::MAX),
        })
    }

    fn buscar_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<IngresoActivoLectura>, DatabaseError> {
        let mut statement = self.connection.prepare(ACTIVO_POR_GAFETE_SQL)?;
        match statement.query_row([gafete_numero], convertir_activo) {
            Ok(registro) => Ok(Some(registro)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn buscar_historial(&self, filtro: &FiltroHistorial) -> Result<PaginaHistorial, DatabaseError> {
        let transaction =
            Transaction::new_unchecked(self.connection, TransactionBehavior::Deferred)?;
        let busqueda = BusquedaTexto::preparar(filtro.texto_persona.as_deref());
        let tipo = filtro.tipo_ingreso.map(tipo_a_texto);
        let estado = estado_a_texto(filtro.estado);
        let desde = fecha_hora_a_texto(filtro.desde);
        let hasta = fecha_hora_a_texto(filtro.hasta);
        let limite = filtro.limite.clamp(1, LIMITE_HISTORIAL_MAXIMO) as i64;
        let offset = offset_sql(filtro.offset);
        let corte_id = match filtro.corte_id {
            Some(corte_id) => corte_id,
            None => transaction.query_row(
                "SELECT COALESCE(MAX(id), 0) FROM registro_ingresos",
                [],
                |row| row.get(0),
            )?,
        };

        let count_sql = format!("SELECT COUNT(*) {HISTORIAL_FROM_WHERE}");
        let total: i64 = transaction.query_row(
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
                ":corte_id": corte_id,
            },
            |row| row.get(0),
        )?;

        let select_sql = format!(
            "SELECT {HISTORIAL_COLUMNAS} {HISTORIAL_FROM_WHERE} \
             ORDER BY r.fecha_hora_ingreso DESC, r.id DESC LIMIT :limite OFFSET :offset"
        );
        let mut statement = transaction.prepare(&select_sql)?;
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
                    ":corte_id": corte_id,
                    ":limite": limite,
                    ":offset": offset,
                },
                convertir_movimiento,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        transaction.commit()?;

        Ok(PaginaHistorial {
            items,
            total: usize::try_from(total).unwrap_or(usize::MAX),
            corte_id,
        })
    }
}

const ACTIVOS_SIN_FILTRO_SQL: &str = "
    SELECT
        r.id, r.contratista_id, r.empresa_id, r.contratista_cedula,
        r.contratista_nombre, r.empresa_nombre,
        r.tipo_ingreso, r.medio_ingreso, r.fecha_hora_ingreso, r.gafete_numero,
        r.usuario_ingreso_nombre, c.fecha_vencimiento_praind,
        c.es_personal_ruta, c.tiene_acceso
    FROM registro_ingresos AS r
    INNER JOIN contratistas AS c ON c.id = r.contratista_id
    WHERE r.fecha_hora_salida IS NULL
    ORDER BY r.fecha_hora_ingreso DESC, r.id DESC
";

const ACTIVOS_CORTO_SQL: &str = "
    SELECT
        r.id, r.contratista_id, r.empresa_id, r.contratista_cedula,
        r.contratista_nombre, r.empresa_nombre,
        r.tipo_ingreso, r.medio_ingreso, r.fecha_hora_ingreso, r.gafete_numero,
        r.usuario_ingreso_nombre, c.fecha_vencimiento_praind,
        c.es_personal_ruta, c.tiene_acceso
    FROM registro_ingresos AS r
    INNER JOIN contratistas AS c ON c.id = r.contratista_id
    WHERE r.fecha_hora_salida IS NULL AND (
        r.contratista_cedula LIKE ?1 COLLATE NOCASE
        OR r.contratista_nombre LIKE ?1 COLLATE NOCASE
        OR r.empresa_nombre LIKE ?1 COLLATE NOCASE
        OR (?2 IS NOT NULL AND r.gafete_numero = ?2)
    )
    ORDER BY r.fecha_hora_ingreso DESC, r.id DESC
";

const ACTIVOS_FTS_SQL: &str = "
    WITH registros_coincidentes(id) AS (
        SELECT rowid FROM registro_ingresos_fts
        WHERE registro_ingresos_fts MATCH ?1
        UNION
        SELECT id FROM registro_ingresos
        WHERE ?2 IS NOT NULL AND gafete_numero = ?2 AND fecha_hora_salida IS NULL
    )
    SELECT
        r.id, r.contratista_id, r.empresa_id, r.contratista_cedula,
        r.contratista_nombre, r.empresa_nombre,
        r.tipo_ingreso, r.medio_ingreso, r.fecha_hora_ingreso, r.gafete_numero,
        r.usuario_ingreso_nombre, c.fecha_vencimiento_praind,
        c.es_personal_ruta, c.tiene_acceso
    FROM registros_coincidentes
    INNER JOIN registro_ingresos AS r ON r.id = registros_coincidentes.id
    INNER JOIN contratistas AS c ON c.id = r.contratista_id
    ORDER BY r.fecha_hora_ingreso DESC, r.id DESC
";

const ACTIVO_POR_GAFETE_SQL: &str = "
    SELECT
        r.id, r.contratista_id, r.empresa_id, r.contratista_cedula,
        r.contratista_nombre, r.empresa_nombre,
        r.tipo_ingreso, r.medio_ingreso, r.fecha_hora_ingreso, r.gafete_numero,
        r.usuario_ingreso_nombre, c.fecha_vencimiento_praind,
        c.es_personal_ruta, c.tiene_acceso
    FROM registro_ingresos AS r
    INNER JOIN contratistas AS c ON c.id = r.contratista_id
    WHERE r.fecha_hora_salida IS NULL AND r.gafete_numero = ?1
    LIMIT 1
";

const HISTORIAL_COLUMNAS: &str = "
    r.id, r.contratista_id, r.contratista_cedula, r.contratista_nombre,
    r.empresa_nombre, r.tipo_ingreso,
    r.medio_ingreso, r.fecha_hora_ingreso, r.fecha_hora_salida,
    r.gafete_numero, r.usuario_ingreso_nombre, r.usuario_salida_nombre,
    r.resultado_acceso, r.motivo_resultado, r.reglas_version
";

const HISTORIAL_FROM_WHERE: &str = "
    FROM registro_ingresos AS r
    WHERE r.fecha_hora_ingreso >= :desde
      AND r.fecha_hora_ingreso < :hasta
      AND r.id <= :corte_id
      AND (
          :modo_busqueda = 0
          OR (:modo_busqueda = 1 AND (
              r.contratista_cedula LIKE :patron COLLATE NOCASE
              OR r.contratista_nombre LIKE :patron COLLATE NOCASE
          ))
          OR (:modo_busqueda = 2 AND r.id IN (
              SELECT rowid FROM registro_ingresos_fts
              WHERE registro_ingresos_fts MATCH :consulta_fts
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
        resultado_acceso: resultado_desde_fila(row, 12)?,
        motivo_resultado: motivo_desde_fila(row, 13)?,
        reglas_version: row.get(14)?,
    })
}

fn resultado_desde_fila(
    row: &Row<'_>,
    indice: usize,
) -> rusqlite::Result<ResultadoIngresoRegistrado> {
    let valor: String = row.get(indice)?;
    match valor.as_str() {
        "PERMITIDO" => Ok(ResultadoIngresoRegistrado::Permitido),
        "PERMITIDO_CON_ADVERTENCIA" => Ok(ResultadoIngresoRegistrado::PermitidoConAdvertencia),
        "MIGRADO" => Ok(ResultadoIngresoRegistrado::Migrado),
        _ => Err(tipo_invalido(indice, "resultado_acceso")),
    }
}

fn motivo_desde_fila(
    row: &Row<'_>,
    indice: usize,
) -> rusqlite::Result<Option<MotivoResultadoIngreso>> {
    let valor: Option<String> = row.get(indice)?;
    valor
        .map(|motivo| match motivo.as_str() {
            "PRAIND_PROXIMO_VENCER" => Ok(MotivoResultadoIngreso::PraindProximoVencer),
            "DATOS_RECONSTRUIDOS" => Ok(MotivoResultadoIngreso::DatosReconstruidos),
            _ => Err(tipo_invalido(indice, "motivo_resultado")),
        })
        .transpose()
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

fn fecha_hora_desde_fila(row: &Row<'_>, indice: usize) -> rusqlite::Result<DateTime<Utc>> {
    let valor: String = row.get(indice)?;
    parsear_fecha_hora(&valor, indice)
}

fn fecha_hora_opcional_desde_fila(
    row: &Row<'_>,
    indice: usize,
) -> rusqlite::Result<Option<DateTime<Utc>>> {
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

fn parsear_fecha_hora(valor: &str, indice: usize) -> rusqlite::Result<DateTime<Utc>> {
    parsear_utc(valor).map_err(|error| {
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

fn fecha_hora_a_texto(fecha: DateTime<Utc>) -> String {
    serializar_utc(fecha)
}

fn offset_sql(offset: usize) -> i64 {
    i64::try_from(offset).unwrap_or(i64::MAX)
}

fn tipo_invalido(indice: usize, nombre: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(indice, nombre.to_owned(), rusqlite::types::Type::Text)
}
