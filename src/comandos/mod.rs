//! Interfaz de terminal dirigida por comandos (`--comandos`).
//!
//! Un input persistente es el protagonista: el área superior se transforma
//! según lo que se escribe (input → parse → resolver → contexto). No hay
//! pantallas, menús ni formularios — el mismo espacio muta alrededor del
//! comando. Convive con la TUI clásica, que sigue siendo la ruta por defecto.
//!
//! Login: la cédula se resuelve contra SQLite de inmediato (rápido) y la
//! verificación Argon2 corre en un hilo aparte con canal, el mismo patrón de
//! `tui::app::auth_jobs` — la interfaz nunca se congela calculando el hash.
//! Antes de aceptar la sesión se vuelve a resolver el candidato contra SQLite,
//! por si la cuenta fue desactivada mientras corría Argon2.

mod estado;
mod parser;
mod render;
mod resolver;

use std::io::{self, stdout};
use std::sync::mpsc;
use std::time::Duration;

use crossterm::{
    cursor::{Hide, Show},
    event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, SetTitle, disable_raw_mode, enable_raw_mode,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::models::medio_ingreso::MedioIngreso;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::{
    AutenticacionError, RegistroIngresoServiceError, UsuarioServiceError,
};
use crate::tiempo::hora_actual_texto;

pub use estado::{AppState, ContextState, Fase, NivelFeedback};
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
/// cédula tecleada (para volver a la pantalla de contraseña si falla).
type AutenticacionPendiente = Option<(
    String,
    mpsc::Receiver<Result<UsuarioSesion, AutenticacionError>>,
)>;

/// Guard de terminal propio de esta interfaz — el de `tui::terminal` es
/// privado y la restricción del proyecto es no tocar la TUI clásica, así que
/// se replica lo mínimo (mismo título, misma restauración al salir).
struct TerminalGuard;

impl TerminalGuard {
    fn acquire() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout(), EnterAlternateScreen, Hide, SetTitle("BRISAS CLI")) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

/// Punto de entrada de la interfaz de comandos. Consume el `AppCore` (la ruta
/// `--comandos` de `main` no hace nada más con él después).
pub fn run(core: AppCore) -> Result<(), ComandosError> {
    if core.requiere_configuracion_inicial()? {
        return avisar_configuracion_inicial();
    }

    let _guard = TerminalGuard::acquire()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = AppState::new();
    let mut autenticacion: AutenticacionPendiente = None;
    recomputar(&core, &mut app);

    while !app.salir {
        recibir_autenticacion(&core, &mut app, &mut autenticacion);
        terminal.draw(|frame| render::render(frame, &app))?;
        if crossterm::event::poll(Duration::from_millis(80))?
            && let Event::Key(key) = crossterm::event::read()?
        {
            manejar_tecla(&core, &mut app, key, &mut autenticacion);
        }
        app.expirar_feedback();
    }
    Ok(())
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
                 Cierre y arranque sin --comandos para crear el usuario ROOT inicial.\n\n\
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
fn recomputar(core: &AppCore, app: &mut AppState) {
    if !matches!(app.fase, Fase::Operando { .. }) {
        return;
    }
    let entrada = parser::parsear(app.input.value());
    app.contexto = resolver::resolver(core, &entrada);
    app.sugerencias = resolver::calcular_sugerencias(core, app.input.value(), &entrada);
}

/// Recoge el resultado del hilo de Argon2 cuando llega. La sesión que viaja
/// por el canal es un snapshot de antes de la verificación: se revalida contra
/// SQLite (rápido, sin Argon2) antes de aceptarla.
fn recibir_autenticacion(
    core: &AppCore,
    app: &mut AppState,
    pendiente: &mut AutenticacionPendiente,
) {
    let Some((_, receptor)) = pendiente.as_ref() else {
        return;
    };
    let Ok(resultado) = receptor.try_recv() else {
        return;
    };
    let Some((cedula, _)) = pendiente.take() else {
        return;
    };
    let resultado = resultado.and_then(|sesion| {
        core.buscar_candidato_autenticacion(&sesion.cedula)
            .map(|candidato| candidato.sesion)
    });
    match resultado {
        Ok(sesion) => {
            let nombre = sesion.nombre.clone();
            app.fase = Fase::Operando { sesion };
            app.mostrar_feedback(format!("Bienvenido, {nombre}"), NivelFeedback::Exito);
            recomputar(core, app);
        }
        Err(error) => {
            app.fase = Fase::LoginPassword {
                cedula,
                error: Some(error.to_string()),
            };
        }
    }
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
        Fase::LoginCedula { .. } => manejar_login_cedula(app, key),
        Fase::LoginPassword { cedula, .. } => {
            manejar_login_password(core, app, key, cedula, autenticacion)
        }
        Fase::Verificando => {}
        Fase::Operando { .. } => manejar_operando(core, app, key),
    }
}

fn manejar_login_cedula(app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let cedula = app.input.value().trim().to_string();
            if cedula.is_empty() {
                app.fase = Fase::LoginCedula {
                    error: Some("Escriba su cédula".to_string()),
                };
                return;
            }
            app.input.reset();
            app.fase = Fase::LoginPassword {
                cedula,
                error: None,
            };
        }
        KeyCode::Esc => {
            app.input.reset();
            app.fase = Fase::LoginCedula { error: None };
        }
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some() {
                app.fase = Fase::LoginCedula { error: None };
            }
        }
    }
}

