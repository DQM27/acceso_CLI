use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::{EMERGENCY_EXIT_HINT, THEME_HINT, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Normal,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandHint<'a> {
    pub key: &'a str,
    pub label: &'a str,
}

impl<'a> CommandHint<'a> {
    pub const fn new(key: &'a str, label: &'a str) -> Self {
        Self { key, label }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellAreas {
    pub body: Rect,
    pub status: Rect,
    pub commands: Rect,
}

/// Marco común de una pantalla: identidad, contexto, estado y comandos.
pub struct ScreenShell<'a> {
    pub product: &'a str,
    pub screen: &'a str,
    pub context: &'a str,
    pub clock: &'a str,
    pub status: &'a str,
    pub status_kind: StatusKind,
    pub commands: &'a [CommandHint<'a>],
    /// Si la ayuda está expandida (F1), se agrega una segunda línea de pie con
    /// los atajos transversales menos frecuentes (tema, salida de emergencia).
    pub help_expanded: bool,
}

impl ScreenShell<'_> {
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: Theme) -> ShellAreas {
        frame.render_widget(Block::default().style(theme.base()), area);
        let viewport = area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });

        let command_lines = self.command_lines(viewport.width as usize, theme);
        let command_height = u16::try_from(command_lines.len()).unwrap_or(u16::MAX);

        let rows = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(command_height),
        ])
        .split(viewport);

        self.render_header(frame, rows[0], theme);
        frame.render_widget(
            Paragraph::new("─".repeat(viewport.width as usize)).style(theme.border()),
            rows[1],
        );
        self.render_status(frame, rows[3], theme);
        frame.render_widget(Paragraph::new(command_lines), rows[4]);

        ShellAreas {
            body: rows[2],
            status: rows[3],
            commands: rows[4],
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        if area.width < 90 {
            let columns =
                Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
                    .split(area);
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(self.product).style(theme.title()),
                    Line::from(self.screen).style(theme.accent()),
                ]),
                columns[0],
            );
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(self.context).style(theme.muted()),
                    Line::from(self.clock).style(theme.warning()),
                ])
                .alignment(Alignment::Right),
                columns[1],
            );
            return;
        }

        let columns = Layout::horizontal([
            Constraint::Percentage(38),
            Constraint::Percentage(34),
            Constraint::Percentage(28),
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(self.product).style(theme.title()),
                Line::from("CONTROL DE ACCESO").style(theme.muted()),
            ]),
            columns[0],
        );
        frame.render_widget(
            Paragraph::new(self.screen)
                .style(theme.accent())
                .alignment(Alignment::Center),
            Rect::new(columns[1].x, columns[1].y + 1, columns[1].width, 1),
        );
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(self.context).style(theme.muted()),
                Line::from(self.clock).style(theme.warning()),
            ])
            .alignment(Alignment::Right),
            columns[2],
        );
    }

    fn render_status(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let style = match self.status_kind {
            StatusKind::Normal => theme.muted(),
            StatusKind::Success => theme.success(),
            StatusKind::Warning => theme.warning(),
            StatusKind::Error => theme.danger(),
        };
        frame.render_widget(Paragraph::new(self.status).style(style), area);
    }

    fn command_lines(&self, max_width: usize, theme: Theme) -> Vec<Line<'static>> {
        let ayuda_label = if self.help_expanded {
            "cerrar ayuda"
        } else {
            "más"
        };
        let primary: Vec<CommandHint<'_>> = self
            .commands
            .iter()
            .copied()
            .chain(std::iter::once(CommandHint::new("F1", ayuda_label)))
            .collect();

        let mut lines = wrap_commands(&primary, max_width, theme);
        if self.help_expanded {
            lines.extend(wrap_commands(
                &[THEME_HINT, EMERGENCY_EXIT_HINT],
                max_width,
                theme,
            ));
        }
        if lines.is_empty() {
            lines.push(Line::default());
        }
        lines
    }
}

fn wrap_commands(commands: &[CommandHint<'_>], max_width: usize, theme: Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut line_width = 0;

    for command in commands {
        let command_width = Line::from(format!("{} {}", command.key, command.label)).width();
        let separator_width = usize::from(line_width > 0) * 2;
        if line_width > 0 && line_width + separator_width + command_width > max_width {
            lines.push(Line::from(std::mem::take(&mut spans)));
            line_width = 0;
        }
        if line_width > 0 {
            spans.push(Span::styled("  ", theme.base()));
            line_width += 2;
        }
        spans.push(Span::styled(command.key.to_owned(), theme.accent()));
        spans.push(Span::styled(format!(" {}", command.label), theme.base()));
        line_width += command_width;
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}

pub fn panel(title: &str, theme: Theme, focused: bool) -> Block<'static> {
    styled_panel(title, theme, focused, Alignment::Left)
}

/// Marco de una ventana auxiliar: el título se centra sin centrar su contenido.
pub fn auxiliary_panel(title: &str, theme: Theme, focused: bool) -> Block<'static> {
    styled_panel(title, theme, focused, Alignment::Center)
}

