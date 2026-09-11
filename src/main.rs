use control_acceso::application::{AppCore, BootstrapError};
use control_acceso::database::connection::{RutaBaseDatosError, ruta_base_datos};
use control_acceso::instancia::{InstanciaError, InstanciaGuard};
use control_acceso::interfaz_preferida::{self, Interfaz};
use control_acceso::tui::app::SalidaApp;

/// Interfaz clásica (menús y paneles). Sin flags, la ruta por defecto es la
/// que diga `interfaz_preferida::leer()` (CLI si no hay ninguna
/// guardada todavía) — este flag y `FLAG_CLI` son overrides puntuales
/// de un solo arranque, no tocan esa preferencia.
const FLAG_TUI_CLASICA: &str = "--tui-clasica";

/// Override puntual hacia la CLI, simétrico a
/// `FLAG_TUI_CLASICA` — hace falta cuando la preferencia guardada es
/// "clasica" pero se quiere probar la CLI una sola vez sin cambiarla.
const FLAG_CLI: &str = "--cli";

/// Marca al proceso hijo como "ya relanzado dentro de Alacritty" — evita que
/// se vuelva a relanzar a sí mismo en bucle. El valor no importa, sólo que
/// exista.
const ENV_RELANZADO_EN_ALACRITTY: &str = "CONTROL_ACCESO_EN_ALACRITTY";

/// Si hay un `Alacritty.exe` al lado del propio ejecutable (y todavía no nos
/// relanzamos), lo lanza con este mismo binario como su comando (`-e`) y
/// termina el proceso actual — el que queda corriendo es el hijo, ya dentro
/// de la ventana con aceleración por GPU de Alacritty, no en la consola que
/// haya abierto Windows. Sin `Alacritty.exe` al lado (o si algo falla al
/// lanzarlo) sigue de largo con el arranque normal: cero cambio de
/// comportamiento sin la carpeta armada. Deliberadamente NO se usa con
/// `--reset-root` (ver `main`): ese flujo es de consola/recuperación, no la
/// experiencia de kiosco que busca Alacritty.
fn relanzar_en_alacritty() -> bool {
    if std::env::var_os(ENV_RELANZADO_EN_ALACRITTY).is_some() {
        return false;
    }
    let Ok(exe_actual) = std::env::current_exe() else {
        return false;
    };
    let Some(carpeta) = exe_actual.parent() else {
        return false;
    };
    let alacritty = carpeta.join("Alacritty.exe");
    if !alacritty.is_file() {
        return false;
    }

    let mut comando = std::process::Command::new(&alacritty);
    comando.env(ENV_RELANZADO_EN_ALACRITTY, "1");
    // Config propia si está al lado; si no, Alacritty cae a la suya por
    // defecto (`%APPDATA%\alacritty\alacritty.toml` o los valores de fábrica).
    let config = carpeta.join("alacritty.toml");
    if config.is_file() {
        comando.arg("--config-file").arg(&config);
    }
    comando.arg("-e").arg(&exe_actual);
    comando.args(std::env::args().skip(1));

    comando.spawn().is_ok()
}

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
    #[error(transparent)]
    Cli(#[from] control_acceso::cli::CliError),
}

fn run() -> Result<(), StartupError> {
    let ruta_base_datos = ruta_base_datos().map_err(StartupError::RutaBaseDatos)?;
    let instancia = InstanciaGuard::adquirir(&ruta_base_datos).map_err(StartupError::Instancia)?;

    let flag_clasica = std::env::args().any(|arg| arg == FLAG_TUI_CLASICA);
    let flag_cli = std::env::args().any(|arg| arg == FLAG_CLI);
    let usar_clasica = if flag_clasica {
        true
    } else if flag_cli {
        false
    } else {
        interfaz_preferida::leer() == Some(Interfaz::Clasica)
    };

    let relanzar_en = if usar_clasica {
        run_tui_clasica(&ruta_base_datos)?
    } else {
        run_cli(&ruta_base_datos)?
    };

    // Libera el lock de instancia única ANTES de relanzar — si no,
    // `InstanciaGuard::adquirir` del proceso nuevo fallaría contra este
    // mismo proceso, que todavía no terminó de salir.
    drop(instancia);
    if let Some(interfaz) = relanzar_en {
        relanzar_en_interfaz(interfaz);
    }
    Ok(())
}

