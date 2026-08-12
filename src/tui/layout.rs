use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::theme;

pub(crate) fn render_overlay(
    frame: &mut Frame,
    area: Rect,
    ancho: u16,
    alto: u16,
    margen_horizontal: u16,
    titulo: &str,
    lineas: Vec<Line<'_>>,
) {
    let area = centrar(
        area,
        ancho.min(area.width.saturating_sub(margen_horizontal)),
        alto,
    );
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lineas).style(theme::texto_normal()).block(
            Block::default()
                .title(format!(" {titulo} "))
                .title_style(theme::foco())
                .borders(Borders::ALL)
                .border_style(theme::foco())
                .padding(ratatui::widgets::Padding::uniform(1)),
        ),
        area,
    );
}

pub(crate) fn render_terminal_pequena(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::FONDO)),
        area,
    );
    let mensaje = vec![
        Line::from("Terminal demasiado pequeña").style(theme::advertencia()),
        Line::from("Tamaño mínimo recomendado: 60 x 22").style(theme::texto_normal()),
        Line::from(format!("Tamaño actual: {} x {}", area.width, area.height))
            .style(theme::texto_secundario()),
    ];
    frame.render_widget(
        Paragraph::new(mensaje)
            .alignment(Alignment::Center)
            .block(Block::default().padding(ratatui::widgets::Padding::top(
                area.height.saturating_sub(3) / 2,
            ))),
        area,
    );
}

fn centrar(area: Rect, ancho: u16, alto: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto.min(area.height),
    )
}
