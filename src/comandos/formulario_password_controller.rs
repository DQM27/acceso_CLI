//! Controlador de teclado del formulario de "cambiar mi contraseña" (`/clave`).
//!
//! A diferencia de `login.rs` (que verifica con Argon2 en un hilo aparte
//! porque corre en el camino más frecuente de toda la app), verificar la
//! contraseña actual acá es una acción explícita y poco frecuente — mismo
//! criterio ya aplicado en `formulario_usuario_controller.rs` para el hash
//! de alta de usuario: no se justifica la plomería de un hilo aparte para
//! algo que no está en el camino caliente.

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::services::error::UsuarioServiceError;

use super::formulario_password::{FormularioPassword, SubfasePassword};
use super::{AppState, Fase, NivelFeedback};

/// Cualquier sesión vigente puede cambiar su propia contraseña — no hay gate
/// de rol como en `formulario_usuario_controller` (gestionar a otros sí
/// requiere permiso; cambiar la propia, no).
pub(super) fn abrir_formulario_cambio_password(app: &mut AppState) {
    app.formulario_password = Some(FormularioPassword::nuevo());
    app.input.reset();
    app.feedback = None;
    app.sugerencias.clear();
}

fn cerrar_formulario_password(core: &AppCore, app: &mut AppState) {
    app.formulario_password = None;
    app.input.reset();
    super::recomputar(core, app);
}

fn sincronizar_input(app: &mut AppState) {
    let texto = app
        .formulario_password
        .as_ref()
        .map(|f| f.texto_campo().to_string())
        .unwrap_or_default();
    app.input = Input::new(texto);
}

pub(super) fn manejar_formulario_password(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    let Some(form) = &app.formulario_password else {
        return;
    };
    match form.subfase {
        SubfasePassword::VerificandoActual => manejar_verificando_actual(core, app, key),
        SubfasePassword::Cambiando => manejar_cambiando(core, app, key),
    }
}

fn manejar_verificando_actual(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => cerrar_formulario_password(core, app),
        KeyCode::Enter => verificar_actual(core, app),
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some()
                && let Some(form) = &mut app.formulario_password
            {
                form.asignar_texto(app.input.value());
            }
        }
    }
}

/// Única lectura de esta Surface (no escribe nada) — sólo confirma que quien
/// está tecleando conoce la contraseña vigente antes de mostrarle los
/// campos de la nueva.
fn verificar_actual(core: &AppCore, app: &mut AppState) {
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    let Some(form) = &app.formulario_password else {
        return;
    };
    if form.actual.is_empty() {
        return;
    }
    match core.verificar_mi_password(sesion, &form.actual) {
        Ok(()) => {
            if let Some(form) = &mut app.formulario_password {
                form.avanzar_a_cambiar();
            }
            sincronizar_input(app);
        }
        Err(error) => {
            if let Some(form) = &mut app.formulario_password {
                form.rechazar_actual(mensaje_error(&error));
            }
            sincronizar_input(app);
        }
    }
}

fn manejar_cambiando(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    if app.formulario_password.is_none() {
        return;
    }
    match key.code {
        // Esc vuelve al primer paso (no cierra la Surface entera) — pedir
        // de nuevo la actual es más seguro que dejar el gate ya superado
        // abierto indefinidamente si el operador se arrepiente a medio
        // escribir la nueva.
        KeyCode::Esc => {
            app.formulario_password = Some(FormularioPassword::nuevo());
            sincronizar_input(app);
        }
        KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
            let cambio = app
                .formulario_password
                .as_mut()
                .is_some_and(FormularioPassword::alternar_campo);
            if cambio {
                sincronizar_input(app);
            }
        }
        KeyCode::Enter => confirmar_cambio(core, app),
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some()
                && let Some(form) = &mut app.formulario_password
            {
                form.asignar_texto(app.input.value());
            }
        }
    }
}

fn confirmar_cambio(core: &AppCore, app: &mut AppState) {
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    let Some(form) = &mut app.formulario_password else {
        return;
    };
    if form.validar_nueva().is_err() {
        return;
    }
    let actual = form.actual.clone();
    let nueva = form.nueva.clone();
    // Vuelve a verificar la actual de punta a punta junto con el hash de la
    // nueva (`cambiar_mi_password`) en vez de reusar el resultado del primer
    // paso: si la cuenta cambió entre medio (contraseña reseteada por un
    // admin, por ejemplo) se detecta con el dato más fresco posible, mismo
    // criterio que ya usa `login.rs` al revalidar antes de aceptar la sesión.
    match core.cambiar_mi_password(sesion, &actual, &nueva) {
        Ok(()) => {
            cerrar_formulario_password(core, app);
            app.mostrar_feedback("Contraseña actualizada".to_string(), NivelFeedback::Exito);
        }
        Err(error) => {
            let mensaje = mensaje_error(&error);
            if let Some(form) = &mut app.formulario_password {
                // Si la actual dejó de ser válida en el ínterin, de vuelta
                // al primer paso — no tiene sentido seguir pidiendo una
                // nueva contraseña sobre un gate que ya no vale.
                if matches!(error, UsuarioServiceError::PasswordActualIncorrecta) {
                    *form = FormularioPassword::nuevo();
                    form.rechazar_actual(mensaje);
                } else {
                    form.error = Some(mensaje);
                }
            }
            sincronizar_input(app);
        }
    }
}

fn mensaje_error(error: &UsuarioServiceError) -> String {
    match error {
        UsuarioServiceError::PasswordActualIncorrecta => {
            "Contraseña actual incorrecta".to_string()
        }
        UsuarioServiceError::OperacionNoAutorizada => "La sesión ya no es válida".to_string(),
        other => other.to_string(),
    }
}
