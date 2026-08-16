//! Mockup de fluidez para empresas: mismo lenguaje que contratistas/
//! ingreso/activos, pero mucho más liviano porque la entidad sólo tiene un
//! campo editable (nombre) — no hace falta selector inline ni Tab entre
//! campos, un solo `Layer::Form` alcanza.
//!
//! cargo run --example brisas_cli -- empresas-v2

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

use super::quick_exit::{QuickExitOutcome, QuickExitOverlay};

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 20;
const WIDE_LAYOUT_WIDTH: u16 = 100;
/// Operador con sesión simulada, igual que en el resto de las pantallas v2.
const CURRENT_OPERATOR: &str = "Daniel Quintana";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Company {
    id: u32,
    name: String,
    contractors: u32,
}

#[derive(Debug)]
struct CompanyForm {
    id: Option<u32>,
    name: Input,
    error: Option<String>,
}

impl CompanyForm {
    fn new() -> Self {
        Self {
            id: None,
            name: Input::default(),
            error: None,
        }
    }

    fn edit(company: &Company) -> Self {
        Self {
            id: Some(company.id),
            name: Input::new(company.name.clone()),
            error: None,
        }
    }

    fn is_editing(&self) -> bool {
        self.id.is_some()
    }
}

#[derive(Debug)]
enum Panel {
    Detail,
    Form(Box<CompanyForm>),
}

