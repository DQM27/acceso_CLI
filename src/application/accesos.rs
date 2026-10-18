//! Preparar/registrar ingreso, registrar salida, y el listado de ingresos activos.

use rusqlite::Connection;

use crate::database::queries::ingresos::{FiltroIngresosActivos, SqliteIngresosQuery};
use crate::database::repositories::contratista_repository::SqliteContratistaRepository;
use crate::database::repositories::empresa_repository::SqliteEmpresaRepository;
use crate::database::repositories::gafete_repository::SqliteGafeteRepository;
use crate::database::repositories::registro_ingreso_repository::SqliteRegistroIngresoRepository;
use crate::domain::permanencia::{
    NivelPermanencia, ResumenPermanencia, nivel_de_permanencia, resumir_permanencia,
};
use crate::domain::resultado_acceso::ResultadoAcceso;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::RegistroIngresoServiceError;
use crate::services::registro_ingreso_service::{
    ListaIngresosActivosResumen, PreparacionIngreso, RegistroIngresoConsultaService,
    RegistroIngresoService, ResultadoRegistroEntrada,
};
use crate::tiempo::{fecha_costa_rica, parsear_utc};

use super::AppCore;
use super::unidad_de_trabajo::en_savepoint;

/// Lo que la barra de estado muestra sobre la gente que lleva demasiado
/// adentro (ver `domain::permanencia`): los conteos y los textos ya redactados.
/// Un texto en `None` significa que no hay nadie en ese nivel y no se muestra
/// nada.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AvisosPermanencia {
    pub en_aviso: usize,
    pub criticos: usize,
    pub texto_aviso: Option<String>,
    pub texto_critico: Option<String>,
}

/// Una salida de un lote que no se pudo cerrar y el motivo, ya redactado
/// para el operador (el núcleo decide los textos; las interfaces sólo los
/// muestran).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SalidaRechazada {
    pub id: i64,
    pub mensaje: String,
}

/// Cómo terminó un lote de salidas: los ingresos cerrados, en el orden en
/// que llegaron, y los rechazados con su motivo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ResultadoSalidas {
    pub cerrados: Vec<i64>,
    pub rechazados: Vec<SalidaRechazada>,
}

impl AvisosPermanencia {
    fn desde(resumen: ResumenPermanencia) -> Self {
        Self {
            en_aviso: resumen.en_aviso,
            criticos: resumen.criticos,
            texto_aviso: (resumen.en_aviso > 0)
                .then(|| crate::mensajes::mensaje_permanencia_aviso(resumen.en_aviso)),
            texto_critico: (resumen.criticos > 0)
                .then(|| crate::mensajes::mensaje_permanencia_critica(resumen.criticos)),
        }
    }
}

impl AppCore {
    pub fn preparar_ingreso(
        &self,
        contratista_id: i64,
    ) -> Result<PreparacionIngreso, RegistroIngresoServiceError> {
        let contratistas = SqliteContratistaRepository::new(&self.connection);
        let empresas = SqliteEmpresaRepository::new(&self.connection);
        let registros = SqliteRegistroIngresoRepository::new(&self.connection);
        let gafetes = SqliteGafeteRepository::new(&self.connection);
        let hoy = fecha_costa_rica(self.reloj.ahora_utc());
        let mut preparacion = RegistroIngresoService::new(&contratistas, &registros, &gafetes)
            .preparar_ingreso(&empresas, contratista_id, hoy)?;
        if preparacion.resultado_acceso == ResultadoAcceso::PermitidoConAdvertencia {
            preparacion.aviso_praind = preparacion
                .fecha_vencimiento_praind
                .map(|fecha| crate::mensajes::mensaje_vencimiento_praind(fecha, hoy));
        }
        Ok(preparacion)
    }

