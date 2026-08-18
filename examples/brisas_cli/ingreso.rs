use std::time::{Duration, Instant};

use chrono::Utc;
use chrono_tz::America::Costa_Rica;
use control_acceso::tui::ui_kit::{
    CommandHint, HELP_HINT, ScreenShell, SelectMenu, SelectMenuState, StandardCommand, StatusKind,
    THEME_HINT, TextInput, TextInputFocus, Theme, ThemePreset, auxiliary_panel, centered_content,
    centered_rect, panel, render_terminal_too_small, standard_command,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Cell, Clear, Paragraph, Row, Table, TableState},
};

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 20;
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const ENTRY_MEDIA: [&str; 2] = ["Caminando", "Vehículo"];
const OCCUPIED_BADGES: [i64; 3] = [12, 25, 48];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Eligibility {
    Available,
    Warning(&'static str),
    Blocked {
        label: &'static str,
        reason: &'static str,
    },
}

impl Eligibility {
    const fn can_register(self) -> bool {
        matches!(self, Self::Available | Self::Warning(_))
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Available => "DISPONIBLE",
            Self::Warning(_) => "REVISAR",
            Self::Blocked { label, .. } => label,
        }
    }

    const fn explanation(self) -> &'static str {
        match self {
            Self::Available => "Disponible para registrar el ingreso.",
            Self::Warning(reason) | Self::Blocked { reason, .. } => reason,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Candidate {
    id: u32,
    identity: &'static str,
    name: &'static str,
    company: &'static str,
    entry_type: &'static str,
    route_staff: bool,
    requires_badge: bool,
    eligibility: Eligibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormField {
    Badge,
    Medium,
}

#[derive(Debug, Clone)]
struct EntryForm {
    contractor_id: u32,
    medium: usize,
    medium_menu: SelectMenuState,
    badge: TextInput,
    field: FormField,
    error: Option<String>,
}

impl EntryForm {
    fn new(candidate: Candidate) -> Self {
        Self {
            contractor_id: candidate.id,
            medium: 0,
            medium_menu: SelectMenuState::closed(),
            badge: TextInput::default().with_max_chars(12),
            field: if candidate.requires_badge {
                FormField::Badge
            } else {
                FormField::Medium
            },
            error: None,
        }
    }

    fn move_focus(&mut self, candidate: Candidate, backwards: bool) -> bool {
        if !candidate.requires_badge {
            return false;
        }
        self.field = match (self.field, backwards) {
            (FormField::Badge, false) | (FormField::Medium, true) => FormField::Medium,
            (FormField::Medium, false) | (FormField::Badge, true) => FormField::Badge,
        };
        self.error = None;
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Search,
    Form,
    MediumMenu,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatOrigin {
    code: KeyCode,
    modifiers: KeyModifiers,
    layer: Layer,
}

#[derive(Debug)]
pub struct EntryPilot {
    candidates: Vec<Candidate>,
    search: TextInput,
    selected: usize,
    form: Option<EntryForm>,
    help_visible: bool,
    theme: ThemePreset,
    status: String,
    status_kind: StatusKind,
    running: bool,
    terminal_size: (u16, u16),
    repeat_origin: Option<RepeatOrigin>,
    cursor_started: Instant,
}

impl Default for EntryPilot {
    fn default() -> Self {
        Self {
            candidates: demo_candidates(),
            search: TextInput::default().with_max_chars(120),
            selected: 0,
            form: None,
            help_visible: false,
            theme: ThemePreset::Classic,
            status: "Escriba una cédula, nombre o empresa para buscar.".into(),
            status_kind: StatusKind::Normal,
            running: true,
            terminal_size: (u16::MAX, u16::MAX),
            repeat_origin: None,
            cursor_started: Instant::now(),
        }
    }
}

impl EntryPilot {
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
                    && standard_command(key) == Some(StandardCommand::Cancel)
            }) {
                self.running = false;
                return true;
            }
            return false;
        }

        let Some(key) = key else {
            if self.help_visible {
                return false;
            }
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
            Layer::Form => self.handle_form_key(key),
            Layer::MediumMenu => self.handle_medium_menu_key(key),
            Layer::Help => self.handle_help_key(key),
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
            Some(StandardCommand::Primary) => self.open_selected(),
            Some(StandardCommand::Cancel) => {
                if self.search.value().is_empty() {
                    self.running = false;
                } else {
                    self.search.clear();
                    self.selected = 0;
                    self.set_status(
                        "Búsqueda limpiada · se muestran todos los contratistas.",
                        StatusKind::Normal,
                    );
                    self.restart_cursor();
                }
            }
            Some(StandardCommand::Help) => self.help_visible = true,
            Some(StandardCommand::Theme) => self.toggle_theme(),
            _ => match key.code {
                KeyCode::Up => self.move_selection(-1),
                KeyCode::Down => self.move_selection(1),
                KeyCode::Home => self.select(0),
                KeyCode::End => {
                    self.select(self.filtered_indices().len().saturating_sub(1));
                }
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
            Some(StandardCommand::Primary) => self.register(),
            Some(StandardCommand::Activate) => {
                let Some(form) = &mut self.form else {
                    return false;
                };
                if form.field != FormField::Medium {
                    return false;
                }
                form.medium_menu.open(form.medium, ENTRY_MEDIA.len());
            }
            Some(StandardCommand::Cancel) => {
                self.form = None;
                self.set_status("Registro cancelado · no hubo cambios.", StatusKind::Warning);
                self.restart_cursor();
            }
            Some(StandardCommand::FocusNext) => self.move_form_focus(false),
            Some(StandardCommand::FocusPrevious) => self.move_form_focus(true),
            Some(StandardCommand::Help) => self.help_visible = true,
            Some(StandardCommand::Theme) => self.toggle_theme(),
            _ => match key.code {
                KeyCode::Left => self.change_medium(true),
                KeyCode::Right => self.change_medium(false),
                _ => return false,
            },
        }
        true
    }

    fn handle_medium_menu_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(key.code, KeyCode::Up | KeyCode::Down);
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return false;
        }
        let Some(form) = &mut self.form else {
            return false;
        };
        match standard_command(key) {
            Some(StandardCommand::Primary) => {
                let Some(selected) = form.medium_menu.accept(ENTRY_MEDIA.len()) else {
                    return false;
                };
                form.medium = selected;
                form.error = None;
            }
            Some(StandardCommand::Cancel) => return form.medium_menu.close(),
            Some(StandardCommand::Help) => self.help_visible = true,
            Some(StandardCommand::Theme) => self.toggle_theme(),
            _ => match key.code {
                KeyCode::Up => return form.medium_menu.move_by(-1, ENTRY_MEDIA.len()),
                KeyCode::Down => return form.medium_menu.move_by(1, ENTRY_MEDIA.len()),
                KeyCode::Home => return form.medium_menu.first(ENTRY_MEDIA.len()),
                KeyCode::End => return form.medium_menu.last(ENTRY_MEDIA.len()),
                _ => return false,
            },
        }
        true
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Repeat {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Cancel | StandardCommand::Help) => self.help_visible = false,
            Some(StandardCommand::Theme) => self.toggle_theme(),
            _ => return false,
        }
        true
    }

    fn handle_text_input(&mut self, event: &Event) -> bool {
        if self.form.is_none() {
            let changed = self.search.handle_event(event);
            if changed {
                self.selected = 0;
                self.set_status(
                    format!("{} resultados.", self.filtered_indices().len()),
                    StatusKind::Normal,
                );
                self.restart_cursor();
            }
            return changed;
        }

        let Some(form) = &mut self.form else {
            return false;
        };
        if form.field != FormField::Badge || form.medium_menu.is_open() {
            return false;
        }
        if !numeric_event(event) {
            if matches!(
                event,
                Event::Paste(_)
                    | Event::Key(KeyEvent {
                        code: KeyCode::Char(_),
                        ..
                    })
            ) {
                form.error = Some("El número de gafete solo admite dígitos.".into());
                return true;
            }
            return false;
        }
        let changed = form.badge.handle_event(event);
        if changed {
            form.error = None;
            self.restart_cursor();
        }
        changed
    }

    fn open_selected(&mut self) {
        let Some(candidate) = self.selected_candidate().copied() else {
            self.set_status("No hay un contratista seleccionado.", StatusKind::Warning);
            return;
        };
        if !candidate.eligibility.can_register() {
            self.set_status(candidate.eligibility.explanation(), StatusKind::Error);
            return;
        }
        self.form = Some(EntryForm::new(candidate));
        self.set_status(
            if candidate.requires_badge {
                "Ingrese el gafete y presione Enter para registrar."
            } else {
                "Caminando seleccionado · presione Enter para registrar."
            },
            if matches!(candidate.eligibility, Eligibility::Warning(_)) {
                StatusKind::Warning
            } else {
                StatusKind::Normal
            },
        );
        self.restart_cursor();
    }

    fn register(&mut self) {
        let Some(form) = &self.form else {
            return;
        };
        let Some(candidate) = self
            .candidates
            .iter()
            .find(|candidate| candidate.id == form.contractor_id)
            .copied()
        else {
            self.form = None;
            self.set_status("El contratista ya no está disponible.", StatusKind::Error);
            return;
        };
        if !candidate.eligibility.can_register() {
            self.form = None;
            self.set_status(candidate.eligibility.explanation(), StatusKind::Error);
            return;
        }

        let badge = if candidate.requires_badge {
            let value = form.badge.value().trim();
            let Ok(number) = value.parse::<i64>() else {
                if let Some(form) = &mut self.form {
                    form.error = Some("Ingrese el número de gafete.".into());
                }
                return;
            };
            if OCCUPIED_BADGES.contains(&number) {
                if let Some(form) = &mut self.form {
                    form.error = Some(format!("El gafete {number} ya está en uso."));
                }
                return;
            }
            Some(number)
        } else {
            None
        };

        let medium = ENTRY_MEDIA[form.medium];
        self.form = None;
        if let Some(stored) = self
            .candidates
            .iter_mut()
            .find(|stored| stored.id == candidate.id)
        {
            stored.eligibility = Eligibility::Blocked {
                label: "DENTRO",
                reason: "Ingreso activo registrado en esta sesión del piloto.",
            };
        }
        self.search.clear();
        self.selected = 0;
        let badge_text = badge.map_or_else(|| "S/G".into(), |number| format!("gafete {number}"));
        self.set_status(
            format!(
                "Ingreso registrado · {} · {} · {badge_text}.",
                candidate.name, medium
            ),
            StatusKind::Success,
        );
        self.restart_cursor();
    }

    fn move_form_focus(&mut self, backwards: bool) {
        let Some(candidate) = self.form_candidate() else {
            return;
        };
        if let Some(form) = &mut self.form
            && form.move_focus(candidate, backwards)
        {
            self.restart_cursor();
        }
    }

    fn change_medium(&mut self, backwards: bool) {
        let Some(form) = &mut self.form else {
            return;
        };
        if form.field != FormField::Medium {
            return;
        }
        form.medium = if backwards {
            form.medium.checked_sub(1).unwrap_or(ENTRY_MEDIA.len() - 1)
        } else {
            (form.medium + 1) % ENTRY_MEDIA.len()
        };
        form.error = None;
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.filtered_indices().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.select(self.selected.saturating_add_signed(delta).min(count - 1));
    }

    fn select(&mut self, index: usize) {
        let count = self.filtered_indices().len();
        self.selected = index.min(count.saturating_sub(1));
        let Some(candidate) = self.selected_candidate() else {
            return;
        };
        let kind = match candidate.eligibility {
            Eligibility::Available => StatusKind::Normal,
            Eligibility::Warning(_) => StatusKind::Warning,
            Eligibility::Blocked { .. } => StatusKind::Error,
        };
        self.set_status(candidate.eligibility.explanation(), kind);
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let query = self.search.value().trim().to_lowercase();
        self.candidates
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                (query.is_empty()
                    || candidate.identity.to_lowercase().contains(&query)
                    || candidate.name.to_lowercase().contains(&query)
                    || candidate.company.to_lowercase().contains(&query))
                .then_some(index)
            })
            .collect()
    }

    fn selected_candidate(&self) -> Option<&Candidate> {
        let index = *self.filtered_indices().get(self.selected)?;
        self.candidates.get(index)
    }

    fn form_candidate(&self) -> Option<Candidate> {
        let id = self.form.as_ref()?.contractor_id;
        self.candidates
            .iter()
            .find(|candidate| candidate.id == id)
            .copied()
    }

    fn set_status(&mut self, status: impl Into<String>, kind: StatusKind) {
        self.status = status.into();
        self.status_kind = kind;
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
        self.set_status(
            format!("Tema {} aplicado.", self.theme.name()),
            StatusKind::Success,
        );
    }

    fn restart_cursor(&mut self) {
        self.cursor_started = Instant::now();
    }

    fn cursor_visible_at(&self, now: Instant) -> bool {
        let phase = now
            .saturating_duration_since(self.cursor_started)
            .as_millis()
            / CURSOR_BLINK_INTERVAL.as_millis();
        phase.is_multiple_of(2)
    }

    fn layer(&self) -> Layer {
        if self.help_visible {
            Layer::Help
        } else if self
            .form
            .as_ref()
            .is_some_and(|form| form.medium_menu.is_open())
        {
            Layer::MediumMenu
        } else if self.form.is_some() {
            Layer::Form
        } else {
            Layer::Search
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

        let clock = Utc::now()
            .with_timezone(&Costa_Rica)
            .format("%d/%m/%Y  %H:%M:%S")
            .to_string();
        let commands = self.command_hints();
        let shell = ScreenShell {
            product: "B R I S A S   C L I",
            screen: "REGISTRAR INGRESO",
            context: "DANIEL QUINTANA",
            clock: &clock,
            status: &self.status,
            status_kind: self.status_kind,
            commands: &commands,
            help_expanded: false,
            ayuda_extra: None,
        };
        let areas = shell.render(frame, area, theme);
        self.render_body(frame, areas.body, theme);

        if self.form.is_some() {
            self.render_form(frame, area, theme);
        }
        if self.help_visible {
            self.render_help(frame, area, theme);
        }
    }

    fn command_hints(&self) -> Vec<CommandHint<'static>> {
        match self.layer() {
            Layer::Search => vec![
                CommandHint::new("ESCRIBIR", "Buscar"),
                CommandHint::new("↑↓", "Mover"),
                CommandHint::new("ENTER", "Seleccionar"),
                CommandHint::new("ESC", "Limpiar/volver"),
                HELP_HINT,
                THEME_HINT,
            ],
            Layer::Form => vec![
                CommandHint::new("TAB/SHIFT+TAB", "Campo"),
                CommandHint::new("←→", "Cambiar"),
                CommandHint::new("ESP", "Abrir"),
                CommandHint::new("ENTER", "Registrar"),
                CommandHint::new("ESC", "Cancelar"),
                HELP_HINT,
                THEME_HINT,
            ],
            Layer::MediumMenu => vec![
                CommandHint::new("↑↓", "Mover"),
                CommandHint::new("ENTER", "Seleccionar"),
                CommandHint::new("ESC", "Cerrar"),
                HELP_HINT,
                THEME_HINT,
            ],
            Layer::Help => vec![CommandHint::new("ESC/F1", "Cerrar ayuda"), THEME_HINT],
        }
    }

    fn render_body(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(5),
        ])
        .split(area);
        let search_focused = self.form.is_none() && !self.help_visible;
        self.search.render_with_cursor(
            frame,
            rows[0],
            "BUSCAR CONTRATISTA",
            "Cédula, nombre o empresa",
            TextInputFocus::new(search_focused, self.cursor_visible_at(Instant::now())),
            theme,
        );
        let count = self.filtered_indices().len();
        frame.render_widget(
            Paragraph::new(format!(
                "{count} RESULTADOS · se muestran también bloqueados y activos"
            ))
            .style(theme.muted()),
            rows[1],
        );
        self.render_table(frame, rows[2], theme);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let indices = self.filtered_indices();
        self.selected = self.selected.min(indices.len().saturating_sub(1));
        let capacity = area.height.saturating_sub(4) as usize;
        let start = self.selected.saturating_sub(capacity.saturating_sub(1));
        let wide = area.width >= 88;
        let rows = indices.iter().skip(start).take(capacity).map(|index| {
            let candidate = self.candidates[*index];
            let status_style = match candidate.eligibility {
                Eligibility::Available => theme.success(),
                Eligibility::Warning(_) => theme.warning(),
                Eligibility::Blocked { .. } => theme.danger(),
            };
            let cells = if wide {
                vec![
                    Cell::from(candidate.identity),
                    Cell::from(candidate.name),
                    Cell::from(candidate.company),
                    Cell::from(candidate.entry_type),
                    Cell::from(candidate.eligibility.label()).style(status_style),
                ]
            } else {
                vec![
                    Cell::from(candidate.identity),
                    Cell::from(candidate.name),
                    Cell::from(candidate.eligibility.label()).style(status_style),
                ]
            };
            Row::new(cells).style(theme.base())
        });
        let (headers, widths) = if wide {
            (
                vec!["CÉDULA", "NOMBRE", "EMPRESA", "TIPO", "ESTADO"],
                vec![
                    Constraint::Length(14),
                    Constraint::Fill(3),
                    Constraint::Fill(2),
                    Constraint::Length(12),
                    Constraint::Length(13),
                ],
            )
        } else {
            (
                vec!["CÉDULA", "NOMBRE", "ESTADO"],
                vec![
                    Constraint::Length(14),
                    Constraint::Fill(1),
                    Constraint::Length(13),
                ],
            )
        };
        let table = Table::new(rows, widths)
            .header(Row::new(headers).style(theme.accent()).bottom_margin(1))
            .block(panel("CONTRATISTAS", theme, true))
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
                Paragraph::new("Sin coincidencias · Esc limpia la búsqueda")
                    .style(theme.warning())
                    .alignment(Alignment::Center),
                Rect::new(
                    area.x + 1,
                    area.y + area.height / 2,
                    area.width.saturating_sub(2),
                    1,
                ),
            );
        }
    }

    fn render_form(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let Some(form) = &self.form else {
            return;
        };
        let Some(candidate) = self.form_candidate() else {
            return;
        };
        let popup = centered_rect(area, 64, if candidate.requires_badge { 16 } else { 13 });
        frame.render_widget(Clear, popup);
        let block = auxiliary_panel("REGISTRAR INGRESO", theme, true);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let content = centered_content(inner, 54);

        frame.render_widget(
            Paragraph::new(candidate.name).style(theme.title()),
            Rect::new(content.x, content.y, content.width, 1),
        );
        frame.render_widget(
            Paragraph::new(format!("{} · {}", candidate.identity, candidate.company))
                .style(theme.base()),
            Rect::new(content.x, content.y + 1, content.width, 1),
        );
        let route = if candidate.route_staff {
            " · PERSONAL DE RUTA"
        } else {
            ""
        };
        frame.render_widget(
            Paragraph::new(format!("{}{}", candidate.entry_type, route)).style(theme.muted()),
            Rect::new(content.x, content.y + 2, content.width, 1),
        );

        let medium_y = content.y + 4;
        render_medium(
            frame,
            Rect::new(content.x, medium_y, content.width, 1),
            ENTRY_MEDIA[form.medium],
            form.field == FormField::Medium && !self.help_visible,
            theme,
        );

        if candidate.requires_badge {
            form.badge.render_with_cursor(
                frame,
                Rect::new(content.x, medium_y + 2, content.width, 3),
                "NÚMERO DE GAFETE",
                "Número asignado",
                TextInputFocus::new(
                    form.field == FormField::Badge
                        && !form.medium_menu.is_open()
                        && !self.help_visible,
                    self.cursor_visible_at(Instant::now()),
                ),
                theme,
            );
        }
        let error_y = if candidate.requires_badge {
            medium_y + 5
        } else {
            medium_y + 2
        };
        frame.render_widget(
            Paragraph::new(form.error.as_deref().unwrap_or_default()).style(theme.danger()),
            Rect::new(content.x, error_y, content.width, 1),
        );
        frame.render_widget(
            Paragraph::new("Enter registra directamente · Esc cancela.").style(theme.muted()),
            Rect::new(content.x, error_y + 2, content.width, 1),
        );

        if form.medium_menu.is_open() {
            let menu = Rect::new(
                content.x + content.width.saturating_sub(34) / 2,
                medium_y + 1,
                34.min(content.width),
                4,
            );
            SelectMenu {
                title: "MEDIO DE INGRESO",
                options: &ENTRY_MEDIA,
                current: Some(form.medium),
            }
            .render(frame, menu, &form.medium_menu, theme);
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let popup = centered_rect(area, 72, 16);
        frame.render_widget(Clear, popup);
        let block = auxiliary_panel("AYUDA DE REGISTRO DE INGRESO", theme, true);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let content = centered_content(inner, 60);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("Escriba para buscar por cédula, nombre o empresa."),
                Line::from("Todos permanecen visibles; el estado explica si pueden ingresar."),
                Line::from("Enter selecciona al contratista o registra el formulario."),
                Line::from("El medio inicia siempre en Caminando."),
                Line::from("Espacio abre el selector; ←/→ cambia el medio."),
                Line::from("El foco inicia en gafete solamente cuando es obligatorio."),
                Line::from("Esc cancela una capa sin guardar."),
                Line::from(""),
                Line::from("Este piloto no consulta ni modifica SQLite.").style(theme.warning()),
                Line::from(""),
                Line::from("ESC / F1  Cerrar ayuda").style(theme.accent()),
            ]),
            content,
        );
    }
}