/// `/` entra a modo búsqueda explícitamente — a diferencia de ingreso/
/// activos/historial, esta pantalla tiene un atajo de letra (`N`) que
/// necesita vivir fuera de un campo de texto siempre activo, o buscar
/// "Mantenimiento" secuestraría la `n` para abrir el formulario en vez de
/// escribirla.
#[derive(Debug, Clone)]
enum Mode {
    Browse,
    Search {
        original: String,
        selected_id: Option<u32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Browse,
    Search,
    Form,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatOrigin {
    code: KeyCode,
    modifiers: KeyModifiers,
    layer: Layer,
}

#[derive(Debug)]
pub struct EmpresasV2Pilot {
    companies: Vec<Company>,
    search: Input,
    mode: Mode,
    selected: usize,
    panel: Panel,
    prepared_message: Option<String>,
    help_expanded: bool,
    theme: ThemePreset,
    running: bool,
    terminal_size: (u16, u16),
    repeat_origin: Option<RepeatOrigin>,
    quick_exit: QuickExitOverlay,
}

impl Default for EmpresasV2Pilot {
    fn default() -> Self {
        Self {
            companies: demo_companies(),
            search: Input::default(),
            mode: Mode::Browse,
            selected: 0,
            panel: Panel::Detail,
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

impl EmpresasV2Pilot {
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
                QuickExitOutcome::Ignored => {}
                QuickExitOutcome::Consumed => return true,
                QuickExitOutcome::ExitRegistered(message) => {
                    self.prepared_message = Some(message);
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
            Layer::Browse => self.handle_browse_key(key),
            Layer::Search => self.handle_search_key(key),
            Layer::Form => self.handle_form_key(key),
        };
        if handled {
            return true;
        }
        self.handle_text_input(event)
    }

    fn handle_browse_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
        );
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => self.open_edit_form(),
            Some(StandardCommand::Cancel) => {
                self.running = false;
                return true;
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
                KeyCode::Char('/') => self.begin_search(),
                KeyCode::Char(character) if character.eq_ignore_ascii_case(&'n') => {
                    self.panel = Panel::Form(Box::new(CompanyForm::new()));
                    self.prepared_message = None;
                }
                _ => return false,
            },
        }
        true
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
            Some(StandardCommand::Primary) => self.apply_search(),
            Some(StandardCommand::Cancel) => self.cancel_search(),
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

    fn handle_form_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Repeat {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => self.save_form(),
            Some(StandardCommand::Cancel) => {
                self.panel = Panel::Detail;
                self.prepared_message = Some("Edición cancelada · no hubo cambios".into());
            }
            Some(StandardCommand::Help) => self.help_expanded = !self.help_expanded,
            Some(StandardCommand::Theme) => self.toggle_theme(),
            _ => return false,
        }
        true
    }

    fn handle_text_input(&mut self, event: &Event) -> bool {
        match &mut self.panel {
            Panel::Form(form) => {
                let changed = apply_event(&mut form.name, event);
                if changed {
                    form.error = None;
                }
                changed
            }
            Panel::Detail if matches!(self.mode, Mode::Search { .. }) => {
                let changed = apply_event(&mut self.search, event);
                if changed {
                    self.selected = 0;
                    self.prepared_message = None;
                }
                changed
            }
            Panel::Detail => false,
        }
    }

    fn begin_search(&mut self) {
        self.mode = Mode::Search {
            original: self.search.value().to_owned(),
            selected_id: self.selected_company().map(|company| company.id),
        };
    }

    fn apply_search(&mut self) {
        self.mode = Mode::Browse;
    }

    fn cancel_search(&mut self) {
        let Mode::Search {
            original,
            selected_id,
        } = self.mode.clone()
        else {
            return;
        };
        self.search = Input::new(original);
        self.select_id(selected_id);
        self.mode = Mode::Browse;
    }

    fn select_id(&mut self, id: Option<u32>) {
        let indices = self.filtered_indices();
        self.selected = id
            .and_then(|id| indices.iter().position(|&index| self.companies[index].id == id))
            .unwrap_or(0);
    }

    fn open_edit_form(&mut self) {
        let Some(company) = self.selected_company().cloned() else {
            return;
        };
        self.panel = Panel::Form(Box::new(CompanyForm::edit(&company)));
    }

    fn save_form(&mut self) {
        let Panel::Form(form) = &self.panel else {
            return;
        };
        let name = form.name.value().trim().to_owned();
        let error = if name.is_empty() {
            Some("El nombre es obligatorio")
        } else if self
            .companies
            .iter()
            .any(|company| company.name.eq_ignore_ascii_case(&name) && Some(company.id) != form.id)
        {
            Some("Ya existe una empresa con ese nombre")
        } else {
            None
        };
        if let Some(error) = error {
            if let Panel::Form(form) = &mut self.panel {
                form.error = Some(error.into());
            }
            return;
        }

        let saved_id = if let Some(id) = form.id {
            if let Some(company) = self.companies.iter_mut().find(|company| company.id == id) {
                company.name = name.clone();
            }
            id
        } else {
            let id = self.companies.iter().map(|company| company.id).max().unwrap_or(0) + 1;
            self.companies.push(Company {
                id,
                name: name.clone(),
                contractors: 0,
            });
            id
        };

        self.search = Input::default();
        self.selected = self
            .companies
            .iter()
            .position(|company| company.id == saved_id)
            .unwrap_or(0);
        self.panel = Panel::Detail;
        self.prepared_message = Some(format!("Empresa guardada en memoria · {name}"));
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let query = self.search.value().trim().to_lowercase();
        self.companies
            .iter()
            .enumerate()
            .filter_map(|(index, company)| {
                (query.is_empty() || company.name.to_lowercase().contains(&query)).then_some(index)
            })
            .collect()
    }

    fn selected_company(&self) -> Option<&Company> {
        let indices = self.filtered_indices();
        self.companies.get(*indices.get(self.selected)?)
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
        match (&self.panel, &self.mode) {
            (Panel::Form(_), _) => Layer::Form,
            (Panel::Detail, Mode::Search { .. }) => Layer::Search,
            (Panel::Detail, Mode::Browse) => Layer::Browse,
        }
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
            .format("%H:%M")
            .to_string();
        let columns =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(area);
        frame.render_widget(
            Paragraph::new("brisas cli · empresas (v2)").style(theme.muted()),
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
                label: &format!("BUSCAR · {count} RESULTADOS"),
                input: &self.search,
                focused: matches!(self.mode, Mode::Search { .. }),
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
            let stacked = Layout::vertical([
                Constraint::Min(4),
                Constraint::Length(1),
                Constraint::Length(7.min(rows[1].height.saturating_sub(5))),
            ])
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
        let rows = indices.iter().skip(start).take(capacity).map(|&index| {
            let company = &self.companies[index];
            Row::new([
                Cell::from(company.name.as_str()),
                Cell::from(company.contractors.to_string()),
            ])
            .style(theme.base())
        });
        let header = Row::new(["NOMBRE", "CONTRATISTAS"])
            .style(theme.muted())
            .bottom_margin(1);
        let table = Table::new(rows, [Constraint::Fill(4), Constraint::Length(14)])
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
                Paragraph::new("Sin empresas con ese filtro · Esc limpia la búsqueda")
                    .style(theme.warning())
                    .alignment(Alignment::Center),
                Rect::new(area.x, area.y + area.height / 2, area.width, 1),
            );
        }
    }

