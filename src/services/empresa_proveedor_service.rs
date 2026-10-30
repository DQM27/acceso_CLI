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

    /// Si el nombre es el de una empresa que el administrador fusionó en
    /// otra, no se crea nada: devuelve la que quedó (el operador escribió el
    /// nombre viejo).
    pub fn crear(&self, nombre: &str) -> Result<i64, EmpresaProveedorServiceError> {
        let nombre = normalizar_nombre(nombre)?;
        if let Some(destino) = self.empresas.destino_de_fusionada(&nombre)? {
            return Ok(destino);
        }
        let empresa = EmpresaProveedor {
            id: 0,
            nombre,
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
            // Con la misma regla con la que se guardó.
            .buscar_por_nombre(&control_acceso_reglas::nombre::nombre_en_mayusculas(nombre))?
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

    /// El buscador de los selectores de la portería: primero las que
    /// contienen lo escrito y después las que se le parecen aunque esté mal
    /// escrito ("FUDDY FACTORY" trae "FOODIE FACTORY"), de la más a la menos
    /// parecida. Así el operador encuentra la empresa en vez de crear otra
    /// con el nombre mal escrito (5 duplicados en dos meses en Brisas).
    pub fn buscar_seleccionables(
        &self,
        texto: &str,
    ) -> Result<Vec<EmpresaProveedor>, EmpresaProveedorServiceError> {
        let mut encontradas = self.empresas.buscar(texto, true)?;
        if encontradas.len() >= MAXIMO_SUGERENCIAS {
            return Ok(encontradas);
        }
        let mut parecidas: Vec<(f64, EmpresaProveedor)> = self
            .empresas
            .listar(true)?
            .into_iter()
            .filter(|empresa| encontradas.iter().all(|ya| ya.id != empresa.id))
            .map(|empresa| {
                (
                    crate::texto::puntaje_busqueda(texto, &empresa.nombre),
                    empresa,
                )
            })
            .filter(|(puntaje, _)| *puntaje >= crate::texto::UMBRAL_NOMBRE_PARECIDO)
            .collect();
        parecidas.sort_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then_with(|| a.1.nombre.cmp(&b.1.nombre))
        });
        let lugar = MAXIMO_SUGERENCIAS - encontradas.len();
        encontradas.extend(
            parecidas
                .into_iter()
                .take(lugar)
                .map(|(_, empresa)| empresa),
        );
        Ok(encontradas)
    }

    /// Las empresas vigentes que se parecen al nombre que se está por dar de
    /// alta, de la más a la menos parecida: si hay alguna, la app pregunta
    /// "¿es alguna de estas?" antes de crear otra.
    pub fn parecidas(
        &self,
        nombre: &str,
    ) -> Result<Vec<EmpresaProveedor>, EmpresaProveedorServiceError> {
        let mut parecidas: Vec<(f64, EmpresaProveedor)> = self
            .empresas
            .listar(true)?
            .into_iter()
            .map(|empresa| {
                (
                    crate::texto::similitud_nombres(nombre, &empresa.nombre),
                    empresa,
                )
            })
            .filter(|(similitud, _)| *similitud >= crate::texto::UMBRAL_NOMBRE_PARECIDO)
            .collect();
        parecidas.sort_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then_with(|| a.1.nombre.cmp(&b.1.nombre))
        });
        Ok(parecidas
            .into_iter()
            .take(MAXIMO_PARECIDAS_AL_CREAR)
            .map(|(_, empresa)| empresa)
            .collect())
    }
}

/// Cuántas empresas como mucho trae el buscador de los selectores (el mismo
/// tope que la búsqueda exacta del repositorio).
const MAXIMO_SUGERENCIAS: usize = 20;

/// Cuántas parecidas se ofrecen antes de crear una empresa nueva.
const MAXIMO_PARECIDAS_AL_CREAR: usize = 5;

