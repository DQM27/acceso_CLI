use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::encargado_ruta::EncargadoRuta;

pub trait EncargadoRutaRepository {
    fn crear(&self, encargado: &EncargadoRuta) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<EncargadoRuta>, DatabaseError>;

    fn buscar_por_codigo_empleado(
        &self,
        codigo_empleado: &str,
    ) -> Result<Option<EncargadoRuta>, DatabaseError>;

    fn actualizar(&self, encargado: &EncargadoRuta) -> Result<(), DatabaseError>;

    /// `solo_activos`: mismo criterio que
    /// `EmpresaProveedorRepository::listar` -- los selectores de un wizard
    /// piden `true` (un encargado desactivado no es una opción válida para
    /// una salida nueva), la pantalla de administración pide `false`.
    fn listar(&self, solo_activos: bool) -> Result<Vec<EncargadoRuta>, DatabaseError>;

    /// Por nombre o código de empleado -- pedido explícito del usuario,
    /// 2026-09-15, mismo criterio que el buscador de contratistas ("busca
    /// por nombre o por número de cédula"). Sin el sistema de filtros
    /// completo de `FiltroContratistas` a propósito -- este catálogo no
    /// necesita paginar ni combinar filtros, sólo un texto corto. `PLEGAR`
    /// (función SQL registrada en `database::schema`) ignora
    /// mayúsculas/diacríticos en el nombre; el código de empleado es
    /// siempre numérico, alcanza con `LIKE` simple. Mismo `solo_activos`
    /// que `listar`.
    fn buscar(&self, texto: &str, solo_activos: bool) -> Result<Vec<EncargadoRuta>, DatabaseError>;
}

