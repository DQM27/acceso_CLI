//! Escritura de movimientos de visita (`docs/plan-control-visitas.md`) --
//! el cruce real en el punto de acceso, análogo a
//! `RegistroIngresoRepository` pero sin ningún campo de PRAIND/SWAT: no hay
//! resultado de acceso ni motivo que guardar, `CitaService`/`domain::cita`
//! ya decidieron "permitido" antes de que esto se llame.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, named_params, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::movimiento_visita::{
    MovimientoVisita, MovimientoVisitaActivoResumen, NuevoMovimientoVisita, SalidaMovimientoVisita,
};
use crate::tiempo::{parsear_utc, serializar_utc};

pub trait MovimientoVisitaRepository {
    fn crear(&self, movimiento: &NuevoMovimientoVisita) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<MovimientoVisita>, DatabaseError>;

    fn buscar_activo_por_visitante(
        &self,
        cita_visitante_id: i64,
    ) -> Result<Option<MovimientoVisita>, DatabaseError>;

    /// Mismo motivo que `RegistroIngresoRepository::buscar_ingreso_activo_por_gafete`:
    /// evitar que un gafete quede asignado a dos movimientos abiertos a la
    /// vez -- acá el `CHECK` ya lo impide (`idx_movimientos_visita_gafete_activo`),
    /// esto es la consulta que deja mostrar el mensaje ANTES de intentar el
    /// `INSERT` y chocar contra esa restricción.
    fn buscar_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<MovimientoVisita>, DatabaseError>;

    fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError>;

    /// Fila aplanada para la pantalla "Visitas activas" -- análoga a
    /// `IngresosQuery::listar_activos` (contratistas), pero sin filtro:
    /// el universo de visitas activas en un sitio nunca es lo bastante
    /// grande como para justificar paginar/filtrar en `SQLite` (mismo
    /// criterio que hizo que este dominio entero se mantuviera más simple
    /// que el de contratistas, ver `docs/plan-control-visitas.md`).
    fn listar_activos(&self) -> Result<Vec<MovimientoVisitaActivoResumen>, DatabaseError>;
}

pub struct SqliteMovimientoVisitaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteMovimientoVisitaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<MovimientoVisita> {
    let fecha_hora_entrada_texto: String = row.get(3)?;
    let fecha_hora_entrada = parsear_utc(&fecha_hora_entrada_texto).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;