fn mapear_nombre_duplicado(error: DatabaseError) -> EmpresaProveedorServiceError {
    if error.es_constraint_unique() {
        EmpresaProveedorServiceError::NombreDuplicado
    } else {
        EmpresaProveedorServiceError::Database(error)
    }
}

/// Como todo nombre de persona o empresa: en mayúscula y sin espacios de
/// más (`control_acceso_reglas::nombre`).
fn normalizar_nombre(nombre: &str) -> Result<String, EmpresaProveedorServiceError> {
    let nombre = control_acceso_reglas::nombre::nombre_en_mayusculas(nombre);

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

    fn catalogo_de_brisas(
        servicio: &EmpresaProveedorService<'_, SqliteEmpresaProveedorRepository<'_>>,
    ) {
        for nombre in [
            "FOODIE FACTORY",
            "PROCESADOS FRESCOS",
            "THE CHIKEN",
            "BIMBO",
            "LA GRAN BODEGA",
        ] {
            servicio.crear(nombre).unwrap();
        }
    }

    fn nombres(empresas: &[EmpresaProveedor]) -> Vec<&str> {
        empresas
            .iter()
            .map(|empresa| empresa.nombre.as_str())
            .collect()
    }

    /// Incidente 2026-10-30: el operador escribía "FUDDY FACTORY", la
    /// búsqueda exacta no traía nada y creaba otra empresa. Ahora la
    /// búsqueda de la portería trae la parecida.
    #[test]
    fn el_buscador_de_la_porteria_trae_la_empresa_aunque_este_mal_escrita() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let servicio = EmpresaProveedorService::new(&repo);
        catalogo_de_brisas(&servicio);

        assert_eq!(
            nombres(&servicio.buscar_seleccionables("FUDDY FACTORY").unwrap()),
            vec!["FOODIE FACTORY"]
        );
        assert_eq!(
            nombres(&servicio.buscar_seleccionables("procesos").unwrap()),
            vec!["PROCESADOS FRESCOS"]
        );
        // Las que contienen lo escrito van primero; nada que no se parezca.
        assert_eq!(
            nombres(&servicio.buscar_seleccionables("BIM").unwrap()),
            vec!["BIMBO"]
        );
        assert!(servicio.buscar_seleccionables("SYSCO").unwrap().is_empty());
    }

    #[test]
    fn antes_de_crear_ofrece_las_parecidas() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let servicio = EmpresaProveedorService::new(&repo);
        catalogo_de_brisas(&servicio);

        assert_eq!(
            nombres(&servicio.parecidas("TEHE CHIKEN").unwrap()),
            vec!["THE CHIKEN"]
        );
        assert!(servicio.parecidas("DOS PINOS").unwrap().is_empty());
    }

    /// La fusión llega del panel con el catálogo: la fusionada no se ofrece
    /// y escribir su nombre al dar de alta lleva a la que quedó.
    #[test]
    fn una_empresa_fusionada_no_se_ofrece_y_su_nombre_lleva_a_la_que_quedo() {
        let connection = conexion();
        let repo = SqliteEmpresaProveedorRepository::new(&connection);
        let servicio = EmpresaProveedorService::new(&repo);
        let foodie = servicio.crear("FOODIE FACTORY").unwrap();
        servicio.crear("FUDDY FACTORY").unwrap();
        connection
            .execute(
                "UPDATE empresas_proveedor
                 SET fusionada_en = (SELECT uuid FROM empresas_proveedor WHERE id = ?1)
                 WHERE nombre = 'FUDDY FACTORY'",
                [foodie],
            )
            .unwrap();

        assert_eq!(
            nombres(&servicio.listar_seleccionables().unwrap()),
            vec!["FOODIE FACTORY"]
        );
        assert_eq!(nombres(&servicio.listar().unwrap()), vec!["FOODIE FACTORY"]);
        assert_eq!(
            nombres(&servicio.buscar_seleccionables("FUDDY").unwrap()),
            vec!["FOODIE FACTORY"]
        );
        assert_eq!(servicio.crear("fuddy factory").unwrap(), foodie);
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