    fn render_panel(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        match &self.panel {
            Panel::Detail => self.render_detail(frame, area, theme),
            Panel::Form(form) => self.render_form(frame, area, form, theme),
        }
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let Some(company) = self.selected_company() else {
            frame.render_widget(
                Paragraph::new("No hay una empresa seleccionada").style(theme.muted()),
                area,
            );
            return;
        };
        let lines = vec![
            Line::from(company.name.as_str()).style(theme.title()),
            Line::from(""),
            Line::from(format!("Contratistas asociados: {}", company.contractors)).style(theme.base()),
        ];
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_form(&self, frame: &mut Frame, area: Rect, form: &CompanyForm, theme: Theme) {
        let rows = Layout::vertical([Constraint::Length(3), Constraint::Length(1)]).split(area);
        render_field(
            frame,
            rows[0],
            FieldSpec {
                label: if form.is_editing() { "NOMBRE" } else { "NOMBRE DE LA NUEVA EMPRESA" },
                input: &form.name,
                focused: true,
                theme,
            },
        );
        frame.render_widget(
            Paragraph::new(form.error.as_deref().unwrap_or_default()).style(theme.danger()),
            rows[1],
        );
    }

    fn status_line(&self, theme: Theme) -> Line<'static> {
        if let Panel::Form(form) = &self.panel
            && let Some(error) = &form.error
        {
            return Line::from(error.clone()).style(theme.danger());
        }
        if let Some(message) = &self.prepared_message {
            return Line::from(message.clone()).style(theme.success());
        }
        Line::from("").style(theme.muted())
    }

    fn hint_lines(&self, theme: Theme) -> Vec<Line<'static>> {
        let primary = match self.layer() {
            Layer::Browse => vec![
                Span::styled("↑↓", theme.accent()),
                Span::styled(" mover   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" editar   ", theme.base()),
                Span::styled("N", theme.accent()),
                Span::styled(" nueva   ", theme.base()),
                Span::styled("/", theme.accent()),
                Span::styled(" buscar   ", theme.base()),
                Span::styled(QUICK_EXIT_HINT.key, theme.accent()),
                Span::styled(format!(" {}   ", QUICK_EXIT_HINT.label), theme.base()),
                Span::styled("F1", theme.accent()),
                Span::styled(
                    if self.help_expanded { " cerrar ayuda" } else { " más" },
                    theme.base(),
                ),
            ],
            Layer::Search => vec![
                Span::styled("ENTER", theme.accent()),
                Span::styled(" aplicar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cancelar", theme.base()),
            ],
            Layer::Form => vec![
                Span::styled("ENTER", theme.accent()),
                Span::styled(" guardar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cancelar", theme.base()),
            ],
        };
        let mut lines = vec![Line::from(primary)];
        if self.help_expanded && self.layer() == Layer::Browse {
            lines.push(Line::from(vec![
                Span::styled("HOME/END", theme.accent()),
                Span::styled(" extremos   ", theme.base()),
                Span::styled("F7", theme.accent()),
                Span::styled(" tema   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" salir", theme.base()),
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

fn demo_companies() -> Vec<Company> {
    [
        (1, "Brisas del Oeste", 4),
        (2, "Aldama Servicios", 3),
        (3, "Expenic Industrial", 5),
        (4, "Logística Central", 2),
        (5, "Mantenimiento CR", 1),
    ]
    .into_iter()
    .map(|(id, name, contractors)| Company {
        id,
        name: name.into(),
        contractors,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{EmpresasV2Pilot, Layer, Panel, ThemePreset};

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
    fn el_detalle_esta_visible_desde_el_inicio() {
        let app = EmpresasV2Pilot::default();
        assert!(matches!(app.panel, Panel::Detail));
        assert_eq!(app.layer(), Layer::Browse);
    }

    #[test]
    fn enter_abre_edicion_junto_a_la_tabla_sin_taparla() {
        let mut app = EmpresasV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter));
        assert!(matches!(app.panel, Panel::Form(_)));

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("Brisas del Oeste"));
        assert!(rendered.contains("NOMBRE"));
    }

    #[test]
    fn n_abre_formulario_nuevo_y_exige_nombre() {
        let mut app = EmpresasV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("obligatorio")));
    }

    #[test]
    fn no_permite_duplicar_un_nombre_existente() {
        let mut app = EmpresasV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&Event::Paste("Brisas del Oeste".into()));
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("existe")));
    }

    #[test]
    fn editar_y_guardar_el_mismo_nombre_no_dispara_el_error_de_duplicado() {
        let mut app = EmpresasV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter)); // edita "Brisas del Oeste"
        app.handle_event(&key(KeyCode::Enter)); // guarda sin cambiar nada

