//! Catálogo de encargados de ruta (`docs/planes-implementados/plan-control-rutas.md`)
//! -- hasta ahora `AppCore` leía este catálogo llamando al repositorio
//! directo, sin esta capa intermedia. Eso empujaba una decisión de negocio
//! (qué encargado es una opción válida para un selector) a quien llamara,
//! el mismo defecto que ya se corrigió para `EmpresaProveedorService`. Sin
//! `crear`/`actualizar` acá a propósito: esas operaciones no tienen hoy más
//! regla de negocio que la autorización del actor, que ya vive en
//! `AppCore` -- este servicio sólo existe para que la lectura (¿cuáles
//! encargados mostrar?) tenga un lugar propio.

use crate::database::error::DatabaseError;
use crate::database::repositories::encargado_ruta_repository::EncargadoRutaRepository;
use crate::models::encargado_ruta::EncargadoRuta;

pub struct EncargadoRutaService<'a, R>
where
    R: EncargadoRutaRepository + ?Sized,
{
    encargados: &'a R,
}

impl<'a, R> EncargadoRutaService<'a, R>
where
    R: EncargadoRutaRepository + ?Sized,
{
    pub fn new(encargados: &'a R) -> Self {
        Self { encargados }
    }

    /// Para la grilla de administración -- trae todos, activos e inactivos,
    /// así se puede reactivar uno. `listar_seleccionables` es la
    /// contraparte para un selector de wizard (entregar gafete
    /// provisional, salida de ruta), donde un encargado desactivado nunca
    /// es una opción válida. Mismo criterio que
    /// `EmpresaProveedorService::listar`/`listar_seleccionables`.
    pub fn listar(&self) -> Result<Vec<EncargadoRuta>, DatabaseError> {
        self.encargados.listar(false)
    }

    pub fn listar_seleccionables(&self) -> Result<Vec<EncargadoRuta>, DatabaseError> {
        self.encargados.listar(true)
    }

    /// Buscador por nombre o código de empleado (checklist mobile, gafete
    /// provisional) -- mismo criterio que `listar`/`listar_seleccionables`.
    pub fn buscar(&self, texto: &str) -> Result<Vec<EncargadoRuta>, DatabaseError> {
        self.encargados.buscar(texto, false)
    }

    pub fn buscar_seleccionables(&self, texto: &str) -> Result<Vec<EncargadoRuta>, DatabaseError> {
        self.encargados.buscar(texto, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::encargado_ruta_repository::SqliteEncargadoRutaRepository;
    use crate::database::schema::initialize_database;
    use rusqlite::Connection;

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
    fn seleccionables_omiten_los_desactivados_pero_administracion_los_incluye() {
        let connection = conexion();
        let repo = SqliteEncargadoRutaRepository::new(&connection);
        repo.crear(&nuevo("5040017", "Michael Araya Retana")).unwrap();
        let id_ramon = repo.crear(&nuevo("77851", "Ramon Rodriguez")).unwrap();
        let mut ramon = repo.buscar_por_id(id_ramon).unwrap().unwrap();
        ramon.activo = false;
        repo.actualizar(&ramon).unwrap();
        let servicio = EncargadoRutaService::new(&repo);

        assert_eq!(servicio.listar().unwrap().len(), 2);
        assert_eq!(servicio.listar_seleccionables().unwrap().len(), 1);
        assert_eq!(servicio.buscar("ramon").unwrap().len(), 1);
        assert!(servicio.buscar_seleccionables("ramon").unwrap().is_empty());
    }
}
