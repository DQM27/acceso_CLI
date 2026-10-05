//! Visitas agendadas en la portería: sólo verificar la cédula y registrar
//! la entrada o la salida, sin listas ni historial (el historial se consulta
//! en escritorio y en el panel web). Mismo molde que `correo.rs`: las reglas
//! las decide el núcleo, acá sólo se traducen para Kotlin.

use control_acceso::mensajes::{mensaje_cita, mensaje_entrada_visita_verificada};
use control_acceso::models::movimiento_visita::MovimientoVisitaActivoResumen;
use control_acceso::services::error::CitaServiceError;

use crate::{Nucleo, NucleoError};

/// Visita que vale hoy: lo que la persona de la portería confirma antes de
/// darle el gafete.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VisitaParaEntrar {
    /// En su forma única, la que se manda al registrar.
    pub cedula: String,
    pub nombre: String,
    pub empresa: Option<String>,
    pub anfitrion: String,
    pub motivo: Option<String>,
    /// Último día de la cita (`AAAA-MM-DD`): igual a hoy si es de un día.
    pub fecha_hasta: String,
    /// Hora estimada de llegada ("HH:MM"), sólo informativa.
    pub hora_estimada: Option<String>,
    /// La placa que dejó anotada el anfitrión, para proponer "vehículo".
    pub placa_sugerida: Option<String>,
}

/// Visita abierta en este equipo: la persona está adentro y se le ofrece la
/// salida.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VisitaAdentro {
    pub movimiento_id: i64,
    pub cedula: String,
    pub nombre: String,
    pub anfitrion: String,
    pub gafete_numero: Option<i64>,
    pub placa: Option<String>,
    /// RFC 3339.
    pub fecha_hora_entrada: String,
}

impl From<MovimientoVisitaActivoResumen> for VisitaAdentro {
    fn from(movimiento: MovimientoVisitaActivoResumen) -> Self {
        Self {
            movimiento_id: movimiento.id,
            cedula: movimiento.cedula,
            nombre: movimiento.nombre,
            anfitrion: movimiento.anfitrion_nombre,
            gafete_numero: movimiento.gafete_numero,
            placa: movimiento.placa,
            fecha_hora_entrada: movimiento.fecha_hora_entrada.to_rfc3339(),
        }
    }
}

/// Qué sigue con una cédula.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum VerificacionVisita {
    /// Tiene una cita que vale hoy: registrar la entrada.
    Entrada { visita: VisitaParaEntrar },
    /// Ya está adentro (en este equipo): registrar la salida.
    Salida { visita: VisitaAdentro },
    /// No puede entrar. `informativo`: la cita existe pero es para otro día
    /// (se muestra como aviso, no como error).
    Aviso { mensaje: String, informativo: bool },
}

fn rechazo(error: CitaServiceError) -> NucleoError {
    NucleoError::Rechazado {
        mensaje: mensaje_cita(error),
    }
}

#[uniffi::export]
impl Nucleo {
    /// Una sola llamada por cédula (escrita o escaneada). Primero mira si ya
    /// está adentro en este equipo, después la cita (`verificar_check_in`, la
    /// misma regla que escritorio). Sólo lectura local, sin red: es lo que
    /// se ve al instante; los chequeos contra la nube corren al registrar.
    pub fn verificar_visita(&self, cedula: String) -> Result<VerificacionVisita, NucleoError> {
        let nucleo = self.core_lock();
        let activa =
            nucleo
                .visita_activa_por_cedula(&cedula)
                .map_err(|error| NucleoError::Interno {
                    mensaje: crate::error::interno(error),
                })?;
        if let Some(adentro) = activa {
            return Ok(VerificacionVisita::Salida {
                visita: adentro.into(),
            });
        }
        Ok(match nucleo.verificar_check_in_visita(&cedula) {
            Ok((cita, visitante)) => VerificacionVisita::Entrada {
                visita: VisitaParaEntrar {
                    cedula: visitante.cedula,
                    nombre: visitante.nombre,
                    empresa: visitante.empresa,
                    anfitrion: cita.anfitrion_nombre,
                    motivo: cita.motivo,
                    fecha_hasta: cita.fecha_hasta.to_string(),
                    hora_estimada: cita.hora_estimada,
                    placa_sugerida: visitante.placa_vehiculo,
                },
            },
            Err(error @ (CitaServiceError::Database(_) | CitaServiceError::RelojRetrocedido)) => {
                return Err(rechazo(error));
            }
            Err(error) => VerificacionVisita::Aviso {
                informativo: error.es_informativo(),
                mensaje: mensaje_cita(error),
            },
        })
    }

    /// Entrada con todas sus reglas (`application::registrar_entrada_visita_verificada`,
    /// la misma que usa escritorio). `placa`: `None` = caminando.
    pub fn registrar_entrada_visita(
        &self,
        cedula: String,
        gafete_numero: Option<i64>,
        placa: Option<String>,
    ) -> Result<i64, NucleoError> {
        let actor = self.actor_autenticado()?;
        control_acceso::application::registrar_entrada_visita_verificada(
            || self.core_lock(),
            &self.cache_token,
            &actor,
            &cedula,
            gafete_numero,
            placa,
        )
        .map_err(|error| NucleoError::Rechazado {
            mensaje: mensaje_entrada_visita_verificada(error),
        })
    }

    pub fn registrar_salida_visita(&self, movimiento_id: i64) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        self.core_lock()
            .registrar_salida_visita(&actor, movimiento_id)
            .map_err(rechazo)
    }
}