        assert!(matches!(app.panel, Panel::Detail));
        assert!(
            app.prepared_message
                .as_deref()
                .is_some_and(|m| m.contains("Empresa guardada"))
        );
    }

    #[test]
    fn guardar_una_empresa_nueva_la_deja_seleccionada() {
        let mut app = EmpresasV2Pilot::default();
        let antes = app.companies.len();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&Event::Paste("Constructora Nueva".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.companies.len(), antes + 1);
        assert!(matches!(app.panel, Panel::Detail));
        assert_eq!(
            app.selected_company().map(|c| c.name.as_str()),
            Some("Constructora Nueva")
        );
    }

    #[test]
    fn buscar_filtra_por_nombre() {
        let mut app = EmpresasV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('/')));
        app.handle_event(&Event::Paste("Expenic".into()));

        assert_eq!(app.filtered_indices().len(), 1);
    }

    #[test]
    fn buscar_una_empresa_con_n_en_el_nombre_no_abre_el_formulario_nuevo() {
        // Esto es justo el bug que motivó separar Browse de Search: antes
        // "Mantenimiento" perdía su primera "n" al abrir el formulario.
        let mut app = EmpresasV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('/')));
        app.handle_event(&Event::Paste("Mantenimiento".into()));

        assert_eq!(app.search.value(), "Mantenimiento");
        assert!(matches!(app.panel, Panel::Detail));
        assert_eq!(app.filtered_indices().len(), 1);
    }

    #[test]
    fn escape_cancela_la_busqueda_y_restaura_la_seleccion_previa() {
        let mut app = EmpresasV2Pilot::default();
        app.handle_event(&key(KeyCode::Down));
        let seleccionada = app.selected_company().map(|c| c.id);

        app.handle_event(&key(KeyCode::Char('/')));
        app.handle_event(&Event::Paste("Expenic".into()));
        app.handle_event(&key(KeyCode::Esc));

        assert!(app.search.value().is_empty());
        assert_eq!(app.selected_company().map(|c| c.id), seleccionada);
        assert_eq!(app.layer(), Layer::Browse);
    }

    #[test]
    fn f2_funciona_en_medio_de_un_formulario_abierto() {
        let mut app = EmpresasV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&Event::Paste("Nombre a medio escribir".into()));

        app.handle_event(&key(KeyCode::F(2)));
        assert!(app.quick_exit.is_open());
        app.handle_event(&Event::Paste("12".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(!app.quick_exit.is_open());
        let Panel::Form(form) = &app.panel else {
            panic!("debe seguir en el formulario, con lo ya escrito");
        };
        assert_eq!(form.name.value(), "Nombre a medio escribir");
    }

    #[test]
    fn pinta_todo_el_lienzo_sin_dejar_parches_sin_color() {
        let mut app = EmpresasV2Pilot::default();
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
        let mut app = EmpresasV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        let count = app.companies.len();

        app.handle_event(&Event::Resize(40, 10));
        app.handle_event(&Event::Paste("Nueva".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.companies.len(), count);
    }
}