    let fecha_hora_salida_texto: Option<String> = row.get(5)?;
    let fecha_hora_salida = fecha_hora_salida_texto
        .map(|fecha| {
            parsear_utc(&fecha).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;
    let usuario_salida_id: Option<i64> = row.get(6)?;
    // `CHECK (fecha_hora_salida IS NULL) = (usuario_salida_id IS NULL)` --
    // ver el CHECK implícito de MIGRACION_28 (salida_unica) garantiza que
    // ambos vienen juntos o ninguno.
    let salida = fecha_hora_salida
        .zip(usuario_salida_id)
        .map(|(fecha_hora, usuario_id)| SalidaMovimientoVisita {
            fecha_hora,
            usuario_id,
        });

    Ok(MovimientoVisita {
        id: row.get(0)?,
        cita_visitante_id: row.get(1)?,
        gafete_numero: row.get(2)?,
        fecha_hora_entrada,
        usuario_entrada_id: row.get(4)?,
        salida,
    })
}

const SELECT_MOVIMIENTO: &str = "
    SELECT id, cita_visitante_id, gafete_numero, fecha_hora_entrada,
           usuario_entrada_id, fecha_hora_salida, usuario_salida_id
    FROM movimientos_visita
";

impl MovimientoVisitaRepository for SqliteMovimientoVisitaRepository<'_> {
    fn crear(&self, movimiento: &NuevoMovimientoVisita) -> Result<i64, DatabaseError> {
        let fecha_hora_entrada = serializar_utc(movimiento.fecha_hora_entrada);
        let uuid = generar_uuid_v4();

        let filas = self.connection.execute(
            "
            INSERT INTO movimientos_visita (
                cita_visitante_id, gafete_numero, fecha_hora_entrada,
                usuario_entrada_id, usuario_entrada_nombre, uuid
            )
            SELECT :cita_visitante_id, :gafete_numero, :fecha_hora_entrada,
                   :usuario_entrada_id, u.nombre, :uuid
            FROM usuarios AS u
            WHERE u.id = :usuario_entrada_id
            ",
            named_params! {
                ":cita_visitante_id": movimiento.cita_visitante_id,
                ":gafete_numero": movimiento.gafete_numero,
                ":fecha_hora_entrada": fecha_hora_entrada,
                ":usuario_entrada_id": movimiento.usuario_entrada_id,
                ":uuid": uuid,
            },
        )?;

        if filas == 0 {
            return Err(DatabaseError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }

        // Capturado antes de encolar: `last_insert_rowid()` refleja el
        // último INSERT de la conexión, y encolar hace el suyo propio.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "movimiento_visita", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<MovimientoVisita>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_MOVIMIENTO} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(movimiento) => Ok(Some(movimiento)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_activo_por_visitante(
        &self,
        cita_visitante_id: i64,
    ) -> Result<Option<MovimientoVisita>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_MOVIMIENTO} WHERE cita_visitante_id = ?1 AND fecha_hora_salida IS NULL
             ORDER BY fecha_hora_entrada DESC LIMIT 1"
        ))?;
        match statement.query_row(params![cita_visitante_id], convertir_fila) {
            Ok(movimiento) => Ok(Some(movimiento)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<MovimientoVisita>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_MOVIMIENTO} WHERE gafete_numero = ?1 AND fecha_hora_salida IS NULL
             ORDER BY fecha_hora_entrada DESC LIMIT 1"
        ))?;
        match statement.query_row(params![gafete_numero], convertir_fila) {
            Ok(movimiento) => Ok(Some(movimiento)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError> {
        let fecha_hora_salida = serializar_utc(fecha_hora_salida);

        let filas_afectadas = self.connection.execute(
            "
            UPDATE movimientos_visita
            SET
                fecha_hora_salida = ?1,
                usuario_salida_id = ?2,
                usuario_salida_nombre = (SELECT nombre FROM usuarios WHERE id = ?2)
            WHERE id = ?3
              AND fecha_hora_salida IS NULL
            ",
            params![fecha_hora_salida, usuario_salida_id, id],
        )?;

        if filas_afectadas == 0 {
            return Err(DatabaseError::MovimientoVisitaNoActivo);
        }

        let uuid: String = self.connection.query_row(
            "SELECT uuid FROM movimientos_visita WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        cola_salida::encolar(self.connection, "movimiento_visita", &uuid, "cerrar")?;

        Ok(())
    }

    fn listar_activos(&self) -> Result<Vec<MovimientoVisitaActivoResumen>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                mv.id, cv.cedula, cv.nombre, cv.empresa, mv.gafete_numero,
                mv.fecha_hora_entrada, c.anfitrion_nombre, c.motivo
            FROM movimientos_visita mv
            JOIN cita_visitantes cv ON cv.id = mv.cita_visitante_id
            JOIN citas c ON c.id = cv.cita_id
            WHERE mv.fecha_hora_salida IS NULL
            ORDER BY mv.fecha_hora_entrada ASC
            ",
        )?;
        let filas = statement
            .query_map([], |row| {
                let fecha_hora_entrada_texto: String = row.get(5)?;
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    fecha_hora_entrada_texto,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        filas
            .into_iter()
            .map(
                |(
                    id,
                    cedula,
                    nombre,
                    empresa,
                    gafete_numero,
                    fecha_hora_entrada_texto,
                    anfitrion_nombre,
                    motivo,
                )| {
                    let fecha_hora_entrada = parsear_utc(&fecha_hora_entrada_texto)
                        .map_err(|error| DatabaseError::FechaCorrupta(error.to_string()))?;
                    Ok(MovimientoVisitaActivoResumen {
                        id,
                        cedula,
                        nombre,
                        empresa,
                        gafete_numero,
                        fecha_hora_entrada,
                        anfitrion_nombre,
                        motivo,
                    })
                },
            )
            .collect()
    }
}

const ULTIMO_INSTANTE_MOVIMIENTO_VISITA_SQL: &str = "
    SELECT MAX(instante)
    FROM (
        SELECT MAX(fecha_hora_entrada) AS instante
        FROM movimientos_visita
        UNION ALL
        SELECT MAX(fecha_hora_salida) AS instante
        FROM movimientos_visita
        WHERE fecha_hora_salida IS NOT NULL
    )";

/// Mismo criterio y misma forma que
/// `database::queries::ingresos::ultimo_instante_movimiento`, pero para
/// `movimientos_visita` -- `AppCore::validar_reloj` (`application/citas.rs`)
/// toma el máximo de los dos para que un sitio que sólo tuvo actividad de
/// visitas (sin ningún contratista todavía) también quede protegido contra
/// un reloj retrocedido.
pub fn ultimo_instante_movimiento_visita(
    connection: &Connection,
) -> Result<Option<DateTime<Utc>>, DatabaseError> {
    let ultima: Option<String> =
        connection.query_row(ULTIMO_INSTANTE_MOVIMIENTO_VISITA_SQL, [], |row| row.get(0))?;
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

    fn conexion_con_visitante() -> (Connection, i64) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                    VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
                 INSERT INTO citas (id, uuid, fecha_desde, fecha_hasta, anfitrion_nombre,
                    anfitrion_correo, estado, creado_en)
                    VALUES (1, 'uuid-cita-1', '2026-08-10', '2026-08-15', 'Anfitrión',
                    'anfitrion@ejemplo.com', 'VIGENTE', '2026-08-01T00:00:00Z');
                 INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre)
                    VALUES (1, 'uuid-visitante-1', 1, '1-2345', 'Visitante');",
            )
            .unwrap();
        (connection, 1)
    }

    fn nuevo(cita_visitante_id: i64, gafete_numero: Option<i64>) -> NuevoMovimientoVisita {
        NuevoMovimientoVisita {
            cita_visitante_id,
            gafete_numero,
            fecha_hora_entrada: Utc::now(),
            usuario_entrada_id: 1,
        }
    }

    #[test]
    fn crear_y_buscar_por_id_redondea_el_viaje() {
        let (connection, visitante_id) = conexion_con_visitante();
        let repo = SqliteMovimientoVisitaRepository::new(&connection);

        let id = repo.crear(&nuevo(visitante_id, Some(7))).unwrap();
        let movimiento = repo.buscar_por_id(id).unwrap().unwrap();

        assert_eq!(movimiento.cita_visitante_id, visitante_id);
        assert_eq!(movimiento.gafete_numero, Some(7));
        assert!(movimiento.salida.is_none());
    }

    #[test]
    fn crear_con_usuario_inexistente_falla() {
        let (connection, visitante_id) = conexion_con_visitante();
        let repo = SqliteMovimientoVisitaRepository::new(&connection);
        let mut movimiento = nuevo(visitante_id, None);
        movimiento.usuario_entrada_id = 999;

        assert!(repo.crear(&movimiento).is_err());
    }

    #[test]
    fn buscar_activo_por_visitante_ignora_movimientos_ya_cerrados() {
        let (connection, visitante_id) = conexion_con_visitante();
        let repo = SqliteMovimientoVisitaRepository::new(&connection);
        let id = repo.crear(&nuevo(visitante_id, None)).unwrap();

        assert!(
            repo.buscar_activo_por_visitante(visitante_id)
                .unwrap()
                .is_some()
        );

        repo.registrar_salida(id, Utc::now(), 1).unwrap();

        assert!(
            repo.buscar_activo_por_visitante(visitante_id)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn buscar_activo_por_gafete_encuentra_el_movimiento_abierto() {
        let (connection, visitante_id) = conexion_con_visitante();
        let repo = SqliteMovimientoVisitaRepository::new(&connection);
        repo.crear(&nuevo(visitante_id, Some(3))).unwrap();

        let activo = repo.buscar_activo_por_gafete(3).unwrap();

        assert!(activo.is_some());
        assert_eq!(activo.unwrap().gafete_numero, Some(3));
        assert!(repo.buscar_activo_por_gafete(99).unwrap().is_none());
    }

    #[test]
    fn registrar_salida_dos_veces_falla_la_segunda() {
        let (connection, visitante_id) = conexion_con_visitante();
        let repo = SqliteMovimientoVisitaRepository::new(&connection);
        let id = repo.crear(&nuevo(visitante_id, None)).unwrap();

        repo.registrar_salida(id, Utc::now(), 1).unwrap();

        assert!(matches!(
            repo.registrar_salida(id, Utc::now(), 1),
            Err(DatabaseError::MovimientoVisitaNoActivo)
        ));
    }

    #[test]
    fn dos_movimientos_abiertos_a_la_vez_con_el_mismo_gafete_falla() {
        let (connection, visitante_id) = conexion_con_visitante();
        connection
            .execute(
                "INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre)
                 VALUES (2, 'uuid-visitante-2', 1, '6-7890', 'Otro visitante')",
                [],
            )
            .unwrap();
        let repo = SqliteMovimientoVisitaRepository::new(&connection);
        repo.crear(&nuevo(visitante_id, Some(5))).unwrap();

        assert!(repo.crear(&nuevo(2, Some(5))).is_err());
    }

    #[test]
    fn listar_activos_trae_el_grupo_abierto_con_datos_de_la_cita_y_omite_al_que_ya_salio() {
        let (connection, visitante_id) = conexion_con_visitante();
        connection
            .execute(
                "INSERT INTO cita_visitantes (id, uuid, cita_id, cedula, nombre)
                 VALUES (2, 'uuid-visitante-2', 1, '6-7890', 'Otro visitante')",
                [],
            )
            .unwrap();
        let repo = SqliteMovimientoVisitaRepository::new(&connection);
        let activo_id = repo.crear(&nuevo(visitante_id, Some(3))).unwrap();
        let cerrado_id = repo.crear(&nuevo(2, None)).unwrap();
        repo.registrar_salida(cerrado_id, Utc::now(), 1).unwrap();

        let activos = repo.listar_activos().unwrap();

        assert_eq!(activos.len(), 1);
        let fila = &activos[0];
        assert_eq!(fila.id, activo_id);
        assert_eq!(fila.cedula, "1-2345");
        assert_eq!(fila.nombre, "Visitante");
        assert_eq!(fila.gafete_numero, Some(3));
        assert_eq!(fila.anfitrion_nombre, "Anfitrión");
    }
}
