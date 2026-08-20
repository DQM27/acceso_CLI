use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, Row, Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::search::BusquedaTexto;
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::registro_ingreso::{MotivoResultadoIngreso, ResultadoIngresoRegistrado};
use crate::models::tipo_ingreso::TipoIngreso;
use crate::tiempo::{parsear_utc, serializar_utc};

/// Más chico que `queries::LIMITE_LISTADO_PREDETERMINADO`/`_MAXIMO` (los que
/// comparten Contratistas/Empresas/Usuarios) a propósito: `registro_ingresos`
/// es append-only y crece sin límite, así que Historial pagina en ventanas
/// más chicas — no es una inconsistencia sin explicar
/// (`docs/hallazgos-buscador.md`).
const LIMITE_HISTORIAL_PREDETERMINADO: usize = 50;
const LIMITE_HISTORIAL_MAXIMO: usize = 200;
/// Tope de seguridad para Ingresos Activos, la única consulta de la app que
/// antes no tenía ninguno. No hay paginación en esa pantalla (a diferencia de
/// Historial) — este límite es sólo para no cargar sin fin si algún día el
/// número de ingresos sin cerrar crece de forma anómala.
const LIMITE_ACTIVOS_PREDETERMINADO: usize = 1000;

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
    pub empresa_activa: bool,
    pub resultado_registrado: ResultadoIngresoRegistrado,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroIngresosActivos {
    pub texto: Option<String>,
    pub empresa_id: Option<i64>,
    /// `None` = todos los tipos; `Some(vec)` filtra a cualquiera de los
    /// listados (como máximo 4, la cantidad de variantes de `TipoIngreso`).
    pub tipos_incluidos: Option<Vec<TipoIngreso>>,
    pub gafete_numero: Option<i64>,
    pub medio_ingreso: Option<MedioIngreso>,
    pub limite: usize,
}

