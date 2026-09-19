//! Catálogo de empresas proveedoras
//! (`docs/features-futuras/plan-control-proveedores.md`) -- mismo molde
//! mínimo que `EmpresaService`, sin las variantes auditadas (este catálogo
//! no tiene pantalla de auditoría propia todavía).

use crate::database::error::DatabaseError;
use crate::database::repositories::empresa_proveedor_repository::EmpresaProveedorRepository;
use crate::models::empresa_proveedor::EmpresaProveedor;

use super::error::EmpresaProveedorServiceError;

pub struct EmpresaProveedorService<'a, R>
where
    R: EmpresaProveedorRepository + ?Sized,
{
    empresas: &'a R,
}

impl<'a, R> EmpresaProveedorService<'a, R>
where
    R: EmpresaProveedorRepository + ?Sized,
{
    pub fn new(empresas: &'a R) -> Self {
        Self { empresas }
    }

    pub fn crear(&self, nombre: &str) -> Result<i64, EmpresaProveedorServiceError> {
        let nombre = normalizar_nombre(nombre)?;
        let empresa = EmpresaProveedor {
            id: 0,
            nombre: nombre.to_string(),
            activo: true,
        };

        self.empresas
            .crear(&empresa)
            .map_err(mapear_nombre_duplicado)
    }

    pub fn buscar_por_id(&self, id: i64) -> Result<EmpresaProveedor, EmpresaProveedorServiceError> {
        self.empresas
            .buscar_por_id(id)?
            .ok_or(EmpresaProveedorServiceError::EmpresaNoEncontrada)
    }

    pub fn buscar_por_nombre(
        &self,
        nombre: &str,
    ) -> Result<EmpresaProveedor, EmpresaProveedorServiceError> {
        self.empresas
            .buscar_por_nombre(nombre.trim())?
            .ok_or(EmpresaProveedorServiceError::EmpresaNoEncontrada)
    }

    pub fn activar(&self, id: i64) -> Result<(), EmpresaProveedorServiceError> {
        self.buscar_por_id(id)?;
        Ok(self.empresas.establecer_activo(id, true)?)
    }

    pub fn desactivar(&self, id: i64) -> Result<(), EmpresaProveedorServiceError> {
        self.buscar_por_id(id)?;
        Ok(self.empresas.establecer_activo(id, false)?)
    }

    /// Para la grilla de administración -- trae activas e inactivas, así se
    /// puede reactivar una. `listar_seleccionables` es la contraparte para
    /// un selector de wizard, donde una empresa inactiva nunca es una
    /// opción válida. La decisión de cuál pedir es de negocio, no de la
    /// pantalla -- por eso son dos métodos con nombre propio en vez de un
    /// parámetro `bool` que cualquier llamador podría pasar sin pensarlo.
    pub fn listar(&self) -> Result<Vec<EmpresaProveedor>, EmpresaProveedorServiceError> {
        Ok(self.empresas.listar(false)?)
    }

    pub fn listar_seleccionables(
        &self,
    ) -> Result<Vec<EmpresaProveedor>, EmpresaProveedorServiceError> {
        Ok(self.empresas.listar(true)?)
    }

    /// Mismo criterio que `listar`/`listar_seleccionables`.
    pub fn buscar(
        &self,
        texto: &str,
    ) -> Result<Vec<EmpresaProveedor>, EmpresaProveedorServiceError> {
        Ok(self.empresas.buscar(texto, false)?)
    }

    pub fn buscar_seleccionables(
        &self,
        texto: &str,
    ) -> Result<Vec<EmpresaProveedor>, EmpresaProveedorServiceError> {
        Ok(self.empresas.buscar(texto, true)?)
    }
}

fn mapear_nombre_duplicado(error: DatabaseError) -> EmpresaProveedorServiceError {
    if error.es_constraint_unique() {
        EmpresaProveedorServiceError::NombreDuplicado
    } else {
        EmpresaProveedorServiceError::Database(error)
    }
}

fn normalizar_nombre(nombre: &str) -> Result<&str, EmpresaProveedorServiceError> {
    let nombre = nombre.trim();

    if nombre.is_empty() {
        return Err(EmpresaProveedorServiceError::NombreEmpresaVacio);
    }

    Ok(nombre)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::empresa_proveedor_repository::SqliteEmpresaProveedorRepository;
    use crate::database::schema::initialize_database;
    use rusqlite::Connection;

    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    #[test]
    fn crear_y_buscar_por_nombre_redondea_el_viaje() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let servicio = EmpresaProveedorService::new(&repo);

        let id = servicio.crear("Maika").unwrap();
        let empresa = servicio.buscar_por_nombre("Maika").unwrap();

        assert_eq!(empresa.id, id);
        assert!(empresa.activo);
    }

    #[test]
    fn crear_con_nombre_vacio_falla() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let servicio = EmpresaProveedorService::new(&repo);

        let error = servicio.crear("  ").unwrap_err();

        assert!(matches!(
            error,
            EmpresaProveedorServiceError::NombreEmpresaVacio
        ));
    }

    #[test]
    fn crear_con_nombre_duplicado_falla() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let servicio = EmpresaProveedorService::new(&repo);
        servicio.crear("Maika").unwrap();

        let error = servicio.crear("Maika").unwrap_err();

        assert!(matches!(
            error,
            EmpresaProveedorServiceError::NombreDuplicado
        ));
    }

    /// Hallazgo del usuario, 2026-09-18: `UNIQUE(nombre)` sólo bloqueaba un
    /// choque exacto -- "Dos Pinos" y "DOS PINOS" se colaban como dos
    /// empresas distintas. El índice único sobre `PLEGAR(nombre)`
    /// (`MIGRACION_47`) cierra ese hueco.
    #[test]
    fn crear_con_nombre_duplicado_ignorando_mayusculas_y_diacriticos_falla() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let servicio = EmpresaProveedorService::new(&repo);
        servicio.crear("Dos Pinos").unwrap();

        let error = servicio.crear("DOS PIÑOS").unwrap_err();

        assert!(matches!(
            error,
            EmpresaProveedorServiceError::NombreDuplicado
        ));
    }

    #[test]
    fn desactivar_y_activar_redondean_el_viaje() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let servicio = EmpresaProveedorService::new(&repo);
        let id = servicio.crear("Maika").unwrap();

        servicio.desactivar(id).unwrap();
        assert!(!servicio.buscar_por_id(id).unwrap().activo);

        servicio.activar(id).unwrap();
        assert!(servicio.buscar_por_id(id).unwrap().activo);
    }

    /// `listar`/`buscar` (administración) traen todo; `_seleccionables`
    /// (wizard) omite las inactivas. La decisión de cuál usar es del
    /// servicio, no de quien lo llama.
    #[test]
    fn seleccionables_omiten_las_desactivadas_pero_administracion_las_incluye() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let servicio = EmpresaProveedorService::new(&repo);
        servicio.crear("Maika").unwrap();
        let id_dos_pinos = servicio.crear("Dos Pinos").unwrap();
        servicio.desactivar(id_dos_pinos).unwrap();

        assert_eq!(servicio.listar().unwrap().len(), 2);
        assert_eq!(servicio.listar_seleccionables().unwrap().len(), 1);
        assert_eq!(servicio.buscar("pinos").unwrap().len(), 1);
        assert!(servicio.buscar_seleccionables("pinos").unwrap().is_empty());
    }
}