fn manejar_login_password(
    core: &AppCore,
    app: &mut AppState,
    key: KeyEvent,
    cedula: String,
    autenticacion: &mut AutenticacionPendiente,
) {
    match key.code {
        KeyCode::Enter => {
            let password = app.input.value().to_string();
            if password.is_empty() {
                app.fase = Fase::LoginPassword {
                    cedula,
                    error: Some("Escriba su contraseña".to_string()),
                };
                return;
            }
            // La parte que toca SQLite es rápida y va en este hilo; Argon2
            // (cientos de ms) corre aparte para no congelar la interfaz.
            match core.buscar_candidato_autenticacion(&cedula) {
                Ok(candidato) => {
                    let (emisor, receptor) = mpsc::channel();
                    std::thread::spawn(move || {
                        let resultado = crate::services::autenticacion_service::verificar_candidato(
                            candidato, &password,
                        );
                        let _ = emisor.send(resultado);
                    });
                    *autenticacion = Some((cedula, receptor));
                    app.input.reset();
                    app.fase = Fase::Verificando;
                }
                Err(error) => {
                    app.input.reset();
                    app.fase = Fase::LoginPassword {
                        cedula,
                        error: Some(error.to_string()),
                    };
                }
            }
        }
        KeyCode::Esc => {
            app.input.reset();
            app.fase = Fase::LoginCedula { error: None };
        }
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some() {
                app.fase = Fase::LoginPassword {
                    cedula,
                    error: None,
                };
            }
        }
    }
}

fn manejar_operando(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        // Esc y Ctrl+L: limpiar todo y volver a Inicio.
        KeyCode::Esc => {
            app.input.reset();
            app.feedback = None;
            recomputar(core, app);
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input.reset();
            app.feedback = None;
            recomputar(core, app);
        }
        KeyCode::Up => mover_seleccion(app, -1),
        KeyCode::Down => mover_seleccion(app, 1),
        KeyCode::Tab => {
            if let Some(nuevo) = resolver::autocompletar(core, app.input.value()) {
                app.input = Input::new(nuevo);
                recomputar(core, app);
            }
        }
        KeyCode::Enter => confirmar(core, app),
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some() {
                // Escribir de nuevo despeja el feedback transitorio.
                app.feedback = None;
                recomputar(core, app);
            }
        }
    }
}

fn mover_seleccion(app: &mut AppState, delta: isize) {
    let ajustar = |seleccion: &mut usize, total: usize| {
        if total == 0 {
            return;
        }
        let actual = *seleccion as isize;
        *seleccion = (actual + delta).clamp(0, total as isize - 1) as usize;
    };
    match &mut app.contexto {
        ContextState::Coincidencias {
            items, seleccion, ..
        } => ajustar(seleccion, items.len()),
        ContextState::CoincidenciasActivos {
            items, seleccion, ..
        } => ajustar(seleccion, items.len()),
        _ => {}
    }
}

