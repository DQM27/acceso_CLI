use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};

use super::Theme;

/// Tamaño mínimo común de las pantallas operativas.
pub const MIN_TERMINAL_WIDTH: u16 = 60;
pub const MIN_TERMINAL_HEIGHT: u16 = 22;

/// A partir de este ancho, las vistas maestro-detalle usan panel lateral.
pub const MASTER_DETAIL_BREAKPOINT: u16 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorOrientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasterDetailAreas {
    pub master: Rect,
    pub detail: Rect,
    pub separator: Rect,
    pub orientation: SeparatorOrientation,
}

/// Distribución adaptable compartida por listados con panel de detalle.
pub fn master_detail_areas(
    area: Rect,
    master_width_percent: u16,
    stacked_detail_height: u16,
) -> MasterDetailAreas {
    if area.width >= MASTER_DETAIL_BREAKPOINT {
        let master_width_percent = master_width_percent.min(99);
        let columns = Layout::horizontal([
            Constraint::Percentage(master_width_percent),
            Constraint::Length(1),
            Constraint::Percentage(99 - master_width_percent),
        ])
        .split(area);
        MasterDetailAreas {
            master: columns[0],
            separator: columns[1],
            detail: columns[2],
            orientation: SeparatorOrientation::Vertical,
        }
    } else {
        let detail_height = stacked_detail_height.min(area.height.saturating_sub(5));
        let rows = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(detail_height),
        ])
        .split(area);
        MasterDetailAreas {
            master: rows[0],
            separator: rows[1],
            detail: rows[2],
            orientation: SeparatorOrientation::Horizontal,
        }
    }
}

pub fn render_separator(
    frame: &mut Frame,
    area: Rect,
    orientation: SeparatorOrientation,
    theme: Theme,
) {
    let content = match orientation {
        SeparatorOrientation::Vertical => {
            let lines: Vec<Line<'static>> = (0..area.height).map(|_| Line::from("│")).collect();
            Paragraph::new(lines)
        }
        SeparatorOrientation::Horizontal => Paragraph::new("─".repeat(area.width as usize)),
    };
    frame.render_widget(content.style(theme.border()), area);
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{SeparatorOrientation, master_detail_areas};

    #[test]
    fn cambia_de_panel_lateral_a_apilado_en_el_breakpoint_comun() {
        let lateral = master_detail_areas(Rect::new(0, 0, 100, 30), 60, 10);
        assert_eq!(lateral.orientation, SeparatorOrientation::Vertical);
        assert_eq!(lateral.separator.width, 1);

        let stacked = master_detail_areas(Rect::new(0, 0, 99, 30), 60, 10);
        assert_eq!(stacked.orientation, SeparatorOrientation::Horizontal);
        assert_eq!(stacked.separator.height, 1);
        assert_eq!(stacked.detail.height, 10);
    }

    #[test]
    fn acota_el_detalle_apilado_para_conservar_la_lista() {
        let areas = master_detail_areas(Rect::new(0, 0, 80, 8), 60, 20);
        assert_eq!(areas.detail.height, 3);
        assert_eq!(areas.master.height, 4);
    }
}