    pub fn registrar_ingreso(
        &self,
        actor: &UsuarioSesion,
        contratista_id: i64,
        medio: crate::models::medio_ingreso::MedioIngreso,
        gafete: Option<i64>,
        placa: Option<String>,
    ) -> Result<ResultadoRegistroEntrada, RegistroIngresoServiceError> {
        // Las interfaces mandan la placa tal cual se tipeó; qué placa
        // corresponde al medio lo decide el dominio (escritorio y móvil).
        let placa = crate::domain::registro_ingreso::placa_segun_medio(medio, placa);
        self.en_unidad_de_trabajo(actor, validar_reloj, |transaction, ahora| {
            let contratistas = SqliteContratistaRepository::new(transaction);
            let registros = SqliteRegistroIngresoRepository::new(transaction);
            let gafetes = SqliteGafeteRepository::new(transaction);
            RegistroIngresoService::new(&contratistas, &registros, &gafetes).registrar_entrada(
                contratista_id,
                medio,
                gafete,
                placa,
                actor.id,
                ahora,
            )
        })
    }

    /// Las reglas de [`Self::registrar_ingreso`] sin escribir ni abrir
    /// transacción: lo que sabe este equipo (y su caché del otro equipo de
    /// la unidad). `con_nube` lo corre antes de ir a la red, así lo que ya se
    /// sabe acá no gasta ninguna consulta; la escritura lo vuelve a validar.
    pub fn validar_ingreso(
        &self,
        actor: &UsuarioSesion,
        contratista_id: i64,
        medio: crate::models::medio_ingreso::MedioIngreso,
        gafete: Option<i64>,
        placa: Option<String>,
    ) -> Result<(), RegistroIngresoServiceError> {
        let placa = crate::domain::registro_ingreso::placa_segun_medio(medio, placa);
        let contratistas = SqliteContratistaRepository::new(&self.connection);
        let registros = SqliteRegistroIngresoRepository::new(&self.connection);
        let gafetes = SqliteGafeteRepository::new(&self.connection);
        RegistroIngresoService::new(&contratistas, &registros, &gafetes).validar_entrada(
            contratista_id,
            medio,
            gafete,
            placa,
            actor.id,
            self.reloj.ahora_utc(),
        )?;
        Ok(())
    }

    pub fn listar_ingresos_activos(
        &self,
        filtro: &FiltroIngresosActivos,
    ) -> Result<ListaIngresosActivosResumen, RegistroIngresoServiceError> {
        RegistroIngresoConsultaService::new(&SqliteIngresosQuery::new(&self.connection))
            .listar_activos(filtro, self.reloj.ahora_utc(), self.umbrales_permanencia)
    }

    /// Nivel de permanencia de un ingreso por su hora de entrada en texto (los
    /// ingresos abiertos en otro dispositivo llegan así desde la caché de la
    /// nube), con el reloj y los umbrales de este núcleo. Una hora que no se
    /// puede leer cuenta como normal: mejor no alertar que alertar sin saber.
    pub fn nivel_permanencia_de_entrada(&self, hora_entrada: &str) -> NivelPermanencia {
        match parsear_utc(hora_entrada) {
            Ok(ingreso) => {
                nivel_de_permanencia(ingreso, self.reloj.ahora_utc(), self.umbrales_permanencia)
            }
            Err(error) => {
                log::warn!("permanencia: hora de entrada ilegible ({hora_entrada}): {error}");
                NivelPermanencia::Normal
            }
        }
    }

