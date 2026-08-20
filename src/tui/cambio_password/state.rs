use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::ui_kit::{StandardCommand, TextInput, standard_command};

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Campo {
    Actual,
    Nueva,
    Confirmacion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionCambioPassword {
    Ninguna,
    Volver,
    Cambiar {
        password_actual: String,
        nueva_password: String,
    },
}

#[derive(Debug)]
pub struct CambioPasswordState {
    actual: TextInput,
    nueva: TextInput,
    confirmacion: TextInput,
    campo: Campo,
    guardando: bool,
    mensaje: Option<Result<String, String>>,
    ayuda_expandida: bool,
}

impl Default for CambioPasswordState {
    fn default() -> Self {
        Self {
            actual: TextInput::default(),
            nueva: TextInput::default(),
            confirmacion: TextInput::default(),
            campo: Campo::Actual,
            guardando: false,
            mensaje: None,
            ayuda_expandida: false,
        }
    }
}

impl CambioPasswordState {
    pub fn reiniciar(&mut self) {
        *self = Self::default();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AccionCambioPassword {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionCambioPassword::Ninguna;
        }
        if self.guardando {
            return AccionCambioPassword::Ninguna;
        }
        if key.code == KeyCode::Esc {
            self.limpiar_secretos();
            return AccionCambioPassword::Volver;
        }
        match key.code {
            KeyCode::Tab | KeyCode::Down => self.siguiente(),
            KeyCode::BackTab | KeyCode::Up => self.anterior(),
            KeyCode::Enter if self.campo != Campo::Confirmacion => self.siguiente(),
            KeyCode::Enter => return self.enviar(),
            _ => {
                self.mensaje = None;
                self.entrada_activa().handle_key(key);
            }
        }
        AccionCambioPassword::Ninguna
    }

    pub fn completar(&mut self, resultado: Result<(), String>) {
        self.guardando = false;
        self.limpiar_secretos();
        self.campo = Campo::Actual;
        self.mensaje = Some(resultado.map(|()| "Contraseña actualizada".to_owned()));
    }

    pub(super) fn mascara(&self, campo: Campo) -> String {
        "•".repeat(self.entrada(campo).value().chars().count())
    }

    fn enviar(&mut self) -> AccionCambioPassword {
        if self.actual.value().is_empty() {
            self.mensaje = Some(Err("Ingrese su contraseña actual".into()));
            self.campo = Campo::Actual;
            return AccionCambioPassword::Ninguna;
        }
        if self.nueva.value().chars().count() < 8 {
            self.mensaje = Some(Err(
                "La contraseña nueva debe tener al menos 8 caracteres".into()
            ));
            self.campo = Campo::Nueva;
            return AccionCambioPassword::Ninguna;
        }
        if self.nueva.value() != self.confirmacion.value() {
            self.mensaje = Some(Err("Las contraseñas nuevas no coinciden".into()));
            self.campo = Campo::Confirmacion;
            return AccionCambioPassword::Ninguna;
        }
        self.guardando = true;
        self.mensaje = None;
        AccionCambioPassword::Cambiar {
            password_actual: self.actual.value().to_owned(),
            nueva_password: self.nueva.value().to_owned(),
        }
    }

    fn entrada(&self, campo: Campo) -> &TextInput {
        match campo {
            Campo::Actual => &self.actual,
            Campo::Nueva => &self.nueva,
            Campo::Confirmacion => &self.confirmacion,
        }
    }

    fn entrada_activa(&mut self) -> &mut TextInput {
        match self.campo {
            Campo::Actual => &mut self.actual,
            Campo::Nueva => &mut self.nueva,
            Campo::Confirmacion => &mut self.confirmacion,
        }
    }

    fn siguiente(&mut self) {
        self.campo = match self.campo {
            Campo::Actual => Campo::Nueva,
            Campo::Nueva => Campo::Confirmacion,
            Campo::Confirmacion => Campo::Actual,
        };
    }

    fn anterior(&mut self) {
        self.campo = match self.campo {
            Campo::Actual => Campo::Confirmacion,
            Campo::Nueva => Campo::Actual,
            Campo::Confirmacion => Campo::Nueva,
        };
    }

    fn limpiar_secretos(&mut self) {
        self.actual.clear();
        self.nueva.clear();
        self.confirmacion.clear();
    }
}
