//! Primitivas visuales para mantener consistencia entre las pantallas TUI.
//!
//! El piloto `brisas_cli` es su primer consumidor. La aplicación productiva aún
//! no depende de estas primitivas, de modo que pueden evaluarse antes de migrar
//! las vistas existentes.

mod keyboard;
mod select_menu;
mod shell;
mod text_input;
mod theme;

pub use keyboard::{
    CANCEL_HINT, HELP_HINT, HELP_KEY, StandardCommand, THEME_HINT, THEME_KEY, standard_command,
};
pub use select_menu::{SelectMenu, SelectMenuState};
pub use shell::{
    CommandHint, ScreenShell, ShellAreas, StatusKind, auxiliary_panel, centered_content,
    centered_rect, panel, render_terminal_too_small,
};
pub use text_input::{TextInput, TextInputFocus};
pub use theme::{Theme, ThemePreset};
