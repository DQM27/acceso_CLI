//! Guard de terminal exclusivo de `--comandos`.
//!
//! El de `tui::terminal` es privado y la restricción del proyecto es no
//! tocar la TUI clásica, así que se replica lo mínimo (mismo título, misma
//! restauración al salir) en vez de compartir abstracción con ella.

use std::io::{self, stdout};

use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, SetTitle, disable_raw_mode, enable_raw_mode,
    },
};

pub struct TerminalGuard;

impl TerminalGuard {
    pub fn acquire() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout(), EnterAlternateScreen, Hide, SetTitle("BRISAS CLI")) {
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