/// Enter: selecciona la coincidencia marcada o confirma la tarjeta vigente.
/// Las escrituras reales (`registrar_ingreso`/`registrar_salida`) sólo ocurren
/// aquí — nada de lo que se muestra mientras se teclea persiste nada.
fn confirmar(core: &AppCore, app: &mut AppState) {
    let entrada = parser::parsear(app.input.value());
    match app.contexto.clone() {
        ContextState::Coincidencias {
            items, seleccion, ..
        } => {
            let Some(item) = items.get(seleccion) else {
                return;
            };
            let comando = match &entrada {
                Entrada::Comando { comando, .. } => *comando,
                _ => Comando::Buscar,
            };
            app.contexto = if comando == Comando::Ingreso {
                let (gafete, medio) = parametros_ingreso(&entrada);
                resolver::preparar_resumen_ingreso(core, item.id, gafete, medio)
            } else {
                resolver::ficha_desde_resumen(item.clone())
            };
        }
        ContextState::CoincidenciasActivos {
            items, seleccion, ..
        } => {
            if let Some(item) = items.get(seleccion) {
                app.contexto = ContextState::ResumenSalida {
                    activo: item.clone(),
                };
            }
        }
        ContextState::ResumenIngreso {
            preparacion,
            gafete,
            medio,
            ..
        } => {
            if !app.contexto.ingreso_confirmable() {
                return;
            }
            let Fase::Operando { sesion } = &app.fase else {
                return;
            };
            match core.registrar_ingreso(sesion, preparacion.contratista_id, medio, gafete) {
                Ok(_) => {
                    let gafete_texto = gafete
                        .map(|numero| format!(" — Gafete {numero}"))
                        .unwrap_or_default();
                    app.mostrar_feedback(
                        format!(
                            "Ingreso registrado — {}{gafete_texto} — {}",
                            preparacion.nombre,
                            hora_actual_texto()
                        ),
                        NivelFeedback::Exito,
                    );
                    app.input.reset();
                    recomputar(core, app);
                }
                Err(error) => {
                    app.mostrar_feedback(mensaje_error_ingreso(&error), NivelFeedback::Error);
                }
            }
        }
        ContextState::ResumenSalida { activo } => {
            let Fase::Operando { sesion } = &app.fase else {
                return;
            };
            match core.registrar_salida(sesion, activo.registro_id) {
                Ok(()) => {
                    let detalle = activo
                        .gafete_numero
                        .map(|numero| format!(" — Gafete {numero} liberado"))
                        .unwrap_or_default();
                    app.mostrar_feedback(
                        format!("Salida registrada — {}{detalle}", activo.contratista_nombre),
                        NivelFeedback::Exito,
                    );
                    app.input.reset();
                    recomputar(core, app);
                }
                Err(error) => {
                    app.mostrar_feedback(mensaje_error_salida(&error), NivelFeedback::Error);
                }
            }
        }
        _ => {}
    }
}

/// Extrae gafete y medio del parseo con los valores ya validados (los inválidos
/// nunca llegan acá: el resolver los convierte en `MensajeError` antes).
fn parametros_ingreso(entrada: &Entrada) -> (Option<i64>, MedioIngreso) {
    match entrada {
        Entrada::Comando { gafete, medio, .. } => {
            let gafete = match gafete {
                Some(GafeteParse::Valido(numero)) => Some(*numero),
                _ => None,
            };
            let medio = match medio {
                Some(MedioParse::Valido(medio)) => *medio,
                _ => MedioIngreso::Caminando,
            };
            (gafete, medio)
        }
        _ => (None, MedioIngreso::Caminando),
    }
}

/// Mensajes operativos en español, mismo criterio que
/// `tui::app::error_messages` (que es privado de la TUI clásica): los errores
/// semánticos conservan su texto accionable y los de base de datos no exponen
/// detalles internos.
fn mensaje_error_ingreso(error: &RegistroIngresoServiceError) -> String {
    use RegistroIngresoServiceError::*;
    match error {
        ContratistaNoEncontrado => "El contratista ya no existe".into(),
        IngresoActivo => "El contratista ya tiene un ingreso activo".into(),
        GafeteRequerido => "El gafete es requerido".into(),
        GafeteOcupado => "El gafete ya está en uso".into(),
        AccesoDenegado(_) => format!("Acceso denegado: {}", motivo_texto(error)),
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        _ => "No se pudo registrar el ingreso".into(),
    }
}

fn motivo_texto(error: &RegistroIngresoServiceError) -> String {
    use crate::domain::resultado_acceso::MotivoDenegacion;
    match error {
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::SinAcceso) => {
            "no tiene acceso autorizado".into()
        }
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::PraindVencido) => {
            "PRAIND vencido".into()
        }
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::PraindNoRegistrado) => {
            "PRAIND sin fecha registrada".into()
        }
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::EmpresaInactiva) => {
            "la empresa está inactiva".into()
        }
        _ => String::new(),
    }
}

fn mensaje_error_salida(error: &RegistroIngresoServiceError) -> String {
    use RegistroIngresoServiceError::*;
    match error {
        RegistroNoActivo => "El ingreso ya no está activo".into(),
        SalidaAnteriorAIngreso => "La salida no puede ser anterior al ingreso".into(),
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        _ => "No se pudo registrar la salida".into(),
    }
}