/// Centra horizontalmente una columna cuyo contenido conservará alineación izquierda.
pub fn centered_content(area: Rect, max_width: u16) -> Rect {
    let width = max_width.min(area.width);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y,
        width,
        area.height,
    )
}

/// Centra un rectángulo dentro del área disponible y respeta un margen exterior.
pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(2));
    let height = height.min(area.height.saturating_sub(2));
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

/// Aviso común que congela visualmente una vista cuando no cabe con seguridad.
pub fn render_terminal_too_small(
    frame: &mut Frame,
    area: Rect,
    min_width: u16,
    min_height: u16,
    exit_hint: &str,
    theme: Theme,
) {
    frame.render_widget(Block::default().style(theme.base()), area);
    let y = area.y + area.height.saturating_sub(3) / 2;
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("TERMINAL DEMASIADO PEQUEÑA").style(theme.warning()),
            Line::from(format!(
                "Actual: {}×{} · mínimo: {min_width}×{min_height}",
                area.width, area.height
            ))
            .style(theme.base()),
            Line::from(format!("Amplíe para continuar · {exit_hint}")).style(theme.muted()),
        ])
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 3),
    );
}

fn styled_panel(
    title: &str,
    theme: Theme,
    focused: bool,
    title_alignment: Alignment,
) -> Block<'static> {
    let style = if focused {
        theme.accent()
    } else {
        theme.border()
    };
    Block::default()
        .title(format!(" {title} "))
        .title_alignment(title_alignment)
        .title_style(style)
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(style)
        .style(theme.base())
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};

    use super::{CommandHint, ScreenShell, StatusKind, centered_content, centered_rect};
    use crate::tui::ui_kit::ThemePreset;

    #[test]
    fn centra_una_columna_sin_cambiar_su_alineacion_interna() {
        let area = Rect::new(10, 4, 30, 8);

        assert_eq!(centered_content(area, 20), Rect::new(15, 4, 20, 8));
        assert_eq!(centered_content(area, 40), area);
    }

    #[test]
    fn centra_un_rectangulo_y_conserva_un_margen_exterior() {
        let area = Rect::new(10, 4, 30, 12);

        assert_eq!(centered_rect(area, 20, 6), Rect::new(15, 7, 20, 6));
        assert_eq!(centered_rect(area, 40, 20), Rect::new(11, 5, 28, 10));
    }

    #[test]
    fn marco_de_pantalla_reserva_una_celda_en_los_cuatro_bordes() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        let commands = [CommandHint::new("ESC", "Salir")];
        let shell = ScreenShell {
            product: "BRISAS CLI",
            screen: "PRUEBA",
            context: "LOCAL",
            clock: "12:00:00",
            status: "Preparado",
            status_kind: StatusKind::Normal,
            commands: &commands,
            help_expanded: false,
        };

        terminal
            .draw(|frame| {
                shell.render(frame, frame.area(), ThemePreset::Classic.theme());
            })
            .expect("el marco debe renderizar");

        let buffer = terminal.backend().buffer();
        for x in 0..buffer.area.width {
            assert_eq!(buffer[(x, 0)].symbol(), " ");
            assert_eq!(buffer[(x, buffer.area.height - 1)].symbol(), " ");
        }
        for y in 0..buffer.area.height {
            assert_eq!(buffer[(0, y)].symbol(), " ");
            assert_eq!(buffer[(buffer.area.width - 1, y)].symbol(), " ");
        }
    }

    fn texto_renderizado(shell: &ScreenShell<'_>) -> String {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal
            .draw(|frame| {
                shell.render(frame, frame.area(), ThemePreset::Classic.theme());
            })
            .expect("debe renderizar");
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn f1_agrega_una_segunda_linea_con_tema_y_salida_de_emergencia() {
        let commands = [CommandHint::new("ESC", "Salir")];
        let mut shell = ScreenShell {
            product: "BRISAS CLI",
            screen: "PRUEBA",
            context: "LOCAL",
            clock: "12:00:00",
            status: "Preparado",
            status_kind: StatusKind::Normal,
            commands: &commands,
            help_expanded: false,
        };

        let colapsado = texto_renderizado(&shell);
        assert!(colapsado.contains("F1"));
        assert!(!colapsado.contains("CTRL+C"));
        assert!(!colapsado.contains("Salida de emergencia"));

        shell.help_expanded = true;
        let expandido = texto_renderizado(&shell);
        assert!(expandido.contains("cerrar ayuda"));
        assert!(expandido.contains("CTRL+C"));
        assert!(expandido.contains("Salida de emergencia"));
        assert!(expandido.contains("F7"));
        assert!(expandido.contains("Tema"));
    }
}
