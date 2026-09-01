use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::ui_kit::{StandardCommand, TextInput, standard_command};

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const DURACION_FRAME: Duration = Duration::from_millis(90);
const DURACION_PARPADEO: Duration = Duration::from_millis(500);
const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampoLogin {
    Cedula,
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoLogin {
    Normal,
    Validando { iniciado: Instant },
    Error(String),
    Exito,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionLogin {
    Ninguna,
    Salir,
    /// Campos completos: hay que autenticar de verdad. Se emite en el
    /// instante en que se presiona ENTER, sin espera artificial de por
    /// medio — el estado `Validando` sigue existiendo para el spinner
    /// mientras se espera el resultado real.
    Autenticar {
        cedula: String,
        password: String,
    },
}

#[derive(Debug)]
pub struct LoginState {
    cedula: TextInput,
    password: TextInput,
    campo_activo: CampoLogin,
    estado: EstadoLogin,
    spinner_frame: usize,
    cursor_iniciado: Instant,
    cursor_visible: bool,
    ayuda_expandida: bool,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            cedula: TextInput::default(),
            password: TextInput::default(),
            campo_activo: CampoLogin::Cedula,
            estado: EstadoLogin::Normal,
            spinner_frame: 0,
            cursor_iniciado: Instant::now(),
            cursor_visible: true,
            ayuda_expandida: false,
        }
    }
}

impl LoginState {
    pub fn reiniciar(&mut self) {
        *self = Self::default();
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionLogin {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionLogin::Ninguna;
        }
        if standard_command(key) == Some(StandardCommand::Cancel) {
            self.password.clear();
            return AccionLogin::Salir;
        }
        if matches!(self.estado, EstadoLogin::Validando { .. }) {
            return AccionLogin::Ninguna;
        }

        match key.code {
            KeyCode::Tab | KeyCode::Down => {
                self.siguiente_campo();
                AccionLogin::Ninguna
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.campo_anterior();
                AccionLogin::Ninguna
            }
            KeyCode::Enter => self.enter(Instant::now()),
            _ => {
                self.editar(key);
                AccionLogin::Ninguna
            }
        }
    }

    pub fn tick(&mut self, ahora: Instant) {
        let ciclos = ahora
            .saturating_duration_since(self.cursor_iniciado)
            .as_millis()
            / DURACION_PARPADEO.as_millis();
        self.cursor_visible = ciclos.is_multiple_of(2);

        let EstadoLogin::Validando { iniciado } = self.estado else {
            return;
        };

        let transcurrido = ahora.saturating_duration_since(iniciado);
        self.spinner_frame = (transcurrido.as_millis() / DURACION_FRAME.as_millis()) as usize;
    }

    pub fn completar_validacion(&mut self, error: Option<String>) {
        self.password.clear();
        self.estado = error.map_or(EstadoLogin::Exito, EstadoLogin::Error);
    }

    /// Muestra un mensaje de error ya en la pantalla de Login, sin haber
    /// pasado por una autenticación real — usado cuando la app se reinicia
    /// tras una restauración de respaldo fallida.
    pub fn preset_error(&mut self, mensaje: String) {
        self.estado = EstadoLogin::Error(mensaje);
    }

    pub fn password_enmascarado(&self) -> String {
        "•".repeat(self.password.value().chars().count())
    }

    pub fn campo_activo(&self) -> CampoLogin {
        self.campo_activo
    }

    pub fn estado(&self) -> &EstadoLogin {
        &self.estado
    }

    pub fn acceso_simulado_exitoso(&self) -> bool {
        matches!(self.estado, EstadoLogin::Exito)
    }

    fn siguiente_campo(&mut self) {
        self.campo_activo = match self.campo_activo {
            CampoLogin::Cedula => CampoLogin::Password,
            CampoLogin::Password => CampoLogin::Cedula,
        };
        self.reiniciar_cursor();
    }

    fn campo_anterior(&mut self) {
        self.siguiente_campo();
    }

    fn enter(&mut self, ahora: Instant) -> AccionLogin {
        if self.campo_activo == CampoLogin::Cedula {
            self.campo_activo = CampoLogin::Password;
            return AccionLogin::Ninguna;
        }

        if self.cedula.value().trim().is_empty() || self.password.value().is_empty() {
            self.estado = EstadoLogin::Error("Complete cédula y contraseña".to_owned());
            return AccionLogin::Ninguna;
        }

        self.estado = EstadoLogin::Validando { iniciado: ahora };
        self.spinner_frame = 0;
        AccionLogin::Autenticar {
            cedula: self.cedula.value().trim().to_owned(),
            password: self.password.value().to_owned(),
        }
    }

    fn editar(&mut self, key: KeyEvent) {
        self.limpiar_estado_transitorio();
        self.reiniciar_cursor();
        match self.campo_activo {
            CampoLogin::Cedula => {
                self.cedula.handle_key(key);
            }
            CampoLogin::Password => {
                self.password.handle_key(key);
            }
        }
    }

    fn limpiar_estado_transitorio(&mut self) {
        if matches!(self.estado, EstadoLogin::Error(_) | EstadoLogin::Exito) {
            self.estado = EstadoLogin::Normal;
        }
    }

    fn reiniciar_cursor(&mut self) {
        self.cursor_iniciado = Instant::now();
        self.cursor_visible = true;
    }

    fn spinner(&self) -> char {
        SPINNER[self.spinner_frame % SPINNER.len()]
    }
}
