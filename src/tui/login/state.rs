use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const DURACION_VALIDACION: Duration = Duration::from_millis(800);
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

#[derive(Debug)]
pub struct LoginState {
    cedula: String,
    password: String,
    campo_activo: CampoLogin,
    estado: EstadoLogin,
    spinner_frame: usize,
    cursor_iniciado: Instant,
    cursor_visible: bool,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            cedula: String::new(),
            password: String::new(),
            campo_activo: CampoLogin::Cedula,
            estado: EstadoLogin::Normal,
            spinner_frame: 0,
            cursor_iniciado: Instant::now(),
            cursor_visible: true,
        }
    }
}

impl LoginState {
    pub fn handle_key(&mut self, key: KeyEvent) {
        if matches!(self.estado, EstadoLogin::Validando { .. }) {
            return;
        }

        match key.code {
            KeyCode::Tab | KeyCode::Down => self.siguiente_campo(),
            KeyCode::BackTab | KeyCode::Up => self.campo_anterior(),
            KeyCode::Enter => self.enter(Instant::now()),
            KeyCode::Backspace => self.borrar(),
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.escribir(character);
            }
            _ => {}
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

    pub fn credenciales_si_validacion_lista(&self, ahora: Instant) -> Option<(String, String)> {
        let EstadoLogin::Validando { iniciado } = self.estado else {
            return None;
        };
        (ahora.saturating_duration_since(iniciado) >= DURACION_VALIDACION)
            .then(|| (self.cedula.clone(), self.password.clone()))
    }

    pub fn completar_validacion(&mut self, error: Option<String>) {
        self.password.clear();
        self.estado = match error {
            Some(error) => EstadoLogin::Error(error),
            None => EstadoLogin::Exito,
        };
    }

    pub fn password_enmascarado(&self) -> String {
        "•".repeat(self.password.chars().count())
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

    fn enter(&mut self, ahora: Instant) {
        if self.campo_activo == CampoLogin::Cedula {
            self.campo_activo = CampoLogin::Password;
            return;
        }

        if self.cedula.trim().is_empty() || self.password.is_empty() {
            self.estado = EstadoLogin::Error("Complete cédula y contraseña".to_owned());
            return;
        }

        self.estado = EstadoLogin::Validando { iniciado: ahora };
        self.spinner_frame = 0;
    }

    fn escribir(&mut self, character: char) {
        self.limpiar_estado_transitorio();
        self.reiniciar_cursor();
        match self.campo_activo {
            CampoLogin::Cedula => self.cedula.push(character),
            CampoLogin::Password => self.password.push(character),
        }
    }

    fn borrar(&mut self) {
        self.limpiar_estado_transitorio();
        self.reiniciar_cursor();
        match self.campo_activo {
            CampoLogin::Cedula => {
                self.cedula.pop();
            }
            CampoLogin::Password => {
                self.password.pop();
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

    #[cfg(test)]
    fn iniciar_validacion_en(&mut self, ahora: Instant) {
        self.campo_activo = CampoLogin::Password;
        self.enter(ahora);
    }
}
