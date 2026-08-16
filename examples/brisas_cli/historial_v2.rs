//! Historial como timeline agrupado por día, no como tabla — un historial
//! es un registro cronológico, y ese es justo el patrón que usan los
//! visores de log/auditoría reales (agrupar por fecha, marcar estado con
//! un glifo, mostrar duración) en vez de una hoja de cálculo.
//!
//! La búsqueda usa sintaxis clave:valor (empresa:Expenic estado:activos
//! gafete:25, más texto libre para nombre/cédula) en un único campo
//! siempre activo — como `gh issue list --search` o la barra de GitHub —
//! en vez de la banda de 6 campos con desplegables que tenían las otras
//! pantallas. No hay paso de "Aplicar": todo filtra en vivo.
//!
//! Reemplaza por completo el primer intento de esta pantalla, que
//! calcaba el patrón lista+panel de contratistas/ingreso/activos sin
//! aportar nada propio de un historial.
//!
//! cargo run --example brisas_cli -- historial-v2

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Utc};
use chrono_tz::America::Costa_Rica;
use control_acceso::tui::ui_kit::{
    QUICK_EXIT_HINT, StandardCommand, Theme, ThemePreset, auxiliary_panel, centered_rect,
    render_terminal_too_small, standard_command,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState},
};
use std::collections::BTreeMap;
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
const WIDE_LAYOUT_WIDTH: u16 = 100;
const JUMP: isize = 5;
const PANEL_HEIGHT: u16 = 12;
/// Operador con sesión simulada, igual que en el resto de las pantallas v2.
const CURRENT_OPERATOR: &str = "Daniel Quintana";
const QUERY_KEYS_HINT: &str = "claves: empresa: tipo: estado: gafete: desde: hasta: ingreso: salida:  ·  texto libre busca por nombre o cédula";

/// Dos formas de leer el mismo conjunto filtrado: el timeline agrupado
/// (curado, con panel de detalle) o la tabla clásica de una sola línea por
/// movimiento (densa, sin panel, todo el dato visible de una vez) para
/// quien prefiera el formato de planilla. Cambiar de vista conserva la
/// selección — es la misma búsqueda, sólo cambia cómo se lee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Timeline,
    Classic,
    Heatmap,
}

impl ViewMode {
    const fn next(self) -> Self {
        match self {
            Self::Timeline => Self::Classic,
            Self::Classic => Self::Heatmap,
            Self::Heatmap => Self::Timeline,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Timeline => "LÍNEA DE TIEMPO",
            Self::Classic => "CLÁSICA",
            Self::Heatmap => "MAPA DE CALOR",
        }
    }
}

/// Columnas de la vista Clásica. El panel de la Línea de tiempo ya muestra
/// todo, pero Clásica no tiene panel — ahí sí tiene sentido dejar
/// ocultar/mostrar columnas para controlar la densidad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClassicColumn {
    Fecha,
    Cedula,
    Nombre,
    Empresa,
    Tipo,
    Entrada,
    Salida,
    Gafete,
    Medio,
    Ingreso,
    Egreso,
}

