//! Control de proveedores
//! (`docs/features-futuras/plan-control-proveedores.md`) -- catálogo de
//! empresas proveedoras con el molde simple de `catalogos.rs` (sólo actor
//! activo, sin reloj), e ingreso/salida con el molde de `citas.rs`/
//! `gafetes_provisionales.rs` (transacción `Immediate`, operador activo
//! confirmado dentro de la misma transacción).

use rusqlite::{Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::repositories::empresa_proveedor_repository::SqliteEmpresaProveedorRepository;
use crate::database::repositories::gafete_repository::SqliteGafeteRepository;
use crate::database::repositories::registro_ingreso_proveedor_repository::SqliteRegistroIngresoProveedorRepository;
use crate::models::empresa_proveedor::EmpresaProveedor;
use crate::models::registro_ingreso_proveedor::RegistroIngresoProveedorActivoResumen;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::empresa_proveedor_service::EmpresaProveedorService;
use crate::services::error::{EmpresaProveedorServiceError, IngresoProveedorServiceError};
use crate::services::ingreso_proveedor_service::IngresoProveedorService;

use super::{AppCore, verificar_actor_activo};

impl AppCore {
    // ---- Catálogo: empresas proveedoras ----

    /// Sin `actor`, mismo criterio que `AppCore::listar_empresas` -- lectura,
    /// no autoriza nada. Para la grilla de administración -- trae activas e
    /// inactivas. `listar_empresas_proveedor_seleccionables` es la
    /// contraparte para un selector de wizard, donde una empresa inactiva
    /// nunca es una opción válida -- la decisión de cuál pedir vive en
    /// `EmpresaProveedorService`, no acá ni en quien llama.
    pub fn listar_empresas_proveedor(
        &self,
    ) -> Result<Vec<EmpresaProveedor>, EmpresaProveedorServiceError> {
        let repositorio = SqliteEmpresaProveedorRepository::new(&self.connection);
        EmpresaProveedorService::new(&repositorio).listar()
    }

    pub fn listar_empresas_proveedor_seleccionables(
        &self,
    ) -> Result<Vec<EmpresaProveedor>, EmpresaProveedorServiceError> {
        let repositorio = SqliteEmpresaProveedorRepository::new(&self.connection);
        EmpresaProveedorService::new(&repositorio).listar_seleccionables()
    }

    /// Sin `actor`, mismo criterio que `AppCore::buscar_encargados_ruta` --
    /// pensado para el buscador de la grilla de administración
    /// (`Empresas.tsx`). `buscar_empresas_proveedor_seleccionables` es la
    /// contraparte para el selector con autocompletado del wizard de
    /// proveedores.
    pub fn buscar_empresas_proveedor(
        &self,
        texto: &str,
    ) -> Result<Vec<EmpresaProveedor>, EmpresaProveedorServiceError> {
        let repositorio = SqliteEmpresaProveedorRepository::new(&self.connection);
        EmpresaProveedorService::new(&repositorio).buscar(texto)
    }

    pub fn buscar_empresas_proveedor_seleccionables(
        &self,
        texto: &str,
    ) -> Result<Vec<EmpresaProveedor>, EmpresaProveedorServiceError> {
        let repositorio = SqliteEmpresaProveedorRepository::new(&self.connection);
        EmpresaProveedorService::new(&repositorio).buscar_seleccionables(texto)
    }

    pub fn crear_empresa_proveedor(
        &self,
        actor: &UsuarioSesion,
        nombre: &str,
    ) -> Result<i64, EmpresaProveedorServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(EmpresaProveedorServiceError::Database)?
            .ok_or(EmpresaProveedorServiceError::OperacionNoAutorizada)?;
        let repositorio = SqliteEmpresaProveedorRepository::new(&transaction);
        let id = EmpresaProveedorService::new(&repositorio).crear(nombre)?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EmpresaProveedorServiceError::Database)?;
        Ok(id)
    }

    /// Sin chequeo de rol, mismo criterio que `crear_empresa_proveedor` --
    /// catálogo chico, cualquier actor activo administra empresas
    /// proveedoras (a diferencia de `Empresa`, que sí distingue permisos por
    /// rol vía `Operacion::ActivarDesactivarEmpresa`).
    pub fn activar_empresa_proveedor(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), EmpresaProveedorServiceError> {
        self.establecer_empresa_proveedor_activa(actor, id, true)
    }

    pub fn desactivar_empresa_proveedor(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), EmpresaProveedorServiceError> {
        self.establecer_empresa_proveedor_activa(actor, id, false)
    }

    fn establecer_empresa_proveedor_activa(
        &self,
        actor: &UsuarioSesion,
        id: i64,
        activa: bool,
    ) -> Result<(), EmpresaProveedorServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        verificar_actor_activo(&transaction, actor)
            .map_err(EmpresaProveedorServiceError::Database)?
            .ok_or(EmpresaProveedorServiceError::OperacionNoAutorizada)?;
        let repositorio = SqliteEmpresaProveedorRepository::new(&transaction);
        let servicio = EmpresaProveedorService::new(&repositorio);
        if activa {
            servicio.activar(id)?;
        } else {
            servicio.desactivar(id)?;
        }
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(EmpresaProveedorServiceError::Database)
    }

    // ---- Ingreso/salida de proveedores ----

    #[allow(clippy::too_many_arguments)]
    pub fn registrar_ingreso_proveedor(
        &self,
        actor: &UsuarioSesion,
        cedula: &str,
        nombre: &str,
        empresa_id: i64,
        placa: Option<String>,
        gafete_numero: i64,
    ) -> Result<i64, IngresoProveedorServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(IngresoProveedorServiceError::Database)?
            .ok_or(IngresoProveedorServiceError::OperadorNoAutorizado)?;
        let registros = SqliteRegistroIngresoProveedorRepository::new(&transaction);
        let empresas = SqliteEmpresaProveedorRepository::new(&transaction);
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let id = IngresoProveedorService::new(&registros, &empresas, &gafetes).registrar_ingreso(
            cedula,
            nombre,
            empresa_id,
            placa,
            gafete_numero,
            actor_actual.id,
            self.reloj.ahora_utc(),
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(IngresoProveedorServiceError::Database)?;
        Ok(id)
    }

    pub fn registrar_salida_proveedor(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), IngresoProveedorServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(IngresoProveedorServiceError::Database)?
            .ok_or(IngresoProveedorServiceError::OperadorNoAutorizado)?;
        let registros = SqliteRegistroIngresoProveedorRepository::new(&transaction);
        let empresas = SqliteEmpresaProveedorRepository::new(&transaction);
        let gafetes = SqliteGafeteRepository::new(&transaction);
        IngresoProveedorService::new(&registros, &empresas, &gafetes).registrar_salida(
            id,
            self.reloj.ahora_utc(),
            actor_actual.id,
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(IngresoProveedorServiceError::Database)
    }

    /// Sin `actor`, mismo criterio que `listar_rutas_activas`/
    /// `listar_gafetes_provisionales_activos`: es una lectura, no una
    /// operación que autorizar.
    pub fn listar_proveedores_activos(
        &self,
    ) -> Result<Vec<RegistroIngresoProveedorActivoResumen>, IngresoProveedorServiceError> {
        let registros = SqliteRegistroIngresoProveedorRepository::new(&self.connection);
        let empresas = SqliteEmpresaProveedorRepository::new(&self.connection);
        let gafetes = SqliteGafeteRepository::new(&self.connection);
        IngresoProveedorService::new(&registros, &empresas, &gafetes).listar_activos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::empresa_proveedor_repository::EmpresaProveedorRepository;
    use crate::database::repositories::gafete_repository::GafeteRepository;
    use crate::database::schema::initialize_database;
    use crate::models::gafete::TipoGafete;
    use crate::tiempo::RelojFijo;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    fn nucleo_con_usuario_empresa_y_gafete() -> (AppCore, UsuarioSesion, i64) {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        let empresa_id = SqliteEmpresaProveedorRepository::new(&connection)
            .crear(&EmpresaProveedor {
                id: 0,
                nombre: "Maika".to_string(),
                activo: true,
            })
            .unwrap();
        SqliteGafeteRepository::new(&connection)
            .crear(7, TipoGafete::Proveedor)
            .unwrap();
        let reloj = Arc::new(RelojFijo::new(
            Utc.with_ymd_and_hms(2026, 9, 16, 12, 0, 0).unwrap(),
        ));
        let core = AppCore::con_reloj(connection, reloj);
        let sesion = UsuarioSesion {
            id: 1,
            cedula: "1001".to_string(),
            nombre: "Operador".to_string(),
            rol: crate::models::usuario::RolUsuario::Operador,
        };
        (core, sesion, empresa_id)
    }

    #[test]
    fn registrar_ingreso_y_salida_de_proveedor_redondea_el_viaje() {
        let (core, actor, empresa_id) = nucleo_con_usuario_empresa_y_gafete();

        let id = core
            .registrar_ingreso_proveedor(&actor, "1-1111", "Juan Perez", empresa_id, None, 7)
            .unwrap();
        assert_eq!(core.listar_proveedores_activos().unwrap().len(), 1);

        core.registrar_salida_proveedor(&actor, id).unwrap();

        assert!(core.listar_proveedores_activos().unwrap().is_empty());
    }

    #[test]
    fn activar_y_desactivar_empresa_proveedor_redondea_el_viaje() {
        let (core, actor, empresa_id) = nucleo_con_usuario_empresa_y_gafete();

        core.desactivar_empresa_proveedor(&actor, empresa_id)
            .unwrap();
        assert!(
            !core
                .listar_empresas_proveedor()
                .unwrap()
                .iter()
                .find(|empresa| empresa.id == empresa_id)
                .unwrap()
                .activo
        );
        assert!(
            core.listar_empresas_proveedor_seleccionables()
                .unwrap()
                .iter()
                .all(|empresa| empresa.id != empresa_id),
            "la desactivada no debe aparecer al pedir sólo activas"
        );

        core.activar_empresa_proveedor(&actor, empresa_id).unwrap();
        assert!(
            core.listar_empresas_proveedor()
                .unwrap()
                .iter()
                .find(|empresa| empresa.id == empresa_id)
                .unwrap()
                .activo
        );
    }

    #[test]
    fn crear_empresa_proveedor_y_buscarla_redondea_el_viaje() {
        let (core, actor, _) = nucleo_con_usuario_empresa_y_gafete();

        core.crear_empresa_proveedor(&actor, "Dos Pinos").unwrap();

        let resultados = core.buscar_empresas_proveedor("dos pinos").unwrap();
        assert_eq!(resultados.len(), 1);
        assert_eq!(resultados[0].nombre, "Dos Pinos");
    }
}
