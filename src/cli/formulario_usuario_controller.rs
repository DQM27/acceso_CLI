//! Controlador de teclado del formulario de usuario (`--cli`) — mismo
//! patrón que `formulario_controller.rs` (Enter confirma desde cualquier
//! campo, DEC-025; Space/←/→ alterna Rol).
//!
//! A diferencia del login, que verifica la contraseña con Argon2 en un
//! hilo aparte (`login.rs`) porque corre en el camino más frecuente de
//! toda la app, crear un usuario hashea de forma síncrona: es una acción
//! administrativa poco frecuente, y threading esto igual que el login
//! exigiría plomería nueva (un tipo de estado pendiente propio, atravesar
//! `mod.rs`/`operando.rs`/`manejar_tecla`) que no se justifica todavía para
//! algo que no está en el camino caliente. El bloqueo real es de cientos de
//! ms, una sola vez, en una acción explícita — no tecla a tecla.

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::database::queries::usuarios::UsuarioResumen;
use crate::domain::autorizacion::Operacion;

use super::formulario_usuario::{CampoUsuario, DatosUsuario, FormularioUsuario, SubfaseUsuario};
use super::{AppState, Fase, NivelFeedback};

/// Sólo abre si el actor puede gestionar usuarios (`Operacion::GestionarUsuarios`)
/// — Operador nunca llega a ver este formulario, ni siquiera vacío.
pub(super) fn abrir_formulario_nuevo_usuario(app: &mut AppState) {
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    if !sesion.rol.puede(Operacion::GestionarUsuarios) {
        app.mostrar_feedback(
            "No tiene permiso para gestionar usuarios".to_string(),
            NivelFeedback::Error,
        );
        return;
    }
    app.formulario_usuario = Some(FormularioUsuario::nuevo(sesion.rol));
    app.input.reset();
    app.feedback = None;
    app.sugerencias.clear();
}

/// Mismo gate que `abrir_formulario_nuevo_usuario` — `CoincidenciasUsuarios`
/// ya lo aplicó en `resolver_busqueda_usuarios`, pero se repite acá como
/// defensa en profundidad, igual criterio que el resto de aperturas de
/// Surface. La búsqueda que trajo hasta acá ya trae cédula/nombre/rol, no
/// hace falta otra consulta.
pub(super) fn abrir_formulario_editar_usuario(app: &mut AppState, resumen: &UsuarioResumen) {
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    if !sesion.rol.puede(Operacion::GestionarUsuarios) {
        app.mostrar_feedback(
            "No tiene permiso para gestionar usuarios".to_string(),
            NivelFeedback::Error,
        );
        return;
    }
    app.formulario_usuario = Some(FormularioUsuario::editar(resumen, sesion.rol));
    app.feedback = None;
    app.sugerencias.clear();
    sincronizar_input(app);
}

fn cerrar_formulario_usuario(core: &AppCore, app: &mut AppState) {
    app.formulario_usuario = None;
    app.input.reset();
    super::recomputar(core, app);
}

fn sincronizar_input(app: &mut AppState) {
    let texto = app
        .formulario_usuario
        .as_ref()
        .and_then(|f| f.texto_campo().map(str::to_string))
        .unwrap_or_default();
    app.input = Input::new(texto);
}

fn mover_campo(app: &mut AppState, delta: isize) {
    let cambio = match &mut app.formulario_usuario {
        Some(f) => f.mover_campo(delta),
        None => false,
    };
    if cambio {
        sincronizar_input(app);
    }
}

pub(super) fn manejar_formulario_usuario(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    let Some(form) = &app.formulario_usuario else {
        return;
    };
    match form.subfase {
        SubfaseUsuario::Editando => manejar_edicion(core, app, key),
        SubfaseUsuario::Resumen => manejar_resumen(core, app, key),
    }
}

