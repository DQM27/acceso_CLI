use control_acceso::application::{AppCore, BootstrapError};
use control_acceso::database::connection::{RutaBaseDatosError, ruta_base_datos};
use control_acceso::instancia::{InstanciaError, InstanciaGuard};

#[derive(Debug)]
enum StartupError {
    RutaBaseDatos(RutaBaseDatosError),
    Instancia(InstanciaError),
    Bootstrap(BootstrapError),
    Terminal(std::io::Error),
    Usuario(control_acceso::services::error::UsuarioServiceError),
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RutaBaseDatos(error) => write!(formatter, "{error}"),
            Self::Instancia(error) => write!(formatter, "{error}"),
            Self::Bootstrap(error) => write!(formatter, "{error}"),
            Self::Terminal(error) => write!(formatter, "No se pudo iniciar la terminal: {error}"),
            Self::Usuario(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for StartupError {}

fn run() -> Result<(), StartupError> {
    let ruta_base_datos = ruta_base_datos().map_err(StartupError::RutaBaseDatos)?;
    let _instancia = InstanciaGuard::adquirir(&ruta_base_datos).map_err(StartupError::Instancia)?;
    let core = AppCore::abrir(&ruta_base_datos).map_err(StartupError::Bootstrap)?;
    let requiere_configuracion_inicial = core
        .requiere_configuracion_inicial()
        .map_err(StartupError::Usuario)?;
    control_acceso::tui::terminal::run(&core, requiere_configuracion_inicial)
        .map_err(StartupError::Terminal)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error de arranque: {error}");
        std::process::exit(1);
    }
}
