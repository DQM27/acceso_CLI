//! Mockup de fluidez para ingresos activos y salidas: lista + panel de
//! detalle (mismo patrón que `contratistas_v2`/`ingreso_v2`), confirmación
//! de salida inline en la barra de estado (mismo patrón que `menu_v2`), y
//! F2 como salida rápida — incluida acá también por consistencia, aunque
//! esta pantalla ya resuelve lo mismo a tamaño completo.
//!
//! No existe un piloto v1 de esta pantalla para comparar: es nueva.
//!
//! cargo run --example brisas_cli -- activos-v2

use chrono::Utc;
use chrono_tz::America::Costa_Rica;
use control_acceso::tui::ui_kit::{
    QUICK_EXIT_HINT, StandardCommand, Theme, ThemePreset, render_terminal_too_small,
    standard_command,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

use super::quick_exit::QuickExitOverlay;

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 20;
const WIDE_LAYOUT_WIDTH: u16 = 100;
/// Operador con sesión simulada: cada pantalla lo muestra en la barra
/// superior y lo estampa al registrar, para que quede trazable quién hizo
/// cada acción. En la app real vendría de la sesión autenticada.
const CURRENT_OPERATOR: &str = "Daniel Quintana";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    Success,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveEntry {
    id: u32,
    badge: Option<i64>,
    identity: String,
    name: String,
    company: String,
    entry_type: String,
    medium: String,
    since: String,
    registered_by: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Search,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatOrigin {
    code: KeyCode,
    modifiers: KeyModifiers,
    layer: Layer,
}

#[derive(Debug)]
pub struct ActivosV2Pilot {
    entries: Vec<ActiveEntry>,
    search: Input,
    selected: usize,
    confirming: bool,
    prepared_message: Option<(String, Tone)>,
    help_expanded: bool,
    theme: ThemePreset,
    running: bool,
    terminal_size: (u16, u16),
    repeat_origin: Option<RepeatOrigin>,
    quick_exit: QuickExitOverlay,
}

impl Default for ActivosV2Pilot {
    fn default() -> Self {
        Self {
            entries: demo_entries(),
            search: Input::default(),
            selected: 0,
            confirming: false,
            prepared_message: None,
            help_expanded: false,
            theme: ThemePreset::Classic,
            running: true,
            terminal_size: (u16::MAX, u16::MAX),
            repeat_origin: None,
            quick_exit: QuickExitOverlay::demo(),
        }
    }
}

impl ActivosV2Pilot {
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::Resize(width, height) => {
                self.terminal_size = (*width, *height);
                return true;
            }
            Event::FocusGained | Event::FocusLost => return true,
            Event::Key(key) if key.kind == KeyEventKind::Release => {
                if self.repeat_origin.is_some_and(|origin| {
                    origin.code == key.code && origin.modifiers == key.modifiers
                }) {
                    self.repeat_origin = None;
                }
                return false;
            }
            _ => {}
        }

        let key = match event {
            Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                Some(*key)
            }
            Event::Paste(_) => None,
            _ => return false,
        };

        if key.is_some_and(|key| standard_command(key) == Some(StandardCommand::EmergencyExit)) {
            self.running = false;
            return true;
        }
        if !self.viewport_is_valid() {
            if key.is_some_and(|key| {
                key.kind == KeyEventKind::Press && standard_command(key) == Some(StandardCommand::Cancel)
            }) {
                self.running = false;
                return true;
            }
            return false;
        }

        if self.quick_exit.is_open() {
            match self.quick_exit.handle_event(event) {
                super::quick_exit::QuickExitOutcome::Ignored => {}
                super::quick_exit::QuickExitOutcome::Consumed => return true,
                super::quick_exit::QuickExitOutcome::ExitRegistered(message) => {
                    self.prepared_message = Some((message, Tone::Success));
                    return true;
                }
            }
        }
        if key.is_some_and(|key| {
            key.kind == KeyEventKind::Press && standard_command(key) == Some(StandardCommand::QuickExit)
        }) {
            self.quick_exit.open();
            return true;
        }

        let Some(key) = key else {
            return self.handle_text_input(event);
        };

        let origin = RepeatOrigin {
            code: key.code,
            modifiers: key.modifiers,
            layer: self.layer(),
        };
        match key.kind {
            KeyEventKind::Press => self.repeat_origin = Some(origin),
            KeyEventKind::Repeat if self.repeat_origin != Some(origin) => return false,
            KeyEventKind::Repeat | KeyEventKind::Release => {}
        }

        let handled = match self.layer() {
            Layer::Search => self.handle_search_key(key),
            Layer::Confirm => self.handle_confirm_key(key),
        };
        if handled {
            return true;
        }
        self.handle_text_input(event)
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
        );
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => {
                if self.selected_entry().is_some() {
                    self.confirming = true;
                } else {
                    self.prepared_message =
                        Some(("No hay un registro seleccionado.".into(), Tone::Warning));
                }
            }
            Some(StandardCommand::Cancel) => {
                if self.search.value().is_empty() {
                    self.running = false;
                    return true;
                }
                self.search = Input::default();
                self.selected = 0;
                self.prepared_message = None;
            }
            Some(StandardCommand::Help) if key.kind == KeyEventKind::Press => {
                self.help_expanded = !self.help_expanded;
            }
            Some(StandardCommand::Theme) if key.kind == KeyEventKind::Press => self.toggle_theme(),
            _ => match key.code {
                KeyCode::Up => self.move_selection(-1),
                KeyCode::Down => self.move_selection(1),
                KeyCode::Home => self.select(0),
                KeyCode::End => self.select(self.filtered_indices().len().saturating_sub(1)),
                _ => return false,
            },
        }
        true
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Repeat {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => self.confirm_exit(),
            Some(StandardCommand::Cancel) => {
                self.confirming = false;
                self.prepared_message = Some(("Salida cancelada · no hubo cambios.".into(), Tone::Warning));
            }
            Some(StandardCommand::Theme) => self.toggle_theme(),
            _ => return false,
        }
        true
    }

    fn handle_text_input(&mut self, event: &Event) -> bool {
        if self.confirming {
            return false;
        }
        let changed = apply_event(&mut self.search, event);
        if changed {
            self.selected = 0;
            self.prepared_message = None;
        }
        changed
    }

    fn confirm_exit(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            self.confirming = false;
            return;
        };
        self.entries.retain(|candidate| candidate.id != entry.id);
        self.confirming = false;
        self.select(0);
        let badge_text = entry
            .badge
            .map_or_else(|| "S/G".into(), |number| format!("gafete {number}"));
        self.prepared_message = Some((
            format!("Salida registrada · {} · {badge_text}", entry.name),
            Tone::Success,
        ));
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let query = self.search.value().trim().to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                (query.is_empty()
                    || entry.identity.to_lowercase().contains(&query)
                    || entry.name.to_lowercase().contains(&query)
                    || entry.company.to_lowercase().contains(&query)
                    || entry.badge.is_some_and(|badge| badge.to_string().contains(&query)))
                .then_some(index)
            })
            .collect()
    }

    fn selected_entry(&self) -> Option<&ActiveEntry> {
        let indices = self.filtered_indices();
        self.entries.get(*indices.get(self.selected)?)
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.filtered_indices().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self.selected.saturating_add_signed(delta).min(count - 1);
    }

    fn select(&mut self, index: usize) {
        let count = self.filtered_indices().len();
        self.selected = index.min(count.saturating_sub(1));
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
    }

    fn layer(&self) -> Layer {
        if self.confirming { Layer::Confirm } else { Layer::Search }
    }

    fn viewport_is_valid(&self) -> bool {
        self.terminal_size.0 >= MIN_WIDTH && self.terminal_size.1 >= MIN_HEIGHT
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.terminal_size = (area.width, area.height);
        let theme = self.theme.theme();

        if !self.viewport_is_valid() {
            render_terminal_too_small(frame, area, MIN_WIDTH, MIN_HEIGHT, "ESC salir", theme);
            return;
        }

        frame.render_widget(Block::default().style(theme.base()), area);

        let hint_lines = self.hint_lines(theme);
        let hint_height = hint_lines.len() as u16;
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(1),
            Constraint::Length(hint_height),
        ])
        .split(area.inner(Margin {
            horizontal: 2,
            vertical: 1,
        }));

        self.render_top_bar(frame, rows[0], theme);
        self.render_body(frame, rows[1], theme);
        frame.render_widget(self.status_line(theme), rows[2]);
        frame.render_widget(Paragraph::new(hint_lines), rows[3]);

        self.quick_exit.render(frame, area, theme);
    }

    fn render_top_bar(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let clock = Utc::now()
            .with_timezone(&Costa_Rica)
            .format("%H:%M:%S")
            .to_string();
        let columns =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(area);
        frame.render_widget(
            Paragraph::new("brisas cli · ingresos activos (v2)").style(theme.muted()),
            columns[0],
        );
        frame.render_widget(
            Paragraph::new(format!("{CURRENT_OPERATOR} · {clock}"))
                .style(theme.muted())
                .alignment(Alignment::Right),
            columns[1],
        );
    }

    fn render_body(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let rows = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);
        let count = self.filtered_indices().len();
        render_field(
            frame,
            rows[0],
            FieldSpec {
                label: &format!("BUSCAR · {count} DENTRO"),
                input: &self.search,
                focused: !self.confirming,
                theme,
            },
        );

        if area.width >= WIDE_LAYOUT_WIDTH {
            let columns = Layout::horizontal([
                Constraint::Percentage(63),
                Constraint::Length(1),
                Constraint::Percentage(35),
            ])
            .split(rows[1]);
            self.render_table(frame, columns[0], theme);
            render_vertical_separator(frame, columns[1], theme);
            self.render_panel(frame, columns[2], theme);
        } else {
            let stacked = Layout::vertical([Constraint::Min(4), Constraint::Length(1), Constraint::Length(9)])
                .split(rows[1]);
            self.render_table(frame, stacked[0], theme);
            render_horizontal_separator(frame, stacked[1], theme);
            self.render_panel(frame, stacked[2], theme);
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let indices = self.filtered_indices();
        self.selected = self.selected.min(indices.len().saturating_sub(1));
        let capacity = area.height.saturating_sub(2) as usize;
        let start = self.selected.saturating_sub(capacity.saturating_sub(1));
        let wide = area.width >= 80;
        let rows = indices.iter().skip(start).take(capacity).map(|index| {
            let entry = &self.entries[*index];
            let badge = entry
                .badge
                .map_or_else(|| "S/G".to_owned(), |number| number.to_string());
            let cells = if wide {
                vec![
                    Cell::from(entry.identity.as_str()),
                    Cell::from(entry.name.as_str()),
                    Cell::from(entry.company.as_str()),
                    Cell::from(entry.since.as_str()),
                    Cell::from(badge),
                ]
            } else {
                vec![
                    Cell::from(entry.identity.as_str()),
                    Cell::from(entry.name.as_str()),
                    Cell::from(entry.since.as_str()),
                ]
            };
            Row::new(cells).style(theme.base())
        });
        let (headers, widths) = if wide {
            (
                vec!["CÉDULA", "NOMBRE", "EMPRESA", "DESDE", "GAFETE"],
                vec![
                    Constraint::Length(14),
                    Constraint::Fill(3),
                    Constraint::Fill(2),
                    Constraint::Length(8),
                    Constraint::Length(8),
                ],
            )
        } else {
            (
                vec!["CÉDULA", "NOMBRE", "DESDE"],
                vec![
                    Constraint::Length(14),
                    Constraint::Fill(1),
                    Constraint::Length(8),
                ],
            )
        };
        let header = Row::new(headers).style(theme.muted()).bottom_margin(1);
        let table = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(theme.selected())
            .highlight_symbol("> ")
            .column_spacing(1);
        let selected = (!indices.is_empty()).then_some(self.selected.saturating_sub(start));
        frame.render_stateful_widget(
            table,
            area,
            &mut TableState::default().with_selected(selected),
        );

        if indices.is_empty() {
            frame.render_widget(
                Paragraph::new("Nadie dentro con ese filtro · Esc limpia la búsqueda")
                    .style(theme.warning())
                    .alignment(Alignment::Center),
                Rect::new(area.x, area.y + area.height / 2, area.width, 1),
            );
        }
    }

    fn render_panel(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let Some(entry) = self.selected_entry() else {
            frame.render_widget(
                Paragraph::new("No hay un registro seleccionado").style(theme.muted()),
                area,
            );
            return;
        };
        let badge_text = entry
            .badge
            .map_or_else(|| "Sin gafete".to_owned(), |number| format!("Gafete {number}"));
        let lines = vec![
            Line::from(entry.name.as_str()).style(theme.title()),
            Line::from(format!("{} · {}", entry.identity, entry.company)).style(theme.base()),
            Line::from(entry.entry_type.as_str()).style(theme.muted()),
            Line::from(""),
            Line::from(format!("Dentro desde las {}", entry.since)).style(theme.warning()),
            Line::from(entry.medium.as_str()).style(theme.base()),
            Line::from(badge_text).style(theme.muted()),
            Line::from(format!("Registrado por {}", entry.registered_by)).style(theme.muted()),
        ];
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn status_line(&self, theme: Theme) -> Line<'static> {
        if self.confirming
            && let Some(entry) = self.selected_entry()
        {
            return Line::from(format!("¿Registrar la salida de {}?", entry.name)).style(theme.warning());
        }
        if let Some((message, tone)) = &self.prepared_message {
            let style = match tone {
                Tone::Success => theme.success(),
                Tone::Warning => theme.warning(),
            };
            return Line::from(message.clone()).style(style);
        }
        Line::from("").style(theme.muted())
    }

    fn hint_lines(&self, theme: Theme) -> Vec<Line<'static>> {
        let primary = if self.confirming {
            vec![
                Span::styled("ENTER", theme.accent()),
                Span::styled(" confirmar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cancelar", theme.base()),
            ]
        } else {
            vec![
                Span::styled("↑↓", theme.accent()),
                Span::styled(" mover   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" registrar salida   ", theme.base()),
                Span::styled(QUICK_EXIT_HINT.key, theme.accent()),
                Span::styled(format!(" {}   ", QUICK_EXIT_HINT.label), theme.base()),
                Span::styled("F1", theme.accent()),
                Span::styled(
                    if self.help_expanded { " cerrar ayuda" } else { " más" },
                    theme.base(),
                ),
            ]
        };
        let mut lines = vec![Line::from(primary)];
        if self.help_expanded && !self.confirming {
            lines.push(Line::from(vec![
                Span::styled("HOME/END", theme.accent()),
                Span::styled(" extremos   ", theme.base()),
                Span::styled("F7", theme.accent()),
                Span::styled(" tema   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" limpiar/salir", theme.base()),
            ]));
        }
        lines
    }
}

