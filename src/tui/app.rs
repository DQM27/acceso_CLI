use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{Terminal, backend::Backend};

use super::{
    activos::{self, AccionActivos, ActivosState},
    login,
    login::LoginState,
};

const EVENT_POLL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vista {
    Login,
    IngresosActivos,
}

#[derive(Debug)]
pub struct App {
    vista: Vista,
    login: LoginState,
    activos: ActivosState,
    salir: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            vista: Vista::Login,
            login: LoginState::default(),
            activos: ActivosState::default(),
            salir: false,
        }
    }
}

impl App {
    pub fn run<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<()> {
        while !self.salir {
            terminal.draw(|frame| match self.vista {
                Vista::Login => login::render(frame, frame.area(), &self.login),
                Vista::IngresosActivos => activos::render(frame, frame.area(), &self.activos),
            })?;

            if event::poll(EVENT_POLL)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.salir = true;
                } else {
                    match self.vista {
                        Vista::Login => {
                            if key.code == KeyCode::Enter && self.login.acceso_simulado_exitoso() {
                                self.vista = Vista::IngresosActivos;
                            } else if key.code == KeyCode::Esc {
                                self.salir = true;
                            } else {
                                self.login.handle_key(key);
                            }
                        }
                        Vista::IngresosActivos => {
                            if self.activos.handle_key(key) == AccionActivos::Volver {
                                self.vista = Vista::Login;
                            }
                        }
                    }
                }
            }

            self.login.tick(std::time::Instant::now());
        }

        Ok(())
    }
}
