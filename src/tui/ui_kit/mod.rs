//! Primitivas visuales para mantener consistencia entre las pantallas TUI.
//!
//! El piloto `brisas_cli` fue su primer consumidor; las 9 pantallas de producción
//! ya usan el shell visual (`ScreenShell`/`Theme`) y, desde la unificación de
//! atajos, también la convención de teclado (`standard_command`).

mod debounce;
mod details;
mod fields;
mod keyboard;
mod layout;
mod query_lang;
mod seleccion;
mod shell;
mod text_input;
mod theme;

pub use crate::texto::plegar_diacriticos;
pub use debounce::Debounce;
pub use details::detail_line;
pub use fields::{
    ChoiceFieldOptions, empty_state, panel_vacio, render_choice_field, render_form_field,
};
pub use keyboard::{
    CANCEL_HINT, EMERGENCY_EXIT_HINT, HELP_HINT, HELP_KEY, QUICK_EXIT_HINT, QUICK_EXIT_KEY,
    StandardCommand, THEME_HINT, THEME_KEY, standard_command,
};
pub use layout::{
    MASTER_DETAIL_BREAKPOINT, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, MasterDetailAreas,
    SeparatorOrientation, master_detail_areas, render_separator,
};
pub use query_lang::{
    ResolucionConsulta, Term, resolver_terminos, resolver_terminos_detallado, valores,
};
pub use seleccion::{
    MARCADOR_SELECCION, SIMBOLO_RESALTADO_TABLA, marcador_seleccion, mover_seleccion,
};
pub use shell::{
    CommandHint, ScreenShell, ShellAreas, StatusKind, TabBar, TabItem, auxiliary_panel,
    centered_content, centered_rect, clasificar_mensaje, identidad_sesion, panel,
    render_terminal_too_small,
};
pub use text_input::{TextInput, TextInputFocus, posicionar_cursor, posicionar_cursor_campo};
pub use theme::{Theme, ThemePreset};
