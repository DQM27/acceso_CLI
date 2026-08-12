use ratatui::style::{Color, Modifier, Style};

pub const FONDO: Color = Color::Rgb(12, 16, 19);
pub const PRINCIPAL: Color = Color::Rgb(220, 225, 228);
pub const SECUNDARIO: Color = Color::Rgb(158, 168, 173);
pub const CIAN: Color = Color::Rgb(70, 200, 215);
pub const AMBAR: Color = Color::Rgb(220, 170, 70);
pub const ROJO: Color = Color::Rgb(220, 95, 100);
pub const VERDE: Color = Color::Rgb(95, 190, 125);

pub fn titulo() -> Style {
    Style::default()
        .fg(CIAN)
        .bg(FONDO)
        .add_modifier(Modifier::BOLD)
}

pub fn subtitulo() -> Style {
    Style::default().fg(PRINCIPAL).bg(FONDO)
}

pub fn texto_normal() -> Style {
    Style::default().fg(PRINCIPAL).bg(FONDO)
}

pub fn texto_secundario() -> Style {
    Style::default().fg(SECUNDARIO).bg(FONDO)
}

pub fn seleccionado() -> Style {
    Style::default()
        .fg(FONDO)
        .bg(CIAN)
        .add_modifier(Modifier::BOLD)
}

pub fn foco() -> Style {
    Style::default()
        .fg(CIAN)
        .bg(FONDO)
        .add_modifier(Modifier::BOLD)
}

pub fn advertencia() -> Style {
    Style::default().fg(AMBAR).bg(FONDO)
}

pub fn error() -> Style {
    Style::default().fg(ROJO).bg(FONDO)
}

pub fn exito() -> Style {
    Style::default().fg(VERDE).bg(FONDO)
}

pub fn ayuda_tecla() -> Style {
    Style::default()
        .fg(CIAN)
        .bg(FONDO)
        .add_modifier(Modifier::BOLD)
}

pub fn borde() -> Style {
    Style::default().fg(SECUNDARIO).bg(FONDO)
}
