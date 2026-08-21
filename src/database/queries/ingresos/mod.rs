//! Dos familias de lectura sobre `registro_ingresos` con requisitos de
//! rendimiento y paginación distintos — Ingresos Activos (sin paginar, tope
//! de seguridad) e Historial (paginado, ventanas más chicas porque la tabla
//! es append-only y crece sin límite). Los conversores de fila y el manejo
//! de errores de parseo que ambas comparten quedan aquí; cada consulta y su
//! `WHERE` dinámico viven en su propio submódulo.

mod activos;
mod historial;

pub use activos::{FiltroIngresosActivos, IngresoActivoLectura, ListaIngresosActivosLectura};
pub use historial::{EstadoMovimiento, FiltroHistorial, MovimientoIngresoResumen, PaginaHistorial};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row};

use crate::database::error::DatabaseError;
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::registro_ingreso::{MotivoResultadoIngreso, ResultadoIngresoRegistrado};
use crate::models::tipo_ingreso::TipoIngreso;
use crate::tiempo::parsear_utc;

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
        activos::listar_activos(self.connection, filtro)
    }

    fn buscar_historial(&self, filtro: &FiltroHistorial) -> Result<PaginaHistorial, DatabaseError> {
        historial::buscar_historial(self.connection, filtro)
    }
}

/// `motivo` viene de la columna `motivo_resultado`, ya parseada aparte — el
/// CHECK de `MIGRACION_5` garantiza que sólo viene `Some` cuando el texto
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

fn fecha_hora_desde_fila(row: &Row<'_>, indice: usize) -> rusqlite::Result<DateTime<Utc>> {
    let valor: String = row.get(indice)?;
    parsear_fecha_hora(&valor, indice)
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

fn tipo_invalido(indice: usize, nombre: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(indice, nombre.to_owned(), rusqlite::types::Type::Text)
}

const ULTIMO_INSTANTE_MOVIMIENTO_SQL: &str = "
    SELECT MAX(instante)
    FROM (
        SELECT MAX(fecha_hora_ingreso) AS instante
        FROM registro_ingresos
        UNION ALL
        SELECT MAX(fecha_hora_salida) AS instante
        FROM registro_ingresos
        WHERE fecha_hora_salida IS NOT NULL
    )";

/// Instante más reciente entre todos los movimientos de entrada/salida
/// registrados — `None` si nunca hubo ninguno. Usado por `AppCore` para
/// detectar si el reloj del equipo retrocedió respecto al último movimiento
/// conocido; vive aquí (no en `application`) para que la única consulta SQL
/// de esa validación quede junto al resto del acceso a `registro_ingresos`.
pub fn ultimo_instante_movimiento(
    connection: &Connection,
) -> Result<Option<DateTime<Utc>>, DatabaseError> {
    let ultima: Option<String> =
        connection.query_row(ULTIMO_INSTANTE_MOVIMIENTO_SQL, [], |row| row.get(0))?;
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

    fn conexion_con_referencias() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO empresas (id, nombre) VALUES (1, 'Empresa');
                 INSERT INTO usuarios (
                    id, cedula, nombre, password_hash, rol, activo
                 ) VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
                 INSERT INTO contratistas (
                    id, cedula, nombre, empresa_id, tipo_ingreso,
                    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
                 ) VALUES (
                    1, '2001', 'Persona', 1, 'PRAIND', '2030-01-01', 0, 1
                 );",
            )
            .unwrap();
        connection
    }

    fn insertar_ingreso(connection: &Connection, id: i64, fecha_hora_ingreso: &str) {
        connection
            .execute(
                "INSERT INTO registro_ingresos(
                    id, contratista_id, empresa_id, fecha_hora_ingreso,
                    medio_ingreso, tipo_ingreso, gafete_numero,
                    usuario_ingreso_id, contratista_cedula,
                    contratista_nombre, empresa_nombre, usuario_ingreso_nombre,
                    fecha_vencimiento_praind, es_personal_ruta, tiene_acceso,
                    resultado_acceso, motivo_resultado, reglas_version
                 ) VALUES (
                    ?1, 1, 1, ?2, 'CAMINANDO', 'PRAIND', NULL, 1,
                    '2001', 'Persona', 'Empresa', 'Operador', '2030-01-01',
                    0, 1, 'PERMITIDO', NULL, 1
                 )",
                rusqlite::params![id, fecha_hora_ingreso],
            )
            .unwrap();
    }

    #[test]
    fn ultimo_instante_movimiento_conserva_semantica_de_entradas_y_salidas() {
        let connection = conexion_con_referencias();

        assert_eq!(ultimo_instante_movimiento(&connection).unwrap(), None);

        insertar_ingreso(&connection, 1, "2026-08-21T10:00:00Z");
        assert_eq!(
            ultimo_instante_movimiento(&connection).unwrap(),
            Some(parsear_utc("2026-08-21T10:00:00Z").unwrap())
        );

        connection
            .execute(
                "UPDATE registro_ingresos
                 SET fecha_hora_salida = '2026-08-21T11:00:00Z',
                     usuario_salida_id = 1,
                     usuario_salida_nombre = 'Operador'
                 WHERE id = 1",
                [],
            )
            .unwrap();
        assert_eq!(
            ultimo_instante_movimiento(&connection).unwrap(),
            Some(parsear_utc("2026-08-21T11:00:00Z").unwrap())
        );

        insertar_ingreso(&connection, 2, "2026-08-21T12:00:00Z");
        assert_eq!(
            ultimo_instante_movimiento(&connection).unwrap(),
            Some(parsear_utc("2026-08-21T12:00:00Z").unwrap())
        );
    }

    #[test]
    fn ultimo_instante_movimiento_usa_indices_cubrientes() {
        let connection = conexion_con_referencias();
        let mut statement = connection
            .prepare(&format!(
                "EXPLAIN QUERY PLAN {ULTIMO_INSTANTE_MOVIMIENTO_SQL}"
            ))
            .unwrap();
        let detalles = statement
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        for indice in [
            "idx_registro_ingresos_fecha_ingreso",
            "idx_registro_ingresos_fecha_salida",
        ] {
            assert!(
                detalles
                    .iter()
                    .any(|detalle| { detalle.contains(&format!("USING COVERING INDEX {indice}")) }),
                "el plan no usa el índice cubriente {indice}: {detalles:?}"
            );
        }
    }
}
