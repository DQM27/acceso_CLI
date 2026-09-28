//! Préstamo y devolución de gafetes provisionales (KOF).

use crate::{Nucleo, NucleoError, PrestamoGafeteProvisionalActivoResumen};

#[uniffi::export]
impl Nucleo {
    /// Entrega un gafete provisional KOF -- espejo de
    /// `AppCore::entregar_gafete_provisional`. El buscador de encargado
    /// reusa `buscar_encargados_ruta` tal cual, sin nada nuevo del lado de
    /// `UniFFI` para eso.
    pub fn entregar_gafete_provisional(
        &self,
        encargado_id: i64,
        gafete_numero: i64,
    ) -> Result<i64, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self
            .core_lock()
            .entregar_gafete_provisional(&actor, encargado_id, gafete_numero)?)
    }

    /// Entrega con su regla en la misma llamada; la decide
    /// `application::entregar_gafete_provisional_verificado`, la misma que
    /// usa escritorio. `secreto` vacío se salta el chequeo de nube.
    pub fn entregar_gafete_provisional_con_secreto(
        &self,
        encargado_id: i64,
        gafete_numero: i64,
        secreto: String,
    ) -> Result<i64, NucleoError> {
        use control_acceso::application::{
            EntregaGafeteProvisionalVerificadaError, NubeDelDispositivo,
            entregar_gafete_provisional_verificado,
        };

        let actor = self.actor_autenticado()?;
        let nube = NubeDelDispositivo {
            cache_token: &self.cache_token,
            secreto: Some(secreto.as_str()),
        };
        entregar_gafete_provisional_verificado(
            || self.core_lock(),
            nube,
            &actor,
            encargado_id,
            gafete_numero,
        )
        .map_err(|error| match error {
            EntregaGafeteProvisionalVerificadaError::Servicio(error) => error.into(),
            EntregaGafeteProvisionalVerificadaError::GafeteOcupadoEnSitio { numero } => {
                NucleoError::GafeteOcupadoEnSitio { numero }
            }
            EntregaGafeteProvisionalVerificadaError::Nube(error) => error.into(),
        })
    }

    /// Registra la devolución de un préstamo de gafete provisional KOF --
    /// espejo de `AppCore::registrar_devolucion_gafete_provisional`.
    pub fn registrar_devolucion_gafete_provisional(
        &self,
        prestamo_id: i64,
    ) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self
            .core_lock()
            .registrar_devolucion_gafete_provisional(&actor, prestamo_id)?)
    }

    /// Sin actor -- es una lectura, mismo criterio que `listar_rutas_activas`.
    pub fn listar_gafetes_provisionales_activos(
        &self,
    ) -> Result<Vec<PrestamoGafeteProvisionalActivoResumen>, NucleoError> {
        Ok(self
            .core_lock()
            .listar_gafetes_provisionales_activos()?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}