fn manejar_edicion(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    let Some(campo) = app.formulario_usuario.as_ref().map(|f| f.campo) else {
        return;
    };
    match key.code {
        KeyCode::Esc => cerrar_formulario_usuario(core, app),
        KeyCode::Up => mover_campo(app, -1),
        KeyCode::Down => mover_campo(app, 1),
        // Enter siempre intenta confirmar el formulario completo, sin
        // importar en qué campo esté el operador — mismo criterio que el
        // formulario de contratista (DEC-025).
        KeyCode::Enter => {
            if let Some(form) = &mut app.formulario_usuario
                && form.validar().is_ok()
            {
                form.subfase = SubfaseUsuario::Resumen;
                app.input.reset();
            }
        }
        KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') if campo == CampoUsuario::Rol => {
            if let Some(form) = &mut app.formulario_usuario {
                form.alternar();
            }
        }
        _ => {
            if campo.es_texto()
                && app.input.handle_event(&Event::Key(key)).is_some()
                && let Some(form) = &mut app.formulario_usuario
            {
                form.asignar_texto(app.input.value());
                let saneado = form.texto_campo().unwrap_or_default();
                if saneado != app.input.value() {
                    app.input = Input::new(saneado.to_string());
                }
            }
        }
    }
}

fn manejar_resumen(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if let Some(form) = &mut app.formulario_usuario {
                form.subfase = SubfaseUsuario::Editando;
            }
            sincronizar_input(app);
        }
        KeyCode::Enter => guardar_formulario_usuario(core, app),
        _ => {}
    }
}

/// Única escritura de esta Surface — nada persiste hasta el Enter en el
/// Resumen, igual criterio que el resto de la app.
fn guardar_formulario_usuario(core: &AppCore, app: &mut AppState) {
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    let Some(form) = &mut app.formulario_usuario else {
        return;
    };
    let nombre = form.nombre.trim().to_string();
    let datos = match form.validar() {
        Ok(datos) => datos,
        // No debería pasar (el resumen sólo se abre tras validar), pero si
        // pasa se vuelve a editar con los errores marcados.
        Err(_) => {
            form.subfase = SubfaseUsuario::Editando;
            return;
        }
    };
    match datos {
        DatosUsuario::Crear(input) => match core.crear_usuario(sesion, input) {
            Ok(_) => {
                cerrar_formulario_usuario(core, app);
                app.mostrar_feedback(
                    format!("Usuario registrado — {nombre}"),
                    NivelFeedback::Exito,
                );
            }
            Err(error) => {
                if let Some(form) = &mut app.formulario_usuario {
                    form.subfase = SubfaseUsuario::Editando;
                }
                sincronizar_input(app);
                app.mostrar_feedback(error.to_string(), NivelFeedback::Error);
            }
        },
        DatosUsuario::Actualizar {
            id,
            datos,
            activo,
            password,
        } => match core.actualizar_usuario(sesion, id, datos, activo) {
            Ok(()) => {
                // La contraseña es una escritura aparte (`cambiar_password_usuario`,
                // otra transacción) — si falla, los demás campos ya quedaron
                // guardados, así que se avisa sin deshacer nada.
                if let Some(nueva) = password
                    && let Err(error) = core.cambiar_password_usuario(sesion, id, &nueva)
                {
                    cerrar_formulario_usuario(core, app);
                    app.mostrar_feedback(
                        format!(
                            "Usuario actualizado, pero la contraseña no se pudo cambiar: {error}"
                        ),
                        NivelFeedback::Advertencia,
                    );
                    return;
                }
                cerrar_formulario_usuario(core, app);
                app.mostrar_feedback(
                    format!("Usuario actualizado — {nombre}"),
                    NivelFeedback::Exito,
                );
            }
            Err(error) => {
                if let Some(form) = &mut app.formulario_usuario {
                    form.subfase = SubfaseUsuario::Editando;
                }
                sincronizar_input(app);
                app.mostrar_feedback(error.to_string(), NivelFeedback::Error);
            }
        },
    }
}