pub struct SqliteEncargadoRutaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteEncargadoRutaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    fn encolar_actualizacion(&self, id: i64) -> Result<(), DatabaseError> {
        let uuid: Option<String> = self.connection.query_row(
            "SELECT uuid FROM encargados_ruta WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if let Some(uuid) = uuid {
            cola_salida::encolar(self.connection, "encargado_ruta", &uuid, "actualizar")?;
        }
        Ok(())
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<EncargadoRuta> {
    Ok(EncargadoRuta {
        id: row.get(0)?,
        codigo_empleado: row.get(1)?,
        nombre: row.get(2)?,
        cedula: row.get(3)?,
        activo: row.get::<_, i64>(4)? != 0,
    })
}

const SELECT_ENCARGADO: &str =
    "SELECT id, codigo_empleado, nombre, cedula, activo FROM encargados_ruta";

/// Tope de `buscar` -- catálogo de ~1438 filas (KOF), un texto corto o
/// vacío podría matchear cientos; el buscador de un checklist mobile sólo
/// necesita ver las primeras coincidencias para elegir, no el listado
/// completo (para eso está `listar`).
const LIMITE_BUSQUEDA: usize = 20;

impl EncargadoRutaRepository for SqliteEncargadoRutaRepository<'_> {
    fn crear(&self, encargado: &EncargadoRuta) -> Result<i64, DatabaseError> {
        let uuid = generar_uuid_v4();

        self.connection.execute(
            "
            INSERT INTO encargados_ruta (codigo_empleado, nombre, cedula, activo, uuid)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                encargado.codigo_empleado,
                encargado.nombre,
                encargado.cedula,
                i64::from(encargado.activo),
                uuid,
            ],
        )?;

        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "encargado_ruta", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<EncargadoRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_ENCARGADO} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(encargado) => Ok(Some(encargado)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_codigo_empleado(
        &self,
        codigo_empleado: &str,
    ) -> Result<Option<EncargadoRuta>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_ENCARGADO} WHERE codigo_empleado = ?1"))?;
        match statement.query_row(params![codigo_empleado], convertir_fila) {
            Ok(encargado) => Ok(Some(encargado)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn actualizar(&self, encargado: &EncargadoRuta) -> Result<(), DatabaseError> {
        self.connection.execute(
            "
            UPDATE encargados_ruta
            SET codigo_empleado = ?1, nombre = ?2, cedula = ?3, activo = ?4
            WHERE id = ?5
            ",
            params![
                encargado.codigo_empleado,
                encargado.nombre,
                encargado.cedula,
                i64::from(encargado.activo),
                encargado.id,
            ],
        )?;

        self.encolar_actualizacion(encargado.id)
    }

    fn listar(&self, solo_activos: bool) -> Result<Vec<EncargadoRuta>, DatabaseError> {
        let filtro_activo = if solo_activos { "WHERE activo = 1" } else { "" };
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_ENCARGADO} {filtro_activo} ORDER BY nombre"))?;
        let encargados = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(encargados)
    }

    fn buscar(
        &self,
        texto: &str,
        solo_activos: bool,
    ) -> Result<Vec<EncargadoRuta>, DatabaseError> {
        let filtro_activo = if solo_activos { "AND activo = 1" } else { "" };
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_ENCARGADO}
             WHERE (PLEGAR(nombre) LIKE '%' || PLEGAR(?1) || '%'
                OR codigo_empleado LIKE '%' || ?1 || '%')
             {filtro_activo}
             ORDER BY nombre
             LIMIT {LIMITE_BUSQUEDA}"
        ))?;
        let encargados = statement
            .query_map(params![texto], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(encargados)
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

    fn nuevo(codigo_empleado: &str, nombre: &str) -> EncargadoRuta {
        EncargadoRuta {
            id: 0,
            codigo_empleado: codigo_empleado.to_string(),
            nombre: nombre.to_string(),
            cedula: None,
            activo: true,
        }
    }

    #[test]
    fn crear_y_buscar_por_codigo_empleado_redondea_el_viaje() {
        let connection = conexion();
        let repo = SqliteEncargadoRutaRepository::new(&connection);

        // Códigos reales de `empleados_costa_rica.sql`: el largo varía
        // 5-7 dígitos, no es fijo -- ver LectorCarnetKof.kt.
        let id = repo.crear(&nuevo("77851", "Ramon Rodriguez")).unwrap();
        let encargado = repo.buscar_por_codigo_empleado("77851").unwrap().unwrap();

        assert_eq!(encargado.id, id);
        assert_eq!(encargado.nombre, "Ramon Rodriguez");
        assert!(encargado.cedula.is_none());
    }

    #[test]
    fn codigo_empleado_duplicado_falla() {
        let connection = conexion();
        let repo = SqliteEncargadoRutaRepository::new(&connection);
        repo.crear(&nuevo("5040017", "Michael Araya Retana"))
            .unwrap();

        let error = repo.crear(&nuevo("5040017", "Otra Persona")).unwrap_err();
        assert!(error.es_constraint_unique());
    }

    #[test]
    fn buscar_encuentra_por_nombre_parcial_sin_importar_tildes_ni_mayusculas() {
        let connection = conexion();
        let repo = SqliteEncargadoRutaRepository::new(&connection);
        repo.crear(&nuevo("5040017", "Michael Araya Retana"))
            .unwrap();

        let resultados = repo.buscar("araya", false).unwrap();

        assert_eq!(resultados.len(), 1);
        assert_eq!(resultados[0].nombre, "Michael Araya Retana");
    }

    #[test]
    fn buscar_encuentra_por_codigo_de_empleado_parcial() {
        let connection = conexion();
        let repo = SqliteEncargadoRutaRepository::new(&connection);
        repo.crear(&nuevo("5040017", "Michael Araya Retana"))
            .unwrap();
        repo.crear(&nuevo("77851", "Ramon Rodriguez")).unwrap();

        let resultados = repo.buscar("5040", false).unwrap();

        assert_eq!(resultados.len(), 1);
        assert_eq!(resultados[0].codigo_empleado, "5040017");
    }

    #[test]
    fn buscar_sin_coincidencias_devuelve_vacio() {
        let connection = conexion();
        let repo = SqliteEncargadoRutaRepository::new(&connection);
        repo.crear(&nuevo("5040017", "Michael Araya Retana"))
            .unwrap();

        assert!(repo.buscar("no existe nadie asi", false).unwrap().is_empty());
    }

    #[test]
    fn buscar_solo_activos_omite_los_desactivados() {
        let connection = conexion();
        let repo = SqliteEncargadoRutaRepository::new(&connection);
        let id = repo
            .crear(&nuevo("5040017", "Michael Araya Retana"))
            .unwrap();
        let mut encargado = repo.buscar_por_id(id).unwrap().unwrap();
        encargado.activo = false;
        repo.actualizar(&encargado).unwrap();

        assert!(repo.buscar("araya", true).unwrap().is_empty());
        assert_eq!(repo.buscar("araya", false).unwrap().len(), 1);
    }

    #[test]
    fn actualizar_desactiva_el_encargado() {
        let connection = conexion();
        let repo = SqliteEncargadoRutaRepository::new(&connection);
        let id = repo
            .crear(&nuevo("5040017", "Michael Araya Retana"))
            .unwrap();
        let mut encargado = repo.buscar_por_id(id).unwrap().unwrap();

        encargado.activo = false;
        repo.actualizar(&encargado).unwrap();

        assert!(!repo.buscar_por_id(id).unwrap().unwrap().activo);
    }

    #[test]
    fn listar_solo_activos_omite_los_desactivados() {
        let connection = conexion();
        let repo = SqliteEncargadoRutaRepository::new(&connection);
        repo.crear(&nuevo("5040017", "Michael Araya Retana"))
            .unwrap();
        let id = repo.crear(&nuevo("77851", "Ramon Rodriguez")).unwrap();
        let mut encargado = repo.buscar_por_id(id).unwrap().unwrap();
        encargado.activo = false;
        repo.actualizar(&encargado).unwrap();

        assert_eq!(repo.listar(true).unwrap().len(), 1);
        assert_eq!(repo.listar(false).unwrap().len(), 2);
    }
}
