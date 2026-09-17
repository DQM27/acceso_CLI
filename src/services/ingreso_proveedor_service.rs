//! Ingreso/salida de proveedores
//! (`docs/features-futuras/plan-control-proveedores.md`) -- mismo espíritu
//! que `RegistroIngresoService`, pero sin PRAIND/bloqueos de contratista:
//! sin catálogo de personas (cédula/nombre son snapshot puro), gafete
//! siempre obligatorio (a diferencia de `requiere_gafete` condicional).

use chrono::{DateTime, Utc};

use crate::database::repositories::empresa_proveedor_repository::EmpresaProveedorRepository;
use crate::database::repositories::gafete_repository::GafeteRepository;
use crate::database::repositories::registro_ingreso_proveedor_repository::RegistroIngresoProveedorRepository;
use crate::domain::gafete::{ValidacionAsignacion, validar_para_asignar};
use crate::domain::registro_ingreso::salida_es_cronologicamente_valida;
use crate::models::gafete::TipoGafete;
use crate::models::registro_ingreso_proveedor::{
    NuevoRegistroIngresoProveedor, RegistroIngresoProveedor, RegistroIngresoProveedorActivoResumen,
};

use super::error::IngresoProveedorServiceError;

pub struct IngresoProveedorService<'a, R, E, G>
where
    R: RegistroIngresoProveedorRepository + ?Sized,
    E: EmpresaProveedorRepository + ?Sized,
    G: GafeteRepository + ?Sized,
{
    registros: &'a R,
    empresas: &'a E,
    gafetes: &'a G,
}