/// Vuelve a lanzar este mismo ejecutable con el flag de la interfaz elegida
/// — mismo patrón que `relanzar_en_alacritty` (spawnea y deja correr al
/// hijo). Un fallo al relanzar (ejecutable no localizable, por ejemplo) se
/// ignora en silencio: el peor caso es que el operador tenga que abrir la
/// app de nuevo a mano, nunca un error a mitad de un cierre ya decidido.
fn relanzar_en_interfaz(interfaz: Interfaz) {
    let Ok(exe_actual) = std::env::current_exe() else {
        return;
    };
    let mut comando = std::process::Command::new(&exe_actual);
    comando.arg(match interfaz {
        Interfaz::Clasica => FLAG_TUI_CLASICA,
        Interfaz::Cli => FLAG_CLI,
    });
    // Conserva la relación con Alacritty si este proceso ya está relanzado
    // ahí — evita que el hijo intente relanzarse de nuevo en otra ventana.
    if std::env::var_os(ENV_RELANZADO_EN_ALACRITTY).is_some() {
        comando.env(ENV_RELANZADO_EN_ALACRITTY, "1");
    }
    let _ = comando.spawn();
}

/// El arranque inicial de un sitio (base local vacía) ya no se resuelve acá
/// -- ni la CLI ni la TUI clásica vuelven a ofrecer "crear un ROOT" a
/// quien sea que tenga el ejecutable y el archivo de la base: ese camino
/// fabricaba una identidad sin verificar nada contra la nube (ver
/// docs/decisiones-tecnicas.md, "cierre del bootstrap local de ROOT"). El
/// arranque real de un sitio nuevo es la app de escritorio, que exige el
/// secreto de dispositivo y trae el catálogo (usuarios incluidos) ya
/// existente en Supabase.
fn mostrar_mensaje_configuracion_inicial() {
    eprintln!(
        "Esta base de datos todavía no tiene usuarios. El arranque inicial de un \
         sitio se hace desde la app de escritorio (pegando el secreto de \
         dispositivo), no desde acá."
    );
}

/// Ruta por defecto: la CLI. Mismo guard de instancia (lo
/// adquiere `run` antes de bifurcar) y mismo respaldo diario que la TUI
/// clásica.
/// `Some(Interfaz::Clasica)` cuando el operador confirmó `/clasico`: la
/// preferencia ya quedó guardada, sólo falta que `run()` relance el proceso.
fn run_cli(ruta_base_datos: &std::path::Path) -> Result<Option<Interfaz>, StartupError> {
    let core = AppCore::abrir(ruta_base_datos).map_err(StartupError::Bootstrap)?;
    if core
        .requiere_configuracion_inicial()
        .map_err(StartupError::Usuario)?
    {
        mostrar_mensaje_configuracion_inicial();
        return Ok(None);
    }
    let reiniciar_en_clasica =
        control_acceso::cli::run(core, None, ruta_base_datos.to_path_buf())
            .map_err(StartupError::Cli)?;
    Ok(reiniciar_en_clasica.then_some(Interfaz::Clasica))
}

/// `--tui-clasica`: la interfaz original de menús y paneles.
fn run_tui_clasica(ruta_base_datos: &std::path::Path) -> Result<Option<Interfaz>, StartupError> {
    let core = match AppCore::abrir(ruta_base_datos) {
        Ok(core) => core,
        Err(error) => {
            // Muestra el error en la TUI en vez de matar el proceso con un
            // eprintln crudo.
            let _ = control_acceso::tui::terminal::run_sin_core(
                Some(format!("No se pudo abrir la base de datos: {error}")),
                ruta_base_datos.to_path_buf(),
            );
            return Err(StartupError::Bootstrap(error));
        }
    };
    if core
        .requiere_configuracion_inicial()
        .map_err(StartupError::Usuario)?
    {
        mostrar_mensaje_configuracion_inicial();
        return Ok(None);
    }
    let salida = control_acceso::tui::terminal::run(&core, false, None, ruta_base_datos.to_path_buf())
        .map_err(StartupError::Terminal)?;

    match salida {
        SalidaApp::Cerrar => Ok(None),
        // "Modo CLI" del Menú Principal: la preferencia ya se
        // guardó al confirmar (`AccionMenu::Cli` en
        // `tui::app`) — acá sólo hace falta cerrar esta conexión y
        // avisarle a `run()` que relance en la CLI.
        SalidaApp::ReiniciarEnCli => {
            drop(core);
            Ok(Some(Interfaz::Cli))
        }
    }
}

fn main() {
    if relanzar_en_alacritty() {
        return;
    }

    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
