//! Controlador de la cadena de configuración inicial (`RootCedula` →
//! `RootNombre` → `RootPassword` → `RootConfirmarPassword` → `RootCreando`)
//! de la interfaz de comandos.
//!
//! Misma mecánica que [`super::login`]: un solo input que muta de campo en
//! campo, con Enter para avanzar y Esc para retroceder (descartando lo
//! tecleado en el campo que se abandona). El hasheo de la contraseña corre
//! en un hilo aparte con canal — mismo patrón que el login — para no
//! congelar la interfaz mientras Argon2 trabaja.
//!
//! Sólo se alcanza cuando `requiere_configuracion_inicial()` da true (ver
//! `AppState::nueva_configuracion_inicial`); tras crear el usuario, vuelve a
//! `Fase::LoginCedula` para que el operador recién dado de alta inicie
//! sesión con sus propias credenciales.

use std::sync::mpsc;

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::services::error::UsuarioServiceError;
use crate::services::password::generar_hash;
use crate::services::usuario_service::CrearRootInicialInput;

use super::{AppState, Fase, NivelFeedback, RootPendiente};

pub(super) fn manejar_root_cedula(app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let cedula = app.input.value().trim().to_string();
            if cedula.is_empty() {
                return;
            }
            app.input.reset();
            app.feedback = None;
            app.fase = Fase::RootNombre { cedula };
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

pub(super) fn manejar_root_nombre(app: &mut AppState, key: KeyEvent, cedula: String) {
    match key.code {
        KeyCode::Enter => {
            let nombre = app.input.value().trim().to_string();
            if nombre.is_empty() {
                return;
            }
            app.input.reset();
            app.feedback = None;
            app.fase = Fase::RootPassword { cedula, nombre };
        }
        KeyCode::Esc => {
            app.input.reset();
            app.feedback = None;
            app.fase = Fase::RootCedula;
        }
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some() {
                app.feedback = None;
            }
        }
    }
}

pub(super) fn manejar_root_password(
    app: &mut AppState,
    key: KeyEvent,
    cedula: String,
    nombre: String,
) {
    match key.code {
        KeyCode::Enter => {
            let password = app.input.value().to_string();
            if password.is_empty() {
                return;
            }
            app.input.reset();
            app.feedback = None;
            app.fase = Fase::RootConfirmarPassword {
                cedula,
                nombre,
                password,
            };
        }
        KeyCode::Esc => {
            app.input.reset();
            app.feedback = None;
            app.fase = Fase::RootNombre { cedula };
        }
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some() {
                app.feedback = None;
            }
        }
    }
}

/// Última parada: confirma la contraseña, valida (rápido, sin Argon2) y sólo
/// entonces arranca el hilo del hash — así un typo evidente (campo vacío,
/// contraseña corta) nunca paga el costo de Argon2 para nada.
pub(super) fn manejar_root_confirmar_password(
    app: &mut AppState,
    key: KeyEvent,
    core: &AppCore,
    cedula: String,
    nombre: String,
    password: String,
    pendiente: &mut RootPendiente,
) {
    match key.code {
        KeyCode::Enter => {
            let confirmacion = app.input.value().to_string();
            if confirmacion.is_empty() {
                return;
            }
            if confirmacion != password {
                app.input.reset();
                app.mostrar_feedback(
                    "Las contraseñas no coinciden".to_string(),
                    NivelFeedback::Error,
                );
                return;
            }
            let input = CrearRootInicialInput {
                cedula,
                nombre: nombre.clone(),
                password,
            };
            if let Err(error) = core.validar_datos_para_root_inicial(&input) {
                app.input.reset();
                app.mostrar_feedback(error.to_string(), NivelFeedback::Error);
                return;
            }
            let (emisor, receptor) = mpsc::channel();
            std::thread::spawn(move || {
                let resultado = generar_hash(&input.password)
                    .map(|hash| (input, hash))
                    .map_err(UsuarioServiceError::from);
                let _ = emisor.send(resultado);
            });
            *pendiente = Some(receptor);
            app.input.reset();
            app.fase = Fase::RootCreando { nombre };
        }
        KeyCode::Esc => {
            app.input.reset();
            app.feedback = None;
            app.fase = Fase::RootPassword { cedula, nombre };
        }
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some() {
                app.feedback = None;
            }
        }
    }
}

/// Recoge el resultado del hilo de Argon2 cuando llega e inserta el usuario
/// (la comprobación atómica de "sólo un ROOT inicial" vive en
/// `crear_root_inicial_con_hash`, no acá). Devuelve `true` si había un
/// resultado y se procesó.
pub(super) fn recibir_root_creado(
    core: &AppCore,
    app: &mut AppState,
    pendiente: &mut RootPendiente,
) -> bool {
    let Some(receptor) = pendiente.as_ref() else {
        return false;
    };
    let Ok(resultado) = receptor.try_recv() else {
        return false;
    };
    *pendiente = None;
    match resultado.and_then(|(input, hash)| core.crear_root_inicial_con_hash(input, hash)) {
        Ok(_) => {
            app.fase = Fase::LoginCedula;
            app.mostrar_feedback(
                "Usuario ROOT creado, ya puede iniciar sesión".to_string(),
                NivelFeedback::Exito,
            );
        }
        // Otra instancia ganó la carrera y ya creó el ROOT primero: no es un
        // error del operador, simplemente ya hay cuenta — al login.
        Err(UsuarioServiceError::ConfiguracionInicialYaRealizada) => {
            app.fase = Fase::LoginCedula;
        }
        // Falló de verdad (cédula duplicada, contraseña rechazada, error de
        // base de datos…): de vuelta al primer campo, no al login — todavía
        // no existe ningún usuario con el que entrar.
        Err(error) => {
            app.fase = Fase::RootCedula;
            app.mostrar_feedback(error.to_string(), NivelFeedback::Error);
        }
    }
    true
}
