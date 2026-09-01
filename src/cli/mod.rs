//! CLI: interfaz de terminal dirigida por comandos — ruta por defecto de la
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
mod formulario_password;
mod formulario_password_controller;
mod formulario_usuario;
mod formulario_usuario_controller;
mod historial;
mod historial_controller;
mod login;
mod operando;
mod preferencias;
mod presentation;
mod query_lang;
mod render;
mod root;
mod salida_gafete;
mod salida_gafete_controller;
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
use crate::services::usuario_service::CrearRootInicialInput;

pub use breakpoint::Breakpoint;
pub use columnas::{Columna, ColumnaActivos, ColumnaBusqueda, ColumnaHistorial, SelectorColumnas};
pub use estado::{
    AppState, ContextState, EdicionColumnas, Fase, NivelFeedback, ObjetivoColumnas, SurfaceActiva,
};
pub use formulario::{
    Campo, FormularioContratista, MAX_VISIBLES_EMPRESAS, ModoFormulario, Subfase,
};
pub use formulario_empresa::FormularioEmpresa;
pub use formulario_password::FormularioPassword;
pub use formulario_usuario::{CampoUsuario, FormularioUsuario, SubfaseUsuario};
pub use historial::HistorialState;
pub use salida_gafete::SalidaGafeteState;

// Re-exportado desde `lenguaje_comandos` (sin dependencia de terminal) para
// no romper a quien ya importaba estos nombres/rutas como
// `cli::parsear`, `cli::parser::parsear`,
// `cli::resolver::pagina_contratistas`, etc. — ver el doc-comment de
// ese módulo. `parser`/`resolver` como alias de módulo (no chocan con la
// función `resolver` de abajo: viven en el namespace de tipos, no el de
// valores).
// `resolver` (el módulo) ya trae consigo `resolver::resolver` (la función) —
// no se re-exporta también suelta para no chocar con el nombre del módulo.
pub use crate::lenguaje_comandos::{
    Comando, Entrada, GafeteParse, MedioParse, autocompletar, calcular_sugerencias,
    ficha_desde_resumen, parsear, preparar_resumen_ingreso,
};
pub use crate::lenguaje_comandos::{parser, resolver};

