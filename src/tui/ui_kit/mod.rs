//! Primitivas visuales para mantener consistencia entre las pantallas TUI.
//!
//! El piloto `brisas_cli` es su primer consumidor. La aplicación productiva aún
//! no depende de estas primitivas, de modo que pueden evaluarse antes de migrar
//! las vistas existentes.

mod select_menu;
mod shell;
mod text_input;
mod theme;

pub use select_menu::{SelectMenu, SelectMenuState};
pub use shell::{
    CommandHint, ScreenShell, ShellAreas, StatusKind, auxiliary_panel, centered_content, panel,
};
pub use text_input::TextInput;
pub use theme::{Theme, ThemePreset};