fn apply_event(input: &mut Input, event: &Event) -> bool {
    if let Event::Paste(text) = event {
        let mut changed = false;
        for character in text.chars().filter(|character| !character.is_control()) {
            input.handle(InputRequest::InsertChar(character));
            changed = true;
        }
        changed
    } else {
        input.handle_event(event).is_some()
    }
}

struct FieldSpec<'a> {
    label: &'a str,
    input: &'a Input,
    focused: bool,
    theme: Theme,
}

fn render_field(frame: &mut Frame, area: Rect, spec: FieldSpec<'_>) {
    let FieldSpec {
        label,
        input,
        focused,
        theme,
    } = spec;
    let label_style = if focused { theme.accent() } else { theme.muted() };
    let line_style = if focused { theme.accent() } else { theme.border() };
    let value_y = area.y.saturating_add(1);
    let line_y = area.y.saturating_add(2);

    frame.render_widget(
        Paragraph::new(label).style(label_style),
        Rect::new(area.x, area.y, area.width, 1),
    );
    let viewport_width = area.width.saturating_sub(1) as usize;
    let visual_cursor = input.visual_cursor();
    let scroll = visual_cursor.saturating_sub(viewport_width);
    frame.render_widget(
        Paragraph::new(Line::from(input.value().to_owned()).style(theme.base()))
            .scroll((0, scroll as u16)),
        Rect::new(area.x, value_y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(line_style),
        Rect::new(area.x, line_y, area.width, 1),
    );
    if focused {
        let column = visual_cursor.saturating_sub(scroll).min(viewport_width);
        frame.set_cursor_position((area.x + column as u16, value_y));
    }
}

fn render_vertical_separator(frame: &mut Frame, area: Rect, theme: Theme) {
    let lines: Vec<Line<'static>> = (0..area.height).map(|_| Line::from("│")).collect();
    frame.render_widget(Paragraph::new(lines).style(theme.border()), area);
}

fn render_horizontal_separator(frame: &mut Frame, area: Rect, theme: Theme) {
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(theme.border()),
        area,
    );
}

