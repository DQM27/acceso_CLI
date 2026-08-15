//! Mockup de fluidez para contratistas: detalle y formulario comparten un
//! único panel lateral (nunca un modal flotante), el selector de empresa se
//! expande inline en vez de abrir una capa aparte, y los campos de texto
//! reutilizan la silueta fija de `login_v2`.
//!
//! cargo run --example brisas_cli -- contratistas       (piloto actual)
//! cargo run --example brisas_cli -- contratistas-v2    (esta propuesta)

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

const COMPANIES: [&str; 5] = [
    "Brisas del Oeste",
    "Aldama Servicios",
    "Expenic Industrial",
    "Logística Central",
    "Mantenimiento CR",
];
const ENTRY_TYPES: [&str; 4] = ["PRAIND", "IN HOUSE", "POR CORREO", "SWAT"];
const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 20;
/// Operador con sesión simulada: cada pantalla lo muestra en la barra
/// superior y lo estampa al registrar, para que quede trazable quién hizo
/// cada acción. En la app real vendría de la sesión autenticada.
const CURRENT_OPERATOR: &str = "Daniel Quintana";
const WIDE_LAYOUT_WIDTH: u16 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Contractor {
    id: u32,
    identity: String,
    name: String,
    company: String,
    entry_type: String,
    route_staff: bool,
    has_access: bool,
}

#[derive(Debug, Clone)]
enum Mode {
    Browse,
    Search {
        original: String,
        selected_id: Option<u32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormField {
    Identity,
    Name,
    Company,
    EntryType,
    RouteStaff,
    Access,
}

impl FormField {
    const NEW_FIELDS: [Self; 6] = [
        Self::Identity,
        Self::Name,
        Self::Company,
        Self::EntryType,
        Self::RouteStaff,
        Self::Access,
    ];
    const EDIT_FIELDS: [Self; 5] = [
        Self::Name,
        Self::Company,
        Self::EntryType,
        Self::RouteStaff,
        Self::Access,
    ];
}

#[derive(Debug)]
struct ContractorForm {
    id: Option<u32>,
    identity: Input,
    name: Input,
    company: usize,
    company_expanded: Option<usize>,
    entry_type: usize,
    route_staff: bool,
    has_access: bool,
    focus: usize,
    error: Option<String>,
}

impl ContractorForm {
    fn new() -> Self {
        Self {
            id: None,
            identity: Input::default(),
            name: Input::default(),
            company: 0,
            company_expanded: None,
            entry_type: 0,
            route_staff: false,
            has_access: true,
            focus: 0,
            error: None,
        }
    }

    fn edit(contractor: &Contractor) -> Self {
        Self {
            id: Some(contractor.id),
            identity: Input::new(contractor.identity.clone()),
            name: Input::new(contractor.name.clone()),
            company: COMPANIES
                .iter()
                .position(|company| *company == contractor.company)
                .unwrap_or(0),
            company_expanded: None,
            entry_type: ENTRY_TYPES
                .iter()
                .position(|entry_type| *entry_type == contractor.entry_type)
                .unwrap_or(0),
            route_staff: contractor.route_staff,
            has_access: contractor.has_access,
            focus: 0,
            error: None,
        }
    }

    fn is_editing(&self) -> bool {
        self.id.is_some()
    }

    fn fields(&self) -> &'static [FormField] {
        if self.is_editing() {
            &FormField::EDIT_FIELDS
        } else {
            &FormField::NEW_FIELDS
        }
    }

    fn field(&self) -> FormField {
        self.fields()[self.focus]
    }

    fn move_focus(&mut self, backwards: bool) {
        let len = self.fields().len();
        self.focus = if backwards {
            self.focus.checked_sub(1).unwrap_or(len.saturating_sub(1))
        } else {
            (self.focus + 1) % len.max(1)
        };
        self.error = None;
    }
}

#[derive(Debug)]
enum Panel {
    Detail,
    Form(Box<ContractorForm>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Browse,
    Search,
    Form,
    CompanyExpand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatOrigin {
    code: KeyCode,
    modifiers: KeyModifiers,
    layer: Layer,
}

#[derive(Debug)]
pub struct ContratistasV2Pilot {
    contractors: Vec<Contractor>,
    selected: usize,
    table_offset: usize,
    page_size: usize,
    search: Input,
    mode: Mode,
    panel: Panel,
    prepared_message: Option<String>,
    help_expanded: bool,
    theme: ThemePreset,
    running: bool,
    terminal_size: (u16, u16),
    repeat_origin: Option<RepeatOrigin>,
    quick_exit: QuickExitOverlay,
}

impl Default for ContratistasV2Pilot {
    fn default() -> Self {
        Self {
            contractors: demo_contractors(),
            selected: 0,
            table_offset: 0,
            page_size: 8,
            search: Input::default(),
            mode: Mode::Browse,
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

impl ContratistasV2Pilot {
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
                key.kind == KeyEventKind::Press
                    && (standard_command(key) == Some(StandardCommand::Cancel)
                        || key_char(key, 'q'))
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
            Layer::CompanyExpand => self.handle_company_expand_key(key),
        };
        if handled {
            return true;
        }
        self.handle_text_input(event)
    }

    fn handle_browse_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(
            key.code,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Home
                | KeyCode::End
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
                KeyCode::PageUp => self.move_selection(-(self.page_size.max(1) as isize)),
                KeyCode::PageDown => self.move_selection(self.page_size.max(1) as isize),
                KeyCode::Home => self.select_first(),
                KeyCode::End => self.select_last(),
                KeyCode::Char('/') => self.begin_search(),
                KeyCode::Char(character) if character.eq_ignore_ascii_case(&'n') => {
                    self.open_new_form();
                }
                KeyCode::Char(character) if character.eq_ignore_ascii_case(&'e') => {
                    self.open_edit_form();
                }
                KeyCode::Char(character) if character.eq_ignore_ascii_case(&'q') => {
                    self.running = false;
                    return true;
                }
                _ => return false,
            },
        }
        true
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(
            key.code,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Home
                | KeyCode::End
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
                KeyCode::PageUp => self.move_selection(-(self.page_size.max(1) as isize)),
                KeyCode::PageDown => self.move_selection(self.page_size.max(1) as isize),
                KeyCode::Home => self.select_first(),
                KeyCode::End => self.select_last(),
                _ => return false,
            },
        }
        true
    }

