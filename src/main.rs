use std::io::{self, Write};

use control_acceso::application::{AppCore, BootstrapError};
use control_acceso::database::connection::{RutaBaseDatosError, ruta_base_datos};
use control_acceso::instancia::{InstanciaError, InstanciaGuard};
use control_acceso::interfaz_preferida::{self, Interfaz};
use control_acceso::tui::app::SalidaApp;

/// Flag de recuperación: restablece la contraseña del usuario ROOT sin pasar por la
/// TUI, para cuando lo olvidó y no hay otro admin/root que se la pueda cambiar desde
/// la app. Sólo sirve para quien tiene acceso al ejecutable y al archivo de la base
/// de datos (ver comentario en `AppCore::resetear_password_root`).
const FLAG_RESET_ROOT: &str = "--reset-root";

/// Interfaz clásica (menús y paneles). Sin flags, la ruta por defecto es la
/// que diga `interfaz_preferida::leer()` (comandos si no hay ninguna
/// guardada todavía) — este flag y `FLAG_COMANDOS` son overrides puntuales
/// de un solo arranque, no tocan esa preferencia.
const FLAG_TUI_CLASICA: &str = "--tui-clasica";

/// Override puntual hacia la interfaz de comandos, simétrico a
/// `FLAG_TUI_CLASICA` — hace falta cuando la preferencia guardada es
/// "clasica" pero se quiere probar comandos una sola vez sin cambiarla.
const FLAG_COMANDOS: &str = "--comandos";

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
    #[error("No se pudo leer la entrada: {0}")]
    Entrada(#[source] std::io::Error),
    #[error("No se pudo crear el respaldo previo: {0}")]
    Respaldo(#[source] control_acceso::database::backup::RespaldoError),
    #[error(transparent)]
    Comandos(#[from] control_acceso::comandos::ComandosError),
}

fn run() -> Result<(), StartupError> {
    let ruta_base_datos = ruta_base_datos().map_err(StartupError::RutaBaseDatos)?;
    let instancia = InstanciaGuard::adquirir(&ruta_base_datos).map_err(StartupError::Instancia)?;

    let flag_clasica = std::env::args().any(|arg| arg == FLAG_TUI_CLASICA);
    let flag_comandos = std::env::args().any(|arg| arg == FLAG_COMANDOS);
    let usar_clasica = if flag_clasica {
        true
    } else if flag_comandos {
        false
    } else {
        interfaz_preferida::leer() == Some(Interfaz::Clasica)
    };

    let relanzar_en = if usar_clasica {
        run_tui_clasica(&ruta_base_datos)?
    } else {
        run_comandos(&ruta_base_datos)?
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
        Interfaz::Comandos => FLAG_COMANDOS,
    });
    // Conserva la relación con Alacritty si este proceso ya está relanzado
    // ahí — evita que el hijo intente relanzarse de nuevo en otra ventana.
    if std::env::var_os(ENV_RELANZADO_EN_ALACRITTY).is_some() {
        comando.env(ENV_RELANZADO_EN_ALACRITTY, "1");
    }
    let _ = comando.spawn();
}

/// Ruta por defecto: la interfaz de comandos. Mismo guard de instancia (lo
/// adquiere `run` antes de bifurcar) y mismo respaldo diario que la TUI
/// clásica; la configuración inicial la detecta y la explica la propia
/// interfaz de comandos (remite a `--tui-clasica` para crear el ROOT).
/// `Some(Interfaz::Clasica)` cuando el operador confirmó `/clasico`: la
/// preferencia ya quedó guardada, sólo falta que `run()` relance el proceso.
fn run_comandos(ruta_base_datos: &std::path::Path) -> Result<Option<Interfaz>, StartupError> {
    let core = AppCore::abrir(ruta_base_datos).map_err(StartupError::Bootstrap)?;
    let _ = core.respaldo_automatico_diario_si_hace_falta();
    let reiniciar_en_clasica =
        control_acceso::comandos::run(core, None).map_err(StartupError::Comandos)?;
    Ok(reiniciar_en_clasica.then_some(Interfaz::Clasica))
}

/// `--tui-clasica`: la interfaz original de menús y paneles. Sigue siendo la
/// única que crea el usuario ROOT inicial y la que permite saltar al modo
/// comandos reusando la sesión ya autenticada (`SalidaApp::ModoComandos`).
/// El resultado puede pedir un reinicio en cualquiera de las dos
/// direcciones: `Some(Interfaz::Comandos)` si el operador confirmó "Modo
/// comandos" en el Menú Principal (`SalidaApp::ReiniciarEnComandos`), o
/// `Some(Interfaz::Clasica)` si, tras saltar a comandos vía `ModoComandos`
/// (reusando la sesión, sin reiniciar), ahí adentro confirmó `/clasico` —
/// en los dos casos la preferencia ya quedó guardada antes de llegar acá.
fn run_tui_clasica(ruta_base_datos: &std::path::Path) -> Result<Option<Interfaz>, StartupError> {
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
            SalidaApp::Cerrar => return Ok(None),
            // El operador ya se autenticó y eligió el modo CLI: se reusa la
            // misma conexión (`core`) y la misma sesión, sin volver a pedir
            // cédula/contraseña.
            SalidaApp::ModoComandos { sesion } => {
                let reiniciar_en_clasica = control_acceso::comandos::run(core, Some(sesion))
                    .map_err(StartupError::Comandos)?;
                return Ok(reiniciar_en_clasica.then_some(Interfaz::Clasica));
            }
            // "Modo comandos" del Menú Principal: la preferencia ya se
            // guardó al confirmar (`AccionMenu::ModoComandos` en
            // `tui::app`) — acá sólo hace falta cerrar esta conexión y
            // avisarle a `run()` que relance en comandos.
            SalidaApp::ReiniciarEnComandos => {
                drop(core);
                return Ok(Some(Interfaz::Comandos));
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
    let es_reset_root = std::env::args().any(|arg| arg == FLAG_RESET_ROOT);
    if !es_reset_root && relanzar_en_alacritty() {
        return;
    }

    let resultado = if es_reset_root {
        ejecutar_reset_root()
    } else {
        run()
    };

    if let Err(error) = resultado {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
