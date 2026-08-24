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

mod breakpoint;
mod columnas;
mod estado;
mod formulario;
mod formulario_controller;
mod formulario_empresa;
mod formulario_empresa_controller;
mod formulario_usuario;
mod formulario_usuario_controller;
mod historial;
mod historial_controller;
mod login;
mod operando;
mod parser;
mod preferencias;
mod presentation;
mod query_lang;
mod render;
mod resolver;
mod terminal;

/// Nombre visual de la app en la escena de login — única fuente de verdad:
/// `render.rs` lo pinta, `estado.rs::firma_login` lo usa para detectar
/// cuándo el título mutó hacia/desde la identidad del operador.
const NOMBRE_APP: &str = "Brisas CLI";

use std::io::{self, stdout};
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::CrosstermBackend};

use terminal::TerminalGuard;

use crate::application::AppCore;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::{AutenticacionError, UsuarioServiceError};

pub use breakpoint::Breakpoint;
pub use columnas::{Columna, ColumnaActivos, ColumnaBusqueda, ColumnaHistorial, SelectorColumnas};
pub use estado::{
    AppState, ContextState, EdicionColumnas, Fase, NivelFeedback, ObjetivoColumnas, SurfaceActiva,
};
pub use formulario::{
    Campo, FormularioContratista, MAX_VISIBLES_EMPRESAS, ModoFormulario, Subfase,
};
pub use formulario_empresa::FormularioEmpresa;
pub use formulario_usuario::{CampoUsuario, FormularioUsuario, SubfaseUsuario};
pub use historial::HistorialState;
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
    // Preferencias propias de --comandos (hoy sólo columnas visibles),
    // archivo independiente del de la TUI clásica (DEC-002/DEC-014). Un
    // disco sin permiso de escritura o sin %LOCALAPPDATA% nunca impide
    // arrancar: sin store, todas las columnas quedan visibles y ese estado
    // simplemente no se persiste.
    let mut preferencias = preferencias::PreferenciasStore::load_default();
    if let Some(store) = &preferencias {
        app.columnas_busqueda
            .aplicar_preferencia(&store.actual().columnas_busqueda);
        app.columnas_activos
            .aplicar_preferencia(&store.actual().columnas_activos);
        app.columnas_historial
            .aplicar_preferencia(&store.actual().columnas_historial);
    }
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
        // Paso "Transition/Animation State" del pipeline (ver
        // docs/lenguaje-visual-mutaciones.md §6), entre actualizar estado y
        // renderizar: detecta mutaciones de contenido y arranca la
        // aparición correspondiente en el motor de presentación.
        if actualizar_presentacion(&mut app) {
            redibujar = true;
        }
        // Con una animación en curso hay que seguir pintando cada tick
        // aunque no haya llegado ningún evento nuevo.
        if app.presentacion.activo() {
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
    if let Some(store) = &mut preferencias {
        let _ = store.guardar_si_cambio(preferencias::Preferencias {
            columnas_busqueda: app.columnas_busqueda.preferencia(),
            columnas_activos: app.columnas_activos.preferencia(),
            columnas_historial: app.columnas_historial.preferencia(),
        });
    }
    Ok(())
}

/// Compara qué debería verse contra lo que se vio la vuelta anterior, por
/// cada Surface con firma propia (login, formulario, Historial — Fase 5), y
/// arranca en el motor de presentación una aparición por cada elemento que
/// mutó de contenido. Nunca se dispara tecla a tecla: ninguna firma incluye
/// texto tecleado, sólo lo que decide qué se ve — typing es instantáneo,
/// como exige DEC-004 en espíritu ("nunca animes el input"). Devuelve
/// `true` si algo cambió en cualquiera de las tres.
fn actualizar_presentacion(app: &mut AppState) -> bool {
    let login = actualizar_presentacion_login(app);
    let formulario = actualizar_presentacion_formulario(app);
    let historial = actualizar_presentacion_historial(app);
    login || formulario || historial
}

