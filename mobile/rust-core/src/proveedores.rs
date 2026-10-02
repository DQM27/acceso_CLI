//! Ingreso y salida de proveedores y su catálogo de empresas.

use control_acceso::services::error::IngresoProveedorServiceError as IngresoProveedorServiceErrorNucleo;

use crate::{
    EmpresaProveedor, Nucleo, NucleoError, RegistroIngresoProveedorActivoResumen, interno,
};

#[uniffi::export]
impl Nucleo {
    /// Selector con autocompletado del wizard de proveedores (Paso 2:
    /// empresa) -- espejo de `AppCore::buscar_empresas_proveedor_seleccionables`.
    /// Es el selector del wizard de proveedores -- una empresa desactivada
    /// no es una opción válida para un ingreso nuevo. Este era justo el bug
    /// reportado: el filtro faltaba acá y en Kotlin, así que el buscador
    /// seguía mostrando empresas desactivadas.
    pub fn buscar_empresas_proveedor(
        &self,
        texto: String,
    ) -> Result<Vec<EmpresaProveedor>, NucleoError> {
        Ok(self
            .core_lock()
            .buscar_empresas_proveedor_seleccionables(texto.trim())
            .map_err(|origen| NucleoError::Interno {
                mensaje: interno(origen),
            })?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Alta inline de empresa proveedora desde el mismo selector -- espejo
    /// de `AppCore::crear_empresa_proveedor`.
    pub fn crear_empresa_proveedor(&self, nombre: String) -> Result<i64, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().crear_empresa_proveedor(&actor, &nombre)?)
    }

    /// Registra el ingreso (apertura) del ciclo de un proveedor -- espejo
    /// de `AppCore::registrar_ingreso_proveedor`.
    #[allow(clippy::too_many_arguments)]
    pub fn registrar_ingreso_proveedor(
        &self,
        cedula: String,
        nombre: String,
        empresa_id: i64,
        placa: Option<String>,
        gafete_numero: i64,
    ) -> Result<i64, NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().registrar_ingreso_proveedor(
            &actor,
            &cedula,
            &nombre,
            empresa_id,
            placa,
            gafete_numero,
        )?)
    }

    /// Ingreso de proveedor con todas sus reglas en una sola llamada; el
    /// orden y qué falla frena los decide
    /// `application::registrar_ingreso_proveedor_verificado`, la misma que
    /// usa escritorio. Sin vincular se salta los chequeos de nube.
    pub fn registrar_ingreso_proveedor_verificado(
        &self,
        cedula: String,
        nombre: String,
        empresa_id: i64,
        placa: Option<String>,
        gafete_numero: i64,
    ) -> Result<i64, NucleoError> {
        use control_acceso::application::{
            IngresoProveedorVerificadoError, NuevoIngresoProveedor,
            registrar_ingreso_proveedor_verificado,
        };

        let actor = self.actor_autenticado()?;
        let nube = &self.cache_token;
        let datos = NuevoIngresoProveedor {
            cedula,
            nombre,
            empresa_id,
            placa,
            gafete_numero,
        };
        registrar_ingreso_proveedor_verificado(|| self.core_lock(), nube, &actor, datos).map_err(
            |error| match error {
                IngresoProveedorVerificadoError::Servicio(error) => error.into(),
                IngresoProveedorVerificadoError::ActivoEnOtroSitio { sitio } => {
                    NucleoError::ProveedorActivoEnOtroSitio { sitio }
                }
                IngresoProveedorVerificadoError::GafeteOcupadoEnSitio { numero } => {
                    NucleoError::GafeteOcupadoEnSitio { numero }
                }
                IngresoProveedorVerificadoError::Nube(error) => error.into(),
            },
        )
    }

    /// Aviso para mostrar mientras se tipea la cédula: `Some(mensaje)` si
    /// ya tiene un ingreso de proveedor abierto en este sitio (este equipo
    /// o el otro dispositivo), `None` si puede entrar. La regla y el texto
    /// son del núcleo; Kotlin sólo lo muestra. Sin actor: es una lectura.
    pub fn aviso_proveedor_con_ingreso_activo(
        &self,
        cedula: String,
    ) -> Result<Option<String>, NucleoError> {
        let activo = self
            .core_lock()
            .proveedor_con_ingreso_activo_en_sitio(&cedula)?;
        Ok(activo.then(|| {
            control_acceso::mensajes::mensaje_ingreso_proveedor(
                IngresoProveedorServiceErrorNucleo::IngresoActivo,
            )
        }))
    }

    /// Registra la salida (cierre) de un ingreso de proveedor activo --
    /// espejo de `AppCore::registrar_salida_proveedor`.
    pub fn registrar_salida_proveedor(&self, registro_id: i64) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self
            .core_lock()
            .registrar_salida_proveedor(&actor, registro_id)?)
    }

    /// Sin actor -- es una lectura, mismo criterio que `listar_rutas_activas`.
    pub fn listar_proveedores_activos(
        &self,
    ) -> Result<Vec<RegistroIngresoProveedorActivoResumen>, NucleoError> {
        Ok(self
            .core_lock()
            .listar_proveedores_activos()?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}
