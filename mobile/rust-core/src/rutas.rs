//! Salidas y retornos de rutas, con sus catálogos de encargados, vehículos y rutas.

use control_acceso::services::ruta_service::SolicitudSalidaRuta as SolicitudSalidaRutaNucleo;

use crate::{
    EncargadoRuta, Nucleo, NucleoError, ResultadoRegistroSalidaRuta, Ruta, SalidaRutaActivaResumen,
    SolicitudSalidaRuta, VehiculoRuta, interno,
};

#[uniffi::export]
impl Nucleo {
    /// Buscador del checklist de rutas (paso "Encargado KOF") -- por nombre
    /// o código de empleado, mismo criterio que `buscar_contratistas`
    /// ("busca por nombre o por número de cédula", pedido explícito del
    /// usuario, 2026-09-15). Tope acotado dentro del núcleo
    /// (`EncargadoRutaRepository::buscar`), no hace falta repetirlo acá.
    pub fn buscar_encargados_ruta(&self, texto: String) -> Result<Vec<EncargadoRuta>, NucleoError> {
        Ok(self
            .core_lock()
            // Es un selector (checklist de rutas/gafete provisional) -- un
            // encargado desactivado no es una opción válida, ver
            // `EncargadoRutaService::buscar_seleccionables`.
            .buscar_encargados_ruta_seleccionables(texto.trim())
            .map_err(|origen| NucleoError::Interno {
                mensaje: interno(origen),
            })?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Buscador del checklist de rutas (paso "Vehículo") -- por placa o
    /// número de unidad, mismo criterio que `buscar_encargados_ruta`.
    /// Reemplaza los dos campos de texto libre que tenía antes ese paso del
    /// lado Kotlin (pedido explícito del usuario, 2026-09-19): ahora es un
    /// buscador contra este catálogo, no texto arbitrario.
    pub fn buscar_vehiculos_ruta(&self, texto: String) -> Result<Vec<VehiculoRuta>, NucleoError> {
        Ok(self
            .core_lock()
            .buscar_vehiculos_ruta_seleccionables(texto.trim())
            .map_err(|origen| NucleoError::Interno {
                mensaje: interno(origen),
            })?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Buscador del checklist de rutas (paso "Documento de ruta") -- el
    /// número de ruta es bloqueante (debe existir en el catálogo, pedido
    /// explícito del usuario, 2026-09-15), así que el checklist confirma
    /// contra este buscador antes de registrar la salida, en vez de
    /// enterarse recién al fallar `registrar_salida_ruta`.
    pub fn buscar_rutas(&self, texto: String) -> Result<Vec<Ruta>, NucleoError> {
        Ok(self
            .core_lock()
            // Es un selector -- una ruta dada de baja no es una opción
            // válida para una salida nueva, ver
            // `RutaCatalogoService::buscar_seleccionables`.
            .buscar_rutas_seleccionables(texto.trim())
            .map_err(|origen| NucleoError::Interno {
                mensaje: interno(origen),
            })?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    /// Registra la salida (apertura) del ciclo de una ruta -- espejo de
    /// `AppCore::registrar_salida_ruta`. `solicitud.fecha_documento` viaja
    /// como texto ISO (`AAAA-MM-DD`), mismo criterio que
    /// `fecha_vencimiento_praind` en `crear_contratista`.
    pub fn registrar_salida_ruta(
        &self,
        solicitud: SolicitudSalidaRuta,
    ) -> Result<ResultadoRegistroSalidaRuta, NucleoError> {
        let actor = self.actor_autenticado()?;
        let fecha_documento =
            solicitud
                .fecha_documento
                .parse()
                .map_err(|_| NucleoError::FechaInvalida {
                    mensaje: solicitud.fecha_documento.clone(),
                })?;
        Ok(self
            .core_lock()
            .registrar_salida_ruta(
                &actor,
                SolicitudSalidaRutaNucleo {
                    vehiculo_placa: solicitud.vehiculo_placa,
                    vehiculo_numero_unidad: solicitud.vehiculo_numero_unidad,
                    encargado_nombre: solicitud.encargado_nombre,
                    encargado_codigo_empleado: solicitud.encargado_codigo_empleado,
                    numero_ruta: solicitud.numero_ruta,
                    sub_numero: solicitud.sub_numero,
                    numero_documento: solicitud.numero_documento,
                    fecha_documento,
                    tiene_correo_autorizacion: solicitud.tiene_correo_autorizacion,
                    // Ignorados por `AppCore::registrar_salida_ruta` -- se
                    // pisan con el actor/reloj reales de la transacción.
                    usuario_salida_id: 0,
                    fecha_hora_salida: chrono::Utc::now(),
                },
            )?
            .into())
    }

    /// Registra el retorno (cierre) de una salida de ruta activa --
    /// espejo de `AppCore::registrar_retorno_ruta`.
    pub fn registrar_retorno_ruta(&self, salida_id: i64) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        Ok(self.core_lock().registrar_retorno_ruta(&actor, salida_id)?)
    }

    /// Sin actor -- es una lectura, mismo criterio que
    /// `listar_ingresos_activos`.
    pub fn listar_rutas_activas(&self) -> Result<Vec<SalidaRutaActivaResumen>, NucleoError> {
        Ok(self
            .core_lock()
            .listar_rutas_activas()?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}
