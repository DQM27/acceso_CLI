use control_acceso::application::{AppCore, BootstrapError};
use control_acceso::database::connection::{RutaBaseDatosError, ruta_base_datos};
use control_acceso::instancia::{InstanciaError, InstanciaGuard};
use control_acceso::tui::app::SalidaApp;

#[derive(Debug, thiserror::Error)]
enum StartupError {
    #[error(transparent)]
    RutaBaseDatos(RutaBaseDatosError),
    #[error(transparent)]
    Instancia(InstanciaError),
    #[error(transparent)]
    Bootstrap(BootstrapError),
    #[error("No se pudo iniciar la terminal: {0}")]
    Terminal(#[source] std::io::Error),
    #[error(transparent)]
    Usuario(control_acceso::services::error::UsuarioServiceError),
}

fn run() -> Result<(), StartupError> {
    let ruta_base_datos = ruta_base_datos().map_err(StartupError::RutaBaseDatos)?;
    let _instancia = InstanciaGuard::adquirir(&ruta_base_datos).map_err(StartupError::Instancia)?;

    let mut mensaje_inicial = None;
    loop {
        let core = match AppCore::abrir(&ruta_base_datos) {
            Ok(core) => core,
            Err(error) => {
                // Muestra el error en la TUI (mismo mecanismo que usa un fallo de
                // restauración) en vez de matar el proceso con un eprintln crudo.
                let _ = control_acceso::tui::terminal::run_sin_core(Some(format!(
                    "No se pudo abrir la base de datos: {error}"
                )));
                return Err(StartupError::Bootstrap(error));
            }
        };
        core.respaldo_automatico_diario_si_hace_falta();
        let requiere_configuracion_inicial = core
            .requiere_configuracion_inicial()
            .map_err(StartupError::Usuario)?;
        let salida = control_acceso::tui::terminal::run(
            &core,
            requiere_configuracion_inicial,
            mensaje_inicial.take(),
        )
        .map_err(StartupError::Terminal)?;
        drop(core); // cierra la conexión SQLite antes de tocar el archivo activo

        match salida {
            SalidaApp::Cerrar => return Ok(()),
            SalidaApp::Restaurar { candidata } => {
                if let Err(error) = control_acceso::database::backup::restaurar_respaldo(
                    &candidata,
                    &ruta_base_datos,
                ) {
                    mensaje_inicial = Some(format!(
                        "No se pudo restaurar: {error}. Se conservó la base anterior."
                    ));
                }
            }
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error de arranque: {error}");
        std::process::exit(1);
    }
}