    fn handle_form_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Repeat && !matches!(key.code, KeyCode::Left | KeyCode::Right) {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => self.save_form(),
            Some(StandardCommand::Activate) => {
                if matches!(self.form_field(), Some(FormField::Identity | FormField::Name)) {
                    return false;
                }
                self.activate_form_field();
            }
            Some(StandardCommand::Cancel) => self.cancel_form(),
            Some(StandardCommand::FocusNext) => self.move_form_focus(false),
            Some(StandardCommand::FocusPrevious) => self.move_form_focus(true),
            Some(StandardCommand::Help) if key.kind == KeyEventKind::Press => {
                self.help_expanded = !self.help_expanded;
            }
            Some(StandardCommand::Theme) if key.kind == KeyEventKind::Press => self.toggle_theme(),
            _ => match key.code {
                KeyCode::Left => self.cycle_form_value(true),
                KeyCode::Right => self.cycle_form_value(false),
                _ => return false,
            },
        }
        true
    }

    fn handle_company_expand_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
        );
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => {
                if let Panel::Form(form) = &mut self.panel
                    && let Some(highlighted) = form.company_expanded
                {
                    form.company = highlighted;
                    form.error = None;
                }
                if let Panel::Form(form) = &mut self.panel {
                    form.company_expanded = None;
                }
            }
            Some(StandardCommand::Cancel) => {
                if let Panel::Form(form) = &mut self.panel {
                    form.company_expanded = None;
                }
            }
            Some(StandardCommand::Help) if key.kind == KeyEventKind::Press => {
                self.help_expanded = !self.help_expanded;
            }
            Some(StandardCommand::Theme) if key.kind == KeyEventKind::Press => self.toggle_theme(),
            _ => {
                let Panel::Form(form) = &mut self.panel else {
                    return false;
                };
                let Some(highlighted) = &mut form.company_expanded else {
                    return false;
                };
                match key.code {
                    KeyCode::Up => {
                        *highlighted = highlighted.checked_sub(1).unwrap_or(COMPANIES.len() - 1);
                    }
                    KeyCode::Down => *highlighted = (*highlighted + 1) % COMPANIES.len(),
                    KeyCode::Home => *highlighted = 0,
                    KeyCode::End => *highlighted = COMPANIES.len() - 1,
                    _ => return false,
                }
            }
        }
        true
    }

    fn handle_text_input(&mut self, event: &Event) -> bool {
        match &mut self.mode {
            Mode::Search { .. } => {
                let changed = apply_event(&mut self.search, event);
                if changed {
                    self.selected = 0;
                    self.table_offset = 0;
                }
                changed
            }
            Mode::Browse => match &mut self.panel {
                Panel::Form(form) if form.company_expanded.is_none() => match form.field() {
                    FormField::Identity if !form.is_editing() => {
                        let changed = apply_event(&mut form.identity, event);
                        if changed {
                            form.error = None;
                        }
                        changed
                    }
                    FormField::Name => {
                        let changed = apply_event(&mut form.name, event);
                        if changed {
                            form.error = None;
                        }
                        changed
                    }
                    _ => false,
                },
                _ => false,
            },
        }
    }

    fn begin_search(&mut self) {
        self.mode = Mode::Search {
            original: self.search.value().to_owned(),
            selected_id: self.selected_contractor().map(|contractor| contractor.id),
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
        self.table_offset = 0;
        self.select_id(selected_id);
        self.mode = Mode::Browse;
    }

    fn open_new_form(&mut self) {
        self.panel = Panel::Form(Box::new(ContractorForm::new()));
    }

    fn open_edit_form(&mut self) {
        let Some(contractor) = self.selected_contractor().cloned() else {
            return;
        };
        self.panel = Panel::Form(Box::new(ContractorForm::edit(&contractor)));
    }

    fn cancel_form(&mut self) {
        self.panel = Panel::Detail;
        self.prepared_message = Some("Edición cancelada · no hubo cambios".into());
    }

    fn form_field(&self) -> Option<FormField> {
        match &self.panel {
            Panel::Form(form) => Some(form.field()),
            Panel::Detail => None,
        }
    }

    fn move_form_focus(&mut self, backwards: bool) {
        if let Panel::Form(form) = &mut self.panel {
            form.move_focus(backwards);
        }
    }

    fn activate_form_field(&mut self) {
        let Panel::Form(form) = &mut self.panel else {
            return;
        };
        match form.field() {
            FormField::Company => form.company_expanded = Some(form.company),
            FormField::RouteStaff => {
                form.route_staff = !form.route_staff;
                form.error = None;
            }
            FormField::Access => {
                form.has_access = !form.has_access;
                form.error = None;
            }
            FormField::EntryType => {
                form.entry_type = cycle(form.entry_type, ENTRY_TYPES.len(), false);
                form.error = None;
            }
            FormField::Identity | FormField::Name => {}
        }
    }

    fn cycle_form_value(&mut self, backwards: bool) {
        let Panel::Form(form) = &mut self.panel else {
            return;
        };
        match form.field() {
            FormField::Company => form.company = cycle(form.company, COMPANIES.len(), backwards),
            FormField::EntryType => {
                form.entry_type = cycle(form.entry_type, ENTRY_TYPES.len(), backwards);
            }
            FormField::RouteStaff => form.route_staff = !form.route_staff,
            FormField::Access => form.has_access = !form.has_access,
            FormField::Identity | FormField::Name => return,
        }
        form.error = None;
    }

    fn save_form(&mut self) {
        let Panel::Form(form) = &self.panel else {
            return;
        };
        let identity = form.identity.value().trim().to_owned();
        let name = form.name.value().trim().to_owned();
        let error = if !form.is_editing() && identity.is_empty() {
            Some("La cédula es obligatoria")
        } else if name.is_empty() {
            Some("El nombre es obligatorio")
        } else if !form.is_editing()
            && self
                .contractors
                .iter()
                .any(|contractor| contractor.identity.eq_ignore_ascii_case(&identity))
        {
            Some("Ya existe un contratista con esa cédula")
        } else {
            None
        };
        if let Some(error) = error {
            if let Panel::Form(form) = &mut self.panel {
                form.error = Some(error.into());
            }
            return;
        }

        let (id, company, entry_type, route_staff, has_access) = (
            form.id,
            form.company,
            form.entry_type,
            form.route_staff,
            form.has_access,
        );
        let saved_id = if let Some(id) = id {
            if let Some(contractor) = self.contractors.iter_mut().find(|item| item.id == id) {
                contractor.name = name.clone();
                contractor.company = COMPANIES[company].into();
                contractor.entry_type = ENTRY_TYPES[entry_type].into();
                contractor.route_staff = route_staff;
                contractor.has_access = has_access;
            }
            id
        } else {
            let id = self
                .contractors
                .iter()
                .map(|contractor| contractor.id)
                .max()
                .unwrap_or(0)
                + 1;
            self.contractors.push(Contractor {
                id,
                identity,
                name: name.clone(),
                company: COMPANIES[company].into(),
                entry_type: ENTRY_TYPES[entry_type].into(),
                route_staff,
                has_access,
            });
            id
        };

        self.search = Input::default();
        self.table_offset = 0;
        self.selected = self
            .contractors
            .iter()
            .position(|contractor| contractor.id == saved_id)
            .unwrap_or(0);
        self.panel = Panel::Detail;
        self.prepared_message = Some(format!("Contratista guardado en memoria · {name}"));
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let filter = self.search.value().trim().to_lowercase();
        self.contractors
            .iter()
            .enumerate()
            .filter_map(|(index, contractor)| {
                (filter.is_empty()
                    || contractor.identity.to_lowercase().contains(&filter)
                    || contractor.name.to_lowercase().contains(&filter)
                    || contractor.company.to_lowercase().contains(&filter)
                    || contractor.entry_type.to_lowercase().contains(&filter))
                .then_some(index)
            })
            .collect()
    }

    fn selected_contractor(&self) -> Option<&Contractor> {
        let indices = self.filtered_indices();
        self.contractors.get(*indices.get(self.selected)?)
    }

    fn select_id(&mut self, id: Option<u32>) {
        let indices = self.filtered_indices();
        self.selected = id
            .and_then(|id| {
                indices
                    .iter()
                    .position(|index| self.contractors[*index].id == id)
            })
            .unwrap_or(0);
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.filtered_indices().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(count.saturating_sub(1));
    }

    fn select_first(&mut self) {
        self.selected = 0;
    }

    fn select_last(&mut self) {
        self.selected = self.filtered_indices().len().saturating_sub(1);
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
    }

    fn layer(&self) -> Layer {
        match &self.mode {
            Mode::Search { .. } => Layer::Search,
            Mode::Browse => match &self.panel {
                Panel::Form(form) if form.company_expanded.is_some() => Layer::CompanyExpand,
                Panel::Form(_) => Layer::Form,
                Panel::Detail => Layer::Browse,
            },
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
            render_terminal_too_small(frame, area, MIN_WIDTH, MIN_HEIGHT, "Q/ESC salir", theme);
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
            Paragraph::new("brisas cli · contratistas (v2)").style(theme.muted()),
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

        let panel_height = self.panel_row_count();
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
                Constraint::Length(panel_height.min(rows[1].height.saturating_sub(5))),
            ])
            .split(rows[1]);
            self.render_table(frame, stacked[0], theme);
            render_horizontal_separator(frame, stacked[1], theme);
            self.render_panel(frame, stacked[2], theme);
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let indices = self.filtered_indices();
        let capacity = area.height.saturating_sub(2).max(1) as usize;
        self.page_size = capacity;

        if indices.is_empty() {
            self.selected = 0;
            self.table_offset = 0;
        } else {
            self.selected = self.selected.min(indices.len() - 1);
            if self.selected < self.table_offset {
                self.table_offset = self.selected;
            } else if self.selected >= self.table_offset.saturating_add(capacity) {
                self.table_offset = self.selected + 1 - capacity;
            }
            self.table_offset = self
                .table_offset
                .min(indices.len().saturating_sub(capacity));
        }

        let wide = area.width >= 76;
        let rows = indices
            .iter()
            .skip(self.table_offset)
            .take(capacity)
            .map(|index| {
                let contractor = &self.contractors[*index];
                let access = if contractor.has_access { "SÍ" } else { "NO" };
                let access_style = if contractor.has_access {
                    theme.success()
                } else {
                    theme.danger()
                };
                let cells = if wide {
                    vec![
                        Cell::from(contractor.identity.as_str()),
                        Cell::from(contractor.name.as_str()),
                        Cell::from(contractor.company.as_str()),
                        Cell::from(contractor.entry_type.as_str()),
                        Cell::from(access).style(access_style),
                    ]
                } else {
                    vec![
                        Cell::from(contractor.identity.as_str()),
                        Cell::from(contractor.name.as_str()),
                        Cell::from(access).style(access_style),
                    ]
                };
                Row::new(cells).style(theme.base())
            });
        let (headers, widths) = if wide {
            (
                vec!["CÉDULA", "NOMBRE", "EMPRESA", "TIPO", "ACCESO"],
                vec![
                    Constraint::Length(14),
                    Constraint::Fill(3),
                    Constraint::Fill(2),
                    Constraint::Length(12),
                    Constraint::Length(8),
                ],
            )
        } else {
            (
                vec!["CÉDULA", "NOMBRE", "ACCESO"],
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
        let mut state = TableState::default().with_selected(
            (!indices.is_empty()).then_some(self.selected.saturating_sub(self.table_offset)),
        );
        frame.render_stateful_widget(table, area, &mut state);

        if indices.is_empty() {
            frame.render_widget(
                Paragraph::new("Sin coincidencias · ESC cancela el filtro")
                    .style(theme.warning())
                    .alignment(Alignment::Center),
                Rect::new(area.x, area.y + area.height / 2, area.width, 1),
            );
        }
    }

    fn panel_row_count(&self) -> u16 {
        match &self.panel {
            Panel::Detail => 7,
            Panel::Form(form) => {
                let mut total: u16 = 0;
                if !form.is_editing() {
                    total += 3;
                }
                total += 3; // nombre
                total += 1; // empresa
                if form.company_expanded.is_some() {
                    total += COMPANIES.len() as u16;
                }
                total += 1; // tipo de ingreso
                total += 1; // personal de ruta
                total += 1; // acceso
                total += 1; // línea de error
                total
            }
        }
    }

    fn render_panel(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        match &self.panel {
            Panel::Detail => self.render_detail(frame, area, theme),
            Panel::Form(form) => self.render_form(frame, area, form, theme),
        }
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let Some(contractor) = self.selected_contractor() else {
            frame.render_widget(
                Paragraph::new("No hay un registro seleccionado").style(theme.muted()),
                area,
            );
            return;
        };
        frame.render_widget(Paragraph::new(detail_lines(contractor, theme)), area);
    }

    fn render_form(&self, frame: &mut Frame, area: Rect, form: &ContractorForm, theme: Theme) {
        let mut constraints = Vec::new();
        if !form.is_editing() {
            constraints.push(Constraint::Length(3));
        }
        constraints.push(Constraint::Length(3));
        constraints.push(Constraint::Length(1));
        if form.company_expanded.is_some() {
            constraints.push(Constraint::Length(COMPANIES.len() as u16));
        }
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Length(1));
        let rows = Layout::vertical(constraints).split(area);
        let mut cursor = 0;

        if !form.is_editing() {
            render_field(
                frame,
                rows[cursor],
                FieldSpec {
                    label: "CÉDULA",
                    input: &form.identity,
                    focused: form.field() == FormField::Identity,
                    theme,
                },
            );
            cursor += 1;
        }
        render_field(
            frame,
            rows[cursor],
            FieldSpec {
                label: "NOMBRE",
                input: &form.name,
                focused: form.field() == FormField::Name,
                theme,
            },
        );
        cursor += 1;

        render_choice(
            frame,
            rows[cursor],
            "EMPRESA",
            COMPANIES[form.company],
            form.field() == FormField::Company,
            theme,
        );
        cursor += 1;

        if let Some(highlighted) = form.company_expanded {
            render_inline_list(frame, rows[cursor], &COMPANIES, highlighted, theme);
            cursor += 1;
        }

        render_choice(
            frame,
            rows[cursor],
            "TIPO DE INGRESO",
            ENTRY_TYPES[form.entry_type],
            form.field() == FormField::EntryType,
            theme,
        );
        cursor += 1;
        render_choice(
            frame,
            rows[cursor],
            "PERSONAL DE RUTA",
            yes_no(form.route_staff),
            form.field() == FormField::RouteStaff,
            theme,
        );
        cursor += 1;
        render_choice(
            frame,
            rows[cursor],
            "TIENE ACCESO",
            yes_no(form.has_access),
            form.field() == FormField::Access,
            theme,
        );
        cursor += 1;
        frame.render_widget(
            Paragraph::new(form.error.as_deref().unwrap_or_default()).style(theme.danger()),
            rows[cursor],
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
                Span::styled(" nuevo   ", theme.base()),
                Span::styled("/", theme.accent()),
                Span::styled(" buscar   ", theme.base()),
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
                Span::styled("TAB", theme.accent()),
                Span::styled(" campo   ", theme.base()),
                Span::styled("←→", theme.accent()),
                Span::styled(" cambiar   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" guardar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cancelar", theme.base()),
            ],
            Layer::CompanyExpand => vec![
                Span::styled("↑↓", theme.accent()),
                Span::styled(" mover   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" elegir   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cerrar", theme.base()),
            ],
        };
        let mut lines = vec![Line::from(primary)];
        if self.help_expanded {
            lines.push(Line::from(vec![
                Span::styled("PGUP/PGDN", theme.accent()),
                Span::styled(" página   ", theme.base()),
                Span::styled("HOME/END", theme.accent()),
                Span::styled(" extremos   ", theme.base()),
                Span::styled(QUICK_EXIT_HINT.key, theme.accent()),
                Span::styled(format!(" {}   ", QUICK_EXIT_HINT.label), theme.base()),
                Span::styled("F7", theme.accent()),
                Span::styled(" tema   ", theme.base()),
                Span::styled("Q/ESC", theme.accent()),
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

/// Misma silueta enfocado o no: etiqueta, valor, línea. Sólo cambia el color.
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

/// Separa la tabla del panel de detalle/formulario con una sola línea tenue,
/// sin caja: apenas lo justo para que dejen de leerse como un único bloque.
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

fn render_choice(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    theme: Theme,
) {
    let marker = if focused { ">" } else { " " };
    let style = if focused { theme.accent() } else { theme.base() };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{marker} {label:<18}"), style),
            Span::styled(if focused { "◀ " } else { "  " }, theme.muted()),
            Span::styled(value.to_owned(), style),
            Span::styled(if focused { " ▶" } else { "" }, theme.muted()),
        ])),
        area,
    );
}

fn render_inline_list(
    frame: &mut Frame,
    area: Rect,
    options: &[&str],
    highlighted: usize,
    theme: Theme,
) {
    let lines: Vec<Line<'_>> = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let selected = index == highlighted;
            let marker = if selected { "  >" } else { "   " };
            Line::from(format!("{marker} {option}")).style(if selected {
                theme.selected()
            } else {
                theme.muted()
            })
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

fn detail_lines(contractor: &Contractor, theme: Theme) -> Vec<Line<'_>> {
    vec![
        Line::from(contractor.name.as_str()).style(theme.title()),
        Line::from(""),
        labeled("Cédula", &contractor.identity, theme),
        labeled("Empresa", &contractor.company, theme),
        labeled("Tipo", &contractor.entry_type, theme),
        labeled("Personal de ruta", yes_no(contractor.route_staff), theme),
        Line::from(vec![
            Span::styled("Acceso            ", theme.muted()),
            Span::styled(
                yes_no(contractor.has_access),
                if contractor.has_access {
                    theme.success()
                } else {
                    theme.danger()
                },
            ),
        ]),
    ]
}

fn labeled<'a>(label: &'a str, value: &'a str, theme: Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<18}"), theme.muted()),
        Span::styled(value, theme.base()),
    ])
}