#[derive(Debug, thiserror::Error)]
pub enum CliError {
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

/// Receptor del hilo que hashea la contraseña del ROOT inicial con Argon2 —
/// mismo patrón que `AutenticacionPendiente`, sobre el alta en vez del login.
/// Viaja junto con `CrearRootInicialInput` (no sólo el hash) porque el
/// insert atómico (`crear_root_inicial_con_hash`) necesita los tres campos,
/// no sólo la contraseña.
type RootPendiente =
    Option<mpsc::Receiver<Result<(CrearRootInicialInput, String), UsuarioServiceError>>>;

/// Receptor del hilo que exporta Historial a XLSX (ver
/// `historial_controller::exportar_en_hilo`) — el destino viaja junto con el
/// resultado, así `historial_controller::recibir_exportacion_si_lista` no
/// necesita guardarlo aparte mientras el hilo está en vuelo.
type HistorialExportacionPendiente =
    Option<mpsc::Receiver<(Result<usize, String>, std::path::PathBuf)>>;

/// Punto de entrada de la CLI. Consume el `AppCore` (la ruta
/// por defecto de `main` no hace nada más con él después).
///
/// `sesion_inicial` conserva el soporte para arrancar directo en
/// `Fase::Operando`, pero el cambio de entorno desde la TUI clásica hoy
/// relanza el proceso y vuelve a pedir login. Con `None` (ruta por defecto,
/// sin flags) hace su propio login, como siempre.
/// Devuelve `true` cuando el operador pidió `/clasico`: la preferencia de
/// interfaz ya quedó guardada en disco (`interfaz_preferida::guardar`,
/// llamado desde `operando.rs` al confirmar) — el `bool` sólo le dice a
/// `main.rs` que además tiene que relanzar el ejecutable en la TUI clásica.
pub fn run(core: AppCore, sesion_inicial: Option<UsuarioSesion>) -> Result<bool, CliError> {
    let requiere_configuracion_inicial = core.requiere_configuracion_inicial()?;

    let _guard = TerminalGuard::acquire()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = match sesion_inicial {
        Some(sesion) => AppState::con_sesion(sesion),
        // Base sin ningún usuario todavía: arranca en la cadena Root* en vez
        // del login (ver `estado::Fase`). `sesion_inicial` sólo llega con
        // `Some` desde la TUI clásica, que ya exige que exista un ROOT para
        // autenticarse — de ahí que este caso sólo aplique con `None`.
        None if requiere_configuracion_inicial => AppState::nueva_configuracion_inicial(),
        None => AppState::new(),
    };
    // Preferencias propias de --cli (hoy sólo columnas visibles),
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
    let mut root_pendiente: RootPendiente = None;
    let mut historial_exportacion_pendiente: HistorialExportacionPendiente = None;
    recomputar(&core, &mut app);

    // Redraw-on-demand: sólo se dibuja cuando algo realmente cambió. La
    // espera del poll es dinámica (ver `proxima_espera`) para no quemar CPU
    // redibujando el mismo frame en reposo, sin sacrificar respuesta al
    // teclado (el evento se procesa y se dibuja en la misma vuelta en que
    // llega, nunca atado a un tick fijo).
    let mut redibujar = true;
    // Si la última vuelta dibujó con una animación en curso, la vuelta que
    // la encuentra ya terminada tiene que dibujar una vez más — sin este
    // flag, el bucle deja de redibujar en la MISMA vuelta en que
    // `activo()` pasa a `false`, y el último frame pintado queda un pelín
    // antes del 100% (el redibujado anterior fue con el reloj de esa
    // vuelta, no el de ésta). Quedaba "congelado" ahí — visible sobre todo
    // en filas con `EaseOut` cerca del final, donde el cambio por tick ya
    // es mínimo — hasta que una tecla forzaba el próximo redibujado.
    let mut animacion_activa_previa = false;
    // El cursor propio del prompt parpadea (`render::blink_on`) mientras haya
    // una línea de input activa — hoy sólo en `Fase::Operando` (login usa su
    // propio "_" fijo, sin parpadeo). Sin este chequeo el redraw-on-demand
    // nunca despertaría solo para alternar el parpadeo entre teclas.
    let mut blink_previo = false;
    while !app.salir {
        if login::recibir_autenticacion(&core, &mut app, &mut autenticacion) {
            redibujar = true;
        }
        if root::recibir_root_creado(&core, &mut app, &mut root_pendiente) {
            redibujar = true;
        }
        if historial_controller::recibir_exportacion_si_lista(
            &mut app,
            &mut historial_exportacion_pendiente,
        ) {
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
        // aunque no haya llegado ningún evento nuevo — y una vuelta más
        // cuando termina, para que el frame final refleje el valor
        // realmente asentado (ver el comentario de `animacion_activa_previa`).
        let animacion_activa = app.presentacion.activo();
        if animacion_activa || animacion_activa_previa {
            redibujar = true;
        }
        animacion_activa_previa = animacion_activa;

        if matches!(app.fase, Fase::Operando { .. }) {
            let blink_ahora = render::blink_on(&app);
            if blink_ahora != blink_previo {
                redibujar = true;
            }
            blink_previo = blink_ahora;
        }

        if redibujar {
            terminal.draw(|frame| render::render(frame, &app))?;
            redibujar = false;
        }

        let espera = proxima_espera(
            &app,
            &autenticacion,
            &root_pendiente,
            &historial_exportacion_pendiente,
        );
        if crossterm::event::poll(espera)? {
            match crossterm::event::read()? {
                Event::Key(key) => {
                    manejar_tecla(
                        &core,
                        &mut app,
                        key,
                        &mut autenticacion,
                        &mut root_pendiente,
                        &mut historial_exportacion_pendiente,
                    );
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
    Ok(app.reiniciar_en_clasica)
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
    let contexto = actualizar_presentacion_contexto(app);
    let prompt_glifo = actualizar_presentacion_prompt_glifo(app);
    login || formulario || historial || contexto || prompt_glifo
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

/// Mismo mecanismo sobre `firma_contexto` (DEC-059) — el área de contexto
/// funde al cambiar de "tipo de pantalla" (Inicio → resultados, resultados
/// → tarjeta de confirmación…). Con una Surface abierta el área de
/// contexto ni se dibuja (`render.rs` renderiza la Surface en su lugar),
/// así que no hace falta pedir una aparición que nadie vería — cuando la
/// Surface cierre y el contexto vuelva a mostrarse, la firma habrá
/// cambiado igual (el reset que hace cada `cerrar_*` ya lo garantiza) y
/// se detectará en el frame siguiente sin ayuda especial acá.
fn actualizar_presentacion_contexto(app: &mut AppState) -> bool {
    if app.surface_activa() != SurfaceActiva::Ninguna {
        return false;
    }
    let firma_actual = app.firma_contexto();
    if app.firma_contexto_previa == Some(firma_actual) {
        return false;
    }
    app.presentacion.aparecer("area_contexto", app.calidad);
    app.firma_contexto_previa = Some(firma_actual);
    true
}

/// El `> ` de la línea de comandos (sin ninguna Surface abierta) muta al
/// símbolo de feedback vigente (✓/!/×) mientras dure, y funde al aparecer —
/// DEC-060. Sólo aplica en `Operando`: el login tiene su propio glifo de
/// feedback, ya cubierto por `actualizar_presentacion_login`.
fn actualizar_presentacion_prompt_glifo(app: &mut AppState) -> bool {
    if !matches!(app.fase, Fase::Operando { .. }) {
        return false;
    }
    let visible_ahora = app.feedback_vigente().is_some();
    if visible_ahora == app.prompt_glifo_previo {
        return false;
    }
    if visible_ahora {
        app.presentacion.aparecer("prompt_glifo", app.calidad);
    }
    app.prompt_glifo_previo = visible_ahora;
    true
}

/// Cuánto puede esperar el próximo `poll` antes de que el loop necesite
/// revisar algo por su cuenta (sin que haya llegado un evento de teclado).
///
/// - Con Argon2 corriendo, hay que revisar el canal seguido.
/// - Con una animación en curso, al ritmo de frame que le corresponde.
/// - Con feedback transitorio visible, sólo hace falta despertar cuando
///   está por expirar.
/// - Con el prompt visible (`Operando`), despertar justo cuando toca el
///   próximo toggle del parpadeo del cursor (ver `render::blink_on`) — ni
///   antes (CPU de más) ni después (parpadeo perceptiblemente atrasado).
/// - En reposo (login, verificando…), esperar casi indefinidamente: el
///   teclado despierta el poll de inmediato, no hace falta sondear nada más.
fn proxima_espera(
    app: &AppState,
    autenticacion: &AutenticacionPendiente,
    root_pendiente: &RootPendiente,
    historial_exportacion_pendiente: &HistorialExportacionPendiente,
) -> Duration {
    const ESPERA_VERIFICACION: Duration = Duration::from_millis(30);
    // ~30 fps: de sobra para una transición de texto/color; no hay razón
    // para gastar más CPU persiguiendo 60 en una interfaz de terminal.
    const ESPERA_ANIMACION: Duration = Duration::from_millis(33);
    const ESPERA_REPOSO: Duration = Duration::from_secs(60 * 60);

    if autenticacion.is_some()
        || root_pendiente.is_some()
        || historial_exportacion_pendiente.is_some()
    {
        return ESPERA_VERIFICACION;
    }
    if app.presentacion.activo() {
        return ESPERA_ANIMACION;
    }
    if let Some(restante) = app.feedback_restante() {
        return restante.max(Duration::from_millis(1));
    }
    if matches!(app.fase, Fase::Operando { .. }) {
        let periodo = estado::PERIODO_BLINK_MS;
        let transcurrido_ms = app.instante_inicio.elapsed().as_millis() as u64;
        let resto = periodo - (transcurrido_ms % periodo);
        return Duration::from_millis(resto.max(1));
    }
    ESPERA_REPOSO
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
    root_pendiente: &mut RootPendiente,
    historial_exportacion_pendiente: &mut HistorialExportacionPendiente,
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
        Fase::Verificando { .. } | Fase::RootCreando { .. } => {}
        Fase::RootCedula => root::manejar_root_cedula(app, key),
        Fase::RootNombre { cedula } => root::manejar_root_nombre(app, key, cedula),
        Fase::RootPassword { cedula, nombre } => {
            root::manejar_root_password(app, key, cedula, nombre)
        }
        Fase::RootConfirmarPassword {
            cedula,
            nombre,
            password,
        } => root::manejar_root_confirmar_password(
            app,
            key,
            core,
            cedula,
            nombre,
            password,
            root_pendiente,
        ),
        Fase::Operando { .. } => {
            operando::manejar_operando(core, app, key, historial_exportacion_pendiente)
        }
    }
}
