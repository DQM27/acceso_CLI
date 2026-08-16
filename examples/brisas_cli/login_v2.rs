//! Mockup de la propuesta de fluidez: menos cromo, sin modales para el flujo
//! principal y feedback asíncrono honesto (spinner con umbral de aparición).
//!
//! Compárese con `login` (el piloto actual) ejecutando:
//!   cargo run --example brisas_cli -- login
//!   cargo run --example brisas_cli -- login-v2

use std::time::{Duration, Instant};

use chrono::Utc;
use chrono_tz::America::Costa_Rica;
use control_acceso::tui::ui_kit::{
    StandardCommand, Theme, ThemePreset, render_terminal_too_small, standard_command,
};
use crossterm::event::{Event, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 18;
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);
const SPINNER_FRAME_INTERVAL: Duration = Duration::from_millis(90);
const SPINNER_APPEAR_THRESHOLD: Duration = Duration::from_millis(150);
const VALIDATION_DELAY: Duration = Duration::from_millis(650);
const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Identity,
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Idle,
    Validating { started: Instant },
    Error(String),
    Success,
}

#[derive(Debug)]
pub struct LoginV2Pilot {
    identity: Input,
    password: Input,
    field: Field,
    status: Status,
    help_expanded: bool,
    theme: ThemePreset,
    running: bool,
    terminal_size: (u16, u16),
    cursor_started: Instant,
}

impl Default for LoginV2Pilot {
    fn default() -> Self {
        Self {
            identity: Input::default(),
            password: Input::default(),
            field: Field::Identity,
            status: Status::Idle,
            help_expanded: false,
            theme: ThemePreset::Classic,
            running: true,
            terminal_size: (u16::MAX, u16::MAX),
            cursor_started: Instant::now(),
        }
    }
}

impl LoginV2Pilot {
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn redraw_interval(&self) -> Duration {
        if matches!(self.status, Status::Validating { .. }) {
            SPINNER_FRAME_INTERVAL
        } else {
            Duration::from_millis(250)
        }
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
            self.password.reset();
            self.running = false;
            return true;
        }
        if !self.viewport_is_valid() {
            if key.is_some_and(|key| {
                key.kind == KeyEventKind::Press
                    && standard_command(key) == Some(StandardCommand::Cancel)
            }) {
                self.password.reset();
                self.running = false;
                return true;
            }
            return false;
        }
        if matches!(self.status, Status::Validating { .. }) {
            return false;
        }

        let Some(key) = key else {
            return self.handle_text_input(event);
        };

