//! Ingreso "por correo" (visita autorizada por correo, comodín previo al
//! módulo de Visitas) -- mismo molde que `proveedores.rs`.

use control_acceso::services::error::IngresoCorreoServiceError as IngresoCorreoServiceErrorNucleo;

use crate::{Nucleo, NucleoError, RegistroIngresoCorreoActivoResumen};

#[uniffi::export]
impl Nucleo {
    /// Ingreso por correo con todas sus reglas en una sola llamada; el orden
    /// y qué falla frena los decide
    /// `application::registrar_ingreso_correo_verificado`, la misma que usa
    /// escritorio. Sin vincular se salta los chequeos de nube. Placa vacía o
    /// en blanco = llegó caminando.
    pub fn registrar_ingreso_correo_verificado(
        &self,
        cedula: String,
        nombre: String,
        motivo: String,
        placa: Option<String>,
        gafete_numero: i64,
    ) -> Result<i64, NucleoError> {
        use control_acceso::application::{
            IngresoCorreoVerificadoError, NuevoIngresoCorreo, registrar_ingreso_correo_verificado,
        };

        let actor = self.actor_autenticado()?;
        let datos = NuevoIngresoCorreo {
            cedula,
            nombre,
            motivo,
            placa,
            gafete_numero,
        };
        registrar_ingreso_correo_verificado(|| self.core_lock(), &self.cache_token, &actor, datos)
            .map_err(|error| match error {
                IngresoCorreoVerificadoError::Servicio(error) => error.into(),
                IngresoCorreoVerificadoError::Nube(error) => error.into(),
                regla => NucleoError::Rechazado {
                    mensaje: control_acceso::mensajes::mensaje_ingreso_correo_verificado(regla),
                },
            })
    }

    /// Aviso para mostrar mientras se tipea la cédula: `Some(mensaje)` si ya
    /// tiene un ingreso por correo abierto en este sitio (este equipo o el
    /// otro dispositivo), `None` si puede entrar. Sin actor: es una lectura.
    pub fn aviso_correo_con_ingreso_activo(
        &self,
        cedula: String,
    ) -> Result<Option<String>, NucleoError> {
        let activo = self
            .core_lock()
            .correo_con_ingreso_activo_en_sitio(&cedula)?;
        Ok(activo.then(|| {
            control_acceso::mensajes::mensaje_ingreso_correo(
                IngresoCorreoServiceErrorNucleo::IngresoActivo,
            )
        }))
    }

    pub fn registrar_salida_correo(&self, registro_id: i64) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self
            .core_lock()
            .registrar_salida_correo(&actor, registro_id)?)
    }

    /// Sin actor -- es una lectura, mismo criterio que
    /// `listar_proveedores_activos`.
    pub fn listar_correos_activos(
        &self,
    ) -> Result<Vec<RegistroIngresoCorreoActivoResumen>, NucleoError> {
        Ok(self
            .core_lock()
            .listar_correos_activos()?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}
