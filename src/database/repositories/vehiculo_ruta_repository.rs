use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::vehiculo_ruta::VehiculoRuta;

pub trait VehiculoRutaRepository {
    fn crear(&self, vehiculo: &VehiculoRuta) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<VehiculoRuta>, DatabaseError>;

    fn buscar_por_placa(&self, placa: &str) -> Result<Option<VehiculoRuta>, DatabaseError>;

    fn buscar_por_numero_unidad(
        &self,
        numero_unidad: &str,
    ) -> Result<Option<VehiculoRuta>, DatabaseError>;

    fn actualizar(&self, vehiculo: &VehiculoRuta) -> Result<(), DatabaseError>;

    /// `solo_activos`: mismo criterio que
    /// `EmpresaProveedorRepository::listar` -- los selectores de un wizard
    /// piden `true` (un vehículo desactivado no es una opción válida para
    /// una salida nueva), la pantalla de administración pide `false`.
    fn listar(&self, solo_activos: bool) -> Result<Vec<VehiculoRuta>, DatabaseError>;
}

pub struct SqliteVehiculoRutaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteVehiculoRutaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    fn encolar_actualizacion(&self, id: i64) -> Result<(), DatabaseError> {
        let uuid: Option<String> = self.connection.query_row(
            "SELECT uuid FROM vehiculos_ruta WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if let Some(uuid) = uuid {
            cola_salida::encolar(self.connection, "vehiculo_ruta", &uuid, "actualizar")?;
        }
        Ok(())
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<VehiculoRuta> {
    Ok(VehiculoRuta {
        id: row.get(0)?,
        numero_unidad: row.get(1)?,
        placa: row.get(2)?,
        activo: row.get::<_, i64>(3)? != 0,
    })
}

const SELECT_VEHICULO: &str = "SELECT id, numero_unidad, placa, activo FROM vehiculos_ruta";

impl VehiculoRutaRepository for SqliteVehiculoRutaRepository<'_> {
    fn crear(&self, vehiculo: &VehiculoRuta) -> Result<i64, DatabaseError> {
        let uuid = generar_uuid_v4();

        self.connection.execute(
            "
            INSERT INTO vehiculos_ruta (numero_unidad, placa, activo, uuid)
            VALUES (?1, ?2, ?3, ?4)
            ",
            params![
                vehiculo.numero_unidad,
                vehiculo.placa,
                i64::from(vehiculo.activo),
                uuid,
            ],
        )?;

        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "vehiculo_ruta", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<VehiculoRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_VEHICULO} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(vehiculo) => Ok(Some(vehiculo)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_placa(&self, placa: &str) -> Result<Option<VehiculoRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_VEHICULO} WHERE placa = ?1"))?;
        match statement.query_row(params![placa], convertir_fila) {
            Ok(vehiculo) => Ok(Some(vehiculo)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_numero_unidad(
        &self,
        numero_unidad: &str,
    ) -> Result<Option<VehiculoRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_VEHICULO} WHERE numero_unidad = ?1"))?;
        match statement.query_row(params![numero_unidad], convertir_fila) {
            Ok(vehiculo) => Ok(Some(vehiculo)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn actualizar(&self, vehiculo: &VehiculoRuta) -> Result<(), DatabaseError> {
        self.connection.execute(
            "
            UPDATE vehiculos_ruta
            SET numero_unidad = ?1, placa = ?2, activo = ?3
            WHERE id = ?4
            ",
            params![
                vehiculo.numero_unidad,
                vehiculo.placa,
                i64::from(vehiculo.activo),
                vehiculo.id,
            ],
        )?;

        self.encolar_actualizacion(vehiculo.id)
    }

    fn listar(&self, solo_activos: bool) -> Result<Vec<VehiculoRuta>, DatabaseError> {
        let filtro_activo = if solo_activos { "WHERE activo = 1" } else { "" };
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_VEHICULO} {filtro_activo} ORDER BY placa"))?;
        let vehiculos = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(vehiculos)
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

    fn nuevo(placa: &str, numero_unidad: Option<&str>) -> VehiculoRuta {
        VehiculoRuta {
            id: 0,
            numero_unidad: numero_unidad.map(str::to_string),
            placa: placa.to_string(),
            activo: true,
        }
    }

    #[test]
    fn crear_y_buscar_por_placa_redondea_el_viaje() {
        let connection = conexion();
        let repo = SqliteVehiculoRutaRepository::new(&connection);

        let id = repo.crear(&nuevo("C12345", Some("22906"))).unwrap();
        let vehiculo = repo.buscar_por_placa("C12345").unwrap().unwrap();

        assert_eq!(vehiculo.id, id);
        assert_eq!(vehiculo.numero_unidad.as_deref(), Some("22906"));
        assert!(vehiculo.activo);
    }

    #[test]
    fn buscar_por_numero_unidad_encuentra_el_vehiculo() {
        let connection = conexion();
        let repo = SqliteVehiculoRutaRepository::new(&connection);
        repo.crear(&nuevo("C12345", Some("22906"))).unwrap();

        assert!(repo.buscar_por_numero_unidad("22906").unwrap().is_some());
        assert!(repo.buscar_por_numero_unidad("99999").unwrap().is_none());
    }

    #[test]
    fn vehiculo_de_apoyo_sin_numero_de_unidad_es_valido() {
        let connection = conexion();
        let repo = SqliteVehiculoRutaRepository::new(&connection);

        let id = repo.crear(&nuevo("BPH485", None)).unwrap();
        let vehiculo = repo.buscar_por_id(id).unwrap().unwrap();

        assert!(vehiculo.numero_unidad.is_none());
    }

    #[test]
    fn placa_duplicada_falla() {
        let connection = conexion();
        let repo = SqliteVehiculoRutaRepository::new(&connection);
        repo.crear(&nuevo("C12345", None)).unwrap();

        let error = repo.crear(&nuevo("C12345", None)).unwrap_err();
        assert!(error.es_constraint_unique());
    }

    #[test]
    fn actualizar_desactiva_el_vehiculo() {
        let connection = conexion();
        let repo = SqliteVehiculoRutaRepository::new(&connection);
        let id = repo.crear(&nuevo("C12345", None)).unwrap();
        let mut vehiculo = repo.buscar_por_id(id).unwrap().unwrap();

        vehiculo.activo = false;
        repo.actualizar(&vehiculo).unwrap();

        assert!(!repo.buscar_por_id(id).unwrap().unwrap().activo);
    }

    #[test]
    fn listar_solo_activos_omite_los_desactivados() {
        let connection = conexion();
        let repo = SqliteVehiculoRutaRepository::new(&connection);
        repo.crear(&nuevo("C12345", None)).unwrap();
        let id = repo.crear(&nuevo("C99999", None)).unwrap();
        let mut vehiculo = repo.buscar_por_id(id).unwrap().unwrap();
        vehiculo.activo = false;
        repo.actualizar(&vehiculo).unwrap();

        assert_eq!(repo.listar(true).unwrap().len(), 1);
        assert_eq!(repo.listar(false).unwrap().len(), 2);
    }
}
