//! Reconexión de un equipo retirado (ver
//! docs/features-futuras/plan-reconexion-equipo-retirado.md). El núcleo
//! decide todo: cuándo hay botón, cuándo hay que confirmar la pérdida de
//! registros y qué pasa con la base. Las pantallas sólo dibujan el
//! resultado.
//!
//! Como en `con_nube`, la red corre sin el candado del núcleo: quien llama
//! pasa `nucleo` (una función que lo toma) y la caché del token, y cada
//! paso local lo toma y lo suelta.

use std::ops::Deref;

use super::AppCore;
use crate::database::error::DatabaseError;
use crate::database::queries::estado_equipo;
use crate::nube::{CacheTokenDispositivo, DetalleReconexion, MetadatosDispositivo, NubeError};

/// Lo que la persona confirmó en la pantalla al ver el aviso de pérdida.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfirmacionReconexion {
    /// Los registros sin subir que vio en el aviso.
    pub pendientes: u64,
    /// Quién tiene la sesión abierta en el equipo, si hay.
    pub confirmado_por: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoReconexion {
    /// Misma unidad: no se borró nada. Conviene sincronizar ya, para que
    /// suba lo pendiente.
    Reconectado { unidad: String },
    /// Va a otra unidad y hay registros que nunca subieron: hace falta que
    /// la persona confirme que se pierden. El código NO se gastó.
    NecesitaConfirmacion { pendientes: u64, unidad: String },
    /// Otra unidad: esta base quedó marcada para descartarse. La app tiene
    /// que reiniciarse; al arrancar la borra y baja la de la unidad nueva.
    ReconectadoEnOtraUnidad { unidad: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ReconexionError {
    /// Sólo un equipo que la nube ya no reconoce se reconecta.
    #[error("Este equipo no está retirado")]
    NoRetirado,
    #[error(transparent)]
    Nube(#[from] NubeError),
    #[error(transparent)]
    Base(#[from] DatabaseError),
}

impl AppCore {
    /// `true` si la nube dijo que no reconoce la clave de este equipo (lo
    /// retiraron en el panel): es cuando aparece "Reconectar este equipo".
    pub fn equipo_retirado(&self) -> bool {
        estado_equipo::equipo_retirado(&self.connection).unwrap_or(false)
    }

    /// Lo que las plataformas llaman cuando falla la autenticación del
    /// equipo: si la nube dijo que no reconoce su clave, queda anotado como
    /// retirado. Cualquier otro error (sin red, versión vieja) no cambia
    /// nada. Sólo la reconexión borra la marca.
    pub fn anotar_rechazo_de_la_nube(&self, error: &NubeError) {
        if matches!(error, NubeError::CredencialesInvalidas)
            && let Err(error) = estado_equipo::marcar_retirado(&self.connection, true)
        {
            log::warn!("no se pudo anotar que este equipo fue retirado: {error}");
        }
    }

    /// `true` si esta base es de una unidad anterior: al arrancar, la app la
    /// borra antes de usarla (ver `database::connection::borrar_archivos_de_base`).
    pub fn base_por_descartar(&self) -> bool {
        estado_equipo::base_por_descartar(&self.connection).unwrap_or(false)
    }

    /// Registros de esta base que todavía no llegaron a la nube.
    pub fn registros_sin_subir(&self) -> Result<u64, DatabaseError> {
        Ok(crate::database::cola_salida::sin_subir(&self.connection)?)
    }
}

/// Qué sigue después de consultar el código.
#[derive(Debug, PartialEq, Eq)]
enum Paso {
    PedirConfirmacion,
    Canjear { descartar_base: bool },
}

/// Al cambiar de unidad con registros sin subir hace falta una confirmación
/// que los cubra a TODOS los que hay ahora: si entre el aviso y la
/// confirmación se registró algo más, se vuelve a preguntar. En la misma
/// unidad no se pierde nada y no se pregunta.
fn siguiente_paso(
    cambio_de_unidad: bool,
    pendientes: u64,
    confirmacion: Option<&ConfirmacionReconexion>,
) -> Paso {
    let cubierto = pendientes == 0 || confirmacion.is_some_and(|c| c.pendientes >= pendientes);
    if cambio_de_unidad && !cubierto {
        Paso::PedirConfirmacion
    } else {
        Paso::Canjear {
            descartar_base: cambio_de_unidad,
        }
    }
}

/// Reconecta este equipo con el código de reconexión que dio el panel.
///
/// 1. Sólo si el equipo está retirado.
/// 2. Consulta a dónde va, sin gastar el código.
/// 3. Si cambia de unidad y hay registros sin subir, devuelve
///    [`ResultadoReconexion::NecesitaConfirmacion`] hasta que llegue una
///    `confirmacion` que cubra todos los que hay ahora.
/// 4. Si cambia de unidad, marca la base para descartar ANTES del canje:
///    desde ese momento no se sincroniza, así que nada de la unidad vieja
///    puede subir a la nueva. Si el canje falla, la marca se quita.
/// 5. Canjea con una clave nueva y aplica el token.
pub fn reconectar_equipo<G: Deref<Target = AppCore>>(
    nucleo: impl Fn() -> G,
    nube: &CacheTokenDispositivo,
    codigo: &str,
    confirmacion: Option<&ConfirmacionReconexion>,
    metadata: Option<&MetadatosDispositivo>,
) -> Result<ResultadoReconexion, ReconexionError> {
    if !nucleo().equipo_retirado() {
        return Err(ReconexionError::NoRetirado);
    }

    let destino = nube.consultar_reconexion(codigo)?;
    let pendientes = if destino.cambio_de_unidad {
        nucleo().registros_sin_subir()?
    } else {
        0
    };
    match siguiente_paso(destino.cambio_de_unidad, pendientes, confirmacion) {
        Paso::PedirConfirmacion => {
            return Ok(ResultadoReconexion::NecesitaConfirmacion {
                pendientes,
                unidad: destino.sitio_nombre,
            });
        }
        Paso::Canjear { descartar_base } => {
            if descartar_base {
                estado_equipo::marcar_base_por_descartar(&nucleo().connection, true)?;
            }
        }
    }
    let detalle = DetalleReconexion {
        pendientes_descartados: pendientes,
        confirmado_por: confirmacion.and_then(|c| c.confirmado_por.clone()),
    };
    let (token, cambio_de_unidad) = match nube.reconectar(codigo, metadata, &detalle) {
        Ok(reconectado) => reconectado,
        Err(error) => {
            if destino.cambio_de_unidad
                && let Err(error) =
                    estado_equipo::marcar_base_por_descartar(&nucleo().connection, false)
            {
                log::error!("no se pudo quitar la marca de base por descartar: {error}");
            }
            return Err(error.into());
        }
    };

    let core = nucleo();
    // Lo que manda es lo que respondió el canje (la consulta fue un instante
    // antes; un administrador pudo emitir otro código en el medio).
    estado_equipo::marcar_base_por_descartar(&core.connection, cambio_de_unidad)?;
    estado_equipo::marcar_retirado(&core.connection, false)?;
    core.aplicar_token(&token);
    let unidad = token.sitio_nombre.unwrap_or(destino.sitio_nombre);
    Ok(if cambio_de_unidad {
        ResultadoReconexion::ReconectadoEnOtraUnidad { unidad }
    } else {
        ResultadoReconexion::Reconectado { unidad }
    })
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::database::schema::initialize_database;

    fn nucleo() -> AppCore {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        AppCore::new(connection)
    }

    fn confirmo(pendientes: u64) -> ConfirmacionReconexion {
        ConfirmacionReconexion {
            pendientes,
            confirmado_por: None,
        }
    }

    #[test]
    fn misma_unidad_canjea_sin_preguntar_ni_descartar() {
        assert_eq!(
            siguiente_paso(false, 0, None),
            Paso::Canjear {
                descartar_base: false
            }
        );
    }

    #[test]
    fn otra_unidad_sin_pendientes_canjea_y_descarta_sin_preguntar() {
        assert_eq!(
            siguiente_paso(true, 0, None),
            Paso::Canjear {
                descartar_base: true
            }
        );
    }

    #[test]
    fn otra_unidad_con_pendientes_pide_una_confirmacion_que_los_cubra_todos() {
        assert_eq!(siguiente_paso(true, 3, None), Paso::PedirConfirmacion);
        assert_eq!(
            siguiente_paso(true, 3, Some(&confirmo(2))),
            Paso::PedirConfirmacion,
            "se registró uno más después del aviso"
        );
        assert_eq!(
            siguiente_paso(true, 3, Some(&confirmo(3))),
            Paso::Canjear {
                descartar_base: true
            }
        );
    }

    #[test]
    fn solo_el_rechazo_de_la_clave_marca_el_equipo_como_retirado() {
        let core = nucleo();
        assert!(!core.equipo_retirado());

        core.anotar_rechazo_de_la_nube(&NubeError::SinCredencial);
        core.anotar_rechazo_de_la_nube(&NubeError::VersionDesactualizada);
        assert!(!core.equipo_retirado());

        core.anotar_rechazo_de_la_nube(&NubeError::CredencialesInvalidas);
        assert!(core.equipo_retirado());
    }

    #[test]
    fn un_equipo_que_no_esta_retirado_no_se_reconecta_ni_toca_la_red() {
        let core = nucleo();
        // Sin firmante: si intentara hablar con la nube fallaría con
        // `SinCredencial`, no con `NoRetirado`.
        let nube = CacheTokenDispositivo::new();

        let resultado = reconectar_equipo(|| &core, &nube, "K7QMR4XT2P", None, None);

        assert!(matches!(resultado, Err(ReconexionError::NoRetirado)));
        assert!(!core.base_por_descartar());
    }

    #[test]
    fn cuenta_los_registros_sin_subir() {
        let core = nucleo();
        assert_eq!(core.registros_sin_subir().unwrap(), 0);
        crate::database::cola_salida::encolar(&core.connection, "ingreso", "a", "crear").unwrap();
        assert_eq!(core.registros_sin_subir().unwrap(), 1);
    }
}
