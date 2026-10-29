//! Quien está adentro por otra vía (contratista, proveedor o por correo):
//! en vez de un "registre primero esa salida" sin nada que hacer, la
//! pantalla que lo frena ofrece cerrarla ahí mismo y seguir (pedido del
//! usuario 2026-10-29). El botón es la confirmación: la pantalla ya muestra
//! a quién se saca y por qué vía.
//!
//! Sólo ve lo de este teléfono y lo del otro equipo de la misma unidad (las
//! cachés `*_remotos`): lo de otra unidad operativa no está acá y se sigue
//! frenando al registrar, sin botón.

use control_acceso::database::queries::persona_adentro::{
    IngresoPorOtraVia as IngresoPorOtraViaNucleo, UbicacionIngreso as UbicacionIngresoNucleo,
};
use control_acceso::models::via_ingreso::ViaIngreso as ViaIngresoNucleo;

use crate::{Nucleo, NucleoError, interno};

/// Las tres vías de ingreso de una persona.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ViaIngreso {
    Contratista,
    Proveedor,
    PorCorreo,
}

impl From<ViaIngreso> for ViaIngresoNucleo {
    fn from(via: ViaIngreso) -> Self {
        match via {
            ViaIngreso::Contratista => Self::Contratista,
            ViaIngreso::Proveedor => Self::Proveedor,
            ViaIngreso::PorCorreo => Self::PorCorreo,
        }
    }
}

impl From<ViaIngresoNucleo> for ViaIngreso {
    fn from(via: ViaIngresoNucleo) -> Self {
        match via {
            ViaIngresoNucleo::Contratista => Self::Contratista,
            ViaIngresoNucleo::Proveedor => Self::Proveedor,
            ViaIngresoNucleo::PorCorreo => Self::PorCorreo,
        }
    }
}

/// Dónde está abierto el ingreso: en este teléfono o en el otro equipo de la
/// unidad. Kotlin no lo mira: lo devuelve tal cual a
/// [`Nucleo::registrar_salida_por_otra_via`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum UbicacionSalida {
    Local { registro_id: i64 },
    Remota { uuid: String },
}

/// El ingreso por otra vía, listo para mostrar y cerrar.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SalidaPorOtraVia {
    pub via: ViaIngreso,
    /// "Ya está adentro como proveedor".
    pub titulo: String,
    pub nombre: String,
    pub gafete_numero: Option<i64>,
    /// Como viene de la base (ISO); la hora la arma Kotlin igual que en las
    /// listas.
    pub fecha_hora_ingreso: String,
    pub ubicacion: UbicacionSalida,
}

impl From<IngresoPorOtraViaNucleo> for SalidaPorOtraVia {
    fn from(ingreso: IngresoPorOtraViaNucleo) -> Self {
        Self {
            via: ingreso.via.into(),
            titulo: format!("Ya está adentro {}", ingreso.via.texto()),
            nombre: ingreso.nombre,
            gafete_numero: ingreso.gafete_numero,
            fecha_hora_ingreso: ingreso.fecha_hora_ingreso,
            ubicacion: match ingreso.ubicacion {
                UbicacionIngresoNucleo::Local(registro_id) => UbicacionSalida::Local { registro_id },
                UbicacionIngresoNucleo::Remoto(uuid) => UbicacionSalida::Remota { uuid },
            },
        }
    }
}

#[uniffi::export]
impl Nucleo {
    /// El ingreso por el que esta cédula está adentro por una vía distinta de
    /// `propia` (la de la pantalla que pregunta), o `None`. Sólo local, sin
    /// red. Sin actor: es una lectura.
    pub fn salida_por_otra_via(
        &self,
        cedula: String,
        propia: ViaIngreso,
    ) -> Result<Option<SalidaPorOtraVia>, NucleoError> {
        let ingreso = self
            .core_lock()
            .ingreso_por_otra_via(&cedula, propia.into())
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?;
        Ok(ingreso.map(Into::into))
    }

    /// Cierra ese ingreso con la misma salida de siempre de su vía: la local
    /// (queda en el historial y se sincroniza) o, si es del otro equipo,
    /// contra la nube.
    pub fn registrar_salida_por_otra_via(
        &self,
        salida: SalidaPorOtraVia,
    ) -> Result<(), NucleoError> {
        match (salida.via, salida.ubicacion) {
            (ViaIngreso::Contratista, UbicacionSalida::Local { registro_id }) => {
                self.registrar_salida(registro_id)
            }
            (ViaIngreso::Proveedor, UbicacionSalida::Local { registro_id }) => {
                self.registrar_salida_proveedor(registro_id)
            }
            (ViaIngreso::PorCorreo, UbicacionSalida::Local { registro_id }) => {
                self.registrar_salida_correo(registro_id)
            }
            (ViaIngreso::Contratista, UbicacionSalida::Remota { uuid }) => {
                self.cerrar_ingreso_remoto(uuid)
            }
            (ViaIngreso::Proveedor, UbicacionSalida::Remota { uuid }) => {
                self.cerrar_ingreso_proveedor_remoto(uuid)
            }
            (ViaIngreso::PorCorreo, UbicacionSalida::Remota { uuid }) => {
                self.cerrar_ingreso_correo_remoto(uuid)
            }
        }
    }
}