fn yes_no(value: bool) -> &'static str {
    if value { "SÍ" } else { "NO" }
}

fn cycle(current: usize, length: usize, backwards: bool) -> usize {
    if backwards {
        current.checked_sub(1).unwrap_or(length.saturating_sub(1))
    } else {
        (current + 1) % length.max(1)
    }
}

fn key_char(key: KeyEvent, expected: char) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        && matches!(key.code, KeyCode::Char(actual) if actual.eq_ignore_ascii_case(&expected))
}

fn demo_contractors() -> Vec<Contractor> {
    [
        (1, "1-1042-0881", "José Peña", 0, 0, false, true),
        (2, "2-0731-0440", "Ana María Solís", 1, 1, false, true),
        (3, "3-0520-0917", "Juan Rodríguez", 2, 0, true, false),
        (4, "4-0198-0772", "Mónica Quesada", 3, 2, false, true),
        (5, "1-1550-0239", "Carlos Méndez", 4, 3, true, true),
        (6, "2-0611-0854", "Sofía Núñez", 0, 1, false, true),
        (7, "3-0488-0312", "Edgar Chacón", 2, 0, false, false),
        (8, "1-1270-0641", "María Fernanda Rojas", 1, 2, false, true),
        (9, "4-0221-0178", "Luis Ángel Mora", 3, 0, true, true),
        (10, "2-0810-0305", "Valeria Jiménez", 4, 1, false, true),
    ]
    .into_iter()
    .map(
        |(id, identity, name, company, entry_type, route_staff, has_access)| Contractor {
            id,
            identity: identity.into(),
            name: name.into(),
            company: COMPANIES[company].into(),
            entry_type: ENTRY_TYPES[entry_type].into(),
            route_staff,
            has_access,
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{ContratistasV2Pilot, Layer, Panel, ThemePreset};

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
    fn el_detalle_esta_visible_desde_el_inicio_sin_abrir_nada() {
        let app = ContratistasV2Pilot::default();
        assert!(matches!(app.panel, Panel::Detail));
        assert_eq!(app.layer(), Layer::Browse);
    }

    #[test]
    fn enter_abre_el_formulario_de_edicion_junto_a_la_tabla_sin_taparla() {
        let mut app = ContratistasV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter));
        assert!(matches!(app.panel, Panel::Form(_)));

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal
            .draw(|frame| app.render(frame))
            .expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("José Peña"));
        assert!(rendered.contains("NOMBRE"));
    }

    #[test]
    fn f2_funciona_en_medio_de_un_formulario_abierto_y_no_pierde_lo_escrito() {
        let mut app = ContratistasV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&Event::Paste("9-8888-7777".into()));

        app.handle_event(&key(KeyCode::F(2)));
        assert!(app.quick_exit.is_open());

        app.handle_event(&Event::Paste("12".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(!app.quick_exit.is_open());
        let Panel::Form(form) = &app.panel else {
            panic!("debe seguir en el formulario, con lo ya escrito, tras cerrar F2");
        };
        assert_eq!(form.identity.value(), "9-8888-7777");
        assert!(
            app.prepared_message
                .as_deref()
                .is_some_and(|message| message.contains("José Peña"))
        );
    }

    #[test]
    fn tab_da_la_vuelta_al_llegar_al_final_y_shift_tab_tambien() {
        let mut app = ContratistasV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter)); // editar José Peña: 5 campos (Nombre..Acceso)

        for _ in 0..5 {
            app.handle_event(&key(KeyCode::Tab));
        }
        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert_eq!(form.focus, 0, "Tab debe volver al primer campo, no quedarse en el último");

        app.handle_event(&key(KeyCode::BackTab));
        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert_eq!(form.focus, 4, "Shift+Tab desde el primer campo debe ir al último");
    }

    #[test]
    fn n_abre_formulario_nuevo_y_valida_cedula_y_nombre_obligatorios() {
        let mut app = ContratistasV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("cédula")));
    }

    #[test]
    fn no_permite_duplicar_una_cedula_existente() {
        let mut app = ContratistasV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&Event::Paste("1-1042-0881".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("Otro Nombre".into()));
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("existe")));
    }

    #[test]
    fn el_selector_de_empresa_se_expande_inline_sin_ocultar_el_resto_del_formulario() {
        let mut app = ContratistasV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter)); // editar José Peña
        app.handle_event(&key(KeyCode::Tab)); // nombre -> empresa
        app.handle_event(&key(KeyCode::Char(' '))); // abrir selector

        assert_eq!(app.layer(), Layer::CompanyExpand);

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal
            .draw(|frame| app.render(frame))
            .expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        // La lista de empresas y el resto del formulario conviven en el mismo frame.
        assert!(rendered.contains("Expenic Industrial"));
        assert!(rendered.contains("TIPO DE INGRESO"));
        assert!(rendered.contains("José Peña") || rendered.contains("NOMBRE"));
    }

    #[test]
    fn confirmar_el_selector_de_empresa_actualiza_el_campo() {
        let mut app = ContratistasV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&key(KeyCode::Char(' ')));
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.company_expanded.is_none());
        assert_eq!(super::COMPANIES[form.company], "Expenic Industrial");
    }

    #[test]
    fn guardar_un_contratista_nuevo_lo_deja_seleccionado_en_la_tabla() {
        let mut app = ContratistasV2Pilot::default();
        let antes = app.contractors.len();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&Event::Paste("9-9999-9999".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("Nueva Persona".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.contractors.len(), antes + 1);
        assert!(matches!(app.panel, Panel::Detail));
        assert_eq!(
            app.selected_contractor().map(|c| c.name.as_str()),
            Some("Nueva Persona")
        );
    }

    #[test]
    fn buscar_filtra_y_escape_restaura_la_seleccion_previa() {
        let mut app = ContratistasV2Pilot::default();
        app.handle_event(&key(KeyCode::Down));
        let seleccionado = app.selected_contractor().map(|c| c.id);

        app.handle_event(&key(KeyCode::Char('/')));
        app.handle_event(&Event::Paste("Mónica".into()));
        assert_eq!(app.filtered_indices().len(), 1);

        app.handle_event(&key(KeyCode::Esc));
        assert!(app.search.value().is_empty());
        assert_eq!(
            app.selected_contractor().map(|c| c.id),
            seleccionado
        );
    }

    #[test]
    fn pinta_todo_el_lienzo_sin_dejar_parches_sin_color() {
        let mut app = ContratistasV2Pilot::default();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| app.render(frame))
            .expect("debe renderizar");

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
        let mut app = ContratistasV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        let count = app.contractors.len();

        app.handle_event(&Event::Resize(40, 10));
        app.handle_event(&Event::Paste("1-9999".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.contractors.len(), count);
    }
}
