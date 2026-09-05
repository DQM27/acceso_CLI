// Generado desde design/brisas.json. Editar la fuente y ejecutar node design/generar.mjs.
use crate::tui::ui_kit::Theme;
use ratatui::style::Color;

pub const LIGHT: Theme = Theme {
    background: Color::Rgb(233, 237, 243),
    text: Color::Rgb(32, 43, 58),
    muted: Color::Rgb(83, 98, 120),
    accent: Color::Rgb(32, 60, 99),
    success: Color::Rgb(53, 114, 79),
    warning: Color::Rgb(137, 98, 29),
    danger: Color::Rgb(173, 73, 69),
    border: Color::Rgb(120, 135, 157),
    selection_foreground: Color::Rgb(255, 255, 255),
    selection_background: Color::Rgb(32, 60, 99),
    navegacion_pestanas: false,
};

pub const DARK: Theme = Theme {
    background: Color::Rgb(12, 15, 20),
    text: Color::Rgb(236, 240, 246),
    muted: Color::Rgb(173, 184, 200),
    accent: Color::Rgb(166, 188, 217),
    success: Color::Rgb(139, 196, 155),
    warning: Color::Rgb(216, 186, 119),
    danger: Color::Rgb(222, 150, 144),
    border: Color::Rgb(124, 139, 162),
    selection_foreground: Color::Rgb(255, 255, 255),
    selection_background: Color::Rgb(32, 60, 99),
    navegacion_pestanas: false,
};
