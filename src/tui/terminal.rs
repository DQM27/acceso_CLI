use std::io::{self, stdout};

use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::application::AppCore;

use super::app::{App, SalidaApp};

struct TerminalGuard;

impl TerminalGuard {
    fn acquire() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout(), EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

pub fn run(
    core: &AppCore,
    requiere_configuracion_inicial: bool,
    mensaje_inicial: Option<String>,
) -> io::Result<SalidaApp> {
    let _guard = TerminalGuard::acquire()?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    let resultado = App::new(requiere_configuracion_inicial, mensaje_inicial)
        .run_with_core(&mut terminal, core);
    let _ = terminal.show_cursor();
    resultado
}
