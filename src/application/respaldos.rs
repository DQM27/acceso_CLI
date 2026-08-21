//! Respaldos: creación/listado/validación, respaldo automático diario, y
//! exportación de una copia ya validada.

use std::path::{Path, PathBuf};

use crate::database::backup::{RespaldoError, RespaldoResumen, ResultadoValidacion, TipoRespaldo};
use crate::database::error::DatabaseError;
use crate::domain::autorizacion::Operacion;
use crate::services::autenticacion_service::UsuarioSesion;

use super::{AppCore, verificar_actor_activo};

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

    /// Crea el respaldo automático del día si todavía no existe uno — a lo
    /// sumo uno por día calendario en Costa Rica. Es best-effort a
    /// propósito (no devuelve `Result`): a diferencia del respaldo previo a
    /// una migración, éste no es obligatorio, así que un fallo (disco lleno,
    /// permisos) no debe impedir que la app arranque.
    pub fn respaldo_automatico_diario_si_hace_falta(&self) {
        let Ok(listado) = self.listar_respaldos_sistema() else {
            return;
        };
        let hoy = crate::tiempo::fecha_costa_rica(self.reloj.ahora_utc());
        let ya_existe_hoy = listado.iter().any(|respaldo| {
            respaldo.tipo == TipoRespaldo::Automatico
                && crate::tiempo::fecha_costa_rica(respaldo.creado_en) == hoy
        });
        if ya_existe_hoy {
            return;
        }
        if self
            .crear_respaldo_sistema(TipoRespaldo::Automatico)
            .is_ok()
        {
            let _ = crate::database::backup::aplicar_retencion(
                &self.directorio_respaldos(),
                TipoRespaldo::Automatico,
                crate::database::backup::RETENCION_AUTOMATICOS,
            );
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
