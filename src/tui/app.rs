use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::Backend};

use crate::application::AppCore;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::UsuarioServiceError;
use crate::services::usuario_service::CrearRootInicialInput;

use super::{
    activos::{self, AccionActivos, ActivosState},
    configuracion_inicial::{self, AccionConfiguracion, ConfiguracionInicialState},
    contratistas::{self, AccionContratistas, ContratistasState},
    historial::{self, AccionHistorial, HistorialState},
    login,
    login::LoginState,
};

const EVENT_POLL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vista {
    ConfiguracionInicial,
    Login,
    IngresosActivos,
    Historial,
    Contratistas,
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent};
    use rusqlite::Connection;

    use super::*;
    use crate::database::schema::initialize_database;

    fn escribir(state: &mut ConfiguracionInicialState, texto: &str) {
        for caracter in texto.chars() {
            state.handle_key(KeyEvent::new(KeyCode::Char(caracter), KeyModifiers::NONE));
        }
    }

    #[test]
    fn configuracion_exitosa_transiciona_a_login_sin_autenticar() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        let core = AppCore::new(connection);
        let mut app = App::new(true);
        escribir(&mut app.configuracion_inicial, "ROOT1");
        app.configuracion_inicial
            .handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        escribir(&mut app.configuracion_inicial, "Root Inicial");
        app.configuracion_inicial
            .handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        escribir(&mut app.configuracion_inicial, "password1");
        app.configuracion_inicial
            .handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        escribir(&mut app.configuracion_inicial, "password1");
        app.configuracion_inicial
            .handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));

        app.procesar_configuracion_pendiente(&core);

        assert_eq!(app.vista, Vista::Login);
        assert!(app.sesion().is_none());
        assert!(core.autenticar("ROOT1", "password1").is_ok());
    }
}

#[derive(Debug)]
pub struct App {
    vista: Vista,
    login: LoginState,
    configuracion_inicial: ConfiguracionInicialState,
    activos: ActivosState,
    historial: HistorialState,
    contratistas: ContratistasState,
    salir: bool,
    sesion: Option<UsuarioSesion>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            vista: Vista::Login,
            login: LoginState::default(),
            configuracion_inicial: ConfiguracionInicialState::default(),
            activos: ActivosState::default(),
            historial: HistorialState::default(),
            contratistas: ContratistasState::default(),
            salir: false,
            sesion: None,
        }
    }
}

impl App {
    pub fn new(requiere_configuracion_inicial: bool) -> Self {
        Self {
            vista: if requiere_configuracion_inicial {
                Vista::ConfiguracionInicial
            } else {
                Vista::Login
            },
            ..Self::default()
        }
    }

    pub fn run<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<()> {
        self.run_internal(terminal, None)
    }

    pub fn run_with_core<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
        core: &AppCore,
    ) -> io::Result<()> {
        self.run_internal(terminal, Some(core))
    }

    fn run_internal<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
        core: Option<&AppCore>,
    ) -> io::Result<()> {
        while !self.salir {
            terminal.draw(|frame| match self.vista {
                Vista::ConfiguracionInicial => {
                    configuracion_inicial::render(frame, frame.area(), &self.configuracion_inicial)
                }
                Vista::Login => login::render(frame, frame.area(), &self.login),
                Vista::IngresosActivos => activos::render(frame, frame.area(), &self.activos),
                Vista::Historial => historial::render(frame, frame.area(), &self.historial),
                Vista::Contratistas => {
                    contratistas::render(frame, frame.area(), &self.contratistas)
                }
            })?;

            if let Some(core) = core {
                self.procesar_configuracion_pendiente(core);
            }

            if event::poll(EVENT_POLL)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.salir = true;
                } else {
                    match self.vista {
                        Vista::ConfiguracionInicial => {
                            if self.configuracion_inicial.handle_key(key)
                                == AccionConfiguracion::Salir
                            {
                                self.salir = true;
                            }
                        }
                        Vista::Login => {
                            if key.code == KeyCode::Enter && self.login.acceso_simulado_exitoso() {
                                self.vista = Vista::IngresosActivos;
                            } else if key.code == KeyCode::Esc {
                                self.salir = true;
                            } else {
                                self.login.handle_key(key);
                            }
                        }
                        Vista::IngresosActivos => match self.activos.handle_key(key) {
                            AccionActivos::Volver => self.vista = Vista::Login,
                            AccionActivos::IrHistorial => self.vista = Vista::Historial,
                            AccionActivos::IrContratistas => self.vista = Vista::Contratistas,
                            AccionActivos::Ninguna => {}
                        },
                        Vista::Historial => {
                            if self.historial.handle_key(key) == AccionHistorial::Volver {
                                self.vista = Vista::IngresosActivos;
                            }
                        }
                        Vista::Contratistas => {
                            if self.contratistas.handle_key(key) == AccionContratistas::Volver {
                                self.vista = Vista::IngresosActivos;
                            }
                        }
                    }
                }
            }

            let ahora = std::time::Instant::now();
            self.configuracion_inicial.tick(ahora);
            self.login.tick(ahora);
            if let Some((cedula, password)) = self.login.credenciales_si_validacion_lista(ahora) {
                if let Some(core) = core {
                    match core.autenticar(&cedula, &password) {
                        Ok(sesion) => {
                            self.sesion = Some(sesion);
                            self.login.completar_validacion(None);
                        }
                        Err(error) => self.login.completar_validacion(Some(error.to_string())),
                    }
                } else {
                    self.login.completar_validacion(None);
                }
            }
        }

        Ok(())
    }

    pub fn sesion(&self) -> Option<&UsuarioSesion> {
        self.sesion.as_ref()
    }

    fn procesar_configuracion_pendiente(&mut self, core: &AppCore) {
        if self.vista != Vista::ConfiguracionInicial {
            return;
        }
        let Some(solicitud) = self.configuracion_inicial.tomar_solicitud() else {
            return;
        };
        match core.crear_root_inicial(CrearRootInicialInput {
            cedula: solicitud.cedula,
            nombre: solicitud.nombre,
            password: solicitud.password,
        }) {
            Ok(_) | Err(UsuarioServiceError::ConfiguracionInicialYaRealizada) => {
                self.configuracion_inicial.limpiar_secretos();
                self.vista = Vista::Login;
            }
            Err(UsuarioServiceError::Database(_)) => self
                .configuracion_inicial
                .completar_con_error("No se pudo crear el usuario ROOT"),
            Err(error) => self
                .configuracion_inicial
                .completar_con_error(error.to_string()),
        }
    }
}