impl ClassicColumn {
    // FECHA y CÉDULA seguidas eran dos columnas numéricas pegadas y
    // costaba distinguirlas de un vistazo; NOMBRE de por medio rompe esa
    // racha de números.
    const ALL: [Self; 11] = [
        Self::Fecha,
        Self::Nombre,
        Self::Cedula,
        Self::Empresa,
        Self::Tipo,
        Self::Entrada,
        Self::Salida,
        Self::Gafete,
        Self::Medio,
        Self::Ingreso,
        Self::Egreso,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Fecha => "FECHA",
            Self::Cedula => "CÉDULA",
            Self::Nombre => "NOMBRE",
            Self::Empresa => "EMPRESA",
            Self::Tipo => "TIPO",
            Self::Entrada => "ENTRADA",
            Self::Salida => "SALIDA",
            Self::Gafete => "GAFETE",
            Self::Medio => "MEDIO",
            Self::Ingreso => "DA INGRESO",
            Self::Egreso => "DA SALIDA",
        }
    }

    const fn constraint(self) -> Constraint {
        match self {
            Self::Fecha => Constraint::Length(10),
            Self::Cedula => Constraint::Length(14),
            Self::Nombre | Self::Empresa => Constraint::Fill(3),
            Self::Tipo => Constraint::Length(11),
            Self::Entrada | Self::Salida => Constraint::Length(8),
            Self::Gafete => Constraint::Length(7),
            Self::Medio => Constraint::Fill(1),
            Self::Ingreso | Self::Egreso => Constraint::Fill(2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Activos,
    Cerrados,
}

fn parse_status(value: &str) -> Option<Status> {
    match value.to_lowercase().as_str() {
        "activos" | "activo" | "dentro" => Some(Status::Activos),
        "cerrados" | "cerrado" | "salieron" | "salio" | "salió" => Some(Status::Cerrados),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MovementRecord {
    id: i64,
    date: String,
    identity: String,
    name: String,
    company: String,
    entry_type: String,
    entry_time: String,
    exit_time: Option<String>,
    badge: Option<i64>,
    medium: String,
    entry_operator: String,
    exit_operator: Option<String>,
}

#[derive(Debug, Default)]
struct ParsedQuery {
    text: String,
    company: Option<String>,
    entry_type: Option<String>,
    status: Option<Status>,
    badge: Option<String>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    entry_operator: Option<String>,
    exit_operator: Option<String>,
    unrecognized_date: Option<String>,
}

/// Respeta comillas para que `empresa:"Expenic Industrial"` sea un solo
/// token en vez de partirse en el espacio.
fn tokenize(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for character in query.chars() {
        match character {
            '"' => in_quotes = !in_quotes,
            character if character.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            character => current.push(character),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_query(raw: &str) -> ParsedQuery {
    let mut parsed = ParsedQuery::default();
    let mut free_text = Vec::new();
    for token in tokenize(raw) {
        let Some((key, value)) = token.split_once(':') else {
            free_text.push(token);
            continue;
        };
        if value.is_empty() {
            free_text.push(token);
            continue;
        }
        match key.to_lowercase().as_str() {
            "empresa" => parsed.company = Some(value.to_owned()),
            "tipo" => parsed.entry_type = Some(value.to_owned()),
            "gafete" => parsed.badge = Some(value.to_owned()),
            "estado" => parsed.status = parse_status(value),
            "desde" => match parse_date(value) {
                Some(date) => parsed.from = Some(date),
                None => parsed.unrecognized_date = Some(format!("desde:{value}")),
            },
            "hasta" => match parse_date(value) {
                Some(date) => parsed.to = Some(date),
                None => parsed.unrecognized_date = Some(format!("hasta:{value}")),
            },
            "ingreso" => parsed.entry_operator = Some(value.to_owned()),
            "salida" => parsed.exit_operator = Some(value.to_owned()),
            _ => free_text.push(token),
        }
    }
    parsed.text = free_text.join(" ");
    parsed
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%d/%m/%Y").ok()
}

fn start_of_week(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

fn end_of_week(date: NaiveDate) -> NaiveDate {
    date + Duration::days(6 - i64::from(date.weekday().num_days_from_monday()))
}

fn bucket_glyph(count: usize, max: usize) -> char {
    if count == 0 {
        return '·';
    }
    let ratio = count as f64 / max.max(1) as f64;
    if ratio >= 0.99 {
        '█'
    } else if ratio >= 0.66 {
        '▓'
    } else if ratio >= 0.33 {
        '▒'
    } else {
        '░'
    }
}

fn bucket_style(count: usize, max: usize, theme: Theme) -> Style {
    if count == 0 {
        return theme.muted();
    }
    let ratio = count as f64 / max.max(1) as f64;
    if ratio >= 0.99 {
        theme.warning()
    } else if ratio >= 0.66 {
        theme.accent()
    } else if ratio >= 0.33 {
        theme.base()
    } else {
        theme.muted()
    }
}

fn matches_record(record: &MovementRecord, parsed: &ParsedQuery) -> bool {
    if !parsed.text.is_empty() {
        let query = parsed.text.to_lowercase();
        if !record.name.to_lowercase().contains(&query) && !record.identity.to_lowercase().contains(&query) {
            return false;
        }
    }
    if let Some(company) = &parsed.company
        && !record.company.to_lowercase().contains(&company.to_lowercase())
    {
        return false;
    }
    if let Some(entry_type) = &parsed.entry_type
        && !record.entry_type.to_lowercase().contains(&entry_type.to_lowercase())
    {
        return false;
    }
    if let Some(badge) = &parsed.badge
        && !record.badge.is_some_and(|number| number.to_string().contains(badge.as_str()))
    {
        return false;
    }
    match parsed.status {
        Some(Status::Activos) if record.exit_time.is_some() => return false,
        Some(Status::Cerrados) if record.exit_time.is_none() => return false,
        _ => {}
    }
    if parsed.from.is_some() || parsed.to.is_some() {
        let Some(date) = parse_date(&record.date) else {
            return false;
        };
        if parsed.from.is_some_and(|from| date < from) || parsed.to.is_some_and(|to| date > to) {
            return false;
        }
    }
    if let Some(entry_operator) = &parsed.entry_operator
        && !record.entry_operator.to_lowercase().contains(&entry_operator.to_lowercase())
    {
        return false;
    }
    if let Some(exit_operator) = &parsed.exit_operator {
        let matches = record
            .exit_operator
            .as_ref()
            .is_some_and(|operator| operator.to_lowercase().contains(&exit_operator.to_lowercase()));
        if !matches {
            return false;
        }
    }
    true
}

fn duration_text(record: &MovementRecord) -> Option<String> {
    let entry = NaiveTime::parse_from_str(&record.entry_time, "%H:%M").ok()?;
    let exit = NaiveTime::parse_from_str(record.exit_time.as_deref()?, "%H:%M").ok()?;
    let minutes = (exit - entry).num_minutes();
    (minutes >= 0).then(|| format!("{}h{:02}m", minutes / 60, minutes % 60))
}

fn day_header(date: NaiveDate) -> String {
    date.format("%d/%m/%Y").to_string()
}

#[derive(Debug)]
pub struct HistorialV2Pilot {
    records: Vec<MovementRecord>,
    today: NaiveDate,
    query: Input,
    selected: usize,
    timeline_offset: usize,
    view: ViewMode,
    classic_columns: Vec<(ClassicColumn, bool)>,
    columns_editor: Option<usize>,
    heatmap_selected: NaiveDate,
    prepared_message: Option<String>,
    help_expanded: bool,
    theme: ThemePreset,
    running: bool,
    terminal_size: (u16, u16),
    quick_exit: QuickExitOverlay,
}

impl Default for HistorialV2Pilot {
    fn default() -> Self {
        let today = Utc::now().with_timezone(&Costa_Rica).date_naive();
        Self {
            records: demo_records(today),
            today,
            query: Input::default(),
            selected: 0,
            timeline_offset: 0,
            view: ViewMode::Timeline,
            classic_columns: ClassicColumn::ALL.into_iter().map(|column| (column, true)).collect(),
            columns_editor: None,
            heatmap_selected: today,
            prepared_message: None,
            help_expanded: false,
            theme: ThemePreset::Classic,
            running: true,
            terminal_size: (u16::MAX, u16::MAX),
            quick_exit: QuickExitOverlay::demo(),
        }
    }
}

impl HistorialV2Pilot {
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        if let Event::Resize(width, height) = event {
            self.terminal_size = (*width, *height);
            return true;
        }
        if matches!(event, Event::FocusGained | Event::FocusLost) {
            return true;
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

        if let Some(focus) = self.columns_editor {
            return self.handle_columns_editor_key(key, focus);
        }

        let repeatable = matches!(
            key.code,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::Tab
                | KeyCode::BackTab
        );
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return self.handle_text_input(event);
        }

        let heatmap = self.view == ViewMode::Heatmap;
        let handled = match standard_command(key) {
            Some(StandardCommand::Cancel) => {
                if self.query.value().is_empty() {
                    self.running = false;
                    return true;
                }
                self.query = Input::default();
                self.selected = 0;
                true
            }
            Some(StandardCommand::Help) if key.kind == KeyEventKind::Press => {
                self.help_expanded = !self.help_expanded;
                true
            }
            Some(StandardCommand::Theme) if key.kind == KeyEventKind::Press => {
                self.toggle_theme();
                true
            }
            // El mapa de calor navega día a día con Tab y confirma con
            // Enter — ambas teclas son inertes en un campo de una sola
            // línea, así que no le quitan nada al buscador siempre activo.
            Some(StandardCommand::FocusNext) if heatmap => {
                self.move_heatmap_day(1);
                true
            }
            Some(StandardCommand::FocusPrevious) if heatmap => {
                self.move_heatmap_day(-1);
                true
            }
            Some(StandardCommand::Primary) if heatmap => {
                self.drill_down_heatmap_day();
                true
            }
            _ => match key.code {
                KeyCode::Up if heatmap => {
                    self.move_heatmap_day(-7);
                    true
                }
                KeyCode::Down if heatmap => {
                    self.move_heatmap_day(7);
                    true
                }
                KeyCode::Up => {
                    self.move_selection(-1);
                    true
                }
                KeyCode::Down => {
                    self.move_selection(1);
                    true
                }
                KeyCode::PageUp => {
                    self.move_selection(-JUMP);
                    true
                }
                KeyCode::PageDown => {
                    self.move_selection(JUMP);
                    true
                }
                KeyCode::Home => {
                    self.selected = 0;
                    true
                }
                KeyCode::End => {
                    self.selected = self.filtered_sorted_indices().len().saturating_sub(1);
                    true
                }
                // F3, no una letra: el campo de búsqueda está siempre
                // activo, así que un atajo de letra chocaría con escribir
                // términos que la contengan (p. ej. "Vehículo").
                KeyCode::F(3) if key.kind == KeyEventKind::Press => {
                    self.view = self.view.next();
                    true
                }
                // Sólo tiene sentido en Clásica: la Línea de tiempo no
                // tiene columnas que ocultar.
                KeyCode::F(4) if key.kind == KeyEventKind::Press && self.view == ViewMode::Classic => {
                    self.columns_editor = Some(0);
                    true
                }
                _ => false,
            },
        };
        if handled {
            return true;
        }
        self.handle_text_input(event)
    }

    fn handle_text_input(&mut self, event: &Event) -> bool {
        if self.columns_editor.is_some() {
            return false;
        }
        let changed = apply_event(&mut self.query, event);
        if changed {
            self.selected = 0;
            self.prepared_message = None;
        }
        changed
    }

    fn parsed_query(&self) -> ParsedQuery {
        parse_query(self.query.value())
    }

    fn filtered_sorted_indices(&self) -> Vec<usize> {
        let parsed = self.parsed_query();
        let mut indices: Vec<usize> = self
            .records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| matches_record(record, &parsed).then_some(index))
            .collect();
        indices.sort_by(|&left, &right| {
            let sort_key = |index: usize| {
                let record = &self.records[index];
                let date = parse_date(&record.date).unwrap_or(NaiveDate::MIN);
                let time = NaiveTime::parse_from_str(&record.entry_time, "%H:%M")
                    .unwrap_or(NaiveTime::MIN);
                (date, time)
            };
            sort_key(right).cmp(&sort_key(left))
        });
        indices
    }

    fn selected_record(&self) -> Option<&MovementRecord> {
        let indices = self.filtered_sorted_indices();
        self.records.get(*indices.get(self.selected)?)
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.filtered_sorted_indices().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self.selected.saturating_add_signed(delta).min(count - 1);
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
    }

    fn day_counts(&self) -> BTreeMap<NaiveDate, usize> {
        let mut counts = BTreeMap::new();
        for index in self.filtered_sorted_indices() {
            if let Some(date) = parse_date(&self.records[index].date) {
                *counts.entry(date).or_insert(0usize) += 1;
            }
        }
        counts
    }

    fn move_heatmap_day(&mut self, delta: i64) {
        if let Some(date) = self.heatmap_selected.checked_add_signed(Duration::days(delta)) {
            self.heatmap_selected = date;
        }
    }

    /// El heatmap es para ubicar "cuándo": confirmar un día reutiliza el
    /// mismo campo de búsqueda para acotar el rango y salta a la Línea de
    /// tiempo a mirar esos movimientos uno por uno.
    fn drill_down_heatmap_day(&mut self) {
        let date_text = self.heatmap_selected.format("%d/%m/%Y").to_string();
        self.query = Input::new(format!("desde:{date_text} hasta:{date_text}"));
        self.view = ViewMode::Timeline;
        self.selected = 0;
        self.prepared_message = None;
    }

    fn handle_columns_editor_key(&mut self, key: KeyEvent, focus: usize) -> bool {
        let repeatable = matches!(key.code, KeyCode::Up | KeyCode::Down);
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return true;
        }
        match key.code {
            KeyCode::Up => {
                self.columns_editor = Some(focus.checked_sub(1).unwrap_or(ClassicColumn::ALL.len() - 1));
            }
            KeyCode::Down => {
                self.columns_editor = Some((focus + 1) % ClassicColumn::ALL.len());
            }
            KeyCode::Char(' ') => {
                let visible_count = self.classic_columns.iter().filter(|(_, visible)| *visible).count();
                let (_, visible) = &mut self.classic_columns[focus];
                if !(*visible && visible_count == 1) {
                    *visible = !*visible;
                }
            }
            KeyCode::Esc | KeyCode::F(4) => self.columns_editor = None,
            _ => {}
        }
        true
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

        if let Some(focus) = self.columns_editor {
            self.render_columns_editor(frame, area, focus, theme);
        }
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
            Paragraph::new(format!("brisas cli · historial (v2) · {}", self.view.label())).style(theme.muted()),
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
        let rows = Layout::vertical([Constraint::Length(3), Constraint::Length(1), Constraint::Min(4)])
            .split(area);

        render_field(
            frame,
            rows[0],
            FieldSpec {
                label: "BUSCAR · CLAVE:VALOR O TEXTO LIBRE",
                input: &self.query,
                theme,
            },
        );
        frame.render_widget(self.facet_line(theme), rows[1]);

        match self.view {
            ViewMode::Timeline => self.render_timeline_view(frame, rows[2], area.width, theme),
            ViewMode::Classic => self.render_classic_table(frame, rows[2], theme),
            ViewMode::Heatmap => self.render_heatmap(frame, rows[2], theme),
        }
    }

    /// El timeline agrupado + panel de detalle — el enfoque "curado".
    fn render_timeline_view(&mut self, frame: &mut Frame, area: Rect, full_width: u16, theme: Theme) {
        if full_width >= WIDE_LAYOUT_WIDTH {
            let columns = Layout::horizontal([
                Constraint::Percentage(63),
                Constraint::Length(1),
                Constraint::Percentage(35),
            ])
            .split(area);
            self.render_timeline(frame, columns[0], theme);
            render_vertical_separator(frame, columns[1], theme);
            self.render_panel(frame, columns[2], theme);
        } else {
            let stacked = Layout::vertical([
                Constraint::Min(4),
                Constraint::Length(1),
                Constraint::Length(PANEL_HEIGHT.min(area.height.saturating_sub(5))),
            ])
            .split(area);
            self.render_timeline(frame, stacked[0], theme);
            render_horizontal_separator(frame, stacked[1], theme);
            self.render_panel(frame, stacked[2], theme);
        }
    }

    /// Una línea por movimiento con todos los datos a la vista — sin panel,
    /// sin separación entre "principal" y "detalle". Es la vista de
    /// planilla para quien la prefiera a la agrupada.
    fn render_classic_table(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let indices = self.filtered_sorted_indices();
        self.selected = self.selected.min(indices.len().saturating_sub(1));

        if indices.is_empty() {
            frame.render_widget(
                Paragraph::new("Sin movimientos con esta búsqueda")
                    .style(theme.warning())
                    .alignment(Alignment::Center),
                Rect::new(area.x, area.y + area.height / 2, area.width, 1),
            );
            return;
        }

        let capacity = area.height.saturating_sub(2) as usize;
        let start = self.selected.saturating_sub(capacity.saturating_sub(1));
        let visible_columns: Vec<ClassicColumn> = self
            .classic_columns
            .iter()
            .filter(|(_, visible)| *visible)
            .map(|(column, _)| *column)
            .collect();
        let rows = indices.iter().skip(start).take(capacity).map(|&index| {
            let record = &self.records[index];
            Row::new(visible_columns.iter().map(|column| classic_cell(record, *column, theme)))
                .style(theme.base())
        });
        let headers: Vec<&str> = visible_columns.iter().map(|column| column.label()).collect();
        let widths: Vec<Constraint> = visible_columns.iter().map(|column| column.constraint()).collect();
        let header = Row::new(headers)
            .style(theme.muted().add_modifier(Modifier::BOLD))
            .bottom_margin(1);
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
    }

    /// Grilla semanal estilo "contribuciones de GitHub": responde "¿cuándo
    /// hubo más actividad?", algo que ni la Línea de tiempo ni la Clásica
    /// contestan de un vistazo porque ambas están pensadas para navegar
    /// registros uno por uno, no para ver el patrón completo.
    fn render_heatmap(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let counts = self.day_counts();
        if counts.is_empty() {
            frame.render_widget(
                Paragraph::new("Sin movimientos con esta búsqueda")
                    .style(theme.warning())
                    .alignment(Alignment::Center),
                Rect::new(area.x, area.y + area.height / 2, area.width, 1),
            );
            return;
        }

        let max_count = *counts.values().max().unwrap_or(&1);
        let min_date = *counts.keys().min().expect("counts no está vacío");
        let max_date = *counts.keys().max().expect("counts no está vacío");
        self.heatmap_selected = self.heatmap_selected.clamp(min_date, max_date);

        let mut lines: Vec<Line<'static>> = vec![
            Line::from("            L  M  M  J  V  S  D").style(theme.muted().add_modifier(Modifier::BOLD)),
        ];
        let mut week_start = start_of_week(min_date);
        let grid_end = end_of_week(max_date);
        while week_start <= grid_end {
            let mut spans = vec![Span::styled(
                format!("{}  ", week_start.format("%d/%m")),
                theme.muted(),
            )];
            for offset in 0..7 {
                let day = week_start + Duration::days(offset);
                let in_range = day >= min_date && day <= max_date;
                let count = counts.get(&day).copied().unwrap_or(0);
                let selected = day == self.heatmap_selected;
                let glyph = bucket_glyph(count, max_count);
                let style = if !in_range {
                    theme.muted()
                } else if selected {
                    theme.selected()
                } else {
                    bucket_style(count, max_count, theme)
                };
                let text = if selected { format!("[{glyph}]") } else { format!(" {glyph} ") };
                spans.push(Span::styled(text, style));
            }
            lines.push(Line::from(spans));
            week_start += Duration::days(7);
        }

        lines.push(Line::from(""));
        let selected_count = counts.get(&self.heatmap_selected).copied().unwrap_or(0);
        lines.push(
            Line::from(format!(
                "{} · {selected_count} movimientos · ENTER ver en Línea de tiempo",
                self.heatmap_selected.format("%d/%m/%Y")
            ))
            .style(theme.accent()),
        );
        lines.push(Line::from("█ mucho   ▓ ▒ ░ menos   · sin datos").style(theme.muted()));

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn facet_line(&self, theme: Theme) -> Line<'static> {
        if let Some(unrecognized) = &self.parsed_query().unrecognized_date {
            return Line::from(format!(
                "\"{unrecognized}\" no se reconoce como fecha · use DD/MM/AAAA"
            ))
            .style(theme.warning());
        }
        Line::from(format!("{} resultados", self.filtered_sorted_indices().len())).style(theme.muted())
    }

    fn render_timeline(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let sections = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
        frame.render_widget(timeline_header_line(theme), sections[0]);
        let scroll_area = sections[1];

        let indices = self.filtered_sorted_indices();
        self.selected = self.selected.min(indices.len().saturating_sub(1));

        if indices.is_empty() {
            frame.render_widget(
                Paragraph::new("Sin movimientos con esta búsqueda")
                    .style(theme.warning())
                    .alignment(Alignment::Center),
                Rect::new(scroll_area.x, scroll_area.y + scroll_area.height / 2, scroll_area.width, 1),
            );
            return;
        }

        let mut rows: Vec<Line<'static>> = Vec::with_capacity(indices.len() * 2);
        let mut selected_row = 0usize;
        let mut last_date: Option<&str> = None;
        for (entry_index, &record_index) in indices.iter().enumerate() {
            let record = &self.records[record_index];
            if last_date != Some(record.date.as_str()) {
                let date = parse_date(&record.date).unwrap_or(self.today);
                let count = indices
                    .iter()
                    .filter(|&&other| self.records[other].date == record.date)
                    .count();
                rows.push(
                    Line::from(format!("{} · {count} movimientos", day_header(date)))
                        .style(theme.muted().add_modifier(Modifier::BOLD)),
                );
                last_date = Some(&record.date);
            }
            if entry_index == self.selected {
                selected_row = rows.len();
            }
            rows.push(timeline_entry_line(record, entry_index == self.selected, theme));
        }

        let capacity = scroll_area.height as usize;
        if selected_row < self.timeline_offset {
            self.timeline_offset = selected_row;
        } else if selected_row >= self.timeline_offset + capacity {
            self.timeline_offset = selected_row + 1 - capacity.max(1);
        }
        self.timeline_offset = self.timeline_offset.min(rows.len().saturating_sub(capacity));

        let visible: Vec<Line<'static>> = rows
            .into_iter()
            .skip(self.timeline_offset)
            .take(capacity)
            .collect();
        frame.render_widget(Paragraph::new(visible), scroll_area);
    }

    fn render_columns_editor(&self, frame: &mut Frame, area: Rect, focus: usize, theme: Theme) {
        let top_slice = Rect::new(area.x, area.y, area.width, area.height.min(20));
        let popup = centered_rect(
            top_slice,
            44.min(area.width),
            (ClassicColumn::ALL.len() as u16 + 4).min(top_slice.height),
        );
        frame.render_widget(Clear, popup);
        let block = auxiliary_panel("COLUMNAS VISIBLES", theme, true);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        if inner.height == 0 {
            return;
        }

        let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
        let lines: Vec<Line<'_>> = self
            .classic_columns
            .iter()
            .enumerate()
            .map(|(index, (column, visible))| {
                let selected = index == focus;
                let marker = if selected { ">" } else { " " };
                let checkbox = if *visible { "[x]" } else { "[ ]" };
                Line::from(format!("{marker} {checkbox} {}", column.label())).style(if selected {
                    theme.selected()
                } else {
                    theme.base()
                })
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), rows[0]);
        frame.render_widget(
            Line::from("↑↓ mover · ESPACIO mostrar/ocultar · F4/ESC cerrar").style(theme.muted()),
            rows[1],
        );
    }

    fn render_panel(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let Some(record) = self.selected_record() else {
            frame.render_widget(
                Paragraph::new("No hay un registro seleccionado").style(theme.muted()),
                area,
            );
            return;
        };
        let badge_text = record
            .badge
            .map_or_else(|| "Sin gafete".to_owned(), |number| format!("Gafete {number}"));
        let mut lines = vec![
            Line::from(record.name.as_str()).style(theme.title()),
            Line::from(format!("{} · {}", record.identity, record.company)).style(theme.base()),
            Line::from(format!("{} · {}", record.date, record.entry_type)).style(theme.muted()),
            Line::from(""),
            Line::from(format!("Entrada {}", record.entry_time)).style(theme.base()),
        ];
        match (&record.exit_time, duration_text(record)) {
            (Some(exit), Some(duration)) => {
                lines.push(Line::from(format!("Salida {exit} · {duration} dentro")).style(theme.base()));
            }
            (Some(exit), None) => lines.push(Line::from(format!("Salida {exit}")).style(theme.base())),
            (None, _) => lines.push(Line::from("Activo · aún dentro").style(theme.warning())),
        }
        lines.push(Line::from(record.medium.as_str()).style(theme.muted()));
        lines.push(Line::from(badge_text).style(theme.muted()));
        lines.push(Line::from(""));
        // Jerarquía de tres niveles: título (negrita+acento) para el nombre,
        // base (sin negrita, color pleno) para la sustancia del registro
        // —incluida la trazabilidad de quién operó—, apagado para detalles
        // de contexto (fecha/tipo, medio, gafete). Nada compite con el
        // título; "quién registró" ya no se confunde con "quién es".
        lines.push(Line::from(format!("Ingreso registrado por {}", record.entry_operator)).style(theme.base()));
        if let Some(exit_operator) = &record.exit_operator {
            lines.push(Line::from(format!("Salida registrada por {exit_operator}")).style(theme.base()));
        }
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn status_line(&self, theme: Theme) -> Line<'static> {
        if let Some(message) = &self.prepared_message {
            return Line::from(message.clone()).style(theme.success());
        }
        Line::from("").style(theme.muted())
    }

    fn hint_lines(&self, theme: Theme) -> Vec<Line<'static>> {
        let mut primary = if self.view == ViewMode::Heatmap {
            vec![
                Span::styled("↑↓", theme.accent()),
                Span::styled(" semana   ", theme.base()),
                Span::styled("TAB", theme.accent()),
                Span::styled(" día   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" ver ese día   ", theme.base()),
            ]
        } else {
            vec![
                Span::styled("↑↓", theme.accent()),
                Span::styled(" mover   ", theme.base()),
                Span::styled("PGUP/PGDN", theme.accent()),
                Span::styled(" saltar   ", theme.base()),
            ]
        };
        primary.push(Span::styled("F3", theme.accent()));
        primary.push(Span::styled(format!(" vista ({})   ", self.view.next().label()), theme.base()));
        if self.view == ViewMode::Classic {
            primary.push(Span::styled("F4", theme.accent()));
            primary.push(Span::styled(" columnas   ", theme.base()));
        }
        primary.push(Span::styled(QUICK_EXIT_HINT.key, theme.accent()));
        primary.push(Span::styled(format!(" {}   ", QUICK_EXIT_HINT.label), theme.base()));
        primary.push(Span::styled("F1", theme.accent()));
        primary.push(Span::styled(
            if self.help_expanded { " cerrar ayuda" } else { " más" },
            theme.base(),
        ));
        let mut lines = vec![Line::from(primary)];
        if self.help_expanded {
            lines.push(Line::from(vec![
                Span::styled("HOME/END", theme.accent()),
                Span::styled(" extremos   ", theme.base()),
                Span::styled("F7", theme.accent()),
                Span::styled(" tema   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" limpiar/salir", theme.base()),
            ]));
            lines.push(Line::from(QUERY_KEYS_HINT).style(theme.muted()));
        }
        lines
    }
}

fn classic_cell(record: &MovementRecord, column: ClassicColumn, theme: Theme) -> Cell<'_> {
    match column {
        ClassicColumn::Fecha => Cell::from(record.date.as_str()),
        ClassicColumn::Cedula => Cell::from(record.identity.as_str()),
        ClassicColumn::Nombre => Cell::from(record.name.as_str()),
        ClassicColumn::Empresa => Cell::from(record.company.as_str()),
        ClassicColumn::Tipo => Cell::from(record.entry_type.as_str()),
        ClassicColumn::Entrada => Cell::from(record.entry_time.as_str()),
        ClassicColumn::Salida => {
            let salida = record.exit_time.as_deref().unwrap_or("Activo");
            let style = if record.exit_time.is_some() { theme.base() } else { theme.warning() };
            Cell::from(salida).style(style)
        }
        ClassicColumn::Gafete => {
            Cell::from(record.badge.map_or_else(|| "S/G".to_owned(), |number| number.to_string()))
        }
        ClassicColumn::Medio => Cell::from(record.medium.as_str()),
        ClassicColumn::Ingreso => Cell::from(record.entry_operator.as_str()),
        ClassicColumn::Egreso => Cell::from(record.exit_operator.as_deref().unwrap_or("—")),
    }
}

fn timeline_entry_line(record: &MovementRecord, selected: bool, theme: Theme) -> Line<'static> {
    let active = record.exit_time.is_none();
    let status_glyph = if active { "●" } else { "○" };
    let status_style = if active { theme.warning() } else { theme.muted() };
    let salida = record.exit_time.as_deref().unwrap_or("ahora");
    let marker = if selected { ">" } else { " " };
    let row_style = if selected { theme.selected() } else { theme.base() };
    Line::from(vec![
        Span::styled(format!("{marker} "), row_style),
        Span::styled(format!("{status_glyph} "), if selected { row_style } else { status_style }),
        Span::styled(format!("{:<5} → {:<5}  ", record.entry_time, salida), row_style),
        Span::styled(format!("{:<20.20} ", record.name), row_style),
        Span::styled(format!("{:<18.18} ", record.company), row_style),
        Span::styled(record.entry_type.clone(), row_style),
    ])
}

/// Encabezado fijo arriba del timeline — mismas columnas y anchos que
/// `timeline_entry_line`, para que se lean como una tabla sin serlo.
/// Etiqueta estructural, no contenido: negrita para que se distinga como
/// encabezado, pero en el color apagado para no competir con las filas.
fn timeline_header_line(theme: Theme) -> Line<'static> {
    Line::from(format!(
        "    {:<5} → {:<5}  {:<20} {:<18} {}",
        "ENTRA", "SALE", "NOMBRE", "EMPRESA", "TIPO"
    ))
    .style(theme.muted().add_modifier(Modifier::BOLD))
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
    theme: Theme,
}

/// El campo de búsqueda está siempre enfocado: no hay otro modo que le
/// dispute el teclado, así que se dibuja siempre en estado activo.
fn render_field(frame: &mut Frame, area: Rect, spec: FieldSpec<'_>) {
    let FieldSpec { label, input, theme } = spec;
    let value_y = area.y.saturating_add(1);
    let line_y = area.y.saturating_add(2);

    frame.render_widget(
        Paragraph::new(label).style(theme.accent()),
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
        Paragraph::new("─".repeat(area.width as usize)).style(theme.accent()),
        Rect::new(area.x, line_y, area.width, 1),
    );
    let column = visual_cursor.saturating_sub(scroll).min(viewport_width);
    frame.set_cursor_position((area.x + column as u16, value_y));
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

type DemoRow = (
    i64,
    i64,
    Option<i64>,
    &'static str,
    &'static str,
    usize,
    usize,
    &'static str,
    Option<&'static str>,
    &'static str,
    &'static str,
    Option<&'static str>,
);

fn demo_records(today: NaiveDate) -> Vec<MovementRecord> {
    let day = |offset: i64| {
        today
            .checked_sub_signed(Duration::days(offset))
            .unwrap_or(today)
            .format("%d/%m/%Y")
            .to_string()
    };
    let raw: Vec<DemoRow> = vec![
        (0, 1, Some(12), "1-1042-0881", "José Peña", 0, 0, "07:42", None, "En vehículo", "Marta Solano", None),
        (0, 2, Some(25), "3-0520-0917", "Juan Rodríguez", 2, 0, "08:10", None, "Caminando", "Marta Solano", None),
        (0, 3, None, "2-0731-0440", "Ana María Solís", 1, 1, "08:55", Some("15:30"), "Caminando", CURRENT_OPERATOR, Some(CURRENT_OPERATOR)),
        (1, 4, Some(48), "1-1550-0239", "Carlos Méndez", 0, 3, "09:20", Some("17:05"), "En vehículo", CURRENT_OPERATOR, Some("Marta Solano")),
        (1, 5, Some(9), "2-0611-0854", "Sofía Núñez", 3, 0, "06:58", Some("15:30"), "Caminando", "Marta Solano", Some("Marta Solano")),
        (1, 6, None, "3-0488-0312", "Edgar Chacón", 1, 3, "10:15", Some("18:40"), "Caminando", CURRENT_OPERATOR, Some(CURRENT_OPERATOR)),
        (2, 7, Some(31), "1-1270-0641", "María Fernanda Rojas", 1, 2, "07:30", Some("16:00"), "En vehículo", "Marta Solano", Some("Marta Solano")),
        (2, 8, Some(14), "4-0221-0178", "Luis Ángel Mora", 3, 0, "08:05", Some("17:45"), "En vehículo", CURRENT_OPERATOR, Some(CURRENT_OPERATOR)),
        (3, 9, None, "2-0810-0305", "Valeria Jiménez", 4, 1, "09:00", Some("14:20"), "Caminando", "Marta Solano", Some(CURRENT_OPERATOR)),
        (3, 10, Some(17), "1-0991-0518", "Óscar Zamora", 0, 3, "06:40", Some("19:10"), "En vehículo", CURRENT_OPERATOR, Some("Marta Solano")),
        (4, 11, Some(22), "3-0633-0720", "Gabriela Araya", 2, 0, "07:55", Some("16:30"), "Caminando", "Marta Solano", Some(CURRENT_OPERATOR)),
        (5, 12, None, "4-0198-0772", "Mónica Quesada", 3, 2, "08:20", Some("15:50"), "Caminando", CURRENT_OPERATOR, Some("Marta Solano")),
        (5, 13, Some(6), "2-0731-0441", "Diego Salas", 1, 0, "09:10", Some("18:00"), "En vehículo", "Marta Solano", Some(CURRENT_OPERATOR)),
        (6, 14, Some(41), "1-1042-0882", "Pamela Herrera", 0, 1, "07:05", Some("17:20"), "En vehículo", CURRENT_OPERATOR, Some("Marta Solano")),
    ];
    raw.into_iter()
        .map(
            |(
                offset,
                id,
                badge,
                identity,
                name,
                company_index,
                entry_type_index,
                entry_time,
                exit_time,
                medium,
                entry_operator,
                exit_operator,
            )| MovementRecord {
                id,
                date: day(offset),
                identity: identity.into(),
                name: name.into(),
                company: COMPANIES[company_index].into(),
                entry_type: ENTRY_TYPES[entry_type_index].into(),
                entry_time: entry_time.into(),
                exit_time: exit_time.map(Into::into),
                badge,
                medium: medium.into(),
                entry_operator: entry_operator.into(),
                exit_operator: exit_operator.map(Into::into),
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{HistorialV2Pilot, ThemePreset};

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
    fn sin_busqueda_muestra_todo_agrupado_por_dia() {
        let app = HistorialV2Pilot::default();
        assert_eq!(app.filtered_sorted_indices().len(), app.records.len());
    }

    #[test]
    fn el_encabezado_del_dia_es_solo_la_fecha_sin_hoy_ni_dia_de_semana() {
        let app = HistorialV2Pilot::default();
        let encabezado = super::day_header(app.today);

        assert_eq!(encabezado, app.today.format("%d/%m/%Y").to_string());
        assert!(!encabezado.contains("HOY"));
        assert!(!encabezado.contains("AYER"));
    }

    #[test]
    fn busca_por_clave_empresa_y_filtra_en_vivo_sin_aplicar() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("empresa:Expenic".into()));

        let indices = app.filtered_sorted_indices();
        assert!(!indices.is_empty());
        assert!(indices.iter().all(|&i| app.records[i].company == "Expenic Industrial"));
    }

    #[test]
    fn empresa_con_espacios_entre_comillas_funciona_como_un_solo_token() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("empresa:\"Brisas del Oeste\" estado:activos".into()));

        let indices = app.filtered_sorted_indices();
        assert!(indices.iter().all(|&i| {
            app.records[i].company == "Brisas del Oeste" && app.records[i].exit_time.is_none()
        }));
    }

    #[test]
    fn estado_activos_solo_deja_los_que_siguen_dentro() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("estado:activos".into()));

        let indices = app.filtered_sorted_indices();
        assert!(!indices.is_empty());
        assert!(indices.iter().all(|&i| app.records[i].exit_time.is_none()));
    }

    #[test]
    fn texto_libre_busca_por_nombre() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("Sofía".into()));

        assert_eq!(app.filtered_sorted_indices().len(), 1);
    }

    #[test]
    fn clave_ingreso_filtra_por_quien_registro_la_entrada() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("ingreso:Marta".into()));

        let indices = app.filtered_sorted_indices();
        assert!(!indices.is_empty());
        assert!(indices.iter().all(|&i| app.records[i].entry_operator == "Marta Solano"));
    }

    #[test]
    fn clave_salida_filtra_por_quien_registro_la_salida_y_excluye_los_que_siguen_dentro() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("salida:Marta".into()));

        let indices = app.filtered_sorted_indices();
        assert!(!indices.is_empty());
        assert!(
            indices
                .iter()
                .all(|&i| app.records[i].exit_operator.as_deref() == Some("Marta Solano"))
        );
    }

    #[test]
    fn una_fecha_no_reconocida_no_bloquea_el_resto_de_la_busqueda() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("desde:no-es-una-fecha empresa:Expenic".into()));

        let indices = app.filtered_sorted_indices();
        assert!(!indices.is_empty());
        assert!(indices.iter().all(|&i| app.records[i].company == "Expenic Industrial"));
        assert_eq!(
            app.parsed_query().unrecognized_date.as_deref(),
            Some("desde:no-es-una-fecha")
        );
    }

    #[test]
    fn el_panel_muestra_duracion_y_ambos_operadores() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("María Fernanda".into()));
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("dentro"));
        assert!(rendered.contains("Ingreso registrado por Marta Solano"));
        assert!(rendered.contains("Salida registrada por Marta Solano"));
    }

    #[test]
    fn el_timeline_muestra_encabezados_de_columna_fijos() {
        let mut app = HistorialV2Pilot::default();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("ENTRA"));
        assert!(rendered.contains("SALE"));
        assert!(rendered.contains("NOMBRE"));
        assert!(rendered.contains("EMPRESA"));
        assert!(rendered.contains("TIPO"));
    }

    #[test]
    fn f2_funciona_mientras_se_esta_escribiendo_una_busqueda() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("empresa:Expenic".into()));

        app.handle_event(&key(KeyCode::F(2)));
        assert!(app.quick_exit.is_open());
        app.handle_event(&Event::Paste("25".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(!app.quick_exit.is_open());
        assert_eq!(app.query.value(), "empresa:Expenic");
    }

    #[test]
    fn f3_cambia_de_vista_sin_perder_la_seleccion_ni_la_busqueda() {
        use super::ViewMode;

        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("empresa:Expenic".into()));
        let seleccion_previa = app.selected;

        app.handle_event(&key(KeyCode::F(3)));

        assert_eq!(app.view, ViewMode::Classic);
        assert_eq!(app.query.value(), "empresa:Expenic");
        assert_eq!(app.selected, seleccion_previa);
    }

    #[test]
    fn f3_completa_el_ciclo_de_tres_vistas() {
        use super::ViewMode;

        let mut app = HistorialV2Pilot::default();
        assert_eq!(app.view, ViewMode::Timeline);
        app.handle_event(&key(KeyCode::F(3)));
        assert_eq!(app.view, ViewMode::Classic);
        app.handle_event(&key(KeyCode::F(3)));
        assert_eq!(app.view, ViewMode::Heatmap);
        app.handle_event(&key(KeyCode::F(3)));
        assert_eq!(app.view, ViewMode::Timeline);
    }

    #[test]
    fn el_mapa_de_calor_cuenta_movimientos_por_dia() {
        let app = HistorialV2Pilot::default();
        let counts = app.day_counts();

        assert_eq!(counts.values().sum::<usize>(), app.records.len());
        assert_eq!(counts.get(&app.today), Some(&3));
    }

    #[test]
    fn tab_mueve_un_dia_y_shift_tab_retrocede_en_el_mapa_de_calor() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&key(KeyCode::F(3)));
        app.handle_event(&key(KeyCode::F(3)));
        assert_eq!(app.view, super::ViewMode::Heatmap);
        let dia_inicial = app.heatmap_selected;

        app.handle_event(&key(KeyCode::Tab));
        assert_eq!(app.heatmap_selected, dia_inicial + chrono::Duration::days(1));

        app.handle_event(&key(KeyCode::BackTab));
        app.handle_event(&key(KeyCode::BackTab));
        assert_eq!(app.heatmap_selected, dia_inicial - chrono::Duration::days(1));
    }

    #[test]
    fn enter_en_el_mapa_de_calor_salta_a_la_linea_de_tiempo_filtrada_a_ese_dia() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&key(KeyCode::F(3)));
        app.handle_event(&key(KeyCode::F(3)));
        app.heatmap_selected = app.today - chrono::Duration::days(1);

        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.view, super::ViewMode::Timeline);
        let esperado = (app.today - chrono::Duration::days(1)).format("%d/%m/%Y").to_string();
        assert_eq!(app.query.value(), format!("desde:{esperado} hasta:{esperado}"));
        // Ese día tuvo 3 movimientos en los datos de demo.
        assert_eq!(app.filtered_sorted_indices().len(), 3);
    }

    #[test]
    fn el_mapa_de_calor_se_renderiza_sin_panel_como_las_otras_vistas_sin_navegacion_de_registros() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&key(KeyCode::F(3)));
        app.handle_event(&key(KeyCode::F(3)));

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("MAPA DE CALOR"));
        assert!(rendered.contains("ver ese día"));
    }

    #[test]
    fn la_letra_v_se_escribe_en_la_busqueda_en_vez_de_cambiar_de_vista() {
        use super::ViewMode;

        let mut app = HistorialV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('v')));

        assert_eq!(app.query.value(), "v");
        assert_eq!(app.view, ViewMode::Timeline);
    }

    #[test]
    fn la_vista_clasica_muestra_todo_en_una_sola_linea_sin_panel() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&key(KeyCode::F(3)));
        app.handle_event(&Event::Paste("María Fernanda".into()));

        let backend = TestBackend::new(150, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("María Fernanda Rojas"));
        assert!(rendered.contains("Marta Solano"));
        assert!(rendered.contains("CLÁSICA"));
        // No hay panel separado: no aparece la frase que sólo usa el panel.
        assert!(!rendered.contains("Ingreso registrado por"));
    }

    #[test]
    fn f4_solo_abre_el_editor_de_columnas_en_vista_clasica() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&key(KeyCode::F(4)));
        assert!(app.columns_editor.is_none(), "en Línea de tiempo, F4 no debe hacer nada");

        app.handle_event(&key(KeyCode::F(3)));
        app.handle_event(&key(KeyCode::F(4)));
        assert_eq!(app.columns_editor, Some(0));
    }

    #[test]
    fn ocultar_una_columna_la_saca_de_la_tabla_clasica() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&key(KeyCode::F(3))); // vista clásica
        app.handle_event(&key(KeyCode::F(4))); // editor de columnas
        for _ in 0..8 {
            app.handle_event(&key(KeyCode::Down)); // llega a "MEDIO"
        }
        app.handle_event(&key(KeyCode::Char(' '))); // oculta MEDIO
        app.handle_event(&key(KeyCode::Esc));

        assert!(!app.columns_editor.is_some());
        let backend = TestBackend::new(150, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(!rendered.contains("MEDIO"));
        assert!(rendered.contains("NOMBRE"));
    }

    #[test]
    fn no_se_puede_ocultar_la_ultima_columna_visible() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&key(KeyCode::F(3)));
        app.handle_event(&key(KeyCode::F(4)));
        for (column, visible) in app.classic_columns.iter_mut().skip(1) {
            let _ = column;
            *visible = false;
        }
        app.handle_event(&key(KeyCode::Char(' '))); // intenta ocultar la única visible

        assert!(app.classic_columns.iter().any(|(_, visible)| *visible));
    }

    #[test]
    fn escribir_mientras_el_editor_de_columnas_esta_abierto_no_toca_la_busqueda() {
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Paste("empresa:Expenic".into()));
        app.handle_event(&key(KeyCode::F(3)));
        app.handle_event(&key(KeyCode::F(4)));
        app.handle_event(&Event::Paste("esto no debe escribirse".into()));

        assert_eq!(app.query.value(), "empresa:Expenic");
    }

    #[test]
    fn pinta_todo_el_lienzo_sin_dejar_parches_sin_color() {
        let mut app = HistorialV2Pilot::default();
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
        let mut app = HistorialV2Pilot::default();
        app.handle_event(&Event::Resize(40, 10));
        app.handle_event(&Event::Paste("empresa:Expenic".into()));

        assert!(app.query.value().is_empty());
    }
}
