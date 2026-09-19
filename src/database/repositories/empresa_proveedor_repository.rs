//! Catálogo de empresas proveedoras
//! (`docs/features-futuras/plan-control-proveedores.md`) -- mismo CRUD
//! mínimo que `EmpresaRepository`, más un `buscar` por texto igual al de
//! `EncargadoRutaRepository` (el selector de empresa en el wizard mobile
//! necesita búsqueda en vivo, no un `<select>` fully-preloaded). Sin
//! generalizar/compartir código con `EmpresaRepository` a propósito -- el
//! plan es explícito en que ambos catálogos son chicos y el proyecto ya
//! tolera esta duplicación entre catálogos similares.

use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
use crate::models::empresa_proveedor::EmpresaProveedor;

pub trait EmpresaProveedorRepository {
    fn crear(&self, empresa: &EmpresaProveedor) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<EmpresaProveedor>, DatabaseError>;

    fn buscar_por_nombre(&self, nombre: &str) -> Result<Option<EmpresaProveedor>, DatabaseError>;

    fn establecer_activo(&self, id: i64, activo: bool) -> Result<(), DatabaseError>;

    /// `solo_activos`: los selectores de un wizard (elegir empresa para un
    /// nuevo ingreso) deben pedir `true` -- una empresa desactivada no es
    /// una opción válida para una operación nueva. La pantalla de
    /// administración (activar/desactivar) pide `false`, porque ahí sí hay
    /// que ver y poder reactivar las inactivas. El filtro vive acá a
    /// propósito, no en cada pantalla: dejarlo del lado de la presentación
    /// es justo el bug que originó este parámetro (una pantalla se olvidó
    /// de filtrar y siguió mostrando empresas desactivadas).
    fn listar(&self, solo_activos: bool) -> Result<Vec<EmpresaProveedor>, DatabaseError>;

    /// Por nombre -- mismo criterio que `EncargadoRutaRepository::buscar`,
    /// pensado para el selector con autocompletado del wizard de
    /// proveedores (mobile) y el desktop equivalente. Mismo `solo_activos`
    /// que `listar`.
    fn buscar(
        &self,
        texto: &str,
        solo_activos: bool,
    ) -> Result<Vec<EmpresaProveedor>, DatabaseError>;
}

pub struct SqliteEmpresaProveedorRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteEmpresaProveedorRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    fn encolar_actualizacion(&self, id: i64) -> Result<(), DatabaseError> {
        let uuid: Option<String> = self.connection.query_row(
            "SELECT uuid FROM empresas_proveedor WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if let Some(uuid) = uuid {
            cola_salida::encolar(self.connection, "empresa_proveedor", &uuid, "actualizar")?;
        }
        Ok(())
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<EmpresaProveedor> {
    Ok(EmpresaProveedor {
        id: row.get(0)?,
        nombre: row.get(1)?,
        activo: row.get::<_, i64>(2)? != 0,
    })
}

const SELECT_EMPRESA_PROVEEDOR: &str = "SELECT id, nombre, activo FROM empresas_proveedor";

/// Mismo criterio y mismo valor que `encargado_ruta_repository::LIMITE_BUSQUEDA`.
const LIMITE_BUSQUEDA: usize = 20;

impl EmpresaProveedorRepository for SqliteEmpresaProveedorRepository<'_> {
    fn crear(&self, empresa: &EmpresaProveedor) -> Result<i64, DatabaseError> {
        let uuid = generar_uuid_v4();

        self.connection.execute(
            "INSERT INTO empresas_proveedor (nombre, activo, uuid) VALUES (?1, ?2, ?3)",
            params![empresa.nombre, i64::from(empresa.activo), uuid],
        )?;

        // Capturado antes de encolar: `last_insert_rowid()` refleja el
        // último INSERT de la conexión, y encolar hace el suyo propio.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "empresa_proveedor", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<EmpresaProveedor>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_EMPRESA_PROVEEDOR} WHERE id = ?1"))?;
        match statement.query_row(params![id], convertir_fila) {
            Ok(empresa) => Ok(Some(empresa)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_nombre(&self, nombre: &str) -> Result<Option<EmpresaProveedor>, DatabaseError> {
        let mut statement = self
            .connection
            .prepare(&format!("{SELECT_EMPRESA_PROVEEDOR} WHERE nombre = ?1"))?;
        match statement.query_row(params![nombre], convertir_fila) {
            Ok(empresa) => Ok(Some(empresa)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn establecer_activo(&self, id: i64, activo: bool) -> Result<(), DatabaseError> {
        self.connection.execute(
            "UPDATE empresas_proveedor SET activo = ?1 WHERE id = ?2",
            params![i64::from(activo), id],
        )?;
        self.encolar_actualizacion(id)
    }

    fn listar(&self, solo_activos: bool) -> Result<Vec<EmpresaProveedor>, DatabaseError> {
        let filtro_activo = if solo_activos { "WHERE activo = 1" } else { "" };
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_EMPRESA_PROVEEDOR} {filtro_activo} ORDER BY nombre"
        ))?;
        let empresas = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(empresas)
    }

    fn buscar(
        &self,
        texto: &str,
        solo_activos: bool,
    ) -> Result<Vec<EmpresaProveedor>, DatabaseError> {
        let filtro_activo = if solo_activos { "AND activo = 1" } else { "" };
        let mut statement = self.connection.prepare(&format!(
            "{SELECT_EMPRESA_PROVEEDOR}
             WHERE PLEGAR(nombre) LIKE '%' || PLEGAR(?1) || '%'
             {filtro_activo}
             ORDER BY nombre
             LIMIT {LIMITE_BUSQUEDA}"
        ))?;
        let empresas = statement
            .query_map(params![texto], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(empresas)
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

    fn nueva(nombre: &str) -> EmpresaProveedor {
        EmpresaProveedor {
            id: 0,
            nombre: nombre.to_string(),
            activo: true,
        }
    }

    #[test]
    fn crear_y_buscar_por_nombre_redondea_el_viaje() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);

        let id = repo.crear(&nueva("Maika")).unwrap();
        let empresa = repo.buscar_por_nombre("Maika").unwrap().unwrap();

        assert_eq!(empresa.id, id);
        assert!(empresa.activo);
    }

    #[test]
    fn nombre_duplicado_viola_unique() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        repo.crear(&nueva("Maika")).unwrap();

        let error = repo.crear(&nueva("Maika")).unwrap_err();
        assert!(error.es_constraint_unique());
    }

    #[test]
    fn buscar_ignora_mayusculas_y_diacriticos() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        repo.crear(&nueva("Dos Pinos")).unwrap();

        let resultados = repo.buscar("dos pinos", false).unwrap();
        assert_eq!(resultados.len(), 1);
        assert_eq!(resultados[0].nombre, "Dos Pinos");
    }

    #[test]
    fn buscar_solo_activos_omite_las_desactivadas() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let id = repo.crear(&nueva("Dos Pinos")).unwrap();
        repo.establecer_activo(id, false).unwrap();

        assert!(repo.buscar("dos pinos", true).unwrap().is_empty());
        assert_eq!(repo.buscar("dos pinos", false).unwrap().len(), 1);
    }

    #[test]
    fn establecer_activo_desactiva_la_empresa() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let id = repo.crear(&nueva("Maika")).unwrap();

        repo.establecer_activo(id, false).unwrap();

        assert!(!repo.buscar_por_id(id).unwrap().unwrap().activo);
    }

    #[test]
    fn listar_trae_todas_ordenadas_por_nombre() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        repo.crear(&nueva("Dos Pinos")).unwrap();
        repo.crear(&nueva("Maika")).unwrap();

        let empresas = repo.listar(false).unwrap();
        assert_eq!(
            empresas
                .iter()
                .map(|e| e.nombre.as_str())
                .collect::<Vec<_>>(),
            vec!["Dos Pinos", "Maika"]
        );
    }

    #[test]
    fn listar_solo_activos_omite_las_desactivadas() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        repo.crear(&nueva("Dos Pinos")).unwrap();
        let id_maika = repo.crear(&nueva("Maika")).unwrap();
        repo.establecer_activo(id_maika, false).unwrap();

        let empresas = repo.listar(true).unwrap();
        assert_eq!(
            empresas
                .iter()
                .map(|e| e.nombre.as_str())
                .collect::<Vec<_>>(),
            vec!["Dos Pinos"]
        );
        assert_eq!(repo.listar(false).unwrap().len(), 2);
    }
}
