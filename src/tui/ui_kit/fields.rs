use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::{Theme, marcador_seleccion};

/// Mensaje de "sin resultados" centrado a media altura de `area` — mismo
/// patrón repetido en cada pantalla con lista: una línea de advertencia sin
/// caja ni layout propio.
pub fn empty_state(frame: &mut Frame, area: Rect, texto: &str, theme: Theme) {
    frame.render_widget(
        Paragraph::new(texto)
            .style(theme.warning())
            .alignment(Alignment::Center),
        Rect::new(area.x, area.y + area.height / 2, area.width, 1),
    );
}

/// Mensaje de "nada seleccionado" para un panel de detalle vacío — mismo
/// patrón repetido en cada pantalla con maestro-detalle.
pub fn panel_vacio(frame: &mut Frame, area: Rect, texto: &str, theme: Theme) {
    frame.render_widget(Paragraph::new(texto).style(theme.muted()), area);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceFieldOptions {
    label_width: usize,
    show_arrows: bool,
}

impl ChoiceFieldOptions {
    pub const fn plain(label_width: usize) -> Self {
        Self {
            label_width,
            show_arrows: false,
        }
    }

    pub const fn arrows(label_width: usize) -> Self {
        Self {
            label_width,
            show_arrows: true,
        }
    }
}

/// Campo de texto con una señal de foco que no depende únicamente del color.
pub fn render_form_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    theme: Theme,
) -> Rect {
    let label_style = if focused {
        theme.accent()
    } else {
        theme.muted()
    };
    let line_style = if focused {
        theme.accent()
    } else {
        theme.border()
    };
    let marker = marcador_seleccion(focused);
    let value_y = area.y.saturating_add(1);
    let line_y = area.y.saturating_add(2);

    frame.render_widget(
        Paragraph::new(format!("{marker} {label}")).style(label_style),
        Rect::new(area.x, area.y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(value.to_owned()).style(theme.base())),
        Rect::new(area.x, value_y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(line_style),
        Rect::new(area.x, line_y, area.width, 1),
    );

    Rect::new(area.x, value_y, area.width, 1)
}

/// Opción seleccionable con marcador textual y flechas opcionales.
pub fn render_choice_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    theme: Theme,
    options: ChoiceFieldOptions,
) {
    let marker = marcador_seleccion(focused);
    let style = if focused {
        theme.accent()
    } else {
        theme.base()
    };
    let (left, right) = if focused && options.show_arrows {
        (" ◀ ", " ▶")
    } else {
        (" ", "")
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{marker} {label:<width$}", width = options.label_width),
                style,
            ),
            Span::styled(format!("{left}{value}{right}"), style),
        ])),
        area,
    );
}
