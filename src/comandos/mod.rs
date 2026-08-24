//! Interfaz de terminal dirigida por comandos — ruta por defecto de la
//! aplicación (sin flags).
//!
//! Un input persistente es el protagonista: el área superior se transforma
//! según lo que se escribe (input → parse → resolver → contexto). No hay
//! pantallas, menús ni formularios — el mismo espacio muta alrededor del
//! comando. Convive con la TUI clásica, alcanzable con `--tui-clasica`.
//!
//! Login: la cédula se resuelve contra SQLite de inmediato (rápido) y la
//! verificación Argon2 corre en un hilo aparte con canal, el mismo patrón de
//! `tui::app::auth_jobs` — la interfaz nunca se congela calculando el hash.
//! Antes de aceptar la sesión se vuelve a resolver el candidato contra SQLite,
//! por si la cuenta fue desactivada mientras corría Argon2.
//!
//! Este archivo sólo orquesta el loop y despacha teclas por fase. Cada
//! controlador vive en su propio módulo: [`login`] (autenticación),
//! [`operando`] (comandos y confirmaciones) y [`formulario_controller`]
//! (alta/edición de contratista) — los tres leen y escriben el mismo
//! [`AppState`], nunca estado propio.

mod estado;
mod formulario;
mod formulario_controller;
mod login;
mod operando;
mod parser;
mod render;
mod resolver;
mod terminal;

use std::io::{self, stdout};
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};

use terminal::TerminalGuard;

use crate::application::AppCore;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::{AutenticacionError, UsuarioServiceError};

pub use estado::{AppState, ContextState, Fase, NivelFeedback};
pub use formulario::{
    Campo, FormularioContratista, MAX_VISIBLES_EMPRESAS, ModoFormulario, Subfase,
};
pub use parser::{Comando, Entrada, GafeteParse, MedioParse, parsear};
pub use resolver::{
    autocompletar, calcular_sugerencias, ficha_desde_resumen, preparar_resumen_ingreso, resolver,
};

