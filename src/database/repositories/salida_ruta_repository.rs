use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::salida_ruta::{
    NuevaSalidaRuta, RetornoSalidaRuta, SalidaRuta, SalidaRutaActivaResumen,
};
use crate::tiempo::{parsear_utc, serializar_utc};

/// Un tramo (cruce físico de portón) -- ver
/// `docs/planes-implementados/plan-control-rutas.md`, sección "Rediseño
/// del núcleo de rutas -- documento/tramo/viaje". A qué documento(s)
/// corresponde vive aparte, ver `crate::database::repositories::salida_ruta_documento_repository`.
pub trait SalidaRutaRepository {
    fn crear(&self, salida: &NuevaSalidaRuta) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<SalidaRuta>, DatabaseError>;

    /// Mismo motivo que `RegistroIngresoRepository::buscar_ingreso_activo`:
    /// evitar que un vehículo quede con dos salidas abiertas a la vez.
    /// Por placa (texto), no por `vehiculo_id` -- el índice único de la
    /// base (`idx_salidas_ruta_placa_activa`) tampoco depende de que haya
    /// match de catálogo.
    fn buscar_activa_por_placa(&self, placa: &str) -> Result<Option<SalidaRuta>, DatabaseError>;

    fn registrar_retorno(
        &self,
        id: i64,
        fecha_hora_retorno: DateTime<Utc>,
        usuario_retorno_id: i64,
    ) -> Result<(), DatabaseError>;

    /// Fila aplanada para la pantalla "Rutas activas" -- análoga a
    /// `RegistroIngresoRepository`/`MovimientoVisitaRepository::listar_activos`.
    fn listar_activas(&self) -> Result<Vec<SalidaRutaActivaResumen>, DatabaseError>;
}

