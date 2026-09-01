use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::ui_kit::{StandardCommand, TextInput, standard_command};

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const DURACION_PARPADEO: Duration = Duration::from_millis(500);
const LONGITUD_MINIMA_PASSWORD: usize = 8;
// Mismos topes que Usuarios (`usuarios/state.rs`) — ROOT inicial era la
// única cuenta sin ninguno.
const LONGITUD_MAXIMA_CEDULA: usize = 30;
const LONGITUD_MAXIMA_NOMBRE: usize = 60;
const LONGITUD_MAXIMA_PASSWORD: usize = 128;

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

impl std::fmt::Debug for SolicitudRoot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SolicitudRoot")
            .field("cedula", &self.cedula)
            .field("nombre", &self.nombre)
            .field("password", &"[OCULTA]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionConfiguracion {
    Ninguna,
    Salir,
}

pub struct ConfiguracionInicialState {
    cedula: TextInput,
    nombre: TextInput,
    password: TextInput,
    confirmar_password: TextInput,
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
            .field("cedula", &self.cedula.value())
            .field("nombre", &self.nombre.value())
            .field("password", &"[OCULTA]")
            .field("confirmar_password", &"[OCULTA]")
            .field("campo_activo", &self.campo_activo)
            .field("estado", &self.estado)
            .field("solicitud", &self.solicitud)
            .field("cursor_iniciado", &self.cursor_iniciado)
            .field("cursor_visible", &self.cursor_visible)
            .field("ayuda_expandida", &self.ayuda_expandida)
            .finish()
    }
}

impl Default for ConfiguracionInicialState {
    fn default() -> Self {
        Self {
            cedula: TextInput::default().with_max_chars(LONGITUD_MAXIMA_CEDULA),
            nombre: TextInput::default().with_max_chars(LONGITUD_MAXIMA_NOMBRE),
            password: TextInput::default().with_max_chars(LONGITUD_MAXIMA_PASSWORD),
            confirmar_password: TextInput::default().with_max_chars(LONGITUD_MAXIMA_PASSWORD),
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
            KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End => self.editar(key),
            KeyCode::Char(_)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.editar(key)
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
        "•".repeat(self.password.value().chars().count())
    }

    pub fn confirmacion_enmascarada(&self) -> String {
        "•".repeat(self.confirmar_password.value().chars().count())
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

    fn editar(&mut self, key: KeyEvent) {
        self.limpiar_error();
        match self.campo_activo {
            CampoConfiguracion::Cedula => {
                self.cedula.handle_key(key);
            }
            CampoConfiguracion::Nombre => {
                self.nombre.handle_key(key);
            }
            CampoConfiguracion::Password => {
                self.password.handle_key(key);
            }
            CampoConfiguracion::ConfirmarPassword => {
                self.confirmar_password.handle_key(key);
            }
        }
        self.reiniciar_cursor();
    }

    fn intentar_crear(&mut self) {
        let error = if self.cedula.value().trim().is_empty() {
            self.campo_activo = CampoConfiguracion::Cedula;
            Some("La cédula es obligatoria")
        } else if self.nombre.value().trim().is_empty() {
            self.campo_activo = CampoConfiguracion::Nombre;
            Some("El nombre es obligatorio")
        } else if self.password.value().is_empty() {
            self.campo_activo = CampoConfiguracion::Password;
            Some("La contraseña es obligatoria")
        } else if self.confirmar_password.value().is_empty() {
            self.campo_activo = CampoConfiguracion::ConfirmarPassword;
            Some("Debe confirmar la contraseña")
        } else if self.password.value().chars().count() < LONGITUD_MINIMA_PASSWORD {
            self.campo_activo = CampoConfiguracion::Password;
            Some("La contraseña debe tener al menos 8 caracteres")
        } else if self.password.value() != self.confirmar_password.value() {
            self.campo_activo = CampoConfiguracion::ConfirmarPassword;
            Some("Las contraseñas no coinciden")
        } else {
            None
        };

        if let Some(error) = error {
            self.estado = EstadoConfiguracion::Error(error.to_owned());
            self.reiniciar_cursor();
            return;
        }

        self.solicitud = Some(SolicitudRoot {
            cedula: self.cedula.value().trim().to_owned(),
            nombre: self.nombre.value().trim().to_owned(),
            password: self.password.value().to_owned(),
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
