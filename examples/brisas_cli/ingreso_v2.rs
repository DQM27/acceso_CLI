//! Mockup de fluidez para registrar ingreso: mismos principios que
//! `contratistas_v2` (panel único que alterna entre vista previa y
//! formulario, medio de ingreso expandido inline, separador sutil entre
//! tabla y panel, lienzo pintado completo).
//!
//! cargo run --example brisas_cli -- ingreso        (piloto actual)
//! cargo run --example brisas_cli -- ingreso-v2     (esta propuesta)

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
    widgets::{Block, Cell, Paragraph, Row, Table, TableState, Wrap},
};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

use super::quick_exit::{QuickExitOutcome, QuickExitOverlay};

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 20;
const WIDE_LAYOUT_WIDTH: u16 = 100;
/// Operador con sesión simulada: cada pantalla lo muestra en la barra
/// superior y lo estampa al registrar, para que quede trazable quién hizo
/// cada acción. En la app real vendría de la sesión autenticada.
const CURRENT_OPERATOR: &str = "Daniel Quintana";
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

    const fn tone(self) -> Tone {
        match self {
            Self::Available => Tone::Normal,
            Self::Warning(_) => Tone::Warning,
            Self::Blocked { .. } => Tone::Danger,
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

#[derive(Debug)]
struct EntryForm {
    contractor_id: u32,
    medium: usize,
    medium_expanded: Option<usize>,
    badge: Input,
    field: FormField,
    error: Option<String>,
}

impl EntryForm {
    fn new(candidate: Candidate) -> Self {
        Self {
            contractor_id: candidate.id,
            medium: 0,
            medium_expanded: None,
            badge: Input::default(),
            field: if candidate.requires_badge {
                FormField::Badge
            } else {
                FormField::Medium
            },
            error: None,
        }
    }

    fn move_focus(&mut self, backwards: bool) {
        self.field = match (self.field, backwards) {
            (FormField::Badge, false) | (FormField::Medium, true) => FormField::Medium,
            (FormField::Medium, false) | (FormField::Badge, true) => FormField::Badge,
        };
        self.error = None;
    }
}

#[derive(Debug)]
enum Panel {
    Preview,
    Form(Box<EntryForm>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Search,
    Form,
    MediumExpand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    Normal,
    Success,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatOrigin {
    code: KeyCode,
    modifiers: KeyModifiers,
    layer: Layer,
}

#[derive(Debug)]
pub struct IngresoV2Pilot {
    candidates: Vec<Candidate>,
    search: Input,
    selected: usize,
    panel: Panel,
    prepared_message: Option<(String, Tone)>,
    help_expanded: bool,
    theme: ThemePreset,
    running: bool,
    terminal_size: (u16, u16),
    repeat_origin: Option<RepeatOrigin>,
    quick_exit: QuickExitOverlay,
}

impl Default for IngresoV2Pilot {
    fn default() -> Self {
        Self {
            candidates: demo_candidates(),
            search: Input::default(),
            selected: 0,
            panel: Panel::Preview,
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

impl IngresoV2Pilot {
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
            Layer::Form => self.handle_form_key(key),
            Layer::MediumExpand => self.handle_medium_expand_key(key),
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

    fn handle_form_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Repeat && !matches!(key.code, KeyCode::Left | KeyCode::Right) {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => self.register(),
            Some(StandardCommand::Activate) => {
                let Panel::Form(form) = &mut self.panel else {
                    return false;
                };
                if form.field != FormField::Medium {
                    return false;
                }
                form.medium_expanded = Some(form.medium);
            }
            Some(StandardCommand::Cancel) => {
                self.panel = Panel::Preview;
                self.prepared_message =
                    Some(("Registro cancelado · no hubo cambios.".into(), Tone::Warning));
            }
            Some(StandardCommand::FocusNext) => self.move_form_focus(false),
            Some(StandardCommand::FocusPrevious) => self.move_form_focus(true),
            Some(StandardCommand::Help) if key.kind == KeyEventKind::Press => {
                self.help_expanded = !self.help_expanded;
            }
            Some(StandardCommand::Theme) if key.kind == KeyEventKind::Press => self.toggle_theme(),
            _ => match key.code {
                KeyCode::Left => self.cycle_medium(true),
                KeyCode::Right => self.cycle_medium(false),
                _ => return false,
            },
        }
        true
    }

    fn handle_medium_expand_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(key.code, KeyCode::Up | KeyCode::Down);
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => {
                if let Panel::Form(form) = &mut self.panel
                    && let Some(highlighted) = form.medium_expanded
                {
                    form.medium = highlighted;
                    form.error = None;
                }
                if let Panel::Form(form) = &mut self.panel {
                    form.medium_expanded = None;
                }
            }
            Some(StandardCommand::Cancel) => {
                if let Panel::Form(form) = &mut self.panel {
                    form.medium_expanded = None;
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
                let Some(highlighted) = &mut form.medium_expanded else {
                    return false;
                };
                match key.code {
                    KeyCode::Up => {
                        *highlighted = highlighted.checked_sub(1).unwrap_or(ENTRY_MEDIA.len() - 1);
                    }
                    KeyCode::Down => *highlighted = (*highlighted + 1) % ENTRY_MEDIA.len(),
                    _ => return false,
                }
            }
        }
        true
    }

    fn handle_text_input(&mut self, event: &Event) -> bool {
        if let Panel::Form(form) = &mut self.panel {
            if form.field != FormField::Badge || form.medium_expanded.is_some() {
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
            let changed = apply_event(&mut form.badge, event);
            if changed {
                form.error = None;
            }
            return changed;
        }
        let changed = apply_event(&mut self.search, event);
        if changed {
            self.selected = 0;
            self.prepared_message = None;
        }
        changed
    }

    fn open_selected(&mut self) {
        let Some(candidate) = self.selected_candidate().copied() else {
            self.prepared_message = Some(("No hay un contratista seleccionado.".into(), Tone::Warning));
            return;
        };
        if !candidate.eligibility.can_register() {
            self.prepared_message = Some((
                candidate.eligibility.explanation().into(),
                candidate.eligibility.tone(),
            ));
            return;
        }
        self.panel = Panel::Form(Box::new(EntryForm::new(candidate)));
        self.prepared_message = None;
    }

    fn register(&mut self) {
        let Panel::Form(form) = &self.panel else {
            return;
        };
        let Some(candidate) = self
            .candidates
            .iter()
            .find(|candidate| candidate.id == form.contractor_id)
            .copied()
        else {
            self.panel = Panel::Preview;
            self.prepared_message = Some(("El contratista ya no está disponible.".into(), Tone::Danger));
            return;
        };
        if !candidate.eligibility.can_register() {
            self.panel = Panel::Preview;
            self.prepared_message = Some((
                candidate.eligibility.explanation().into(),
                candidate.eligibility.tone(),
            ));
            return;
        }

        let badge = if candidate.requires_badge {
            let value = form.badge.value().trim();
            let Ok(number) = value.parse::<i64>() else {
                if let Panel::Form(form) = &mut self.panel {
                    form.error = Some("Ingrese el número de gafete.".into());
                }
                return;
            };
            if OCCUPIED_BADGES.contains(&number) {
                if let Panel::Form(form) = &mut self.panel {
                    form.error = Some(format!("El gafete {number} ya está en uso."));
                }
                return;
            }
            Some(number)
        } else {
            None
        };

        let medium = ENTRY_MEDIA[form.medium];
        self.panel = Panel::Preview;
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
        self.search = Input::default();
        self.selected = 0;
        let badge_text = badge.map_or_else(|| "S/G".into(), |number| format!("gafete {number}"));
        self.prepared_message = Some((
            format!(
                "Ingreso registrado · {} · {medium} · {badge_text} · registró {CURRENT_OPERATOR}",
                candidate.name
            ),
            Tone::Success,
        ));
    }

    fn move_form_focus(&mut self, backwards: bool) {
        let requires_badge = self.form_requires_badge();
        let Panel::Form(form) = &mut self.panel else {
            return;
        };
        if requires_badge {
            form.move_focus(backwards);
        }
    }

    fn cycle_medium(&mut self, backwards: bool) {
        let Panel::Form(form) = &mut self.panel else {
            return;
        };
        if form.field != FormField::Medium {
            return;
        }
        form.medium = cycle(form.medium, ENTRY_MEDIA.len(), backwards);
        form.error = None;
    }

    fn form_requires_badge(&self) -> bool {
        let Panel::Form(form) = &self.panel else {
            return false;
        };
        self.candidates
            .iter()
            .find(|candidate| candidate.id == form.contractor_id)
            .is_some_and(|candidate| candidate.requires_badge)
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

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
    }

    fn layer(&self) -> Layer {
        match &self.panel {
            Panel::Form(form) if form.medium_expanded.is_some() => Layer::MediumExpand,
            Panel::Form(_) => Layer::Form,
            Panel::Preview => Layer::Search,
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
            .format("%H:%M:%S")
            .to_string();
        let columns =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(area);
        frame.render_widget(
            Paragraph::new("brisas cli · registrar ingreso (v2)").style(theme.muted()),
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
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(4),
        ])
        .split(area);
        render_field(
            frame,
            rows[0],
            FieldSpec {
                label: "BUSCAR CONTRATISTA",
                input: &self.search,
                focused: matches!(self.panel, Panel::Preview),
                theme,
            },
        );
        let count = self.filtered_indices().len();
        frame.render_widget(
            Paragraph::new(format!(
                "{count} resultados · se muestran también bloqueados y activos"
            ))
            .style(theme.muted()),
            rows[1],
        );

        let panel_height = self.panel_row_count();
        if area.width >= WIDE_LAYOUT_WIDTH {
            let columns = Layout::horizontal([
                Constraint::Percentage(63),
                Constraint::Length(1),
                Constraint::Percentage(35),
            ])
            .split(rows[2]);
            self.render_table(frame, columns[0], theme);
            render_vertical_separator(frame, columns[1], theme);
            self.render_panel(frame, columns[2], theme);
        } else {
            let stacked = Layout::vertical([
                Constraint::Min(4),
                Constraint::Length(1),
                Constraint::Length(panel_height.min(rows[2].height.saturating_sub(5))),
            ])
            .split(rows[2]);
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
        let wide = area.width >= 76;
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
                Paragraph::new("Sin coincidencias · Esc limpia la búsqueda")
                    .style(theme.warning())
                    .alignment(Alignment::Center),
                Rect::new(area.x, area.y + area.height / 2, area.width, 1),
            );
        }
    }

    fn panel_row_count(&self) -> u16 {
        match &self.panel {
            Panel::Preview => 6,
            Panel::Form(form) => {
                let mut total: u16 = 4; // nombre + cédula·empresa + blanco + medio
                if form.medium_expanded.is_some() {
                    total += ENTRY_MEDIA.len() as u16;
                }
                if self.form_requires_badge() {
                    total += 3;
                }
                total += 1; // error
                total
            }
        }
    }

    fn render_panel(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        match &self.panel {
            Panel::Preview => self.render_preview(frame, area, theme),
            Panel::Form(form) => self.render_form(frame, area, form, theme),
        }
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let Some(candidate) = self.selected_candidate() else {
            frame.render_widget(
                Paragraph::new("No hay un registro seleccionado").style(theme.muted()),
                area,
            );
            return;
        };
        let status_style = match candidate.eligibility {
            Eligibility::Available => theme.success(),
            Eligibility::Warning(_) => theme.warning(),
            Eligibility::Blocked { .. } => theme.danger(),
        };
        let route = if candidate.route_staff {
            " · PERSONAL DE RUTA"
        } else {
            ""
        };
        let lines = vec![
            Line::from(candidate.name).style(theme.title()),
            Line::from(format!("{} · {}", candidate.identity, candidate.company)).style(theme.base()),
            Line::from(format!("{}{route}", candidate.entry_type)).style(theme.muted()),
            Line::from(""),
            Line::from(candidate.eligibility.label()).style(status_style),
            Line::from(candidate.eligibility.explanation()).style(theme.muted()),
        ];
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn render_form(&self, frame: &mut Frame, area: Rect, form: &EntryForm, theme: Theme) {
        let Some(candidate) = self
            .candidates
            .iter()
            .find(|candidate| candidate.id == form.contractor_id)
        else {
            return;
        };
        let mut constraints = vec![
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ];
        if form.medium_expanded.is_some() {
            constraints.push(Constraint::Length(ENTRY_MEDIA.len() as u16));
        }
        if candidate.requires_badge {
            constraints.push(Constraint::Length(3));
        }
        constraints.push(Constraint::Length(1));
        let rows = Layout::vertical(constraints).split(area);
        let mut cursor = 0;

        frame.render_widget(
            Paragraph::new(candidate.name).style(theme.title()),
            rows[cursor],
        );
        cursor += 1;
        let route = if candidate.route_staff {
            " · PERSONAL DE RUTA"
        } else {
            ""
        };
        frame.render_widget(
            Paragraph::new(format!("{} · {}{route}", candidate.identity, candidate.company))
                .style(theme.muted()),
            rows[cursor],
        );
        cursor += 2; // deja una fila en blanco antes del formulario

        render_choice(
            frame,
            rows[cursor],
            "MEDIO DE INGRESO",
            ENTRY_MEDIA[form.medium],
            form.field == FormField::Medium,
            theme,
        );
        cursor += 1;

        if let Some(highlighted) = form.medium_expanded {
            render_inline_list(frame, rows[cursor], &ENTRY_MEDIA, highlighted, theme);
            cursor += 1;
        }

        if candidate.requires_badge {
            render_field(
                frame,
                rows[cursor],
                FieldSpec {
                    label: "NÚMERO DE GAFETE",
                    input: &form.badge,
                    focused: form.field == FormField::Badge && form.medium_expanded.is_none(),
                    theme,
                },
            );
            cursor += 1;
        }

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
        if let Some((message, tone)) = &self.prepared_message {
            let style = match tone {
                Tone::Normal => theme.muted(),
                Tone::Success => theme.success(),
                Tone::Warning => theme.warning(),
                Tone::Danger => theme.danger(),
            };
            return Line::from(message.clone()).style(style);
        }
        Line::from("").style(theme.muted())
    }

    fn hint_lines(&self, theme: Theme) -> Vec<Line<'static>> {
        let primary = match self.layer() {
            Layer::Search => vec![
                Span::styled("↑↓", theme.accent()),
                Span::styled(" mover   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" seleccionar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" limpiar/salir   ", theme.base()),
                Span::styled("F1", theme.accent()),
                Span::styled(
                    if self.help_expanded { " cerrar ayuda" } else { " más" },
                    theme.base(),
                ),
            ],
            Layer::Form => vec![
                Span::styled("TAB", theme.accent()),
                Span::styled(" campo   ", theme.base()),
                Span::styled("←→", theme.accent()),
                Span::styled(" cambiar   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" registrar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cancelar", theme.base()),
            ],
            Layer::MediumExpand => vec![
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
                Span::styled("ESP", theme.accent()),
                Span::styled(" abrir medio   ", theme.base()),
                Span::styled(QUICK_EXIT_HINT.key, theme.accent()),
                Span::styled(format!(" {}   ", QUICK_EXIT_HINT.label), theme.base()),
                Span::styled("F7", theme.accent()),
                Span::styled(" tema", theme.base()),
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

fn cycle(current: usize, length: usize, backwards: bool) -> usize {
    if backwards {
        current.checked_sub(1).unwrap_or(length.saturating_sub(1))
    } else {
        (current + 1) % length.max(1)
    }
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
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{FormField, IngresoV2Pilot, Layer, Panel, ThemePreset};

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
    fn inicia_mostrando_todos_con_vista_previa_del_primero() {
        let app = IngresoV2Pilot::default();
        assert_eq!(app.filtered_indices().len(), 7);
        assert!(matches!(app.panel, Panel::Preview));
        assert_eq!(app.layer(), Layer::Search);
    }

    #[test]
    fn buscar_encuentra_incluso_un_contratista_bloqueado_y_no_lo_deja_registrar() {
        let mut app = IngresoV2Pilot::default();
        app.handle_event(&Event::Paste("Mónica".into()));

        assert_eq!(app.filtered_indices().len(), 1);
        app.handle_event(&key(KeyCode::Enter));

        assert!(matches!(app.panel, Panel::Preview));
        assert!(
            app.prepared_message
                .as_ref()
                .is_some_and(|(m, _)| m.contains("PRAIND está vencido"))
        );
    }

    #[test]
    fn disponible_con_gafete_abre_el_formulario_junto_a_la_lista_sin_taparla() {
        let mut app = IngresoV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Form(form) = &app.panel else {
            panic!("debe abrir el formulario");
        };
        assert_eq!(form.field, FormField::Badge);

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal
            .draw(|frame| app.render(frame))
            .expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("Juan Rodríguez"));
        assert!(rendered.contains("NÚMERO DE GAFETE"));
    }

    #[test]
    fn f2_funciona_en_medio_de_un_formulario_de_ingreso_abierto() {
        let mut app = IngresoV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter)); // Juan Rodríguez, requiere gafete
        app.handle_event(&Event::Paste("31".into()));

        app.handle_event(&key(KeyCode::F(2)));
        assert!(app.quick_exit.is_open());
        app.handle_event(&Event::Paste("12".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(!app.quick_exit.is_open());
        let Panel::Form(form) = &app.panel else {
            panic!("debe seguir en el formulario de Juan Rodríguez, con el gafete ya tipeado");
        };
        assert_eq!(form.badge.value(), "31");
        assert!(
            app.prepared_message
                .as_ref()
                .is_some_and(|(m, _)| m.contains("José Peña"))
        );
    }

    #[test]
    fn sin_gafete_registra_directamente_con_caminando() {
        let mut app = IngresoV2Pilot::default();
        app.select(1);
        app.handle_event(&key(KeyCode::Enter));
        assert_eq!(
            match &app.panel {
                Panel::Form(form) => Some(form.field),
                Panel::Preview => None,
            },
            Some(FormField::Medium)
        );
        app.handle_event(&key(KeyCode::Enter));

        assert!(matches!(app.panel, Panel::Preview));
        assert!(
            app.prepared_message
                .as_ref()
                .is_some_and(|(m, _)| m.contains("Ana María Solís · Caminando · S/G"))
        );
        assert!(
            app.prepared_message
                .as_ref()
                .is_some_and(|(m, _)| m.contains("registró Daniel Quintana")),
            "el mensaje debe dejar trazado quién registró el ingreso"
        );
    }

    #[test]
    fn gafete_es_obligatorio_numerico_y_no_puede_estar_ocupado() {
        let mut app = IngresoV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("Ingrese")));

        app.handle_event(&Event::Paste("abc".into()));
        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("dígitos")));

        app.handle_event(&Event::Paste("25".into()));
        app.handle_event(&key(KeyCode::Enter));
        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("ya está en uso")));
    }

    #[test]
    fn el_medio_se_expande_inline_sin_ocultar_el_resto_del_formulario() {
        let mut app = IngresoV2Pilot::default();
        app.select(1); // Ana María, sin gafete
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::Char(' ')));

        assert_eq!(app.layer(), Layer::MediumExpand);

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal
            .draw(|frame| app.render(frame))
            .expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("Vehículo"));
        assert!(rendered.contains("Ana María Solís"));
    }

    #[test]
    fn confirmar_el_medio_expandido_actualiza_el_campo_y_registra() {
        let mut app = IngresoV2Pilot::default();
        app.select(1);
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::Char(' ')));
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::Enter));

        assert!(matches!(app.panel, Panel::Preview));
        assert!(
            app.prepared_message
                .as_ref()
                .is_some_and(|(m, _)| m.contains("Vehículo"))
        );
    }

    #[test]
    fn pinta_todo_el_lienzo_sin_dejar_parches_sin_color() {
        let mut app = IngresoV2Pilot::default();
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
    fn terminal_pequena_congela_el_estado() {
        let mut app = IngresoV2Pilot::default();
        app.handle_event(&Event::Resize(40, 10));
        app.handle_event(&Event::Paste("Juan".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(app.search.value().is_empty());
        assert!(matches!(app.panel, Panel::Preview));
    }
}