pub struct SqliteSalidaRutaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteSalidaRutaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<SalidaRuta> {
    let fecha_hora_salida_texto: String = row.get(7)?;
    let fecha_hora_salida = parsear_utc(&fecha_hora_salida_texto).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(error))
    })?;

    let fecha_hora_retorno_texto: Option<String> = row.get(9)?;
    let fecha_hora_retorno = fecha_hora_retorno_texto
        .map(|fecha| {
            parsear_utc(&fecha).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    9,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;
    let usuario_retorno_id: Option<i64> = row.get(10)?;
    // `CHECK (fecha_hora_retorno IS NULL) = (usuario_retorno_id IS NULL)` en
    // el esquema garantiza que ambos vienen juntos o ninguno.
    let retorno = fecha_hora_retorno
        .zip(usuario_retorno_id)
        .map(|(fecha_hora, usuario_id)| RetornoSalidaRuta {
            fecha_hora,
            usuario_id,
        });

    Ok(SalidaRuta {
        id: row.get(0)?,
        viaje_id: row.get(1)?,
        vehiculo_id: row.get(2)?,
        vehiculo_placa: row.get(3)?,
        vehiculo_numero_unidad: row.get(4)?,
        encargado_id: row.get(5)?,
        encargado_nombre: row.get(6)?,
        fecha_hora_salida,
        usuario_salida_id: row.get(8)?,
        retorno,
    })
}

const SELECT_SALIDA: &str = "
    SELECT id, viaje_id, vehiculo_id, vehiculo_placa, vehiculo_numero_unidad,
           encargado_id, encargado_nombre,
           fecha_hora_salida, usuario_salida_id,
           fecha_hora_retorno, usuario_retorno_id
    FROM salidas_ruta
";

impl SalidaRutaRepository for SqliteSalidaRutaRepository<'_> {
    fn crear(&self, salida: &NuevaSalidaRuta) -> Result<i64, DatabaseError> {
        let fecha_hora_salida = serializar_utc(salida.fecha_hora_salida);
        let uuid = generar_uuid_v4();

        let filas = self.connection.execute(
            "
            INSERT INTO salidas_ruta (
                viaje_id, vehiculo_id, vehiculo_placa, vehiculo_numero_unidad,
                encargado_id, encargado_nombre,
                fecha_hora_salida, usuario_salida_id, usuario_salida_nombre,
                uuid
            )
            SELECT
                :viaje_id, :vehiculo_id, :vehiculo_placa, :vehiculo_numero_unidad,
                :encargado_id, :encargado_nombre,
                :fecha_hora_salida, :usuario_salida_id, u.nombre,
                :uuid
            FROM usuarios AS u
            WHERE u.id = :usuario_salida_id
            ",
            rusqlite::named_params! {
                ":viaje_id": salida.viaje_id,
                ":vehiculo_id": salida.vehiculo_id,
                ":vehiculo_placa": salida.vehiculo_placa,
                ":vehiculo_numero_unidad": salida.vehiculo_numero_unidad,
                ":encargado_id": salida.encargado_id,
                ":encargado_nombre": salida.encargado_nombre,
                ":fecha_hora_salida": fecha_hora_salida,
                ":usuario_salida_id": salida.usuario_salida_id,
                ":uuid": uuid,
            },
        )?;

        if filas == 0 {
            return Err(DatabaseError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }

        // Capturado antes de encolar: `last_insert_rowid()` refleja el
        // último INSERT de la conexión, y encolar hace el suyo propio.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "salida_ruta", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<SalidaRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_SALIDA} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(salida) => Ok(Some(salida)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_activa_por_placa(&self, placa: &str) -> Result<Option<SalidaRuta>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_SALIDA} WHERE vehiculo_placa = ?1 AND fecha_hora_retorno IS NULL
             ORDER BY fecha_hora_salida DESC LIMIT 1"
        ))?;
        match statement.query_row(params![placa], convertir_fila) {
            Ok(salida) => Ok(Some(salida)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn registrar_retorno(
        &self,
        id: i64,
        fecha_hora_retorno: DateTime<Utc>,
        usuario_retorno_id: i64,
    ) -> Result<(), DatabaseError> {
        let fecha_hora_retorno = serializar_utc(fecha_hora_retorno);

        let filas_afectadas = self.connection.execute(
            "
            UPDATE salidas_ruta
            SET
                fecha_hora_retorno = ?1,
                usuario_retorno_id = ?2,
                usuario_retorno_nombre = (SELECT nombre FROM usuarios WHERE id = ?2)
            WHERE id = ?3
              AND fecha_hora_retorno IS NULL
            ",
            params![fecha_hora_retorno, usuario_retorno_id, id],
        )?;

        if filas_afectadas == 0 {
            return Err(DatabaseError::SalidaRutaNoActiva);
        }

        let uuid: String = self.connection.query_row(
            "SELECT uuid FROM salidas_ruta WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        cola_salida::encolar(self.connection, "salida_ruta", &uuid, "cerrar")?;

        Ok(())
    }

    fn listar_activas(&self) -> Result<Vec<SalidaRutaActivaResumen>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id, viaje_id, vehiculo_placa, vehiculo_numero_unidad, encargado_nombre,
                fecha_hora_salida, usuario_salida_nombre
            FROM salidas_ruta
            WHERE fecha_hora_retorno IS NULL
            ORDER BY fecha_hora_salida ASC
            ",
        )?;
        let filas = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        filas
            .into_iter()
            .map(
                |(
                    id,
                    viaje_id,
                    vehiculo_placa,
                    vehiculo_numero_unidad,
                    encargado_nombre,
                    fecha_hora_salida_texto,
                    usuario_salida_nombre,
                )| {
                    let fecha_hora_salida = parsear_utc(&fecha_hora_salida_texto)
                        .map_err(|error| DatabaseError::FechaCorrupta(error.to_string()))?;
                    Ok(SalidaRutaActivaResumen {
                        id,
                        viaje_id,
                        vehiculo_placa,
                        vehiculo_numero_unidad,
                        encargado_nombre,
                        fecha_hora_salida,
                        usuario_salida_nombre,
                    })
                },
            )
            .collect()
    }
}

const ULTIMO_INSTANTE_SALIDA_RUTA_SQL: &str = "
    SELECT MAX(instante)
    FROM (
        SELECT MAX(fecha_hora_salida) AS instante
        FROM salidas_ruta
        UNION ALL
        SELECT MAX(fecha_hora_retorno) AS instante
        FROM salidas_ruta
        WHERE fecha_hora_retorno IS NOT NULL
    )";

/// Mismo criterio y misma forma que
/// `movimiento_visita_repository::ultimo_instante_movimiento_visita` --
/// `AppCore::registrar_salida_ruta`/`registrar_retorno_ruta`
/// (`application/rutas.rs`) toman el máximo entre esto y los demás
/// dominios (ingresos, visitas) para que un sitio que sólo tuvo actividad
/// de rutas (sin ingresos/visitas todavía) también quede protegido contra
/// un reloj retrocedido.
pub fn ultimo_instante_salida_ruta(
    connection: &Connection,
) -> Result<Option<DateTime<Utc>>, DatabaseError> {
    let ultima: Option<String> =
        connection.query_row(ULTIMO_INSTANTE_SALIDA_RUTA_SQL, [], |row| row.get(0))?;
    ultima
        .map(|texto| {
            parsear_utc(&texto).map_err(|error| DatabaseError::FechaCorrupta(error.to_string()))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::viaje_ruta_repository::{
        SqliteViajeRutaRepository, ViajeRutaRepository,
    };
    use crate::database::schema::initialize_database;
    use crate::models::viaje_ruta::NuevoViajeRuta;
    use chrono::TimeZone;

    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        connection
    }

    fn crear_viaje(connection: &Connection, placa: &str) -> i64 {
        SqliteViajeRutaRepository::new(connection)
            .crear(&NuevoViajeRuta {
                vehiculo_id: None,
                vehiculo_placa: placa.to_string(),
                vehiculo_numero_unidad: None,
                encargado_id: None,
                encargado_nombre: "Carlos Mendez".to_string(),
                fecha_hora_creacion: Utc.with_ymd_and_hms(2026, 9, 19, 8, 0, 0).unwrap(),
                usuario_creacion_id: 1,
            })
            .unwrap()
    }

    fn nueva(viaje_id: i64, placa: &str) -> NuevaSalidaRuta {
        NuevaSalidaRuta {
            viaje_id,
            vehiculo_id: None,
            vehiculo_placa: placa.to_string(),
            vehiculo_numero_unidad: Some("22906".to_string()),
            encargado_id: None,
            encargado_nombre: "Carlos Mendez".to_string(),
            fecha_hora_salida: Utc.with_ymd_and_hms(2026, 9, 19, 8, 0, 0).unwrap(),
            usuario_salida_id: 1,
        }
    }

    #[test]
    fn crear_y_registrar_retorno_redondea_el_viaje() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);
        let viaje_id = crear_viaje(&connection, "C12345");

        let id = repo.crear(&nueva(viaje_id, "C12345")).unwrap();
        repo.registrar_retorno(id, Utc.with_ymd_and_hms(2026, 9, 19, 10, 0, 0).unwrap(), 1)
            .unwrap();

        let salida = repo.buscar_por_id(id).unwrap().unwrap();
        assert!(salida.retorno.is_some());
        assert_eq!(salida.viaje_id, viaje_id);
    }

    #[test]
    fn dos_salidas_abiertas_para_la_misma_placa_falla() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);
        let viaje_id = crear_viaje(&connection, "C12345");
        repo.crear(&nueva(viaje_id, "C12345")).unwrap();

        let error = repo.crear(&nueva(viaje_id, "C12345")).unwrap_err();
        assert!(error.es_constraint_unique());
    }

    #[test]
    fn listar_activas_omite_las_ya_retornadas() {
        let connection = conexion();
        let repo = SqliteSalidaRutaRepository::new(&connection);
        let viaje_1 = crear_viaje(&connection, "C12345");
        let viaje_2 = crear_viaje(&connection, "C99999");
        let id = repo.crear(&nueva(viaje_1, "C12345")).unwrap();
        repo.crear(&nueva(viaje_2, "C99999")).unwrap();
        repo.registrar_retorno(id, Utc.with_ymd_and_hms(2026, 9, 19, 10, 0, 0).unwrap(), 1)
            .unwrap();

        let activas = repo.listar_activas().unwrap();

        assert_eq!(activas.len(), 1);
        assert_eq!(activas[0].vehiculo_placa, "C99999");
    }
}