    /// Cuánta gente lleva más de 12 h (aviso) y más de 14 h (crítico) adentro,
    /// sumando los ingresos de este dispositivo y, si la sesión puede usar la
    /// nube, los abiertos por otros dispositivos del sitio.
    pub fn avisos_permanencia(
        &self,
        actor: &UsuarioSesion,
    ) -> Result<AvisosPermanencia, RegistroIngresoServiceError> {
        let locales = self.listar_ingresos_activos(&FiltroIngresosActivos::default())?;
        let mut niveles: Vec<NivelPermanencia> = locales
            .items
            .iter()
            .map(|ingreso| ingreso.nivel_permanencia)
            .collect();
        #[cfg(feature = "nube")]
        if let Ok(remotos) = self.listar_ingresos_remotos(actor) {
            niveles.extend(
                remotos
                    .iter()
                    .map(|remoto| self.nivel_permanencia_de_entrada(&remoto.hora_entrada)),
            );
        }
        #[cfg(not(feature = "nube"))]
        let _ = actor;
        Ok(AvisosPermanencia::desde(resumir_permanencia(niveles)))
    }

    pub fn registrar_salida(
        &self,
        actor: &UsuarioSesion,
        id: i64,
    ) -> Result<(), RegistroIngresoServiceError> {
        self.en_unidad_de_trabajo(actor, validar_reloj, |transaction, ahora| {
            let contratistas = SqliteContratistaRepository::new(transaction);
            let registros = SqliteRegistroIngresoRepository::new(transaction);
            let gafetes = SqliteGafeteRepository::new(transaction);
            RegistroIngresoService::new(&contratistas, &registros, &gafetes)
                .registrar_salida(id, ahora, actor.id)
        })
    }

    /// Cierra varias salidas de contratistas en **una sola transacción**: el
    /// reloj y el operador se validan una vez, las que se pueden cerrar se
    /// confirman todas juntas, y una que falla (ya cerrada, inexistente) no
    /// frena a las demás: queda en `rechazados` con su mensaje. Cada salida
    /// va en su propio tramo (`en_savepoint`), así que una rechazada no deja
    /// escrito nada a medias.
    ///
    /// Todas llevan la misma hora, la del reloj del núcleo al abrir la
    /// transacción. Si el reloj retrocedió o el operador ya no es válido, no
    /// se cierra ninguna y la respuesta es el error, igual que en
    /// [`Self::registrar_salida`].
    pub fn registrar_salidas(
        &self,
        actor: &UsuarioSesion,
        ids: &[i64],
    ) -> Result<ResultadoSalidas, RegistroIngresoServiceError> {
        self.en_unidad_de_trabajo(actor, validar_reloj, |transaction, ahora| {
            let contratistas = SqliteContratistaRepository::new(transaction);
            let registros = SqliteRegistroIngresoRepository::new(transaction);
            let gafetes = SqliteGafeteRepository::new(transaction);
            let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);
            let mut resultado = ResultadoSalidas::default();
            for &id in ids {
                let salida = en_savepoint(transaction, || {
                    servicio.registrar_salida(id, ahora, actor.id)
                });
                match salida {
                    Ok(()) => resultado.cerrados.push(id),
                    // Lo de la base corta el lote: no es culpa de esa salida.
                    Err(RegistroIngresoServiceError::Database(error)) => {
                        return Err(RegistroIngresoServiceError::Database(error));
                    }
                    Err(error) => resultado.rechazados.push(SalidaRechazada {
                        id,
                        mensaje: crate::mensajes::mensaje_salida(error),
                    }),
                }
            }
            Ok(resultado)
        })
    }
}

/// Comprobación de sanidad de todo el sistema, no una regla de negocio de
/// `RegistroIngresoService` (ver el comentario de
/// `AppCore::en_unidad_de_trabajo`). Sin SQL propio —
/// `ultimo_instante_movimiento` vive en `database::queries::ingresos`, junto
/// al resto del acceso a `registro_ingresos`.
fn validar_reloj(
    connection: &Connection,
    ahora: chrono::DateTime<chrono::Utc>,
) -> Result<(), RegistroIngresoServiceError> {
    let Some(ultima) = crate::database::queries::ingresos::ultimo_instante_movimiento(connection)?
    else {
        return Ok(());
    };
    if ahora < ultima {
        return Err(RegistroIngresoServiceError::RelojRetrocedido);
    }
    Ok(())
}
