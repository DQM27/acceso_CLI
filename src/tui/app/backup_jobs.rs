//! Creación de respaldo en un hilo aparte, para no congelar el bucle de la
//! TUI. Medido (`docs/pendientes.md`): copiar y validar la base completa
//! (Online Backup API + `integrity_check` + `foreign_key_check`) tarda
//! ~200ms con unos pocos miles de movimientos y ~2 segundos con ~100,000 —
//! ya perceptible hoy, y crece con la antigüedad de la instalación. Esto
//! corría antes en el mismo hilo que dibuja la pantalla, tanto para el botón
//! manual (Respaldos → Crear) como para la revisión automática diaria.
//!
//! Mismo patrón que `auth_jobs.rs` (hilo + `mpsc::Receiver` sondeado en cada
//! vuelta del bucle), pero el hilo abre su PROPIA conexión de sólo lectura
//! al mismo archivo en vez de compartir la conexión viva de `AppCore`:
//! `SQLite` permite varias conexiones concurrentes al mismo archivo desde
//! hilos distintos, y la Online Backup API ya reintenta sola ante un
//! bloqueo transitorio (`Backup::run_to_completion`, ver
//! `database::backup::crear_respaldo`) — no hace falta ninguna otra
//! sincronización.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use rusqlite::Connection;

use crate::application::AppCore;
use crate::database::backup::{RespaldoError, RespaldoResumen, TipoRespaldo, crear_respaldo};

use super::App;

pub(super) type ReceptorRespaldo = mpsc::Receiver<Result<RespaldoResumen, RespaldoError>>;

fn crear_respaldo_en_hilo(
    ruta_base_datos: PathBuf,
    directorio_respaldos: PathBuf,
    tipo: TipoRespaldo,
) -> ReceptorRespaldo {
    let (emisor, receptor) = mpsc::channel();
    std::thread::spawn(move || {
        let resultado: Result<RespaldoResumen, RespaldoError> = (|| {
            let conexion = Connection::open(&ruta_base_datos)?;
            conexion.busy_timeout(Duration::from_secs(5))?;
            crear_respaldo(&conexion, &directorio_respaldos, tipo)
        })();
        let _ = emisor.send(resultado);
    });
    receptor
}

impl App {
    /// Autoriza rápido (una fila, en el hilo principal) y, si procede,
    /// dispara la creación real en un hilo aparte. Un error de autorización
    /// nunca llega a disparar ningún hilo.
    pub(in crate::tui::app) fn iniciar_creacion_respaldo_manual(&mut self, core: Option<&AppCore>) {
        let resultado = core
            .ok_or_else(|| "No se pudo crear el respaldo".to_owned())
            .and_then(|core| {
                let actor = self
                    .sesion
                    .as_ref()
                    .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                core.autorizar_creacion_respaldo(actor)
                    .map_err(|error| error.to_string())?;
                Ok((
                    core.ruta_base_datos().to_path_buf(),
                    core.directorio_respaldos(),
                ))
            });
        match resultado {
            Ok((ruta_base_datos, directorio_respaldos)) => {
                self.respaldo_manual_pendiente = Some(crear_respaldo_en_hilo(
                    ruta_base_datos,
                    directorio_respaldos,
                    TipoRespaldo::Manual,
                ));
                self.configuracion.marcar_creando_respaldo();
            }
            Err(error) => self.configuracion.completar_creacion(Err(error)),
        }
    }

    /// Revisa sin bloquear si el respaldo manual en curso ya terminó.
    /// Devuelve si acaba de resolverse (para invalidar el frame actual).
    pub(in crate::tui::app) fn recibir_respaldo_manual_si_listo(&mut self) -> bool {
        let Some(receptor) = &self.respaldo_manual_pendiente else {
            return false;
        };
        let Ok(resultado) = receptor.try_recv() else {
            return false;
        };
        self.respaldo_manual_pendiente = None;
        self.configuracion
            .completar_creacion(resultado.map_err(|error| error.to_string()));
        true
    }

    /// Revisión periódica (cada 60s, `run_internal`): decide rápido si hace
    /// falta un respaldo automático hoy y, si sí, lo dispara en un hilo
    /// aparte. Nunca superpone dos respaldos (manual o automático) en
    /// vuelo a la vez.
    pub(in crate::tui::app) fn revisar_respaldo_automatico(&mut self, core: &AppCore) {
        if self.respaldo_automatico_pendiente.is_some() || self.respaldo_manual_pendiente.is_some()
        {
            return;
        }
        match core.hace_falta_respaldo_automatico_hoy() {
            Ok(true) => {
                self.respaldo_automatico_pendiente = Some(crear_respaldo_en_hilo(
                    core.ruta_base_datos().to_path_buf(),
                    core.directorio_respaldos(),
                    TipoRespaldo::Automatico,
                ));
            }
            Ok(false) => {}
            Err(error) => self.reportar_fallo_respaldo_automatico(Some(error.to_string())),
        }
    }

    /// Revisa sin bloquear si el respaldo automático en curso ya terminó;
    /// si tuvo éxito, aplica la retención de inmediato (barata, sólo borra
    /// archivos viejos). Devuelve si acaba de resolverse.
    pub(in crate::tui::app) fn recibir_respaldo_automatico_si_listo(
        &mut self,
        core: &AppCore,
    ) -> bool {
        let Some(receptor) = &self.respaldo_automatico_pendiente else {
            return false;
        };
        let Ok(resultado) = receptor.try_recv() else {
            return false;
        };
        self.respaldo_automatico_pendiente = None;
        match resultado {
            Ok(_) => {
                core.aplicar_retencion_automatica();
                self.reportar_fallo_respaldo_automatico(None);
            }
            Err(error) => self.reportar_fallo_respaldo_automatico(Some(error.to_string())),
        }
        true
    }

    fn reportar_fallo_respaldo_automatico(&mut self, fallo: Option<String>) {
        self.menu.fallo_respaldo_automatico.clone_from(&fallo);
        self.configuracion
            .actualizar_fallo_respaldo_automatico(fallo);
    }
}