fn actualizar_presentacion_login(app: &mut AppState) -> bool {
    let firma_actual = app.firma_login();
    if firma_actual == app.firma_login_previa {
        return false;
    }
    match (&app.firma_login_previa, &firma_actual) {
        // Primer frame de la escena: título y prompt aparecen juntos.
        (None, Some(_)) => {
            app.presentacion.aparecer("titulo", app.calidad);
            app.presentacion.aparecer("prompt", app.calidad);
        }
        (Some(anterior), Some(actual)) => {
            if anterior.titulo != actual.titulo {
                app.presentacion.aparecer("titulo", app.calidad);
            }
            if anterior.prompt != actual.prompt {
                app.presentacion.aparecer("prompt", app.calidad);
            }
            // Sólo la aparición anima; que desaparezca (expira solo, ver
            // `AppState::expirar_feedback`) queda instantáneo por ahora —
            // ver nota en docs/lenguaje-visual-mutaciones.md §14.1.
            if !anterior.feedback && actual.feedback {
                app.presentacion.aparecer("feedback", app.calidad);
            }
        }
        _ => {}
    }
    app.firma_login_previa = firma_actual;
    true
}

/// Mismo mecanismo que el login, sobre `FirmaFormulario`: el campo activo
/// (o el selector de empresa) funde al cambiar de foco, el resumen funde al
/// aparecer, y los `×` de error funden juntos la primera vez que aparecen
/// tras un intento de confirmar — no hay una firma por campo individual, así
/// que todos los errores vigentes comparten una sola aparición.
fn actualizar_presentacion_formulario(app: &mut AppState) -> bool {
    let firma_actual = app.firma_formulario();
    if firma_actual == app.firma_formulario_previa {
        return false;
    }
    match (&app.firma_formulario_previa, &firma_actual) {
        (None, Some(_)) => app.presentacion.aparecer("form_campo", app.calidad),
        (Some(anterior), Some(actual)) => {
            if anterior.campo != actual.campo
                || anterior.en_selector_empresa != actual.en_selector_empresa
            {
                app.presentacion.aparecer("form_campo", app.calidad);
            }
            if !anterior.en_resumen && actual.en_resumen {
                app.presentacion.aparecer("form_resumen", app.calidad);
            }
            if !anterior.tiene_error && actual.tiene_error {
                app.presentacion.aparecer("form_error", app.calidad);
            }
        }
        _ => {}
    }
    app.firma_formulario_previa = firma_actual;
    true
}

/// Mismo mecanismo, sobre `FirmaHistorial`: la tabla de resultados funde al
/// aparecer (o al cambiar de página/consulta — `total` distinto) y la
/// pantalla de exportación funde al abrirse con `F5`.
fn actualizar_presentacion_historial(app: &mut AppState) -> bool {
    let firma_actual = app.firma_historial();
    if firma_actual == app.firma_historial_previa {
        return false;
    }
    if let (Some(anterior), Some(actual)) = (&app.firma_historial_previa, &firma_actual) {
        if (!anterior.tiene_resultado && actual.tiene_resultado) || anterior.total != actual.total {
            app.presentacion
                .aparecer("historial_resultado", app.calidad);
        }
        if !anterior.exportando && actual.exportando {
            app.presentacion.aparecer("historial_exportar", app.calidad);
        }
    }
    app.firma_historial_previa = firma_actual;
    true
}

/// Cuánto puede esperar el próximo `poll` antes de que el loop necesite
/// revisar algo por su cuenta (sin que haya llegado un evento de teclado).
///
/// - Con Argon2 corriendo, hay que revisar el canal seguido.
/// - Con una animación en curso, al ritmo de frame que le corresponde.
/// - Con feedback transitorio visible, sólo hace falta despertar cuando
///   está por expirar.
/// - En reposo, esperar casi indefinidamente: el teclado despierta el poll
///   de inmediato, no hace falta sondear nada más.
fn proxima_espera(app: &AppState, autenticacion: &AutenticacionPendiente) -> Duration {
    const ESPERA_VERIFICACION: Duration = Duration::from_millis(30);
    // ~30 fps: de sobra para una transición de texto/color; no hay razón
    // para gastar más CPU persiguiendo 60 en una interfaz de terminal.
    const ESPERA_ANIMACION: Duration = Duration::from_millis(33);
    const ESPERA_REPOSO: Duration = Duration::from_secs(60 * 60);

    if autenticacion.is_some() {
        return ESPERA_VERIFICACION;
    }
    if app.presentacion.activo() {
        return ESPERA_ANIMACION;
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
    if app.formulario.is_some() {
        return;
    }
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    let entrada = parser::parsear(app.input.value());
    app.contexto = resolver::resolver(core, &entrada, sesion);
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
