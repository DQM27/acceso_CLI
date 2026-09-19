use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::viaje_ruta::{CierreViajeRuta, EstadoViaje, NuevoViajeRuta, ViajeRuta};
use crate::tiempo::{parsear_utc, serializar_utc};

/// Ver `docs/planes-implementados/plan-control-rutas.md`, sección
/// "Rediseño del núcleo de rutas -- documento/tramo/viaje". Sin
/// `actualizar` genérico a propósito -- lo único que cambia en un viaje
/// después de creado es su cierre, que tiene su propio método con su
/// propia regla de negocio (una sola vez), igual criterio que
/// `SalidaRutaRepository::registrar_retorno` en vez de un `actualizar`
/// plano.
pub trait ViajeRutaRepository {
    fn crear(&self, viaje: &NuevoViajeRuta) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<ViajeRuta>, DatabaseError>;

    /// El punto de "¿continuar el mismo viaje o abrir uno nuevo?" --
    /// `RutaService` lo consulta cuando el guardia declara "misma ruta"
    /// al confirmar un retorno. Por placa (texto), mismo criterio que
    /// `SalidaRutaRepository::buscar_activa_por_placa`: vale incluso sin
    /// match de catálogo.
    fn buscar_abierto_por_placa(&self, placa: &str) -> Result<Option<ViajeRuta>, DatabaseError>;

    fn cerrar(
        &self,
        id: i64,
        fecha_hora_cierre: DateTime<Utc>,
        usuario_cierre_id: i64,
    ) -> Result<(), DatabaseError>;
}

pub struct SqliteViajeRutaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteViajeRutaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<ViajeRuta> {
    let estado_texto: String = row.get(5)?;
    let estado = match estado_texto.as_str() {
        "ABIERTO" => EstadoViaje::Abierto,
        "CERRADO" => EstadoViaje::Cerrado,
        _ => {
            return Err(rusqlite::Error::InvalidColumnType(
                5,
                "estado".to_string(),
                rusqlite::types::Type::Text,
            ));
        }
    };

