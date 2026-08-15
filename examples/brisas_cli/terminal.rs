use std::{
    io::{self, stdout},
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{self, DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use super::PilotScreen;

struct TerminalGuard {
    raw_mode: bool,
    alternate_screen: bool,
    bracketed_paste: bool,
    cursor_style: bool,
}

impl TerminalGuard {
    fn acquire() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut guard = Self {
            raw_mode: true,
            alternate_screen: false,
            bracketed_paste: false,
            cursor_style: false,
        };

        execute!(stdout(), EnterAlternateScreen)?;
        guard.alternate_screen = true;
        execute!(stdout(), EnableBracketedPaste)?;
        guard.bracketed_paste = true;
        execute!(stdout(), SetCursorStyle::BlinkingBlock)?;
        guard.cursor_style = true;

        Ok(guard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.bracketed_paste {
            let _ = execute!(stdout(), DisableBracketedPaste);
        }
        if self.alternate_screen {
            let _ = execute!(stdout(), LeaveAlternateScreen);
        }
        if self.cursor_style {
            let _ = execute!(stdout(), SetCursorStyle::DefaultUserShape);
        }
        let _ = execute!(stdout(), Show);
        if self.raw_mode {
            let _ = disable_raw_mode();
        }
    }
}

pub fn run(mut app: impl PilotScreen) -> io::Result<()> {
    let _guard = TerminalGuard::acquire()?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut dirty = true;
    let mut next_redraw = Instant::now();

    while app.is_running() {
        if dirty || Instant::now() >= next_redraw {
            terminal.draw(|frame| app.render(frame))?;
            dirty = false;
            next_redraw = Instant::now() + app.redraw_interval();
        }

        let timeout = next_redraw.saturating_duration_since(Instant::now());
        if event::poll(timeout)? {
            dirty = app.handle_event(&event::read()?);

            // Una ráfaga de resize o repetición no necesita un frame intermedio
            // por cada evento; se procesa antes del siguiente dibujo.
            for _ in 0..63 {
                if !event::poll(Duration::ZERO)? {
                    break;
                }
                dirty |= app.handle_event(&event::read()?);
            }
        }
    }

    Ok(())
}
