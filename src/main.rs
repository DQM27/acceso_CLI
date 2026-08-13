use control_acceso::application::{AppCore, BootstrapError};
use control_acceso::database::connection::ruta_base_datos;

#[derive(Debug)]
enum StartupError {
    Bootstrap(BootstrapError),
    ConfiguracionInicialRequerida,
    Terminal(std::io::Error),
    Usuario(control_acceso::services::error::UsuarioServiceError),
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bootstrap(error) => write!(formatter, "{error}"),
            Self::ConfiguracionInicialRequerida => write!(
                formatter,
                "Se requiere crear el usuario ROOT inicial antes de iniciar sesión"
            ),
            Self::Terminal(error) => write!(formatter, "No se pudo iniciar la terminal: {error}"),
            Self::Usuario(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for StartupError {}

fn run() -> Result<(), StartupError> {
    let core = AppCore::abrir(ruta_base_datos()).map_err(StartupError::Bootstrap)?;
    if core
        .requiere_configuracion_inicial()
        .map_err(StartupError::Usuario)?
    {
        return Err(StartupError::ConfiguracionInicialRequerida);
    }
    control_acceso::tui::terminal::run(&core).map_err(StartupError::Terminal)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error de arranque: {error}");
        std::process::exit(1);
    }
}
