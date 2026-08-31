//! Exportación de Historial a XLSX en un hilo aparte, para no congelar el
//! bucle de la TUI. Medido (`docs/pendientes.md`): armar el XLSX de 100,000
//! movimientos tarda ~33 segundos — mucho más que el respaldo, y corría
//! igual de síncrono en el mismo hilo que dibuja la pantalla. Antes de este
//! cambio ni siquiera llegaba a pintarse el aviso "Exportando…": el mensaje
//! se fijaba en el mismo tick que arrancaba la exportación, pero el
//! `terminal.draw()` que lo hubiera mostrado corre en la vuelta *siguiente*
//! del bucle — que nunca llegaba hasta que la exportación (síncrona)
//! terminaba.
//!
//! Mismo patrón que `backup_jobs.rs`: hilo + `mpsc::Receiver` sondeado en el
//! bucle, con su propia conexión de sólo lectura al archivo en vez de
//! compartir la de `AppCore` entre hilos.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use rusqlite::Connection;

use crate::application::{AppCore, exportar_historial_seleccion_con_conexion};
use crate::database::queries::ingresos::FiltroHistorial;
use crate::historial::ColumnaHistorial;

use super::App;

/// El destino viaja junto con el resultado — `completar_exportacion` lo
/// necesita para el mensaje final, y así no hace falta un campo aparte en
/// `App` sólo para recordarlo mientras el hilo está en vuelo.
pub(super) type ReceptorExportacion = mpsc::Receiver<(Result<usize, String>, PathBuf)>;

fn exportar_historial_en_hilo(
    ruta_base_datos: PathBuf,
    filtro: FiltroHistorial,
    columnas: Vec<ColumnaHistorial>,
    destino: PathBuf,
) -> ReceptorExportacion {
    let (emisor, receptor) = mpsc::channel();
    std::thread::spawn(move || {
        let resultado: Result<usize, String> = (|| {
            let conexion = Connection::open(&ruta_base_datos).map_err(|error| error.to_string())?;
            conexion
                .busy_timeout(Duration::from_secs(5))
                .map_err(|error| error.to_string())?;
            exportar_historial_seleccion_con_conexion(&conexion, &filtro, None, &columnas, &destino)
                .map_err(|error| error.to_string())
        })();
        let _ = emisor.send((resultado, destino));
    });
    receptor
}

impl App {
    /// Dispara la exportación en un hilo aparte. Sin `core` no hay archivo
    /// que abrir — falla de inmediato, igual que antes.
    pub(in crate::tui::app) fn iniciar_exportacion_historial(
        &mut self,
        filtro: FiltroHistorial,
        columnas: Vec<ColumnaHistorial>,
        destino: PathBuf,
        core: Option<&AppCore>,
    ) {
        let Some(core) = core else {
            self.historial
                .completar_exportacion(Err("No se pudo exportar el historial".into()), &destino);
            return;
        };
        self.historial_exportacion_pendiente = Some(exportar_historial_en_hilo(
            core.ruta_base_datos().to_path_buf(),
            filtro,
            columnas,
            destino,
        ));
        self.historial.marcar_exportando();
    }

    /// Revisa sin bloquear si la exportación en curso ya terminó. Devuelve
    /// si acaba de resolverse (para invalidar el frame actual).
    pub(in crate::tui::app) fn recibir_exportacion_historial_si_lista(&mut self) -> bool {
        let Some(receptor) = &self.historial_exportacion_pendiente else {
            return false;
        };
        let Ok((resultado, destino)) = receptor.try_recv() else {
            return false;
        };
        self.historial_exportacion_pendiente = None;
        self.historial.completar_exportacion(resultado, &destino);
        true
    }
}
