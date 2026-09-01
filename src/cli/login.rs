//! Controlador de las fases de login (`LoginCedula` → `LoginPassword` →
//! `Verificando`) de la interfaz de comandos.
//!
//! La cédula se resuelve contra `SQLite` al confirmar con Enter (lectura
//! rápida): eso ya deja saber si el usuario existe y, sobre todo, su
//! nombre — con eso el título muta de "Brisas CLI" a la identidad del
//! operador antes incluso de pedir la contraseña. La contraseña se verifica
//! con Argon2 en un hilo aparte con canal, el mismo patrón de
//! `tui::app::auth_jobs` — la interfaz nunca se congela calculando el hash.
//! Justo antes de spawnear ese hilo se vuelve a resolver la cédula (en vez
//! de reusar el candidato de la identificación): si la cuenta cambió entre
//! que se reconoció la identidad y que se tecleó la contraseña, se detecta
//! con el dato más fresco posible.
//!
//! Los errores (usuario no válido, credenciales inválidas) no viven en la
//! fase: usan `AppState::feedback`, el mismo canal transitorio y
//! autoexpirable de toda la aplicación — así heredan gratis el scheduler de
//! Fase 0 (`proxima_espera` ya sabe despertar cuando el feedback está por
//! vencer) sin inventar un temporizador nuevo sólo para el login.

use std::sync::mpsc;

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;

use super::{AppState, AutenticacionPendiente, Fase, NivelFeedback};

// `match Ok/Err` explícito en vez de `if let/else` (lo que pide este lint)
// se deja a propósito en las tres funciones de este archivo: nombrar los dos
// casos de un `Result` de autenticación es más claro que una condición con
// negativo implícito, y no vale el riesgo de una transcripción manual en
// código de login.
#[allow(clippy::single_match_else)]
pub(super) fn manejar_login_cedula(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let cedula = app.input.value().trim().to_string();
            if cedula.is_empty() {
                return;
            }
            match core.buscar_candidato_autenticacion(&cedula) {
                Ok(candidato) => {
                    app.input.reset();
                    app.feedback = None;
                    app.fase = Fase::LoginPassword {
                        cedula,
                        nombre: candidato.sesion.nombre,
                    };
                }
                Err(_) => {
                    app.input.reset();
                    app.mostrar_feedback("Usuario no válido".to_string(), NivelFeedback::Error);
                }
            }
        }
        KeyCode::Esc => {
            app.input.reset();
            app.feedback = None;
        }
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some() {
                app.feedback = None;
            }
        }
    }
}

#[allow(clippy::single_match_else)]
pub(super) fn manejar_login_password(
    core: &AppCore,
    app: &mut AppState,
    key: KeyEvent,
    cedula: String,
    nombre: String,
    autenticacion: &mut AutenticacionPendiente,
) {
    match key.code {
        KeyCode::Enter => {
            let password = app.input.value().to_string();
            if password.is_empty() {
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
                    *autenticacion = Some((cedula, nombre.clone(), receptor));
                    app.input.reset();
                    app.fase = Fase::Verificando { nombre };
                }
                // La cuenta dejó de ser válida entre identificarse y teclear
                // la contraseña (p. ej. se desactivó): es un fallo de
                // identificación, no de contraseña — vuelve al título de la
                // app, no se queda con una identidad que ya no existe.
                Err(_) => {
                    app.input.reset();
                    app.fase = Fase::LoginCedula;
                    app.mostrar_feedback("Usuario no válido".to_string(), NivelFeedback::Error);
                }
            }
        }
        KeyCode::Esc => {
            app.input.reset();
            app.feedback = None;
            app.fase = Fase::LoginCedula;
        }
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some() {
                app.feedback = None;
            }
        }
    }
}

/// Recoge el resultado del hilo de Argon2 cuando llega. La sesión que viaja
/// por el canal es un snapshot de antes de la verificación: se revalida contra
/// `SQLite` (rápido, sin Argon2) antes de aceptarla. Devuelve `true` si había un
/// resultado y se procesó (el llamador lo usa para saber si hace falta
/// redibujar).
#[allow(clippy::single_match_else)]
pub(super) fn recibir_autenticacion(
    core: &AppCore,
    app: &mut AppState,
    pendiente: &mut AutenticacionPendiente,
) -> bool {
    let Some((_, _, receptor)) = pendiente.as_ref() else {
        return false;
    };
    let Ok(resultado) = receptor.try_recv() else {
        return false;
    };
    let Some((cedula, nombre, _)) = pendiente.take() else {
        return false;
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
            super::recomputar(core, app);
        }
        // La identidad reconocida sobrevive: no se vuelve a pedir la cédula,
        // sólo la contraseña, con el input ya limpio.
        Err(_) => {
            app.fase = Fase::LoginPassword { cedula, nombre };
            app.mostrar_feedback("Credenciales inválidas".to_string(), NivelFeedback::Error);
        }
    }
    true
}