        if key.kind == KeyEventKind::Press {
            match standard_command(key) {
                Some(StandardCommand::FocusNext) => return self.change_field(false),
                Some(StandardCommand::FocusPrevious) => return self.change_field(true),
                Some(StandardCommand::Primary) => return self.continue_form(),
                Some(StandardCommand::Cancel) => {
                    self.password.reset();
                    self.running = false;
                    return true;
                }
                Some(StandardCommand::Help) => {
                    self.help_expanded = !self.help_expanded;
                    return true;
                }
                Some(StandardCommand::Theme) => {
                    self.toggle_theme();
                    return true;
                }
                _ => {}
            }
        }
        self.handle_text_input(event)
    }

    fn handle_text_input(&mut self, event: &Event) -> bool {
        let changed = match self.field {
            Field::Identity => apply_event(&mut self.identity, event),
            Field::Password => apply_event(&mut self.password, event),
        };
        if changed {
            self.status = Status::Idle;
            self.restart_cursor();
        }
        changed
    }

    fn change_field(&mut self, backwards: bool) -> bool {
        self.field = match (self.field, backwards) {
            (Field::Identity, false) | (Field::Password, true) => Field::Password,
            (Field::Password, false) | (Field::Identity, true) => Field::Identity,
        };
        self.restart_cursor();
        true
    }

    fn continue_form(&mut self) -> bool {
        if self.field == Field::Identity {
            self.field = Field::Password;
            self.restart_cursor();
            return true;
        }
        if self.identity.value().trim().is_empty() {
            self.field = Field::Identity;
            self.status = Status::Error("La cédula es obligatoria.".into());
            self.restart_cursor();
            return true;
        }
        if self.password.value().is_empty() {
            self.status = Status::Error("La contraseña es obligatoria.".into());
            self.restart_cursor();
            return true;
        }
        self.status = Status::Validating {
            started: Instant::now(),
        };
        true
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
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

    fn advance_validation_at(&mut self, now: Instant) {
        let Status::Validating { started } = self.status else {
            return;
        };
        if now.saturating_duration_since(started) < VALIDATION_DELAY {
            return;
        }
        let failed = self.password.value().contains("error");
        self.password.reset();
        self.status = if failed {
            Status::Error("Credenciales simuladas inválidas.".into())
        } else {
            Status::Success
        };
    }

    fn viewport_is_valid(&self) -> bool {
        self.terminal_size.0 >= MIN_WIDTH && self.terminal_size.1 >= MIN_HEIGHT
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.terminal_size = (area.width, area.height);
        self.advance_validation_at(Instant::now());
        let theme = self.theme.theme();

        if !self.viewport_is_valid() {
            render_terminal_too_small(frame, area, MIN_WIDTH, MIN_HEIGHT, "ESC salir", theme);
            return;
        }

        // Pinta todo el lienzo primero: si no, sólo quedan coloreadas las
        // franjas exactas de cada Paragraph y el resto se ve con el negro
        // por defecto de la terminal, dando esos parches grisáceos.
        frame.render_widget(Block::default().style(theme.base()), area);

        let hint_lines = hint_lines(self.help_expanded, theme);
        let hint_height = hint_lines.len() as u16;
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(1),
            Constraint::Length(hint_height),
        ])
        .split(area.inner(ratatui::layout::Margin {
            horizontal: 2,
            vertical: 1,
        }));

        self.render_top_bar(frame, rows[0], theme);
        self.render_form(frame, rows[1], theme);
        frame.render_widget(
            status_line(theme, &self.status, Instant::now()),
            rows[2],
        );
        frame.render_widget(Paragraph::new(hint_lines), rows[3]);
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
            Paragraph::new("brisas cli · inicio de sesión (v2)").style(theme.muted()),
            columns[0],
        );
        frame.render_widget(
            Paragraph::new(clock)
                .style(theme.muted())
                .alignment(Alignment::Right),
            columns[1],
        );
    }

    fn render_form(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        // El bloque completo (marca + formulario) se centra como una sola
        // composición para que el espacio sobrante no se sienta vacío.
        let hero = centered(area, 46, 10);
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(hero);

        frame.render_widget(
            Paragraph::new("B R I S A S   C L I")
                .style(theme.title())
                .alignment(Alignment::Center),
            rows[0],
        );
        frame.render_widget(
            Paragraph::new("control de acceso")
                .style(theme.muted())
                .alignment(Alignment::Center),
            rows[1],
        );

        let cursor_visible =
            !matches!(self.status, Status::Validating { .. }) && self.cursor_visible_at(Instant::now());

        render_field(
            frame,
            rows[3],
            FieldSpec {
                label: "CÉDULA",
                input: &self.identity,
                focused: self.field == Field::Identity,
                cursor_visible,
                masked: false,
                theme,
            },
        );
        render_field(
            frame,
            rows[5],
            FieldSpec {
                label: "CONTRASEÑA",
                input: &self.password,
                focused: self.field == Field::Password,
                cursor_visible,
                masked: true,
                theme,
            },
        );
    }
}

/// `tui_input::Input::handle_event` sólo procesa eventos de teclado; el pegado
/// llega como `Event::Paste` y hay que insertarlo carácter a carácter.
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
    cursor_visible: bool,
    masked: bool,
    theme: Theme,
}

/// Misma silueta enfocado o no (etiqueta, valor, línea) — sólo cambian color
/// y peso. Cambiar de campo nunca reordena ni redimensiona nada en pantalla.
fn render_field(frame: &mut Frame, area: Rect, spec: FieldSpec<'_>) {
    let FieldSpec {
        label,
        input,
        focused,
        cursor_visible,
        masked,
        theme,
    } = spec;
    let displayed = if masked {
        "•".repeat(input.value().chars().count())
    } else {
        input.value().to_owned()
    };
    let visual_cursor = if masked {
        input.value().chars().count()
    } else {
        input.visual_cursor()
    };

    let label_style = if focused { theme.accent() } else { theme.muted() };
    let line_style = if focused { theme.accent() } else { theme.border() };
    let value_y = area.y.saturating_add(1);
    let line_y = area.y.saturating_add(2);

    frame.render_widget(
        Paragraph::new(label).style(label_style),
        Rect::new(area.x, area.y, area.width, 1),
    );

    let viewport_width = area.width.saturating_sub(1) as usize;
    let scroll = visual_cursor.saturating_sub(viewport_width);
    frame.render_widget(
        Paragraph::new(Line::from(displayed).style(theme.base())).scroll((0, scroll as u16)),
        Rect::new(area.x, value_y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(line_style),
        Rect::new(area.x, line_y, area.width, 1),
    );

    if focused && cursor_visible {
        let column = visual_cursor.saturating_sub(scroll).min(viewport_width);
        frame.set_cursor_position((area.x + column as u16, value_y));
    }
}

fn status_line(theme: Theme, status: &Status, now: Instant) -> Line<'static> {
    match status {
        Status::Idle => Line::from(""),
        Status::Validating { started } => {
            let elapsed = now.saturating_duration_since(*started);
            if elapsed < SPINNER_APPEAR_THRESHOLD {
                Line::from("")
            } else {
                let frame_index =
                    (elapsed.as_millis() / SPINNER_FRAME_INTERVAL.as_millis()) as usize;
                Line::from(format!(
                    "{} Verificando credenciales…",
                    SPINNER[frame_index % SPINNER.len()]
                ))
                .style(theme.warning())
            }
        }
        Status::Error(message) => Line::from(format!("✕ {message}")).style(theme.danger()),
        Status::Success => Line::from("✓ Acceso simulado autorizado").style(theme.success()),
    }
}

