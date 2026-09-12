//! Lectura de citas para el check-in en el punto de acceso
//! (`docs/planes-implementados/plan-control-visitas.md`). Sólo lectura a propósito: `citas`/
//! `cita_visitantes` las llena la sincronización (pull desde la nube,
//! agendadas por el anfitrión -- ver el commit de esquema), nunca el
//! guardia -- mismo motivo por el que este repositorio no tiene ningún
//! `crear`, a diferencia de `GafeteRepository`/`ContratistaRepository`.
//!
//! Devuelve TODAS las citas de una cédula, sin filtrar por fecha/estado --
//! esa decisión es de `domain::cita::verificar_cita`, no de SQL (mismo
//! criterio que separa "¿existe en el catálogo?" de "¿está disponible?" en
//! `GafeteRepository`/`domain::gafete`). Una persona puede aparecer en más
//! de una cita a lo largo del tiempo (agendas distintas, semanas distintas)
//! -- quien llama recorre el resultado y aplica la regla de negocio a cada
//! una hasta encontrar la que aplica hoy.

use chrono::NaiveDate;
use rusqlite::{Connection, Row, params};

use crate::database::error::DatabaseError;
use crate::models::cita::{Cita, CitaVisitante, EstadoCita};

pub trait CitaRepository {
    fn buscar_por_cedula(&self, cedula: &str) -> Result<Vec<(Cita, CitaVisitante)>, DatabaseError>;
}

pub struct SqliteCitaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteCitaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

/// `rusqlite` no tiene el feature `chrono` prendido en este crate -- mismo
/// motivo que `contratista_repository::convertir_fila` parsea
/// `fecha_vencimiento_praind` a mano en vez de pedirle un `NaiveDate`
/// directo al driver.
fn parsear_fecha(row: &Row, indice: usize, columna: &'static str) -> rusqlite::Result<NaiveDate> {
    let texto: String = row.get(indice)?;
    NaiveDate::parse_from_str(&texto, "%Y-%m-%d").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            indice,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::other(format!("{columna}: {error}"))),
        )
    })
}

fn convertir_fila(row: &Row) -> rusqlite::Result<(Cita, CitaVisitante)> {
    let estado_texto: String = row.get(6)?;
    let Some(estado) = EstadoCita::from_str_sql(&estado_texto) else {
        return Err(rusqlite::Error::InvalidColumnType(
            6,
            "estado".to_string(),
            rusqlite::types::Type::Text,
        ));
    };
    let cita_id: i64 = row.get(0)?;

    let cita = Cita {
        id: cita_id,
        motivo: row.get(1)?,
        fecha_desde: parsear_fecha(row, 2, "fecha_desde")?,
        fecha_hasta: parsear_fecha(row, 3, "fecha_hasta")?,
        anfitrion_nombre: row.get(4)?,
        anfitrion_correo: row.get(5)?,
        estado,
        hora_estimada: row.get(7)?,
    };
    let visitante = CitaVisitante {
        id: row.get(8)?,
        cita_id,
        cedula: row.get(9)?,
        nombre: row.get(10)?,
        empresa: row.get(11)?,
        placa_vehiculo: row.get(12)?,
    };
    Ok((cita, visitante))
}

const SELECT_CITA_VISITANTE: &str = "
    SELECT c.id, c.motivo, c.fecha_desde, c.fecha_hasta, c.anfitrion_nombre,
           c.anfitrion_correo, c.estado, c.hora_estimada,
           v.id, v.cedula, v.nombre, v.empresa, v.placa_vehiculo
    FROM cita_visitantes v
    INNER JOIN citas c ON c.id = v.cita_id
";

impl CitaRepository for SqliteCitaRepository<'_> {
    fn buscar_por_cedula(&self, cedula: &str) -> Result<Vec<(Cita, CitaVisitante)>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_CITA_VISITANTE} WHERE v.cedula = ?1"))?;
        let filas = statement
            .query_map(params![cedula], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(filas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    fn insertar_cita(connection: &Connection, id: i64, desde: &str, hasta: &str, estado: &str) {
        connection
            .execute(
                "INSERT INTO citas (id, uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                    anfitrion_correo, estado, creado_en)
                 VALUES (?1, ?2, ?3, ?4, 'Anfitrión', 'anfitrion@ejemplo.com', ?5,
                    '2026-08-01T00:00:00Z')",
                params![id, format!("uuid-cita-{id}"), desde, hasta, estado],
            )
            .unwrap();
    }

    fn insertar_visitante(connection: &Connection, id: i64, cita_id: i64, cedula: &str) {
        connection
            .execute(
                "INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre)
                 VALUES (?1, ?2, ?3, ?4, 'Visitante')",
                params![id, format!("uuid-visitante-{id}"), cita_id, cedula],
            )
            .unwrap();
    }

    #[test]
    fn cedula_sin_ninguna_cita_devuelve_vacio() {
        let connection = conexion();
        let repo = SqliteCitaRepository::new(&connection);

        assert!(repo.buscar_por_cedula("1-2345").unwrap().is_empty());
    }

    #[test]
    fn encuentra_la_cita_y_arma_ambas_structs_completas() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        let repo = SqliteCitaRepository::new(&connection);

        let resultado = repo.buscar_por_cedula("1-2345").unwrap();

        assert_eq!(resultado.len(), 1);
        let (cita, visitante) = &resultado[0];
        assert_eq!(cita.id, 1);
        assert_eq!(cita.estado, EstadoCita::Vigente);
        assert_eq!(cita.fecha_desde.to_string(), "2026-08-10");
        assert_eq!(cita.fecha_hasta.to_string(), "2026-08-15");
        assert_eq!(visitante.cita_id, 1);
        assert_eq!(visitante.cedula, "1-2345");
        assert_eq!(visitante.nombre, "Visitante");
    }

    #[test]
    fn una_cedula_con_varias_citas_a_lo_largo_del_tiempo_devuelve_todas() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-01-10", "2026-01-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        insertar_cita(&connection, 2, "2026-08-10", "2026-08-15", "CANCELADA");
        insertar_visitante(&connection, 2, 2, "1-2345");

        let repo = SqliteCitaRepository::new(&connection);
        let resultado = repo.buscar_por_cedula("1-2345").unwrap();

        assert_eq!(resultado.len(), 2);
    }

    #[test]
    fn una_cita_grupal_solo_devuelve_la_fila_de_la_cedula_pedida() {
        let connection = conexion();
        insertar_cita(&connection, 1, "2026-08-10", "2026-08-15", "VIGENTE");
        insertar_visitante(&connection, 1, 1, "1-2345");
        insertar_visitante(&connection, 2, 1, "6-7890");

        let repo = SqliteCitaRepository::new(&connection);
        let resultado = repo.buscar_por_cedula("6-7890").unwrap();

        assert_eq!(resultado.len(), 1);
        assert_eq!(resultado[0].1.cedula, "6-7890");
    }
}
