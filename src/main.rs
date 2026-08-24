use std::io::{self, Write};

use control_acceso::application::{AppCore, BootstrapError};
use control_acceso::database::connection::{RutaBaseDatosError, ruta_base_datos};
use control_acceso::instancia::{InstanciaError, InstanciaGuard};
use control_acceso::tui::app::SalidaApp;

/// Flag de recuperación: restablece la contraseña del usuario ROOT sin pasar por la
/// TUI, para cuando lo olvidó y no hay otro admin/root que se la pueda cambiar desde
/// la app. Sólo sirve para quien tiene acceso al ejecutable y al archivo de la base
/// de datos (ver comentario en `AppCore::resetear_password_root`).
const FLAG_RESET_ROOT: &str = "--reset-root";

/// Interfaz clásica (menús y paneles). La ruta por defecto (sin flags) es
/// ahora la interfaz de comandos; este flag existe mientras ambas conviven.
const FLAG_TUI_CLASICA: &str = "--tui-clasica";

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
    #[error("No se pudo leer la entrada: {0}")]
    Entrada(#[source] std::io::Error),
    #[error("No se pudo crear el respaldo previo: {0}")]
    Respaldo(#[source] control_acceso::database::backup::RespaldoError),
    #[error(transparent)]
    Comandos(#[from] control_acceso::comandos::ComandosError),
}

fn run() -> Result<(), StartupError> {
    let ruta_base_datos = ruta_base_datos().map_err(StartupError::RutaBaseDatos)?;
    let _instancia = InstanciaGuard::adquirir(&ruta_base_datos).map_err(StartupError::Instancia)?;

    if std::env::args().any(|arg| arg == FLAG_TUI_CLASICA) {
        return run_tui_clasica(&ruta_base_datos);
    }

    run_comandos(&ruta_base_datos)
}

/// Ruta por defecto: la interfaz de comandos. Mismo guard de instancia (lo
/// adquiere `run` antes de bifurcar) y mismo respaldo diario que la TUI
/// clásica; la configuración inicial la detecta y la explica la propia
/// interfaz de comandos (remite a `--tui-clasica` para crear el ROOT).
fn run_comandos(ruta_base_datos: &std::path::Path) -> Result<(), StartupError> {
    let core = AppCore::abrir(ruta_base_datos).map_err(StartupError::Bootstrap)?;
    let _ = core.respaldo_automatico_diario_si_hace_falta();
    control_acceso::comandos::run(core, None).map_err(StartupError::Comandos)
}

/// `--tui-clasica`: la interfaz original de menús y paneles. Sigue siendo la
/// única que crea el usuario ROOT inicial y la que permite saltar al modo
/// comandos reusando la sesión ya autenticada (`SalidaApp::ModoComandos`).
fn run_tui_clasica(ruta_base_datos: &std::path::Path) -> Result<(), StartupError> {
    let mut mensaje_inicial = None;
    loop {
        let core = match AppCore::abrir(ruta_base_datos) {
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
        // El resultado real se recoge dentro de la TUI (`App::run_internal`
        // vuelve a revisar en su primera vuelta de todos modos) para poder
        // avisarle al operador si falla; acá sólo hace falta que se intente.
        let _ = core.respaldo_automatico_diario_si_hace_falta();
        let requiere_configuracion_inicial = core
            .requiere_configuracion_inicial()
            .map_err(StartupError::Usuario)?;
        let salida = control_acceso::tui::terminal::run(
            &core,
            requiere_configuracion_inicial,
            mensaje_inicial.take(),
        )
        .map_err(StartupError::Terminal)?;

        match salida {
            SalidaApp::Cerrar => return Ok(()),
            // El operador ya se autenticó y eligió el modo CLI: se reusa la
            // misma conexión (`core`) y la misma sesión, sin volver a pedir
            // cédula/contraseña.
            SalidaApp::ModoComandos { sesion } => {
                return control_acceso::comandos::run(core, Some(sesion))
                    .map_err(StartupError::Comandos);
            }
            SalidaApp::Restaurar { candidata } => {
                drop(core); // cierra la conexión SQLite antes de tocar el archivo activo
                if let Err(error) = control_acceso::database::backup::restaurar_respaldo(
                    &candidata,
                    ruta_base_datos,
                ) {
                    mensaje_inicial = Some(format!(
                        "No se pudo restaurar: {error}. Se conservó la base anterior."
                    ));
                }
            }
        }
    }
}

fn leer_linea(prompt: &str) -> Result<String, StartupError> {
    print!("{prompt}");
    io::stdout().flush().map_err(StartupError::Entrada)?;
    let mut linea = String::new();
    io::stdin()
        .read_line(&mut linea)
        .map_err(StartupError::Entrada)?;
    Ok(linea.trim().to_string())
}

/// Flujo de `--reset-root`: adquiere el mismo bloqueo de instancia que la TUI (para no
/// pisar una sesión abierta), abre la base y restablece la contraseña del ROOT que se
/// indique, sin necesidad de loguearse.
fn ejecutar_reset_root() -> Result<(), StartupError> {
    let ruta_base_datos = ruta_base_datos().map_err(StartupError::RutaBaseDatos)?;
    let _instancia = InstanciaGuard::adquirir(&ruta_base_datos).map_err(StartupError::Instancia)?;
    let core = AppCore::abrir(&ruta_base_datos).map_err(StartupError::Bootstrap)?;

    let roots = core.listar_roots_activos().map_err(StartupError::Usuario)?;
    let root = match roots.as_slice() {
        [] => {
            eprintln!("No hay ningún usuario ROOT activo en la base de datos.");
            std::process::exit(1);
        }
        [unico] => unico.clone(),
        varios => {
            println!(
                "Hay varios usuarios ROOT activos. Indique la cédula del que desea restablecer:"
            );
            for usuario in varios {
                println!("  {} - {}", usuario.cedula, usuario.nombre);
            }
            let cedula = leer_linea("Cédula: ")?;
            match varios.iter().find(|usuario| usuario.cedula == cedula) {
                Some(usuario) => usuario.clone(),
                None => {
                    eprintln!("Esa cédula no corresponde a ningún ROOT activo.");
                    std::process::exit(1);
                }
            }
        }
    };

    println!(
        "Restableciendo la contraseña de {} ({}).",
        root.nombre, root.cedula
    );
    let nueva = rpassword::prompt_password("Nueva contraseña: ").map_err(StartupError::Entrada)?;
    let confirmacion =
        rpassword::prompt_password("Confirme la contraseña: ").map_err(StartupError::Entrada)?;
    if nueva != confirmacion {
        eprintln!("Las contraseñas no coinciden. No se hizo ningún cambio.");
        std::process::exit(1);
    }

    let respuesta = leer_linea(
        "Se creará un respaldo de la base antes de aplicar el cambio. Escriba SI para continuar: ",
    )?;
    if respuesta != "SI" {
        eprintln!("Operación cancelada. No se hizo ningún cambio.");
        std::process::exit(1);
    }

    let respaldo = core
        .crear_respaldo_por_flag()
        .map_err(StartupError::Respaldo)?;
    println!("Respaldo creado en {}.", respaldo.ruta.display());

    core.resetear_password_root(root.id, &nueva)
        .map_err(StartupError::Usuario)?;
    println!("Contraseña actualizada correctamente.");
    Ok(())
}

fn main() {
    let resultado = if std::env::args().any(|arg| arg == FLAG_RESET_ROOT) {
        ejecutar_reset_root()
    } else {
        run()
    };

    if let Err(error) = resultado {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
