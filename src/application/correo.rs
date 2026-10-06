//! Ingreso "por correo" (visita autorizada por correo, comodín previo al
//! módulo de Visitas) -- mismo molde que el ingreso/salida de
//! `proveedores.rs`: transacción `Immediate`, operador activo confirmado
//! dentro de la misma transacción.

use rusqlite::{Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::repositories::gafete_repository::SqliteGafeteRepository;
use crate::database::repositories::registro_ingreso_correo_repository::{
    RegistroIngresoCorreoRepository, SqliteRegistroIngresoCorreoRepository,
};
use crate::models::registro_ingreso_correo::RegistroIngresoCorreoActivoResumen;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::IngresoCorreoServiceError;
use crate::services::ingreso_correo_service::{DatosIngresoCorreo, IngresoCorreoService};

use super::{AppCore, verificar_actor_activo};

impl AppCore {
    pub fn registrar_ingreso_correo(
        &self,
        actor: &UsuarioSesion,
        datos: &DatosIngresoCorreo<'_>,
    ) -> Result<i64, IngresoCorreoServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(IngresoCorreoServiceError::Database)?
            .ok_or(IngresoCorreoServiceError::OperadorNoAutorizado)?;
        let registros = SqliteRegistroIngresoCorreoRepository::new(&transaction);
        let gafetes = SqliteGafeteRepository::new(&transaction);
        let id = IngresoCorreoService::new(&registros, &gafetes).registrar_ingreso(
            datos,
            actor_actual.id,
            self.reloj.ahora_utc(),
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(IngresoCorreoServiceError::Database)?;
        Ok(id)
    }

    pub fn registrar_salida_correo(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), IngresoCorreoServiceError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        let actor_actual = verificar_actor_activo(&transaction, actor)
            .map_err(IngresoCorreoServiceError::Database)?
            .ok_or(IngresoCorreoServiceError::OperadorNoAutorizado)?;
        let registros = SqliteRegistroIngresoCorreoRepository::new(&transaction);
        let gafetes = SqliteGafeteRepository::new(&transaction);
        IngresoCorreoService::new(&registros, &gafetes).registrar_salida(
            id,
            self.reloj.ahora_utc(),
            actor_actual.id,
        )?;
        transaction
            .commit()
            .map_err(DatabaseError::from)
            .map_err(IngresoCorreoServiceError::Database)
    }

    /// Sin `actor`: es una lectura, no una operación que autorizar (mismo
    /// criterio que `listar_proveedores_activos`).
    pub fn listar_correos_activos(
        &self,
    ) -> Result<Vec<RegistroIngresoCorreoActivoResumen>, IngresoCorreoServiceError> {
        let registros = SqliteRegistroIngresoCorreoRepository::new(&self.connection);
        let gafetes = SqliteGafeteRepository::new(&self.connection);
        IngresoCorreoService::new(&registros, &gafetes).listar_activos()
    }

    /// Regla: una cédula no puede tener dos ingresos por correo abiertos en
    /// el sitio. Mira los de este equipo y los del otro dispositivo (caché
    /// `ingresos_correo_remotos`) -- mismo criterio que
    /// `proveedor_con_ingreso_activo_en_sitio`.
    pub fn correo_con_ingreso_activo_en_sitio(
        &self,
        cedula: &str,
    ) -> Result<bool, IngresoCorreoServiceError> {
        let Ok(cedula) = crate::domain::cedula::Cedula::normalizar(cedula) else {
            return Ok(false);
        };
        let cedula = cedula.as_str();
        let registros = SqliteRegistroIngresoCorreoRepository::new(&self.connection);
        if registros.buscar_ingreso_activo(cedula)?.is_some() {
            return Ok(true);
        }
        self.connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM ingresos_correo_remotos
                 WHERE NORMALIZAR_CEDULA(cedula) = ?1)",
                [cedula],
                |fila| fila.get(0),
            )
            .map_err(DatabaseError::from)
            .map_err(IngresoCorreoServiceError::Database)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::gafete_repository::GafeteRepository;
    use crate::database::schema::initialize_database;
    use crate::models::gafete::TipoGafete;
    use crate::tiempo::RelojFijo;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    fn nucleo() -> (AppCore, UsuarioSesion) {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO usuarios (id, cedula, nombre, password_hash, rol, activo)
                 VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1)",
                [],
            )
            .unwrap();
        SqliteGafeteRepository::new(&connection)
            .crear(5, TipoGafete::Visita)
            .unwrap();
        let reloj = Arc::new(RelojFijo::new(
            Utc.with_ymd_and_hms(2026, 10, 3, 14, 0, 0).unwrap(),
        ));
        let sesion = UsuarioSesion {
            id: 1,
            cedula: "1001".to_string(),
            nombre: "Operador".to_string(),
            rol: crate::models::usuario::RolUsuario::Operador,
        };
        (AppCore::con_reloj(connection, reloj), sesion)
    }

    fn datos(cedula: &str) -> DatosIngresoCorreo<'_> {
        DatosIngresoCorreo {
            cedula,
            nombre: "Ana Solano",
            motivo: "Entrevista RH",
            placa: None,
            gafete_numero: 5,
        }
    }

    #[test]
    fn ingreso_y_salida_por_correo_redondean_el_viaje() {
        let (core, actor) = nucleo();
        let id = core
            .registrar_ingreso_correo(&actor, &datos("111111111"))
            .unwrap();
        let activos = core.listar_correos_activos().unwrap();
        assert_eq!(activos.len(), 1);
        assert_eq!(activos[0].usuario_ingreso_nombre, "Operador");

        core.registrar_salida_correo(&actor, id).unwrap();
        assert!(core.listar_correos_activos().unwrap().is_empty());
    }

    #[test]
    fn correo_con_ingreso_activo_mira_local_y_otro_dispositivo() {
        let (core, actor) = nucleo();
        assert!(
            !core
                .correo_con_ingreso_activo_en_sitio("111111111")
                .unwrap()
        );
        core.registrar_ingreso_correo(&actor, &datos("111111111"))
            .unwrap();
        assert!(
            core.correo_con_ingreso_activo_en_sitio(" 1-1111-1111 ")
                .unwrap()
        );

        core.connection
            .execute(
                "INSERT INTO ingresos_correo_remotos (uuid, sitio_id, cedula, nombre, motivo,
                     placa, gafete_numero, hora_entrada, usuario_entrada_nombre,
                     dispositivo_entrada_id, actualizado_en)
                 VALUES ('u1', 's1', '222222222', 'Luis', 'Entrevista RH', NULL, 8,
                     '2026-10-03T13:00:00Z', 'Otro', 'd2', '2026-10-03T13:00:00Z')",
                [],
            )
            .unwrap();
        assert!(
            core.correo_con_ingreso_activo_en_sitio("222222222")
                .unwrap()
        );
        assert!(
            !core
                .correo_con_ingreso_activo_en_sitio("333333333")
                .unwrap()
        );
        assert!(!core.correo_con_ingreso_activo_en_sitio("  ").unwrap());
    }

    #[test]
    fn un_operador_inexistente_no_registra() {
        let (core, mut actor) = nucleo();
        actor.id = 99;
        assert!(matches!(
            core.registrar_ingreso_correo(&actor, &datos("111111111")),
            Err(IngresoCorreoServiceError::OperadorNoAutorizado)
        ));
    }
}
