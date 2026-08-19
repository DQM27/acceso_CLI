//! Primitivas visuales para mantener consistencia entre las pantallas TUI.
//!
//! El piloto `brisas_cli` fue su primer consumidor; las 9 pantallas de producción
//! ya usan el shell visual (`ScreenShell`/`Theme`) y, desde la unificación de
//! atajos, también la convención de teclado (`standard_command`).

mod debounce;
mod keyboard;
pub mod query_lang;
mod seleccion;
mod shell;
mod text_input;
mod theme;

pub use debounce::Debounce;
pub use keyboard::{
    CANCEL_HINT, EMERGENCY_EXIT_HINT, HELP_HINT, HELP_KEY, QUICK_EXIT_HINT, QUICK_EXIT_KEY,
    StandardCommand, THEME_HINT, THEME_KEY, standard_command,
};
pub use seleccion::mover_seleccion;
pub use shell::{
    CommandHint, ScreenShell, ShellAreas, StatusKind, auxiliary_panel, centered_content,
    centered_rect, panel, render_terminal_too_small,
};
pub use text_input::{TextInput, TextInputFocus};
pub use theme::{Theme, ThemePreset};