impl<'a, R, E, G> IngresoProveedorService<'a, R, E, G>
where
    R: RegistroIngresoProveedorRepository + ?Sized,
    E: EmpresaProveedorRepository + ?Sized,
    G: GafeteRepository + ?Sized,
{
    pub fn new(registros: &'a R, empresas: &'a E, gafetes: &'a G) -> Self {
        Self {
            registros,
            empresas,
            gafetes,
        }
    }

    /// Valida cédula/nombre, que la empresa exista y esté activa, que la
    /// cédula no tenga ya un ingreso abierto, y que el gafete exista, esté
    /// disponible en el catálogo y no esté ya ocupado por otro ingreso de
    /// proveedor activo -- mismo criterio en capas que
    /// `RegistroIngresoService::registrar_entrada`
    /// (catálogo primero, ocupación después porque depende de otro repo).
    #[allow(clippy::too_many_arguments)]
    pub fn registrar_ingreso(
        &self,
        cedula: &str,
        nombre: &str,
        empresa_id: i64,
        placa: Option<String>,
        gafete_numero: i64,
        usuario_id: i64,
        ahora: DateTime<Utc>,
    ) -> Result<i64, IngresoProveedorServiceError> {
        let cedula = cedula.trim();
        if cedula.is_empty() {
            return Err(IngresoProveedorServiceError::CedulaVacia);
        }
        let nombre = nombre.trim();
        if nombre.is_empty() {
            return Err(IngresoProveedorServiceError::NombreVacio);
        }

        let empresa = self
            .empresas
            .buscar_por_id(empresa_id)?
            .ok_or(IngresoProveedorServiceError::EmpresaNoEncontrada)?;
        if !empresa.activo {
            return Err(IngresoProveedorServiceError::EmpresaInactiva);
        }

        if self.registros.buscar_ingreso_activo(cedula)?.is_some() {
            return Err(IngresoProveedorServiceError::IngresoActivo);
        }

        let gafete_encontrado = self
            .gafetes
            .buscar_por_numero(gafete_numero, TipoGafete::Proveedor)?;
        match validar_para_asignar(gafete_encontrado.as_ref()) {
            ValidacionAsignacion::NoRegistrado => {
                return Err(IngresoProveedorServiceError::GafeteNoRegistrado);
            }
            ValidacionAsignacion::NoDisponible(estado) => {
                return Err(IngresoProveedorServiceError::GafeteNoDisponible(estado));
            }
            ValidacionAsignacion::Asignable => {}
        }
        if self
            .registros
            .buscar_ingreso_activo_por_gafete(gafete_numero)?
            .is_some()
        {
            return Err(IngresoProveedorServiceError::GafeteOcupado);
        }

        Ok(self.registros.crear(&NuevoRegistroIngresoProveedor {
            cedula: cedula.to_string(),
            nombre: nombre.to_string(),
            empresa_id,
            empresa_nombre: empresa.nombre,
            placa,
            gafete_numero,
            fecha_hora_ingreso: ahora,
            usuario_ingreso_id: usuario_id,
        })?)
    }

    pub fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), IngresoProveedorServiceError> {
        let registro = self
            .registros
            .buscar_por_id(id)?
            .ok_or(IngresoProveedorServiceError::RegistroNoActivo)?;
        if registro.salida.is_some() {
            return Err(IngresoProveedorServiceError::RegistroNoActivo);
        }
        if !salida_es_cronologicamente_valida(registro.fecha_hora_ingreso, fecha_hora_salida) {
            return Err(IngresoProveedorServiceError::SalidaAnteriorAIngreso);
        }

        Ok(self
            .registros
            .registrar_salida(id, fecha_hora_salida, usuario_salida_id)?)
    }

    pub fn listar_activos(
        &self,
    ) -> Result<Vec<RegistroIngresoProveedorActivoResumen>, IngresoProveedorServiceError> {
        Ok(self.registros.listar_activos()?)
    }

    pub fn buscar_por_id(
        &self,
        id: i64,
    ) -> Result<Option<RegistroIngresoProveedor>, IngresoProveedorServiceError> {
        Ok(self.registros.buscar_por_id(id)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::empresa_proveedor_repository::SqliteEmpresaProveedorRepository;
    use crate::database::repositories::gafete_repository::SqliteGafeteRepository;
    use crate::database::repositories::registro_ingreso_proveedor_repository::SqliteRegistroIngresoProveedorRepository;
    use crate::database::schema::initialize_database;
    use chrono::Utc;
    use rusqlite::Connection;

    fn conexion_con_empresa_y_gafete() -> (Connection, i64) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                    VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
                 INSERT INTO empresas_proveedor (id, nombre, activo, uuid)
                    VALUES (1, 'Maika', 1, 'uuid-empresa-proveedor-1');",
            )
            .unwrap();
        let gafetes = SqliteGafeteRepository::new(&connection);
        gafetes.crear(7, TipoGafete::Proveedor).unwrap();
        (connection, 1)
    }

    #[test]
    fn registrar_ingreso_y_salida_redondea_el_viaje() {
        let (connection, empresa_id) = conexion_con_empresa_y_gafete();
        let registros = SqliteRegistroIngresoProveedorRepository::new(&connection);
        let empresas = SqliteEmpresaProveedorRepository::new(&connection);
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = IngresoProveedorService::new(&registros, &empresas, &gafetes);

        let id = servicio
            .registrar_ingreso("1-1111", "Juan Perez", empresa_id, None, 7, 1, Utc::now())
            .unwrap();
        assert_eq!(servicio.listar_activos().unwrap().len(), 1);

        servicio.registrar_salida(id, Utc::now(), 1).unwrap();

        assert!(servicio.listar_activos().unwrap().is_empty());
    }

    #[test]
    fn registrar_ingreso_con_empresa_inexistente_falla() {
        let (connection, _) = conexion_con_empresa_y_gafete();
        let registros = SqliteRegistroIngresoProveedorRepository::new(&connection);
        let empresas = SqliteEmpresaProveedorRepository::new(&connection);
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = IngresoProveedorService::new(&registros, &empresas, &gafetes);

        let error = servicio
            .registrar_ingreso("1-1111", "Juan Perez", 999, None, 7, 1, Utc::now())
            .unwrap_err();

        assert!(matches!(
            error,
            IngresoProveedorServiceError::EmpresaNoEncontrada
        ));
    }

    #[test]
    fn registrar_ingreso_con_gafete_no_registrado_falla() {
        let (connection, empresa_id) = conexion_con_empresa_y_gafete();
        let registros = SqliteRegistroIngresoProveedorRepository::new(&connection);
        let empresas = SqliteEmpresaProveedorRepository::new(&connection);
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = IngresoProveedorService::new(&registros, &empresas, &gafetes);

        let error = servicio
            .registrar_ingreso("1-1111", "Juan Perez", empresa_id, None, 999, 1, Utc::now())
            .unwrap_err();

        assert!(matches!(
            error,
            IngresoProveedorServiceError::GafeteNoRegistrado
        ));
    }

    #[test]
    fn registrar_ingreso_dos_veces_a_la_misma_cedula_falla() {
        let (connection, empresa_id) = conexion_con_empresa_y_gafete();
        let registros = SqliteRegistroIngresoProveedorRepository::new(&connection);
        let empresas = SqliteEmpresaProveedorRepository::new(&connection);
        let gafetes = SqliteGafeteRepository::new(&connection);
        gafetes.crear(8, TipoGafete::Proveedor).unwrap();
        let servicio = IngresoProveedorService::new(&registros, &empresas, &gafetes);
        servicio
            .registrar_ingreso("1-1111", "Juan Perez", empresa_id, None, 7, 1, Utc::now())
            .unwrap();

        let error = servicio
            .registrar_ingreso("1-1111", "Juan Perez", empresa_id, None, 8, 1, Utc::now())
            .unwrap_err();

        assert!(matches!(error, IngresoProveedorServiceError::IngresoActivo));
    }

    #[test]
    fn registrar_ingreso_con_gafete_ya_ocupado_falla() {
        let (connection, empresa_id) = conexion_con_empresa_y_gafete();
        let registros = SqliteRegistroIngresoProveedorRepository::new(&connection);
        let empresas = SqliteEmpresaProveedorRepository::new(&connection);
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = IngresoProveedorService::new(&registros, &empresas, &gafetes);
        servicio
            .registrar_ingreso("1-1111", "Juan Perez", empresa_id, None, 7, 1, Utc::now())
            .unwrap();

        let error = servicio
            .registrar_ingreso("2-2222", "Ana Mora", empresa_id, None, 7, 1, Utc::now())
            .unwrap_err();

        assert!(matches!(error, IngresoProveedorServiceError::GafeteOcupado));
    }

    #[test]
    fn registrar_salida_dos_veces_falla_la_segunda() {
        let (connection, empresa_id) = conexion_con_empresa_y_gafete();
        let registros = SqliteRegistroIngresoProveedorRepository::new(&connection);
        let empresas = SqliteEmpresaProveedorRepository::new(&connection);
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = IngresoProveedorService::new(&registros, &empresas, &gafetes);
        let id = servicio
            .registrar_ingreso("1-1111", "Juan Perez", empresa_id, None, 7, 1, Utc::now())
            .unwrap();
        servicio.registrar_salida(id, Utc::now(), 1).unwrap();

        let error = servicio.registrar_salida(id, Utc::now(), 1).unwrap_err();

        assert!(matches!(
            error,
            IngresoProveedorServiceError::RegistroNoActivo
        ));
    }
}