    let fecha_hora_creacion_texto: String = row.get(6)?;
    let fecha_hora_creacion = parsear_utc(&fecha_hora_creacion_texto).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(error))
    })?;

    let fecha_hora_cierre_texto: Option<String> = row.get(8)?;
    let fecha_hora_cierre = fecha_hora_cierre_texto
        .map(|fecha| {
            parsear_utc(&fecha).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;
    let usuario_cierre_id: Option<i64> = row.get(9)?;
    // `CHECK` de la base (MIGRACION_49) garantiza que ambos vienen juntos o ninguno.
    let cierre = fecha_hora_cierre
        .zip(usuario_cierre_id)
        .map(|(fecha_hora, usuario_id)| CierreViajeRuta {
            fecha_hora,
            usuario_id,
        });

    Ok(ViajeRuta {
        id: row.get(0)?,
        vehiculo_id: row.get(1)?,
        vehiculo_placa: row.get(2)?,
        vehiculo_numero_unidad: row.get(3)?,
        encargado_id: row.get(4)?,
        encargado_nombre: row.get(11)?,
        estado,
        fecha_hora_creacion,
        usuario_creacion_id: row.get(7)?,
        cierre,
    })
}

const SELECT_VIAJE: &str = "
    SELECT id, vehiculo_id, vehiculo_placa, vehiculo_numero_unidad, encargado_id,
           estado, fecha_hora_creacion, usuario_creacion_id,
           fecha_hora_cierre, usuario_cierre_id, usuario_cierre_nombre,
           encargado_nombre
    FROM viajes_ruta
";

impl ViajeRutaRepository for SqliteViajeRutaRepository<'_> {
    fn crear(&self, viaje: &NuevoViajeRuta) -> Result<i64, DatabaseError> {
        let fecha_hora_creacion = serializar_utc(viaje.fecha_hora_creacion);
        let uuid = generar_uuid_v4();

        let filas = self.connection.execute(
            "
            INSERT INTO viajes_ruta (
                vehiculo_id, vehiculo_placa, vehiculo_numero_unidad,
                encargado_id, encargado_nombre, estado,
                fecha_hora_creacion, usuario_creacion_id, usuario_creacion_nombre,
                uuid
            )
            SELECT
                :vehiculo_id, :vehiculo_placa, :vehiculo_numero_unidad,
                :encargado_id, :encargado_nombre, 'ABIERTO',
                :fecha_hora_creacion, :usuario_creacion_id, u.nombre,
                :uuid
            FROM usuarios AS u
            WHERE u.id = :usuario_creacion_id
            ",
            rusqlite::named_params! {
                ":vehiculo_id": viaje.vehiculo_id,
                ":vehiculo_placa": viaje.vehiculo_placa,
                ":vehiculo_numero_unidad": viaje.vehiculo_numero_unidad,
                ":encargado_id": viaje.encargado_id,
                ":encargado_nombre": viaje.encargado_nombre,
                ":fecha_hora_creacion": fecha_hora_creacion,
                ":usuario_creacion_id": viaje.usuario_creacion_id,
                ":uuid": uuid,
            },
        )?;

        if filas == 0 {
            return Err(DatabaseError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }

        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "viaje_ruta", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<ViajeRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_VIAJE} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(viaje) => Ok(Some(viaje)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_abierto_por_placa(&self, placa: &str) -> Result<Option<ViajeRuta>, DatabaseError> {
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_VIAJE} WHERE vehiculo_placa = ?1 AND estado = 'ABIERTO'
             ORDER BY fecha_hora_creacion DESC LIMIT 1"
        ))?;
        match statement.query_row(params![placa], convertir_fila) {
            Ok(viaje) => Ok(Some(viaje)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn cerrar(
        &self,
        id: i64,
        fecha_hora_cierre: DateTime<Utc>,
        usuario_cierre_id: i64,
    ) -> Result<(), DatabaseError> {
        let fecha_hora_cierre = serializar_utc(fecha_hora_cierre);

        let filas_afectadas = self.connection.execute(
            "
            UPDATE viajes_ruta
            SET
                estado = 'CERRADO',
                fecha_hora_cierre = ?1,
                usuario_cierre_id = ?2,
                usuario_cierre_nombre = (SELECT nombre FROM usuarios WHERE id = ?2)
            WHERE id = ?3
              AND estado = 'ABIERTO'
            ",
            params![fecha_hora_cierre, usuario_cierre_id, id],
        )?;

        if filas_afectadas == 0 {
            return Err(DatabaseError::ViajeRutaNoAbierto);
        }

        let uuid: String = self.connection.query_row(
            "SELECT uuid FROM viajes_ruta WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        cola_salida::encolar(self.connection, "viaje_ruta", &uuid, "cerrar")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;
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

    fn nuevo(placa: &str) -> NuevoViajeRuta {
        NuevoViajeRuta {
            vehiculo_id: None,
            vehiculo_placa: placa.to_string(),
            vehiculo_numero_unidad: None,
            encargado_id: None,
            encargado_nombre: "Carlos Mendez".to_string(),
            fecha_hora_creacion: Utc.with_ymd_and_hms(2026, 9, 19, 8, 0, 0).unwrap(),
            usuario_creacion_id: 1,
        }
    }

    #[test]
    fn crear_y_buscar_abierto_por_placa_redondea_el_viaje() {
        let connection = conexion();
        let repo = SqliteViajeRutaRepository::new(&connection);

        let id = repo.crear(&nuevo("C12345")).unwrap();
        let viaje = repo.buscar_abierto_por_placa("C12345").unwrap().unwrap();

        assert_eq!(viaje.id, id);
        assert_eq!(viaje.estado, EstadoViaje::Abierto);
        assert!(viaje.cierre.is_none());
    }

    #[test]
    fn cerrar_deja_de_aparecer_como_abierto() {
        let connection = conexion();
        let repo = SqliteViajeRutaRepository::new(&connection);
        let id = repo.crear(&nuevo("C12345")).unwrap();

        repo.cerrar(id, Utc.with_ymd_and_hms(2026, 9, 19, 10, 0, 0).unwrap(), 1)
            .unwrap();

        assert!(repo.buscar_abierto_por_placa("C12345").unwrap().is_none());
        let viaje = repo.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(viaje.estado, EstadoViaje::Cerrado);
        assert!(viaje.cierre.is_some());
    }

    #[test]
    fn cerrar_un_viaje_ya_cerrado_falla() {
        let connection = conexion();
        let repo = SqliteViajeRutaRepository::new(&connection);
        let id = repo.crear(&nuevo("C12345")).unwrap();
        repo.cerrar(id, Utc.with_ymd_and_hms(2026, 9, 19, 10, 0, 0).unwrap(), 1)
            .unwrap();

        let error = repo
            .cerrar(id, Utc.with_ymd_and_hms(2026, 9, 19, 11, 0, 0).unwrap(), 1)
            .unwrap_err();

        assert!(matches!(error, DatabaseError::ViajeRutaNoAbierto));
    }
}