fn numeric_event(event: &Event) -> bool {
    match event {
        Event::Paste(text) => {
            !text.is_empty() && text.chars().all(|character| character.is_ascii_digit())
        }
        Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
            matches!(key.code, KeyCode::Char(character) if character.is_ascii_digit())
                || matches!(
                    key.code,
                    KeyCode::Backspace
                        | KeyCode::Delete
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Home
                        | KeyCode::End
                )
        }
        _ => false,
    }
}

fn render_medium(frame: &mut Frame, area: Rect, value: &str, focused: bool, theme: Theme) {
    let style = if focused {
        theme.accent()
    } else {
        theme.base()
    };
    let marker = if focused { ">" } else { " " };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{marker} MEDIO DE INGRESO      "), style),
            Span::styled(if focused { "◀ " } else { "  " }, theme.muted()),
            Span::styled(format!("{value:<12}"), style),
            Span::styled(if focused { " ▶" } else { "  " }, theme.muted()),
        ])),
        area,
    );
}

fn demo_candidates() -> Vec<Candidate> {
    vec![
        Candidate {
            id: 1,
            identity: "3-0520-0917",
            name: "Juan Rodríguez",
            company: "Expenic Industrial",
            entry_type: "PRAIND",
            route_staff: false,
            requires_badge: true,
            eligibility: Eligibility::Available,
        },
        Candidate {
            id: 2,
            identity: "2-0731-0440",
            name: "Ana María Solís",
            company: "Aldama Servicios",
            entry_type: "IN HOUSE",
            route_staff: false,
            requires_badge: false,
            eligibility: Eligibility::Available,
        },
        Candidate {
            id: 3,
            identity: "1-1550-0239",
            name: "Carlos Méndez",
            company: "Brisas del Oeste",
            entry_type: "POR CORREO",
            route_staff: false,
            requires_badge: true,
            eligibility: Eligibility::Available,
        },
        Candidate {
            id: 4,
            identity: "2-0611-0854",
            name: "Sofía Núñez",
            company: "Logística Central",
            entry_type: "PRAIND",
            route_staff: true,
            requires_badge: false,
            eligibility: Eligibility::Warning("PRAIND próximo a vencer · ingreso permitido."),
        },
        Candidate {
            id: 5,
            identity: "4-0198-0772",
            name: "Mónica Quesada",
            company: "Mantenimiento CR",
            entry_type: "PRAIND",
            route_staff: false,
            requires_badge: true,
            eligibility: Eligibility::Blocked {
                label: "VENCIDO",
                reason: "Acceso denegado · el PRAIND está vencido.",
            },
        },
        Candidate {
            id: 6,
            identity: "3-0488-0312",
            name: "Edgar Chacón",
            company: "Aldama Servicios",
            entry_type: "SWAT",
            route_staff: false,
            requires_badge: false,
            eligibility: Eligibility::Blocked {
                label: "SIN ACCESO",
                reason: "Acceso administrativo deshabilitado.",
            },
        },
        Candidate {
            id: 7,
            identity: "1-1042-0881",
            name: "José Peña",
            company: "Brisas del Oeste",
            entry_type: "PRAIND",
            route_staff: false,
            requires_badge: true,
            eligibility: Eligibility::Blocked {
                label: "DENTRO",
                reason: "Ya tiene un ingreso activo desde las 07:42.",
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{EntryPilot, FormField, Layer, ThemePreset};

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn repeat(code: KeyCode) -> Event {
        let mut key = KeyEvent::new(code, KeyModifiers::NONE);
        key.kind = KeyEventKind::Repeat;
        Event::Key(key)
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
    fn inicia_mostrando_todos_y_con_busqueda_enfocada() {
        let app = EntryPilot::default();

        assert_eq!(app.filtered_indices().len(), 7);
        assert!(app.form.is_none());
        assert_eq!(app.layer(), Layer::Search);
        assert!(app.cursor_visible_at(app.cursor_started));
        assert!(!app.cursor_visible_at(app.cursor_started + super::CURSOR_BLINK_INTERVAL));
    }

    #[test]
    fn buscar_encuentra_incluso_un_contratista_bloqueado() {
        let mut app = EntryPilot::default();

        app.handle_event(&Event::Paste("Mónica".into()));

        assert_eq!(app.filtered_indices().len(), 1);
        assert_eq!(
            app.selected_candidate().map(|candidate| candidate.id),
            Some(5)
        );
        app.handle_event(&key(KeyCode::Enter));
        assert!(app.form.is_none());
        assert!(app.status.contains("PRAIND está vencido"));
    }

    #[test]
    fn disponible_con_gafete_abre_unico_modal_y_enfoca_gafete() {
        let mut app = EntryPilot::default();

        app.handle_event(&key(KeyCode::Enter));

        let form = app.form.as_ref().expect("debe abrir el formulario");
        assert_eq!(form.field, FormField::Badge);
        assert_eq!(form.medium, 0);
        assert!(form.badge.value().is_empty());
    }

    #[test]
    fn sin_gafete_registra_directamente_con_caminando() {
        let mut app = EntryPilot::default();
        app.selected = 1;

        app.handle_event(&key(KeyCode::Enter));
        assert_eq!(
            app.form.as_ref().map(|form| form.field),
            Some(FormField::Medium)
        );
        app.handle_event(&key(KeyCode::Enter));

        assert!(app.form.is_none());
        assert!(app.status.contains("Ana María Solís · Caminando · S/G"));
        app.selected = 1;
        app.handle_event(&key(KeyCode::Enter));
        assert!(app.form.is_none());
        assert!(app.status.contains("Ingreso activo registrado"));
    }

    #[test]
    fn gafete_es_obligatorio_numerico_y_no_puede_estar_ocupado() {
        let mut app = EntryPilot::default();
        app.handle_event(&key(KeyCode::Enter));

        app.handle_event(&key(KeyCode::Enter));
        assert!(
            app.form
                .as_ref()
                .and_then(|form| form.error.as_deref())
                .is_some_and(|error| error.contains("Ingrese"))
        );

        app.handle_event(&Event::Paste("abc".into()));
        assert!(
            app.form
                .as_ref()
                .and_then(|form| form.error.as_deref())
                .is_some_and(|error| error.contains("dígitos"))
        );

        app.handle_event(&Event::Paste("25".into()));
        app.handle_event(&key(KeyCode::Enter));
        assert!(
            app.form
                .as_ref()
                .and_then(|form| form.error.as_deref())
                .is_some_and(|error| error.contains("ya está en uso"))
        );
    }

    #[test]
    fn selector_cambia_medio_y_enter_registra_sin_confirmacion_extra() {
        let mut app = EntryPilot::default();
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&Event::Paste("31".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&key(KeyCode::Char(' ')));
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.form.as_ref().map(|form| form.medium), Some(1));
        app.handle_event(&key(KeyCode::Enter));

        assert!(app.form.is_none());
        assert!(app.status.contains("Vehículo · gafete 31"));
    }

    #[test]
    fn repeat_no_atraviesa_la_capa_que_abre_el_selector() {
        let mut app = EntryPilot::default();
        app.selected = 1;
        app.handle_event(&key(KeyCode::Enter));

        app.handle_event(&key(KeyCode::Char(' ')));
        app.handle_event(&repeat(KeyCode::Char(' ')));

        assert_eq!(app.layer(), Layer::MediumMenu);
        assert_eq!(
            app.form
                .as_ref()
                .and_then(|form| form.medium_menu.highlighted()),
            Some(0)
        );
    }

    #[test]
    fn renderiza_tamanos_objetivo_estados_y_cursor_real() {
        for (width, height) in [(60, 20), (80, 24), (120, 30)] {
            let mut app = EntryPilot::default();
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("backend de prueba");

            terminal
                .draw(|frame| app.render(frame))
                .expect("la pantalla debe renderizar");

            let rendered = buffer_text(terminal.backend());
            assert!(rendered.contains("REGISTRAR INGRESO"), "ancho {width}");
            assert!(rendered.contains("DISPONIBLE"), "ancho {width}");
            assert!(terminal.backend().cursor_visible());

            app.select(6);
            terminal
                .draw(|frame| app.render(frame))
                .expect("la tabla debe desplazarse al último registro");
            assert!(
                buffer_text(terminal.backend()).contains("DENTRO"),
                "ancho {width}"
            );
        }
    }

    #[test]
    fn ayuda_y_tema_no_pierden_el_formulario() {
        let mut app = EntryPilot::default();
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::F(1)));
        app.handle_event(&Event::Paste("99".into()));
        app.handle_event(&key(KeyCode::F(7)));

        assert!(app.help_visible);
        assert_eq!(app.theme, ThemePreset::Brisas);
        assert!(
            app.form
                .as_ref()
                .is_some_and(|form| form.badge.value().is_empty())
        );
    }

    #[test]
    fn terminal_pequena_congela_el_estado() {
        let mut app = EntryPilot::default();
        app.handle_event(&Event::Resize(40, 10));
        app.handle_event(&Event::Paste("Juan".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(app.search.value().is_empty());
        assert!(app.form.is_none());
    }

    #[test]
    fn cursor_del_gafete_parpadea() {
        let mut app = EntryPilot::default();
        app.handle_event(&key(KeyCode::Enter));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| app.render(frame))
            .expect("el modal debe renderizar");

        assert!(terminal.backend().cursor_visible());
        assert!(app.cursor_visible_at(app.cursor_started));
        assert!(!app.cursor_visible_at(app.cursor_started + Duration::from_millis(500)));
    }
}
