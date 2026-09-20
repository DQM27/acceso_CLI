//! Catálogo de vehículos de ruta (`docs/planes-implementados/plan-control-rutas.md`)
//! -- mismo motivo que `EncargadoRutaService`: sólo existe para que la
//! decisión "¿cuáles vehículos mostrar?" tenga un lugar propio, en vez de
//! quedar en manos de `AppCore` o de la pantalla. Sin `crear`/`actualizar`
//! acá por el mismo criterio que ese servicio.

use crate::database::error::DatabaseError;
use crate::database::repositories::vehiculo_ruta_repository::VehiculoRutaRepository;
use crate::models::vehiculo_ruta::VehiculoRuta;

pub struct VehiculoRutaService<'a, R>
where
    R: VehiculoRutaRepository + ?Sized,
{
    vehiculos: &'a R,
}

impl<'a, R> VehiculoRutaService<'a, R>
where
    R: VehiculoRutaRepository + ?Sized,
{
    pub fn new(vehiculos: &'a R) -> Self {
        Self { vehiculos }
    }

    /// Para la grilla de administración -- trae todos, activos e inactivos,
    /// así se puede reactivar uno. `listar_seleccionables` es la
    /// contraparte para el selector de salida de ruta, donde un vehículo
    /// desactivado nunca es una opción válida. Mismo criterio que
    /// `EmpresaProveedorService::listar`/`listar_seleccionables`.
    pub fn listar(&self) -> Result<Vec<VehiculoRuta>, DatabaseError> {
        self.vehiculos.listar(false)
    }

    pub fn listar_seleccionables(&self) -> Result<Vec<VehiculoRuta>, DatabaseError> {
        self.vehiculos.listar(true)
    }

    /// Buscador por placa o número de unidad (checklist mobile) -- mismo
    /// criterio que `EncargadoRutaService::buscar`/`buscar_seleccionables`.
    pub fn buscar(&self, texto: &str) -> Result<Vec<VehiculoRuta>, DatabaseError> {
        self.vehiculos.buscar(texto, false)
    }

    pub fn buscar_seleccionables(&self, texto: &str) -> Result<Vec<VehiculoRuta>, DatabaseError> {
        self.vehiculos.buscar(texto, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::vehiculo_ruta_repository::SqliteVehiculoRutaRepository;
    use crate::database::schema::initialize_database;
    use rusqlite::Connection;

    fn conexion() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    fn nuevo(placa: &str) -> VehiculoRuta {
        VehiculoRuta {
            id: 0,
            numero_unidad: None,
            placa: placa.to_string(),
            activo: true,
        }
    }

    #[test]
    fn seleccionables_omiten_los_desactivados_pero_administracion_los_incluye() {
        let connection = conexion();
        let repo = SqliteVehiculoRutaRepository::new(&connection);
        repo.crear(&nuevo("C12345")).unwrap();
        let id = repo.crear(&nuevo("C99999")).unwrap();
        let mut vehiculo = repo.buscar_por_id(id).unwrap().unwrap();
        vehiculo.activo = false;
        repo.actualizar(&vehiculo).unwrap();
        let servicio = VehiculoRutaService::new(&repo);

        assert_eq!(servicio.listar().unwrap().len(), 2);
        assert_eq!(servicio.listar_seleccionables().unwrap().len(), 1);
    }

    #[test]
    fn buscar_seleccionables_omite_los_desactivados() {
        let connection = conexion();
        let repo = SqliteVehiculoRutaRepository::new(&connection);
        repo.crear(&nuevo("C12345")).unwrap();
        let id = repo.crear(&nuevo("C99999")).unwrap();
        let mut vehiculo = repo.buscar_por_id(id).unwrap().unwrap();
        vehiculo.activo = false;
        repo.actualizar(&vehiculo).unwrap();
        let servicio = VehiculoRutaService::new(&repo);

        assert_eq!(servicio.buscar("C1").unwrap().len(), 1);
        assert!(servicio.buscar_seleccionables("C99999").unwrap().is_empty());
    }
}
