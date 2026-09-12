//! Check-in/check-out de visitas (`docs/planes-implementados/plan-control-visitas.md`) --
//! fachada sobre `CitaService`, mismo armazón que `accesos.rs` para
//! contratistas (transacción `Immediate`, reloj validado, operador activo
//! confirmado dentro de la misma transacción).

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::repositories::cita_repository::SqliteCitaRepository;
use crate::database::repositories::movimiento_visita_repository::{
    MovimientoVisitaRepository, SqliteMovimientoVisitaRepository,
};
use crate::models::cita::{Cita, CitaVisitante};
use crate::models::movimiento_visita::MovimientoVisitaActivoResumen;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::cita_service::CitaService;
use crate::services::error::CitaServiceError;
use crate::tiempo::fecha_costa_rica;

use super::{AppCore, verificar_actor_activo};

impl AppCore {
    /// Sin `actor`, mismo criterio que `AppCore::preparar_ingreso`: es una
    /// consulta de sólo lectura, no una autorización cacheada -- quien
    /// confirma el check-in (`registrar_entrada_visita`) vuelve a correr
    /// esta misma regla internamente antes de persistir.
    pub fn verificar_check_in_visita(
        &self,
        cedula: &str,
    ) -> Result<(Cita, CitaVisitante), CitaServiceError> {
        let citas = SqliteCitaRepository::new(&self.connection);
        let movimientos = SqliteMovimientoVisitaRepository::new(&self.connection);
        CitaService::new(&citas, &movimientos)
            .verificar_check_in(cedula, fecha_costa_rica(self.reloj.ahora_utc()))
    }

    /// Mismo armazón que `AppCore::en_transaccion_con_reloj_validado`
    /// (`accesos.rs`), duplicado en vez de generalizado a propósito: los dos
    /// dominios devuelven tipos de error distintos
    /// (`RegistroIngresoServiceError`/`CitaServiceError`) sin un motivo de
    /// negocio para unificarlos, y esta función es chica -- generalizarla
    /// hubiera significado tocar el camino de contratistas, que ya funciona,
    /// sólo para que este dominio nuevo la reutilice. Sufijo `_visita`
    /// porque un `impl AppCore` no admite dos métodos con el mismo nombre
    /// aunque vivan en archivos distintos -- son el mismo tipo.
    fn en_transaccion_con_reloj_validado_visita<T>(
        &self,
        actor: &UsuarioSesion,
        operar: impl FnOnce(
            &Transaction<'_>,
            chrono::DateTime<chrono::Utc>,
        ) -> Result<T, CitaServiceError>,
    ) -> Result<T, CitaServiceError> {
        let ahora = self.reloj.ahora_utc();
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        validar_reloj(&transaction, ahora)?;
        verificar_operador_activo(&transaction, actor)?;
        let resultado = operar(&transaction, ahora)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(resultado)
    }

    pub fn registrar_entrada_visita(
        &self,
        actor: &UsuarioSesion,
        cedula: &str,
        gafete_numero: Option<i64>,
    ) -> Result<i64, CitaServiceError> {
        self.en_transaccion_con_reloj_validado_visita(actor, |transaction, ahora| {
            let citas = SqliteCitaRepository::new(transaction);
            let movimientos = SqliteMovimientoVisitaRepository::new(transaction);
            CitaService::new(&citas, &movimientos).registrar_entrada(
                cedula,
                gafete_numero,
                actor.id,
                ahora,
                fecha_costa_rica(ahora),
            )
        })
    }

    pub fn registrar_salida_visita(
        &self,
        actor: &UsuarioSesion,
        movimiento_id: i64,
    ) -> Result<(), CitaServiceError> {
        self.en_transaccion_con_reloj_validado_visita(actor, |transaction, ahora| {
            let citas = SqliteCitaRepository::new(transaction);
            let movimientos = SqliteMovimientoVisitaRepository::new(transaction);
            CitaService::new(&citas, &movimientos).registrar_salida(movimiento_id, ahora, actor.id)
        })
    }

    /// Sin `actor`, mismo criterio que `listar_ingresos_activos`
    /// (`accesos.rs`): es una lectura, no una operación que autorizar.
    pub fn listar_visitas_activas(
        &self,
    ) -> Result<Vec<MovimientoVisitaActivoResumen>, DatabaseError> {
        SqliteMovimientoVisitaRepository::new(&self.connection).listar_activos()
    }
}

/// Mismo criterio que `accesos::validar_reloj`: comprobación de sanidad de
/// todo el sistema (¿el reloj de la máquina retrocedió respecto al último
/// movimiento conocido, de cualquier tipo?), no una regla de negocio de una
/// entrada/salida puntual de visitas. Toma el máximo entre
/// `registro_ingresos` (contratistas) y `movimientos_visita` -- un sitio
/// que sólo tuvo actividad de un tipo (ej. recién estrenando visitas, sin
/// ningún contratista todavía en esta base) queda igual de protegido.
fn validar_reloj(
    connection: &Connection,
    ahora: chrono::DateTime<chrono::Utc>,
) -> Result<(), CitaServiceError> {
    let ultima_contratista =
        crate::database::queries::ingresos::ultimo_instante_movimiento(connection)?;
    let ultima_visita = crate::database::repositories::movimiento_visita_repository::ultimo_instante_movimiento_visita(
        connection,
    )?;
    let Some(ultima) = ultima_contratista.into_iter().chain(ultima_visita).max() else {
        return Ok(());
    };
    if ahora < ultima {
        return Err(CitaServiceError::RelojRetrocedido);
    }
    Ok(())
}

fn verificar_operador_activo(
    connection: &Connection,
    actor: &UsuarioSesion,
) -> Result<(), CitaServiceError> {
    if verificar_actor_activo(connection, actor)?.is_some() {
        Ok(())
    } else {
        Err(CitaServiceError::OperadorNoAutorizado)
    }
}
