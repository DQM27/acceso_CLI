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
    /// `true` si la última respuesta con autoridad de la nube fue que este
    /// equipo está retirado: es cuando aparece "Reconectar este equipo".
    pub fn equipo_retirado(&self) -> bool {
        estado_equipo::equipo_retirado(&self.connection).unwrap_or(false)
    }

    /// Lo que las plataformas llaman después de CADA autenticación del
    /// equipo. La marca de retirado no se decide acá ni se deja pegada:
    /// refleja lo último que la nube dijo con autoridad.
    ///
    /// - `Ok`: la nube aceptó al equipo, así que no está retirado (se borra
    ///   la marca si quedó de antes).
    /// - [`NubeError::EquipoRetirado`]: la nube lo confirmó con prueba
    ///   (`403 dispositivo_retirado`), se marca.
    /// - Cualquier otro error (sin red, nube caída, `401`, versión vieja):
    ///   no dice nada sobre el retiro, no cambia nada.
    ///
    /// Incidente del 2026-10-26: un `401` pasajero de la nube marcaba al
    /// equipo y sólo una reconexión borraba la marca, aunque la siguiente
    /// autenticación ya hubiera funcionado.
    pub fn anotar_autenticacion<T>(&self, resultado: &Result<T, NubeError>) {
        let retirado = match resultado {
            Ok(_) => false,
            Err(NubeError::EquipoRetirado) => true,
            Err(_) => return,
        };
        if self.equipo_retirado() == retirado {
            return;
        }
        if let Err(error) = estado_equipo::marcar_retirado(&self.connection, retirado) {
            log::warn!("no se pudo anotar si este equipo está retirado: {error}");
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

    fn fallo(error: NubeError) -> Result<(), NubeError> {
        Err(error)
    }

    /// Todos los errores que NO son un retiro confirmado.
    fn errores_que_no_son_retiro() -> Vec<NubeError> {
        vec![
            NubeError::CredencialesInvalidas,
            NubeError::SinCredencial,
            NubeError::VersionDesactualizada,
            NubeError::CodigoVinculacionInvalido,
            NubeError::ClaveEnUso,
            NubeError::Firmante(crate::nube::ErrorFirmante::Almacen("keystore".into())),
        ]
    }

    #[test]
    fn solo_un_retiro_confirmado_por_la_nube_marca_el_equipo() {
        let core = nucleo();
        for error in errores_que_no_son_retiro() {
            let descripcion = format!("{error:?}");
            core.anotar_autenticacion(&fallo(error));
            assert!(!core.equipo_retirado(), "{descripcion} no es un retiro");
        }

        core.anotar_autenticacion(&fallo(NubeError::EquipoRetirado));
        assert!(core.equipo_retirado());
    }

    /// El incidente del 2026-10-26, paso a paso: un `401` pasajero y, un
    /// segundo después, una autenticación que funciona.
    #[test]
    fn incidente_un_401_pasajero_no_marca_el_equipo_como_retirado() {
        let core = nucleo();

        core.anotar_autenticacion(&fallo(NubeError::CredencialesInvalidas));
        assert!(!core.equipo_retirado(), "un 401 no es un retiro");

        core.anotar_autenticacion(&Ok(()));
        assert!(!core.equipo_retirado());
    }

    /// Un equipo que quedó marcado por la versión anterior (v1.7.0) se
    /// arregla solo con la primera autenticación que funcione, sin
    /// reconectar.
    #[test]
    fn una_autenticacion_exitosa_borra_una_marca_de_retirado_que_quedo_de_antes() {
        let core = nucleo();
        estado_equipo::marcar_retirado(&core.connection, true).unwrap();
        assert!(core.equipo_retirado());

        core.anotar_autenticacion(&Ok(()));

        assert!(!core.equipo_retirado());
    }

    /// Con el equipo retirado de verdad, perder la red (o que la nube no
    /// responda) no lo "des-retira": sólo una respuesta con autoridad cambia
    /// la marca.
    #[test]
    fn un_error_que_no_es_retiro_tampoco_borra_un_retiro_confirmado() {
        let core = nucleo();
        core.anotar_autenticacion(&fallo(NubeError::EquipoRetirado));

        for error in errores_que_no_son_retiro() {
            let descripcion = format!("{error:?}");
            core.anotar_autenticacion(&fallo(error));
            assert!(core.equipo_retirado(), "{descripcion} no cambia el retiro");
        }
    }

    /// Retirado, reconectado desde el panel sin pasar por la app (por
    /// ejemplo, el retiro se deshizo por SQL): la nube vuelve a aceptarlo y
    /// la franja se va sola.
    #[test]
    fn la_marca_sigue_a_la_nube_en_los_dos_sentidos() {
        let core = nucleo();
        let respuestas = [
            (fallo(NubeError::EquipoRetirado), true),
            (fallo(NubeError::CredencialesInvalidas), true),
            (Ok(()), false),
            (fallo(NubeError::SinCredencial), false),
            (fallo(NubeError::EquipoRetirado), true),
            (Ok(()), false),
        ];
        for (indice, (respuesta, esperado)) in respuestas.iter().enumerate() {
            core.anotar_autenticacion(respuesta);
            assert_eq!(core.equipo_retirado(), *esperado, "paso {indice}");
        }
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
