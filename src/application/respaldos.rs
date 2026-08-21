//! Respaldos: creación/listado/validación, respaldo automático diario, y
//! exportación de una copia ya validada.

use std::path::{Path, PathBuf};

use chrono::Timelike;

use crate::database::backup::{RespaldoError, RespaldoResumen, ResultadoValidacion, TipoRespaldo};
use crate::database::error::DatabaseError;
use crate::domain::autorizacion::Operacion;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::tiempo::a_costa_rica;

/// Hora (Costa Rica) a partir de la cual corre el respaldo automático del
/// día — de madrugada, cuando es menos probable que un operador esté
/// registrando un ingreso a la vez.
const HORA_RESPALDO_AUTOMATICO: u32 = 1;

use super::{AppCore, verificar_actor_activo};

/// Resultado de una revisión del respaldo automático diario — para que la
/// TUI pueda avisarle al operador cuando algo falla, sin necesitar guardar
/// nada aparte de lo que ya vive en memoria durante la sesión actual (no es
/// un sistema de log persistente, sólo el último resultado conocido).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoRespaldoAutomatico {
    /// No hacía falta hacer nada: aún no es la hora, o ya existe el de hoy.
    SinCambios,
    /// Se creó el respaldo del día sin problema.
    Creado,
    /// Falló crear el respaldo — mensaje ya listo para mostrar, no un error
    /// técnico crudo. La limpieza por retención se queda best-effort/en
    /// silencio como antes: acumular respaldos de más no es un riesgo de
    /// pérdida de datos, a diferencia de no poder crear uno nuevo.
    Fallo(String),
}

impl AppCore {
    pub fn crear_respaldo(
        &self,
        actor: &UsuarioSesion,
        tipo: TipoRespaldo,
    ) -> Result<RespaldoResumen, RespaldoError> {
        self.autorizar_respaldo(actor)?;
        self.crear_respaldo_sistema(tipo)
    }

    fn crear_respaldo_sistema(&self, tipo: TipoRespaldo) -> Result<RespaldoResumen, RespaldoError> {
        crate::database::backup::crear_respaldo(
            &self.connection,
            &self.directorio_respaldos(),
            tipo,
        )
    }

    pub fn listar_respaldos(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<Vec<RespaldoResumen>, RespaldoError> {
        self.autorizar_respaldo(actor)?;
        self.listar_respaldos_sistema()
    }

    fn listar_respaldos_sistema(&self) -> Result<Vec<RespaldoResumen>, RespaldoError> {
        crate::database::backup::listar_respaldos(&self.directorio_respaldos())
    }

    pub fn validar_respaldo(
        &self,
        actor: &UsuarioSesion,
        ruta: &Path,
    ) -> Result<ResultadoValidacion, RespaldoError> {
        self.autorizar_respaldo(actor)?;
        crate::database::backup::validar_respaldo(ruta)
    }

    /// Crea el respaldo automático del día si todavía no existe uno y ya
    /// pasó la 01:00 (hora Costa Rica) — a lo sumo uno por día calendario.
    /// Se llama tanto al abrir la app (por si el día anterior nunca llegó a
    /// correr, p. ej. la app estuvo cerrada) como periódicamente mientras
    /// sigue abierta (la app puede quedarse corriendo varios días seguidos
    /// sin reiniciar, y antes esta función sólo se evaluaba una vez al
    /// arrancar el proceso). No interrumpe al operador ni impide que la app
    /// arranque: un fallo (disco lleno, permisos) se devuelve para que la
    /// TUI decida cómo avisar, no se convierte en un error fatal aquí.
    pub fn respaldo_automatico_diario_si_hace_falta(&self) -> EstadoRespaldoAutomatico {
        let ahora = a_costa_rica(self.reloj.ahora_utc());
        if ahora.hour() < HORA_RESPALDO_AUTOMATICO {
            return EstadoRespaldoAutomatico::SinCambios;
        }
        let listado = match self.listar_respaldos_sistema() {
            Ok(listado) => listado,
            Err(error) => return EstadoRespaldoAutomatico::Fallo(error.to_string()),
        };
        let hoy = ahora.date_naive();
        let ya_existe_hoy = listado.iter().any(|respaldo| {
            respaldo.tipo == TipoRespaldo::Automatico
                && crate::tiempo::fecha_costa_rica(respaldo.creado_en) == hoy
        });
        if ya_existe_hoy {
            return EstadoRespaldoAutomatico::SinCambios;
        }
        match self.crear_respaldo_sistema(TipoRespaldo::Automatico) {
            Ok(_) => {
                let _ = crate::database::backup::aplicar_retencion(
                    &self.directorio_respaldos(),
                    TipoRespaldo::Automatico,
                    crate::database::backup::RETENCION_AUTOMATICOS,
                );
                EstadoRespaldoAutomatico::Creado
            }
            Err(error) => EstadoRespaldoAutomatico::Fallo(error.to_string()),
        }
    }

    /// El archivo interno ya fue validado por `crear_respaldo`; exportarlo es una
    /// copia simple a la ruta que indique el operador, sin volver a pasar por el
    /// motor de respaldo.
    pub fn exportar_respaldo(
        &self,
        actor: &UsuarioSesion,
        origen: &Path,
        destino: &Path,
    ) -> Result<(), RespaldoError> {
        self.autorizar_respaldo(actor)?;
        std::fs::copy(origen, destino)
            .map(|_| ())
            .map_err(Into::into)
    }

    fn autorizar_respaldo(&self, actor: &UsuarioSesion) -> Result<(), RespaldoError> {
        let usuario = verificar_actor_activo(&self.connection, actor)
            .map_err(|error| match error {
                DatabaseError::Sqlite(error) => RespaldoError::Sqlite(error),
                _ => RespaldoError::OperacionNoAutorizada,
            })?
            .ok_or(RespaldoError::OperacionNoAutorizada)?;
        if !usuario.rol.puede(Operacion::GestionarRespaldos) {
            return Err(RespaldoError::OperacionNoAutorizada);
        }
        Ok(())
    }

    fn directorio_respaldos(&self) -> PathBuf {
        self.ruta_base_datos
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups")
    }
}