fn hint_lines(expanded: bool, theme: Theme) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("ENTER", theme.accent()),
        Span::styled(" continuar   ", theme.base()),
        Span::styled("ESC", theme.accent()),
        Span::styled(" salir   ", theme.base()),
        Span::styled("F1", theme.accent()),
        Span::styled(if expanded { " cerrar ayuda" } else { " más" }, theme.base()),
    ])];
    if expanded {
        lines.push(Line::from(vec![
            Span::styled("TAB/SHIFT+TAB", theme.accent()),
            Span::styled(" cambiar de campo   ", theme.base()),
            Span::styled("F7", theme.accent()),
            Span::styled(" cambiar tema   ", theme.base()),
            Span::styled("CTRL+C", theme.accent()),
            Span::styled(" salida de emergencia", theme.base()),
        ]));
    }
    lines
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{Field, LoginV2Pilot, Status};

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
    fn enter_avanza_de_cedula_a_password_y_luego_valida() {
        let mut app = LoginV2Pilot::default();
        app.handle_event(&Event::Paste("1-1234-5678".into()));

        app.handle_event(&key(KeyCode::Enter));
        assert_eq!(app.field, Field::Password);

        app.handle_event(&Event::Paste("secreto".into()));
        app.handle_event(&key(KeyCode::Enter));
        assert!(matches!(app.status, Status::Validating { .. }));
    }

    #[test]
    fn el_spinner_solo_aparece_tras_el_umbral_y_no_antes() {
        let mut app = LoginV2Pilot::default();
        app.handle_event(&Event::Paste("1-1234-5678".into()));
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&Event::Paste("secreto".into()));
        app.handle_event(&key(KeyCode::Enter));

        let Status::Validating { started } = app.status else {
            panic!("debe quedar validando");
        };

        let temprano = super::status_line(
            super::ThemePreset::Classic.theme(),
            &app.status,
            started + Duration::from_millis(50),
        );
        assert_eq!(temprano.to_string(), "");

        let tarde = super::status_line(
            super::ThemePreset::Classic.theme(),
            &app.status,
            started + Duration::from_millis(300),
        );
        assert!(tarde.to_string().contains("Verificando"));
    }

    #[test]
    fn completa_la_validacion_tras_el_retraso_simulado() {
        let mut app = LoginV2Pilot::default();
        app.handle_event(&Event::Paste("1-1234-5678".into()));
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&Event::Paste("secreto".into()));
        app.handle_event(&key(KeyCode::Enter));

        let Status::Validating { started } = app.status else {
            panic!("debe quedar validando");
        };
        app.advance_validation_at(started + Duration::from_millis(700));

        assert!(matches!(app.status, Status::Success));
        assert!(app.password.value().is_empty());
    }

    #[test]
    fn la_ayuda_expande_sin_bloquear_el_campo_activo() {
        let mut app = LoginV2Pilot::default();
        app.handle_event(&key(KeyCode::F(1)));
        assert!(app.help_expanded);

        app.handle_event(&Event::Paste("sigue-escribiendo".into()));
        assert_eq!(app.identity.value(), "sigue-escribiendo");
    }

    #[test]
    fn renderiza_sin_bordes_en_el_campo_sin_foco() {
        let mut app = LoginV2Pilot::default();
        app.handle_event(&Event::Paste("1-1234-5678".into()));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| app.render(frame))
            .expect("debe renderizar");

        let rendered = buffer_text(terminal.backend());
        assert!(rendered.contains("1-1234-5678"));
        assert!(rendered.contains("CONTRASEÑA"));
        assert!(terminal.backend().cursor_visible());
    }

    #[test]
    fn pinta_todo_el_lienzo_sin_dejar_parches_sin_color() {
        let mut app = LoginV2Pilot::default();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| app.render(frame))
            .expect("debe renderizar");

        let theme = super::ThemePreset::Classic.theme();
        let buffer = terminal.backend().buffer();
        for (x, y) in [(0, 0), (79, 0), (0, 23), (79, 23), (40, 12)] {
            assert_eq!(
                buffer[(x, y)].bg,
                theme.background,
                "celda ({x},{y}) quedó con el fondo por defecto de la terminal"
            );
        }
    }

    #[test]
    fn terminal_pequena_no_pierde_estado() {
        let mut app = LoginV2Pilot::default();
        app.handle_event(&Event::Resize(40, 10));
        app.handle_event(&Event::Paste("oculto".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(app.identity.value().is_empty());
        assert_eq!(app.field, Field::Identity);
    }
}