impl Default for FiltroIngresosActivos {
    fn default() -> Self {
        Self {
            texto: None,
            empresa_id: None,
            tipos_incluidos: None,
            gafete_numero: None,
            medio_ingreso: None,
            limite: LIMITE_ACTIVOS_PREDETERMINADO,
        }
    }
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
    /// Si la empresa del contratista estaba activa al momento de este
    /// ingreso — parte de la fotografía histórica, no un dato recalculado
    /// (`docs/auditoria-dominio-2026-08-20.md`, hallazgo #7).
    pub empresa_activa_snapshot: bool,
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
    /// `None` = todos los tipos. `Some(vec)` filtra a cualquiera de los tipos
    /// listados (como un `IN`); como máximo 4 (la cantidad de variantes de
    /// `TipoIngreso`), los excedentes se ignoran.
    pub tipos_incluidos: Option<Vec<TipoIngreso>>,
    pub gafete_numero: Option<i64>,
    pub estado: EstadoMovimiento,
    /// Nombre (parcial, sin distinguir mayúsculas) del usuario que registró
    /// el ingreso.
    pub usuario_ingreso: Option<String>,
    /// Nombre (parcial, sin distinguir mayúsculas) del usuario que registró
    /// la salida. Un movimiento sin salida nunca matchea.
    pub usuario_salida: Option<String>,
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
            tipos_incluidos: None,
            gafete_numero: None,
            estado: EstadoMovimiento::Todos,
            usuario_ingreso: None,
            usuario_salida: None,
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
        // Total real de personas dentro, sin aplicar el filtro — decisión
        // deliberada (ver docs/hallazgos-buscador.md): la UI usa "N de M
        // dentro" con M = total sin filtrar, no M = coincidencias.
        let total: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM registro_ingresos WHERE fecha_hora_salida IS NULL",
            [],
            |row| row.get(0),
        )?;

        let limite = filtro.limite.clamp(1, LIMITE_ACTIVOS_PREDETERMINADO) as i64;
        let (where_sql, parametros) = construir_where_activos(&busqueda, filtro);
        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = parametros
            .iter()
            .map(|(n, v)| (n.as_str(), v.as_ref()))
            .collect();
        params.push((":limite", &limite));

        let select_sql = format!(
            "SELECT
                r.id, r.contratista_id, r.empresa_id, r.contratista_cedula,
                r.contratista_nombre, r.empresa_nombre,
                r.tipo_ingreso, r.medio_ingreso, r.fecha_hora_ingreso, r.gafete_numero,
                r.usuario_ingreso_nombre, c.fecha_vencimiento_praind,
                c.es_personal_ruta, c.tiene_acceso, e.activo,
                r.resultado_acceso, r.motivo_resultado
             FROM registro_ingresos AS r
             INNER JOIN contratistas AS c ON c.id = r.contratista_id
             INNER JOIN empresas AS e ON e.id = c.empresa_id
             {where_sql}
             ORDER BY r.fecha_hora_ingreso DESC, r.id DESC
             LIMIT :limite"
        );
        let mut statement = self.connection.prepare(&select_sql)?;
        let items = statement
            .query_map(params.as_slice(), convertir_activo)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ListaIngresosActivosLectura {
            items,
            total: usize::try_from(total).unwrap_or(usize::MAX),
        })
    }

    fn buscar_historial(&self, filtro: &FiltroHistorial) -> Result<PaginaHistorial, DatabaseError> {
        let transaction =
            Transaction::new_unchecked(self.connection, TransactionBehavior::Deferred)?;
        let busqueda = BusquedaTexto::preparar(filtro.texto_persona.as_deref());
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

        let (where_sql, parametros) = construir_where_historial(&busqueda, filtro, corte_id);
        let params_comunes: Vec<(&str, &dyn rusqlite::ToSql)> = parametros
            .iter()
            .map(|(n, v)| (n.as_str(), v.as_ref()))
            .collect();

        let count_sql = format!("SELECT COUNT(*) FROM registro_ingresos AS r {where_sql}");
        let total: i64 =
            transaction.query_row(&count_sql, params_comunes.as_slice(), |row| row.get(0))?;

        let select_sql = format!(
            "SELECT {HISTORIAL_COLUMNAS} FROM registro_ingresos AS r {where_sql} \
             ORDER BY r.fecha_hora_ingreso DESC, r.id DESC LIMIT :limite OFFSET :offset"
        );
        let mut params_select = params_comunes;
        params_select.push((":limite", &limite));
        params_select.push((":offset", &offset));

        let mut statement = transaction.prepare(&select_sql)?;
        let items = statement
            .query_map(params_select.as_slice(), convertir_movimiento)?
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

const HISTORIAL_COLUMNAS: &str = "
    r.id, r.contratista_id, r.contratista_cedula, r.contratista_nombre,
    r.empresa_nombre, r.tipo_ingreso,
    r.medio_ingreso, r.fecha_hora_ingreso, r.fecha_hora_salida,
    r.gafete_numero, r.usuario_ingreso_nombre, r.usuario_salida_nombre,
    r.resultado_acceso, r.motivo_resultado, r.reglas_version,
    r.empresa_activa_snapshot
";

/// Nombre de parámetro (`:algo`) y valor bindeado.
type ParametrosSql = Vec<(String, Box<dyn rusqlite::ToSql>)>;

/// Igual que `contratistas::construir_where`: arma el `WHERE` sólo con las
/// condiciones realmente activas, sin flags `:x IS NULL OR col = :x`
/// evaluados en cada fila — eso es lo que le impedía a SQLite usar
/// `idx_registro_ingresos_empresa` aunque existiera (confirmado con
/// `EXPLAIN QUERY PLAN`, `docs/hallazgos-buscador.md`).
fn construir_where_activos(
    busqueda: &BusquedaTexto,
    filtro: &FiltroIngresosActivos,
) -> (String, ParametrosSql) {
    let mut condiciones: Vec<String> = vec!["r.fecha_hora_salida IS NULL".into()];
    let mut parametros: ParametrosSql = Vec::new();

    match busqueda.modo {
        1 => {
            let mut ramas = vec![
                "PLEGAR(r.contratista_cedula) LIKE PLEGAR(:patron)".to_string(),
                "PLEGAR(r.contratista_nombre) LIKE PLEGAR(:patron)".to_string(),
                "PLEGAR(r.empresa_nombre) LIKE PLEGAR(:patron)".to_string(),
            ];
            parametros.push((":patron".into(), Box::new(busqueda.patron_like.clone())));
            if let Some(numero) = busqueda.numero_exacto {
                ramas.push("r.gafete_numero = :numero_exacto".to_string());
                parametros.push((":numero_exacto".into(), Box::new(numero)));
            }
            condiciones.push(format!("({})", ramas.join(" OR ")));
        }
        2 => {
            let mut sub =
                "SELECT rowid FROM registro_ingresos_fts WHERE registro_ingresos_fts MATCH :consulta_fts"
                    .to_string();
            parametros.push((
                ":consulta_fts".into(),
                Box::new(busqueda.consulta_fts.clone()),
            ));
            if let Some(numero) = busqueda.numero_exacto {
                sub.push_str(
                    " UNION SELECT id FROM registro_ingresos \
                      WHERE gafete_numero = :numero_exacto AND fecha_hora_salida IS NULL",
                );
                parametros.push((":numero_exacto".into(), Box::new(numero)));
            }
            condiciones.push(format!("r.id IN ({sub})"));
        }
        _ => {}
    }

    if let Some(empresa_id) = filtro.empresa_id {
        condiciones.push("r.empresa_id = :empresa_id".into());
        parametros.push((":empresa_id".into(), Box::new(empresa_id)));
    }
    if let Some(tipos) = &filtro.tipos_incluidos {
        let marcadores: Vec<String> = (0..tipos.len()).map(|i| format!(":tipo{i}")).collect();
        condiciones.push(format!("r.tipo_ingreso IN ({})", marcadores.join(", ")));
        for (marcador, tipo) in marcadores.into_iter().zip(tipos.iter()) {
            parametros.push((marcador, Box::new(tipo.as_str_sql())));
        }
    }
    if let Some(gafete) = filtro.gafete_numero {
        condiciones.push("r.gafete_numero = :gafete".into());
        parametros.push((":gafete".into(), Box::new(gafete)));
    }
    if let Some(medio) = filtro.medio_ingreso {
        condiciones.push("r.medio_ingreso = :medio".into());
        parametros.push((":medio".into(), Box::new(medio_a_texto(medio))));
    }

    (format!("WHERE {}", condiciones.join(" AND ")), parametros)
}

/// Mismo criterio que `construir_where_activos`. También resuelve el
/// hallazgo "Historial no encuentra por número de gafete en texto libre"
/// (`docs/hallazgos-buscador.md`): `registro_ingresos_fts` no indexa
/// `gafete_numero` (sólo cédula/nombre/empresa), así que el modo FTS/LIKE
/// necesita la misma unión explícita por gafete exacto que ya usa
/// `construir_where_activos` — antes esta función no la tenía.
fn construir_where_historial(
    busqueda: &BusquedaTexto,
    filtro: &FiltroHistorial,
    corte_id: i64,
) -> (String, ParametrosSql) {
    let mut condiciones: Vec<String> = vec![
        "r.fecha_hora_ingreso >= :desde".into(),
        "r.fecha_hora_ingreso < :hasta".into(),
        "r.id <= :corte_id".into(),
    ];
    let mut parametros: ParametrosSql = vec![
        (":desde".into(), Box::new(fecha_hora_a_texto(filtro.desde))),
        (":hasta".into(), Box::new(fecha_hora_a_texto(filtro.hasta))),
        (":corte_id".into(), Box::new(corte_id)),
    ];

    match busqueda.modo {
        1 => {
            let mut ramas = vec![
                "PLEGAR(r.contratista_cedula) LIKE PLEGAR(:patron)".to_string(),
                "PLEGAR(r.contratista_nombre) LIKE PLEGAR(:patron)".to_string(),
            ];
            parametros.push((":patron".into(), Box::new(busqueda.patron_like.clone())));
            if let Some(numero) = busqueda.numero_exacto {
                ramas.push("r.gafete_numero = :numero_exacto".to_string());
                parametros.push((":numero_exacto".into(), Box::new(numero)));
            }
            condiciones.push(format!("({})", ramas.join(" OR ")));
        }
        2 => {
            let mut sub =
                "SELECT rowid FROM registro_ingresos_fts WHERE registro_ingresos_fts MATCH :consulta_fts"
                    .to_string();
            parametros.push((
                ":consulta_fts".into(),
                Box::new(busqueda.consulta_fts.clone()),
            ));
            if let Some(numero) = busqueda.numero_exacto {
                sub.push_str(
                    " UNION SELECT id FROM registro_ingresos WHERE gafete_numero = :numero_exacto",
                );
                parametros.push((":numero_exacto".into(), Box::new(numero)));
            }
            condiciones.push(format!("r.id IN ({sub})"));
        }
        _ => {}
    }

    if let Some(empresa_id) = filtro.empresa_id {
        condiciones.push("r.empresa_id = :empresa_id".into());
        parametros.push((":empresa_id".into(), Box::new(empresa_id)));
    }
    if let Some(tipos) = &filtro.tipos_incluidos {
        let marcadores: Vec<String> = (0..tipos.len()).map(|i| format!(":tipo{i}")).collect();
        condiciones.push(format!("r.tipo_ingreso IN ({})", marcadores.join(", ")));
        for (marcador, tipo) in marcadores.into_iter().zip(tipos.iter()) {
            parametros.push((marcador, Box::new(tipo.as_str_sql())));
        }
    }
    if let Some(gafete) = filtro.gafete_numero {
        condiciones.push("r.gafete_numero = :gafete".into());
        parametros.push((":gafete".into(), Box::new(gafete)));
    }
    match filtro.estado {
        EstadoMovimiento::Todos => {}
        EstadoMovimiento::Activos => condiciones.push("r.fecha_hora_salida IS NULL".into()),
        EstadoMovimiento::Cerrados => condiciones.push("r.fecha_hora_salida IS NOT NULL".into()),
    }
    if let Some(usuario_ingreso) = &filtro.usuario_ingreso {
        condiciones.push("r.usuario_ingreso_nombre LIKE :usuario_ingreso COLLATE NOCASE".into());
        parametros.push((
            ":usuario_ingreso".into(),
            Box::new(patron_like(usuario_ingreso)),
        ));
    }
    if let Some(usuario_salida) = &filtro.usuario_salida {
        condiciones.push("r.usuario_salida_nombre LIKE :usuario_salida COLLATE NOCASE".into());
        parametros.push((
            ":usuario_salida".into(),
            Box::new(patron_like(usuario_salida)),
        ));
    }

    (format!("WHERE {}", condiciones.join(" AND ")), parametros)
}

fn convertir_activo(row: &Row<'_>) -> rusqlite::Result<IngresoActivoLectura> {
    let motivo_resultado = motivo_desde_fila(row, 16)?;
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
        empresa_activa: row.get::<_, i64>(14)? != 0,
        resultado_registrado: resultado_desde_fila(row, 15, motivo_resultado)?,
    })
}

