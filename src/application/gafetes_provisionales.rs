//! Préstamos de gafete provisional KOF
//! (`docs/features-futuras/plan-gafetes-provisionales-kof.md`) -- fachada
//! sobre `GafeteProvisionalService`. Más simple que `rutas.rs`: sin reloj
//! validado entre dominios (pedido explícito del usuario, módulo sin más
//! verificación que la humana), sólo actor activo confirmado dentro de la
//! misma transacción.

use rusqlite::{Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::repositories::encargado_ruta_repository::SqliteEncargadoRutaRepository;
use crate::database::repositories::prestamo_gafete_provisional_repository::SqlitePrestamoGafeteProvisionalRepository;
use crate::models::prestamo_gafete_provisional::PrestamoGafeteProvisionalActivoResumen;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::GafeteProvisionalServiceError;
use crate::services::gafete_provisional_service::GafeteProvisionalService;

use super::{AppCore, verificar_actor_activo};

impl AppCore {
    pub fn entregar_gafete_provisional(
        &self,
        actor: &UsuarioSesion,
        encargado_id: i64,
        gafete_numero: i64,
    ) -> Result<i64, GafeteProvisionalServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(GafeteProvisionalServiceError::Database)?
            .ok_or(GafeteProvisionalServiceError::OperacionNoAutorizada)?;
        let prestamos = SqlitePrestamoGafeteProvisionalRepository::new(&transaction);
        let encargados = SqliteEncargadoRutaRepository::new(&transaction);
        let id = GafeteProvisionalService::new(&prestamos, &encargados).entregar(
            encargado_id,
            gafete_numero,
            actor_actual.id,
            self.reloj.ahora_utc(),
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(GafeteProvisionalServiceError::Database)?;
        Ok(id)
    }

    pub fn registrar_devolucion_gafete_provisional(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), GafeteProvisionalServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(GafeteProvisionalServiceError::Database)?
            .ok_or(GafeteProvisionalServiceError::OperacionNoAutorizada)?;
        let prestamos = SqlitePrestamoGafeteProvisionalRepository::new(&transaction);
        let encargados = SqliteEncargadoRutaRepository::new(&transaction);
        GafeteProvisionalService::new(&prestamos, &encargados).registrar_devolucion(
            id,
            self.reloj.ahora_utc(),
            actor_actual.id,
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(GafeteProvisionalServiceError::Database)
    }

    /// Sin `actor`, mismo criterio que `listar_rutas_activas`: es una
    /// lectura, no una operación que autorizar.
    pub fn listar_gafetes_provisionales_activos(
        &self,
    ) -> Result<Vec<PrestamoGafeteProvisionalActivoResumen>, GafeteProvisionalServiceError> {
        let prestamos = SqlitePrestamoGafeteProvisionalRepository::new(&self.connection);
        let encargados = SqliteEncargadoRutaRepository::new(&self.connection);
        GafeteProvisionalService::new(&prestamos, &encargados).listar_activos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::encargado_ruta_repository::EncargadoRutaRepository;
    use crate::database::schema::initialize_database;
    use crate::tiempo::RelojFijo;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    fn nucleo_con_usuario_y_encargado() -> (AppCore, UsuarioSesion, i64) {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        let encargado_id = SqliteEncargadoRutaRepository::new(&connection)
            .crear(&crate::models::encargado_ruta::EncargadoRuta {
                id: 0,
                codigo_empleado: "5040017".to_string(),
                nombre: "Michael Araya Retana".to_string(),
                cedula: None,
                activo: true,
            })
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
        (core, sesion, encargado_id)
    }

    #[test]
    fn entregar_y_devolver_gafete_provisional_redondea_el_viaje() {
        let (core, actor, encargado_id) = nucleo_con_usuario_y_encargado();

        let id = core
            .entregar_gafete_provisional(&actor, encargado_id, 12)
            .unwrap();
        assert_eq!(core.listar_gafetes_provisionales_activos().unwrap().len(), 1);

        core.registrar_devolucion_gafete_provisional(&actor, id)
            .unwrap();

        assert!(core.listar_gafetes_provisionales_activos().unwrap().is_empty());
    }

    #[test]
    fn listar_gafetes_provisionales_activos_omite_los_ya_devueltos() {
        let (core, actor, encargado_id) = nucleo_con_usuario_y_encargado();
        let id = core
            .entregar_gafete_provisional(&actor, encargado_id, 12)
            .unwrap();
        core.registrar_devolucion_gafete_provisional(&actor, id)
            .unwrap();

        assert!(core.listar_gafetes_provisionales_activos().unwrap().is_empty());
    }
}