#[derive(Debug, thiserror::Error)]
pub enum ComandosError {
    #[error("No se pudo iniciar la terminal: {0}")]
    Terminal(#[from] io::Error),
    #[error(transparent)]
    Usuario(#[from] UsuarioServiceError),
}

/// Receptor del hilo que verifica la contraseña con Argon2, junto con la
/// cédula y el nombre ya resueltos (para volver a la pantalla de contraseña,
/// con la identidad intacta, si falla).
type AutenticacionPendiente = Option<(
    String,
    String,
    mpsc::Receiver<Result<UsuarioSesion, AutenticacionError>>,
)>;

/// Punto de entrada de la interfaz de comandos. Consume el `AppCore` (la ruta
/// por defecto de `main` no hace nada más con él después).
///
/// `sesion_inicial` viene con `Some` cuando el operador ya se autenticó en la
/// TUI clásica (`--tui-clasica`) y eligió el modo CLI desde `ElegirInterfaz`
/// — en ese caso se arranca directo en `Fase::Operando`, sin repetir
/// cédula/contraseña. Con `None` (ruta por defecto, sin flags) hace su
/// propio login, como siempre.
pub fn run(core: AppCore, sesion_inicial: Option<UsuarioSesion>) -> Result<(), ComandosError> {
    if core.requiere_configuracion_inicial()? {
        return avisar_configuracion_inicial();
    }

    let _guard = TerminalGuard::acquire()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = match sesion_inicial {
        Some(sesion) => AppState::con_sesion(sesion),
        None => AppState::new(),
    };
    let mut autenticacion: AutenticacionPendiente = None;
    recomputar(&core, &mut app);

    // Redraw-on-demand: sólo se dibuja cuando algo realmente cambió. La
    // espera del poll es dinámica (ver `proxima_espera`) para no quemar CPU
    // redibujando el mismo frame en reposo, sin sacrificar respuesta al
    // teclado (el evento se procesa y se dibuja en la misma vuelta en que
    // llega, nunca atado a un tick fijo).
    let mut redibujar = true;
    while !app.salir {
        if login::recibir_autenticacion(&core, &mut app, &mut autenticacion) {
            redibujar = true;
        }
        if app.expirar_feedback() {
            redibujar = true;
        }

        if redibujar {
            terminal.draw(|frame| render::render(frame, &app))?;
            redibujar = false;
        }

        let espera = proxima_espera(&app, &autenticacion);
        if crossterm::event::poll(espera)? {
            match crossterm::event::read()? {
                Event::Key(key) => {
                    manejar_tecla(&core, &mut app, key, &mut autenticacion);
                    redibujar = true;
                }
                Event::Resize(_, _) => redibujar = true,
                _ => {}
            }
        }
    }
    Ok(())
}

/// Cuánto puede esperar el próximo `poll` antes de que el loop necesite
/// revisar algo por su cuenta (sin que haya llegado un evento de teclado).
///
/// - Con Argon2 corriendo, hay que revisar el canal seguido.
/// - Con feedback transitorio visible, sólo hace falta despertar cuando
///   está por expirar.
/// - En reposo, esperar casi indefinidamente: el teclado despierta el poll
///   de inmediato, no hace falta sondear nada más.
fn proxima_espera(app: &AppState, autenticacion: &AutenticacionPendiente) -> Duration {
    const ESPERA_VERIFICACION: Duration = Duration::from_millis(30);
    const ESPERA_REPOSO: Duration = Duration::from_secs(60 * 60);

    if autenticacion.is_some() {
        return ESPERA_VERIFICACION;
    }
    if let Some(restante) = app.feedback_restante() {
        return restante.max(Duration::from_millis(1));
    }
    ESPERA_REPOSO
}

/// Aviso para cuando la base todavía no tiene usuario ROOT: la configuración
/// inicial pertenece a la TUI clásica — acá sólo se explica y se sale con
/// cualquier tecla.
fn avisar_configuracion_inicial() -> Result<(), ComandosError> {
    let _guard = TerminalGuard::acquire()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.draw(|frame| {
        frame.render_widget(
            ratatui::widgets::Paragraph::new(
                "Falta la configuración inicial del sistema.\n\n\
                 Cierre y arranque con --tui-clasica para crear el usuario ROOT inicial.\n\n\
                 Presione cualquier tecla para salir.",
            ),
            frame.area(),
        );
    })?;
    loop {
        if let Event::Key(key) = crossterm::event::read()?
            && key.kind == KeyEventKind::Press
        {
            return Ok(());
        }
    }
}

/// Reconstruye parse → contexto → sugerencias tras cualquier cambio del input.
/// Con el formulario abierto el input edita campos, no comandos: el contexto
/// queda congelado hasta que el formulario se cierra. Comparten esta función
/// los tres controladores (`login`, `operando`, `formulario_controller`).
fn recomputar(core: &AppCore, app: &mut AppState) {
    if !matches!(app.fase, Fase::Operando { .. }) || app.formulario.is_some() {
        return;
    }
    let entrada = parser::parsear(app.input.value());
    app.contexto = resolver::resolver(core, &entrada);
    app.sugerencias = resolver::calcular_sugerencias(core, app.input.value(), &entrada);
}

fn manejar_tecla(
    core: &AppCore,
    app: &mut AppState,
    key: KeyEvent,
    autenticacion: &mut AutenticacionPendiente,
) {
    if key.kind != KeyEventKind::Press {
        return;
    }
    // Ctrl+C sale limpio desde cualquier fase.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.salir = true;
        return;
    }

    match app.fase.clone() {
        Fase::LoginCedula => login::manejar_login_cedula(core, app, key),
        Fase::LoginPassword { cedula, nombre } => {
            login::manejar_login_password(core, app, key, cedula, nombre, autenticacion)
        }
        Fase::Verificando { .. } => {}
        Fase::Operando { .. } => operando::manejar_operando(core, app, key),
    }
}
