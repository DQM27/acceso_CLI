use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::ui_kit::{StandardCommand, standard_command};

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const DURACION_PARPADEO: Duration = Duration::from_millis(500);
const LONGITUD_MINIMA_PASSWORD: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampoConfiguracion {
    Cedula,
    Nombre,
    Password,
    ConfirmarPassword,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoConfiguracion {
    Editando,
    Creando,
    Error(String),
}

pub struct SolicitudRoot {
    pub cedula: String,
    pub nombre: String,
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionConfiguracion {
    Ninguna,
    Salir,
}

pub struct ConfiguracionInicialState {
    cedula: String,
    nombre: String,
    password: String,
    confirmar_password: String,
    campo_activo: CampoConfiguracion,
    estado: EstadoConfiguracion,
    solicitud: Option<SolicitudRoot>,
    cursor_iniciado: Instant,
    cursor_visible: bool,
    ayuda_expandida: bool,
}

impl std::fmt::Debug for ConfiguracionInicialState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConfiguracionInicialState")
            .field("cedula", &self.cedula)
            .field("nombre", &self.nombre)
            .field("password", &"[OCULTA]")
            .field("confirmar_password", &"[OCULTA]")
            .field("campo_activo", &self.campo_activo)
            .field("estado", &self.estado)
            .finish()
    }
}

impl Default for ConfiguracionInicialState {
    fn default() -> Self {
        Self {
            cedula: String::new(),
            nombre: String::new(),
            password: String::new(),
            confirmar_password: String::new(),
            campo_activo: CampoConfiguracion::Cedula,
            estado: EstadoConfiguracion::Editando,
            solicitud: None,
            cursor_iniciado: Instant::now(),
            cursor_visible: true,
            ayuda_expandida: false,
        }
    }
}

impl ConfiguracionInicialState {
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionConfiguracion {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionConfiguracion::Ninguna;
        }
        if self.estado == EstadoConfiguracion::Creando {
            return AccionConfiguracion::Ninguna;
        }
        match key.code {
            KeyCode::Esc => return AccionConfiguracion::Salir,
            KeyCode::Tab | KeyCode::Down => self.siguiente_campo(),
            KeyCode::BackTab | KeyCode::Up => self.campo_anterior(),
            KeyCode::Enter if self.campo_activo != CampoConfiguracion::ConfirmarPassword => {
                self.siguiente_campo()
            }
            KeyCode::Enter => self.intentar_crear(),
            KeyCode::Char('g' | 'G') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.intentar_crear()
            }
            KeyCode::Backspace => self.borrar(),
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.escribir(character)
            }
            _ => {}
        }
        AccionConfiguracion::Ninguna
    }

    pub fn tick(&mut self, ahora: Instant) {
        let ciclos = ahora
            .saturating_duration_since(self.cursor_iniciado)
            .as_millis()
            / DURACION_PARPADEO.as_millis();
        self.cursor_visible = ciclos.is_multiple_of(2);
    }

    pub fn tomar_solicitud(&mut self) -> Option<SolicitudRoot> {
        self.solicitud.take()
    }

    pub fn completar_con_error(&mut self, mensaje: impl Into<String>) {
        self.estado = EstadoConfiguracion::Error(mensaje.into());
    }

    pub fn limpiar_secretos(&mut self) {
        self.password.clear();
        self.confirmar_password.clear();
        self.solicitud = None;
    }

    pub fn campo_activo(&self) -> CampoConfiguracion {
        self.campo_activo
    }

    pub fn estado(&self) -> &EstadoConfiguracion {
        &self.estado
    }

    pub fn password_enmascarado(&self) -> String {
        "•".repeat(self.password.chars().count())
    }

    pub fn confirmacion_enmascarada(&self) -> String {
        "•".repeat(self.confirmar_password.chars().count())
    }

    fn siguiente_campo(&mut self) {
        self.campo_activo = match self.campo_activo {
            CampoConfiguracion::Cedula => CampoConfiguracion::Nombre,
            CampoConfiguracion::Nombre => CampoConfiguracion::Password,
            CampoConfiguracion::Password => CampoConfiguracion::ConfirmarPassword,
            CampoConfiguracion::ConfirmarPassword => CampoConfiguracion::Cedula,
        };
        self.reiniciar_cursor();
    }

    fn campo_anterior(&mut self) {
        self.campo_activo = match self.campo_activo {
            CampoConfiguracion::Cedula => CampoConfiguracion::ConfirmarPassword,
            CampoConfiguracion::Nombre => CampoConfiguracion::Cedula,
            CampoConfiguracion::Password => CampoConfiguracion::Nombre,
            CampoConfiguracion::ConfirmarPassword => CampoConfiguracion::Password,
        };
        self.reiniciar_cursor();
    }

    fn escribir(&mut self, character: char) {
        self.limpiar_error();
        match self.campo_activo {
            CampoConfiguracion::Cedula => self.cedula.push(character),
            CampoConfiguracion::Nombre => self.nombre.push(character),
            CampoConfiguracion::Password => self.password.push(character),
            CampoConfiguracion::ConfirmarPassword => self.confirmar_password.push(character),
        }
        self.reiniciar_cursor();
    }

    fn borrar(&mut self) {
        self.limpiar_error();
        match self.campo_activo {
            CampoConfiguracion::Cedula => self.cedula.pop(),
            CampoConfiguracion::Nombre => self.nombre.pop(),
            CampoConfiguracion::Password => self.password.pop(),
            CampoConfiguracion::ConfirmarPassword => self.confirmar_password.pop(),
        };
        self.reiniciar_cursor();
    }

    fn intentar_crear(&mut self) {
        let error = if self.cedula.trim().is_empty() {
            self.campo_activo = CampoConfiguracion::Cedula;
            Some("La cédula es obligatoria")
        } else if self.nombre.trim().is_empty() {
            self.campo_activo = CampoConfiguracion::Nombre;
            Some("El nombre es obligatorio")
        } else if self.password.is_empty() {
            self.campo_activo = CampoConfiguracion::Password;
            Some("La contraseña es obligatoria")
        } else if self.confirmar_password.is_empty() {
            self.campo_activo = CampoConfiguracion::ConfirmarPassword;
            Some("Debe confirmar la contraseña")
        } else if self.password != self.confirmar_password {
            self.campo_activo = CampoConfiguracion::ConfirmarPassword;
            Some("Las contraseñas no coinciden")
        } else if self.password.chars().count() < LONGITUD_MINIMA_PASSWORD {
            self.campo_activo = CampoConfiguracion::Password;
            Some("La contraseña debe tener al menos 8 caracteres")
        } else {
            None
        };

        if let Some(error) = error {
            self.estado = EstadoConfiguracion::Error(error.to_owned());
            self.reiniciar_cursor();
            return;
        }

        self.solicitud = Some(SolicitudRoot {
            cedula: self.cedula.trim().to_owned(),
            nombre: self.nombre.trim().to_owned(),
            password: self.password.clone(),
        });
        self.estado = EstadoConfiguracion::Creando;
    }

    fn limpiar_error(&mut self) {
        if matches!(self.estado, EstadoConfiguracion::Error(_)) {
            self.estado = EstadoConfiguracion::Editando;
        }
    }

    fn reiniciar_cursor(&mut self) {
        self.cursor_iniciado = Instant::now();
        self.cursor_visible = true;
    }
}