fn demo_entries() -> Vec<ActiveEntry> {
    // Operadores variados a propósito: en un turno real no siempre registra
    // la misma persona, y eso es justo lo que "quién lo registró" responde.
    [
        (1, Some(12), "1-1042-0881", "José Peña", "Brisas del Oeste", "PRAIND", "En vehículo", "07:42", "Marta Solano"),
        (2, Some(25), "3-0520-0917", "Juan Rodríguez", "Expenic Industrial", "PRAIND", "Caminando", "08:10", "Marta Solano"),
        (3, None, "2-0731-0440", "Ana María Solís", "Aldama Servicios", "IN HOUSE", "Caminando", "08:55", CURRENT_OPERATOR),
        (4, Some(48), "1-1550-0239", "Carlos Méndez", "Brisas del Oeste", "SWAT", "En vehículo", "09:20", CURRENT_OPERATOR),
    ]
    .into_iter()
    .map(
        |(id, badge, identity, name, company, entry_type, medium, since, registered_by)| ActiveEntry {
            id,
            badge,
            identity: identity.into(),
            name: name.into(),
            company: company.into(),
            entry_type: entry_type.into(),
            medium: medium.into(),
            since: since.into(),
            registered_by: registered_by.into(),
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{ActivosV2Pilot, Layer, ThemePreset};

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn buffer_text(backend: &TestBackend) -> String {
        backend
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn enter_pide_confirmacion_sin_ocultar_la_lista() {
        let mut app = ActivosV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter));
        assert_eq!(app.layer(), Layer::Confirm);

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("José Peña"));
        assert!(rendered.contains("¿Registrar la salida de José Peña?"));
    }

    #[test]
    fn el_panel_muestra_quien_registro_el_ingreso_y_el_medio() {
        let mut app = ActivosV2Pilot::default();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("Registrado por Marta Solano"));
        assert!(rendered.contains("En vehículo"));
    }

    #[test]
    fn confirmar_quita_a_la_persona_de_la_lista_de_activos() {
        let mut app = ActivosV2Pilot::default();
        let antes = app.entries.len();
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.entries.len(), antes - 1);
        assert!(
            app.prepared_message
                .as_ref()
                .is_some_and(|(m, _)| m.contains("Salida registrada · José Peña"))
        );
    }

    #[test]
    fn escape_cancela_la_confirmacion_sin_quitar_a_nadie() {
        let mut app = ActivosV2Pilot::default();
        let antes = app.entries.len();
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::Esc));

        assert_eq!(app.entries.len(), antes);
        assert_eq!(app.layer(), Layer::Search);
    }

    #[test]
    fn f2_abre_la_salida_rapida_y_registra_sin_tocar_la_pantalla_de_atras() {
        let mut app = ActivosV2Pilot::default();
        app.handle_event(&key(KeyCode::F(2)));
        assert!(app.quick_exit.is_open());

        app.handle_event(&Event::Paste("25".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(!app.quick_exit.is_open());
        assert!(
            app.prepared_message
                .as_ref()
                .is_some_and(|(m, _)| m.contains("Juan Rodríguez"))
        );
        // La pantalla de activos conserva sus propios 4 registros: el
        // overlay opera sobre su propio set de datos de demostración.
        assert_eq!(app.entries.len(), 4);
    }

    #[test]
    fn pinta_todo_el_lienzo_sin_dejar_parches_sin_color() {
        let mut app = ActivosV2Pilot::default();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");

        let theme = ThemePreset::Classic.theme();
        let buffer = terminal.backend().buffer();
        for (x, y) in [(0, 0), (119, 0), (0, 29), (119, 29), (60, 15)] {
            assert_eq!(
                buffer[(x, y)].bg,
                theme.background,
                "celda ({x},{y}) quedó con el fondo por defecto de la terminal"
            );
        }
    }

    #[test]
    fn terminal_pequena_bloquea_mutaciones_invisibles() {
        let mut app = ActivosV2Pilot::default();
        let antes = app.entries.len();
        app.handle_event(&Event::Resize(40, 10));
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.entries.len(), antes);
    }
}