fn convertir_movimiento(row: &Row<'_>) -> rusqlite::Result<MovimientoIngresoResumen> {
    let motivo_resultado = motivo_desde_fila(row, 13)?;
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
        resultado_acceso: resultado_desde_fila(row, 12, motivo_resultado)?,
        motivo_resultado,
        reglas_version: row.get(14)?,
        empresa_activa_snapshot: row.get::<_, i64>(15)? != 0,
    })
}

/// `motivo` viene de la columna `motivo_resultado` (13), ya parseada aparte —
/// el CHECK de `MIGRACION_5` garantiza que sólo viene `Some` cuando el texto
/// crudo es `PERMITIDO_CON_ADVERTENCIA` o `MIGRADO`.
fn resultado_desde_fila(
    row: &Row<'_>,
    indice: usize,
    motivo: Option<MotivoResultadoIngreso>,
) -> rusqlite::Result<ResultadoIngresoRegistrado> {
    let valor: String = row.get(indice)?;
    match valor.as_str() {
        "PERMITIDO" => Ok(ResultadoIngresoRegistrado::Permitido),
        "PERMITIDO_CON_ADVERTENCIA" => motivo
            .map(ResultadoIngresoRegistrado::PermitidoConAdvertencia)
            .ok_or_else(|| tipo_invalido(indice, "resultado_acceso")),
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
    TipoIngreso::from_str_sql(&valor).ok_or_else(|| tipo_invalido(indice, "tipo_ingreso"))
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

fn patron_like(texto: &str) -> String {
    format!("%{}%", texto.trim())
}

fn medio_a_texto(medio: MedioIngreso) -> &'static str {
    match medio {
        MedioIngreso::Caminando => "CAMINANDO",
        MedioIngreso::Vehiculo => "VEHICULO",
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

/// Instante más reciente entre todos los movimientos de entrada/salida
/// registrados — `None` si nunca hubo ninguno. Usado por `AppCore` para
/// detectar si el reloj del equipo retrocedió respecto al último movimiento
/// conocido; vive aquí (no en `application.rs`) para que la única consulta
/// SQL de esa validación quede junto al resto del acceso a `registro_ingresos`.
pub fn ultimo_instante_movimiento(
    connection: &Connection,
) -> Result<Option<DateTime<Utc>>, DatabaseError> {
    let ultima: Option<String> = connection.query_row(
        "SELECT MAX(instante) FROM (
            SELECT fecha_hora_ingreso AS instante FROM registro_ingresos
            UNION ALL
            SELECT fecha_hora_salida FROM registro_ingresos
            WHERE fecha_hora_salida IS NOT NULL
         )",
        [],
        |row| row.get(0),
    )?;
    ultima
        .map(|texto| {
            parsear_utc(&texto).map_err(|error| DatabaseError::FechaCorrupta(error.to_string()))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    fn plan(
        connection: &Connection,
        sql: &str,
        params: &[(&str, &dyn rusqlite::ToSql)],
    ) -> Vec<String> {
        let mut statement = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap();
        statement
            .query_map(params, |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// Regresión directa del hallazgo "WHERE con flags dinámicos impide usar
    /// índices" (`docs/hallazgos-buscador.md`) para Activos.
    #[test]
    fn activos_filtrar_por_empresa_usa_el_indice() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();

        let busqueda = BusquedaTexto::preparar(None);
        let filtro = FiltroIngresosActivos {
            empresa_id: Some(1),
            ..FiltroIngresosActivos::default()
        };
        let (where_sql, parametros) = construir_where_activos(&busqueda, &filtro);
        let params: Vec<(&str, &dyn rusqlite::ToSql)> = parametros
            .iter()
            .map(|(n, v)| (n.as_str(), v.as_ref()))
            .collect();
        let detalles = plan(
            &connection,
            &format!("SELECT COUNT(*) FROM registro_ingresos AS r {where_sql}"),
            &params,
        );
        assert!(
            detalles
                .iter()
                .any(|d| d.contains("idx_registro_ingresos_empresa")),
            "{detalles:?}"
        );
    }

    /// Mismo hallazgo, para Historial: el rango de fechas (siempre presente,
    /// nunca opcional) debe resolver con `idx_registro_ingresos_fecha_ingreso`.
    #[test]
    fn historial_rango_de_fechas_usa_el_indice() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();

        let busqueda = BusquedaTexto::preparar(None);
        let filtro = FiltroHistorial::nuevo(
            "2026-01-01T00:00:00Z".parse().unwrap(),
            "2026-02-01T00:00:00Z".parse().unwrap(),
        );
        let (where_sql, parametros) = construir_where_historial(&busqueda, &filtro, 0);
        let params: Vec<(&str, &dyn rusqlite::ToSql)> = parametros
            .iter()
            .map(|(n, v)| (n.as_str(), v.as_ref()))
            .collect();
        let detalles = plan(
            &connection,
            &format!("SELECT COUNT(*) FROM registro_ingresos AS r {where_sql}"),
            &params,
        );
        assert!(
            detalles
                .iter()
                .any(|d| d.contains("idx_registro_ingresos_fecha_ingreso")),
            "{detalles:?}"
        );
    }
}
