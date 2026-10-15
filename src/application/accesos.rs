//! Preparar/registrar ingreso, registrar salida, y el listado de ingresos activos.

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
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

use super::{AppCore, verificar_actor_activo};

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

    /// Abre una transacción `Immediate` (el bloqueo se adquiere antes de la
    /// primera lectura definitiva, así los repositorios creados sobre ella
    /// validan e insertan contra el mismo estado de `SQLite`), valida que el
    /// reloj no haya retrocedido, confirma que `usuario_id` sigue siendo un
    /// operador activo, corre `operar` y confirma. Compartido por
    /// `registrar_ingreso` y `registrar_salida` — antes cada uno repetía este
    /// mismo armazón letra por letra.
    ///
    /// La validación del reloj se queda aquí, en `AppCore`, y no en
    /// `RegistroIngresoService` (que declara `RelojRetrocedido` pero no lo
    /// genera) **a propósito**: es una comprobación de sanidad de todo el
    /// sistema (¿el reloj de la máquina retrocedió respecto al último
    /// movimiento conocido, sin importar de qué contratista?), no una regla
    /// de negocio de una entrada/salida puntual. Moverla al servicio se
    /// intentó y se revirtió — rompía tests de integración que llaman al
    /// servicio directo con datos de prueba cuyos tiempos no representan un
    /// reloj real avanzando (`tests/flujo_integracion.rs`).
    ///
    /// La verificación de operador activo vive aquí por el mismo motivo:
    /// `registrar_entrada`/`registrar_salida` recibían el `usuario_id` como
    /// un entero crudo, sin comprobar que existiera ni que siguiera activo —
    /// la FK de `SQLite` sólo exige que exista, no que esté activo. Una
    /// sesión de TUI ya iniciada podía seguir registrando movimientos
    /// después de que un administrador desactivara esa cuenta
    /// (`docs/auditoria-dominio-2026-08-20.md`, hallazgo #2). Se revisa
    /// dentro de esta misma transacción `Immediate`, no antes, para que una
    /// desactivación concurrente no pueda colarse entre el chequeo y la
    /// escritura.
    fn en_transaccion_con_reloj_validado<T>(
        &self,
        actor: &UsuarioSesion,
        operar: impl FnOnce(
            &Transaction<'_>,
            chrono::DateTime<chrono::Utc>,
        ) -> Result<T, RegistroIngresoServiceError>,
    ) -> Result<T, RegistroIngresoServiceError> {
        let ahora = self.reloj.ahora_utc();
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(DatabaseError::from)?;
        validar_reloj(&transaction, ahora)?;
        verificar_operador_activo(&transaction, actor)?;
        let resultado = operar(&transaction, ahora)?;
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(resultado)
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
        self.en_transaccion_con_reloj_validado(actor, |transaction, ahora| {
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
        self.en_transaccion_con_reloj_validado(actor, |transaction, ahora| {
            let contratistas = SqliteContratistaRepository::new(transaction);
            let registros = SqliteRegistroIngresoRepository::new(transaction);
            let gafetes = SqliteGafeteRepository::new(transaction);
            RegistroIngresoService::new(&contratistas, &registros, &gafetes)
                .registrar_salida(id, ahora, actor.id)
        })
    }
}

/// Comprobación de sanidad de todo el sistema, no una regla de negocio de
/// `RegistroIngresoService` (ver el comentario de
/// `en_transaccion_con_reloj_validado`). Sin SQL propio —
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

/// Comprobación de sanidad del actor, mismo criterio que `validar_reloj`: no
/// es una regla de negocio de una entrada/salida puntual, es "¿quién dice
/// que está registrando esto sigue siendo un operador real?". La FK de
/// `usuario_ingreso_id`/`usuario_salida_id` en `SQLite` sólo exige que el ID
/// exista — nunca que la cuenta siga activa.
fn verificar_operador_activo(
    connection: &Connection,
    actor: &UsuarioSesion,
) -> Result<(), RegistroIngresoServiceError> {
    if verificar_actor_activo(connection, actor)?.is_some() {
        Ok(())
    } else {
        Err(RegistroIngresoServiceError::OperadorNoAutorizado)
    }
}
