//! Visitas agendadas en la portería: verificar la cédula, registrar la
//! entrada o la salida, y la lista de quién está adentro en la unidad (este
//! equipo y el otro), igual que contratistas. Sin historial (se consulta en
//! escritorio y en el panel web). Las reglas las decide el núcleo; acá sólo
//! se traducen para Kotlin.

use control_acceso::mensajes::{mensaje_cita, mensaje_entrada_visita_verificada, vigencia_cita};
use control_acceso::models::movimiento_visita::MovimientoVisitaActivoResumen;
use control_acceso::nube::MovimientoVisitaRemoto;
use control_acceso::services::error::CitaServiceError;

use crate::error::interno;
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
    /// "Sólo hoy" o "Hasta el jueves 8 de octubre", calculado con el reloj
    /// del núcleo (`mensajes::vigencia_cita`).
    pub vigencia: String,
    /// Hora estimada de llegada ("HH:MM"), sólo informativa.
    pub hora_estimada: Option<String>,
    /// La placa que dejó anotada el anfitrión, para proponer "vehículo".
    pub placa_sugerida: Option<String>,
}

/// Dónde se registró la entrada: decide cómo se da la salida. Kotlin lo
/// devuelve tal cual a [`Nucleo::registrar_salida_visita`].
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum OrigenVisita {
    /// En este equipo.
    EsteEquipo { movimiento_id: i64 },
    /// En el otro equipo de la unidad (la PC): la salida va a la nube.
    OtroEquipo { uuid: String },
}

/// Visita que está adentro: una tarjeta de la lista, o lo que se ofrece al
/// verificar su cédula.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VisitaAdentro {
    pub origen: OrigenVisita,
    pub cedula: String,
    pub nombre: String,
    pub empresa: Option<String>,
    pub anfitrion: String,
    pub gafete_numero: Option<i64>,
    pub placa: Option<String>,
    /// RFC 3339.
    pub fecha_hora_entrada: String,
    pub usuario_entrada_nombre: String,
}

impl From<MovimientoVisitaActivoResumen> for VisitaAdentro {
    fn from(movimiento: MovimientoVisitaActivoResumen) -> Self {
        Self {
            origen: OrigenVisita::EsteEquipo {
                movimiento_id: movimiento.id,
            },
            cedula: movimiento.cedula,
            nombre: movimiento.nombre,
            empresa: movimiento.empresa,
            anfitrion: movimiento.anfitrion_nombre,
            gafete_numero: movimiento.gafete_numero,
            placa: movimiento.placa,
            fecha_hora_entrada: movimiento.fecha_hora_entrada.to_rfc3339(),
            usuario_entrada_nombre: movimiento.usuario_entrada_nombre,
        }
    }
}

impl From<MovimientoVisitaRemoto> for VisitaAdentro {
    fn from(remoto: MovimientoVisitaRemoto) -> Self {
        Self {
            origen: OrigenVisita::OtroEquipo { uuid: remoto.uuid },
            cedula: remoto.cedula,
            nombre: remoto.nombre,
            empresa: remoto.empresa,
            anfitrion: remoto.anfitrion_nombre.unwrap_or_default(),
            gafete_numero: remoto.gafete_numero,
            placa: remoto.placa,
            fecha_hora_entrada: remoto.hora_entrada,
            usuario_entrada_nombre: remoto.usuario_entrada_nombre.unwrap_or_default(),
        }
    }
}

/// Qué sigue con una cédula.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum VerificacionVisita {
    /// Tiene una cita que vale hoy: registrar la entrada.
    Entrada { visita: VisitaParaEntrar },
    /// Ya está adentro, en este equipo o en el otro: registrar la salida.
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
    /// está adentro (en este equipo o, por la caché, en el otro), después la
    /// cita (`verificar_check_in`, la misma regla que escritorio). Sólo
    /// lectura local, sin red: es lo que se ve al instante; los chequeos
    /// contra la nube corren al registrar.
    pub fn verificar_visita(&self, cedula: String) -> Result<VerificacionVisita, NucleoError> {
        let nucleo = self.core_lock();
        let interno = |error| NucleoError::Interno {
            mensaje: interno(error),
        };
        if let Some(adentro) = nucleo.visita_activa_por_cedula(&cedula).map_err(interno)? {
            return Ok(VerificacionVisita::Salida {
                visita: adentro.into(),
            });
        }
        if let Some(remota) = nucleo.visita_remota_por_cedula(&cedula).map_err(interno)? {
            return Ok(VerificacionVisita::Salida {
                visita: remota.into(),
            });
        }
        let hoy = control_acceso::tiempo::fecha_costa_rica(self.reloj.ahora_utc());
        Ok(match nucleo.verificar_check_in_visita(&cedula) {
            Ok((cita, visitante)) => VerificacionVisita::Entrada {
                visita: VisitaParaEntrar {
                    cedula: visitante.cedula,
                    nombre: visitante.nombre,
                    empresa: visitante.empresa,
                    anfitrion: cita.anfitrion_nombre,
                    motivo: cita.motivo,
                    vigencia: vigencia_cita(cita.fecha_hasta, hoy),
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

    /// Quién está adentro por visita en la unidad: primero las de este
    /// equipo, después las del otro (caché que llena la sincronización y
    /// refresca el aviso en vivo). Sin nube vinculada, sólo las de acá.
    pub fn listar_visitas_adentro(&self) -> Result<Vec<VisitaAdentro>, NucleoError> {
        let actor = self.actor_autenticado()?;
        let (locales, remotas) = {
            let nucleo = self.core_lock();
            // Sin permiso de nube (o sin vincular) la caché no aplica: mismo
            // criterio que la lista de contratistas.
            (
                nucleo.listar_visitas_activas(),
                nucleo.listar_visitas_remotas(&actor).unwrap_or_default(),
            )
        };
        let mut visitas: Vec<VisitaAdentro> = locales
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?
            .into_iter()
            .map(Into::into)
            .collect();
        visitas.extend(remotas.into_iter().map(Into::into));
        Ok(visitas)
    }

    /// Salida de una visita adentro, según dónde entró: en este equipo se
    /// cierra local (y la cola la sube); en el otro, se cierra en la nube
    /// (el candado del núcleo sólo se toma para autorizar, nunca durante la
    /// red).
    pub fn registrar_salida_visita(&self, origen: OrigenVisita) -> Result<(), NucleoError> {
        let actor = self.actor_autenticado()?;
        let uuid = match origen {
            OrigenVisita::EsteEquipo { movimiento_id } => {
                return self
                    .core_lock()
                    .registrar_salida_visita(&actor, movimiento_id)
                    .map_err(rechazo);
            }
            OrigenVisita::OtroEquipo { uuid } => uuid,
        };
        self.core_lock().autorizar_uso_nube(&actor)?;
        let token = self
            .autenticar_con_cache()
            .map_err(|error| NucleoError::Interno {
                mensaje: interno(error),
            })?;
        let contexto = control_acceso::nube::ContextoSincronizacion {
            base_url: control_acceso::nube::base_url(),
            apikey: control_acceso::nube::apikey(),
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        let conexion = self.conexion_secundaria()?;
        let hora = self.core_lock().ahora_utc();
        control_acceso::nube::cerrar_movimiento_visita_remoto(
            &conexion,
            &contexto,
            &uuid,
            &actor.nombre,
            hora,
        )
        .map_err(|error| NucleoError::Interno {
            mensaje: interno(error),
        })
    }
}
