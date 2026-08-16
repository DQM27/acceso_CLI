//! Mockup de fluidez para la configuración inicial (creación del primer
//! usuario ROOT). Estructuralmente es la más simple de las siete: no hay
//! lista ni panel, sólo un formulario — así que reutiliza el patrón de
//! `login_v2` (composición centrada, spinner con umbral de aparición,
//! sin F2 porque todavía no existe una sesión ni nadie "adentro" que
//! buscar) extendido a cuatro campos con Tab y vuelta.
//!
//! cargo run --example brisas_cli -- configuracion-inicial-v2

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
const MIN_HEIGHT: u16 = 20;
const SPINNER_FRAME_INTERVAL: Duration = Duration::from_millis(90);
const SPINNER_APPEAR_THRESHOLD: Duration = Duration::from_millis(150);
const CREATION_DELAY: Duration = Duration::from_millis(650);
const MIN_PASSWORD_LENGTH: usize = 8;
const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Identity,
    Name,
    Password,
    ConfirmPassword,
}

impl Field {
    const ALL: [Self; 4] = [Self::Identity, Self::Name, Self::Password, Self::ConfirmPassword];
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Idle,
    Creating { started: Instant },
    Error(String),
    Success,
}

#[derive(Debug)]
pub struct ConfiguracionInicialV2Pilot {
    identity: Input,
    name: Input,
    password: Input,
    confirm_password: Input,
    field: usize,
    status: Status,
    help_expanded: bool,
    theme: ThemePreset,
    running: bool,
    terminal_size: (u16, u16),
}

impl Default for ConfiguracionInicialV2Pilot {
    fn default() -> Self {
        Self {
            identity: Input::default(),
            name: Input::default(),
            password: Input::default(),
            confirm_password: Input::default(),
            field: 0,
            status: Status::Idle,
            help_expanded: false,
            theme: ThemePreset::Classic,
            running: true,
            terminal_size: (u16::MAX, u16::MAX),
        }
    }
}

impl ConfiguracionInicialV2Pilot {
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn redraw_interval(&self) -> Duration {
        if matches!(self.status, Status::Creating { .. }) {
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
            self.confirm_password.reset();
            self.running = false;
            return true;
        }
        if !self.viewport_is_valid() {
            if key.is_some_and(|key| {
                key.kind == KeyEventKind::Press && standard_command(key) == Some(StandardCommand::Cancel)
            }) {
                self.password.reset();
                self.confirm_password.reset();
                self.running = false;
                return true;
            }
            return false;
        }
        if matches!(self.status, Status::Creating { .. }) {
            return false;
        }

        let Some(key) = key else {
            return self.handle_text_input(event);
        };

        if key.kind == KeyEventKind::Press {
            match standard_command(key) {
                Some(StandardCommand::FocusNext) => return self.move_field(false),
                Some(StandardCommand::FocusPrevious) => return self.move_field(true),
                Some(StandardCommand::Primary) => return self.continue_form(),
                Some(StandardCommand::Cancel) => {
                    self.password.reset();
                    self.confirm_password.reset();
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
        let target = match Field::ALL[self.field] {
            Field::Identity => &mut self.identity,
            Field::Name => &mut self.name,
            Field::Password => &mut self.password,
            Field::ConfirmPassword => &mut self.confirm_password,
        };
        let changed = apply_event(target, event);
        if changed {
            self.clear_transient_status();
        }
        changed
    }

    fn move_field(&mut self, backwards: bool) -> bool {
        let len = Field::ALL.len();
        self.field = if backwards {
            self.field.checked_sub(1).unwrap_or(len - 1)
        } else {
            (self.field + 1) % len
        };
        self.clear_transient_status();
        true
    }

    fn continue_form(&mut self) -> bool {
        if Field::ALL[self.field] != Field::ConfirmPassword {
            self.move_field(false);
            return true;
        }
        self.attempt_create();
        true
    }

    fn attempt_create(&mut self) {
        if self.identity.value().trim().is_empty() {
            self.field = 0;
            self.status = Status::Error("La cédula es obligatoria.".into());
            return;
        }
        if self.name.value().trim().is_empty() {
            self.field = 1;
            self.status = Status::Error("El nombre es obligatorio.".into());
            return;
        }
        if self.password.value().is_empty() {
            self.field = 2;
            self.status = Status::Error("La contraseña es obligatoria.".into());
            return;
        }
        if self.confirm_password.value().is_empty() {
            self.field = 3;
            self.status = Status::Error("Debe confirmar la contraseña.".into());
            return;
        }
        if self.password.value() != self.confirm_password.value() {
            self.field = 3;
            self.status = Status::Error("Las contraseñas no coinciden.".into());
            return;
        }
        if self.password.value().chars().count() < MIN_PASSWORD_LENGTH {
            self.field = 2;
            self.status = Status::Error("La contraseña debe tener al menos 8 caracteres.".into());
            return;
        }
        self.status = Status::Creating {
            started: Instant::now(),
        };
    }

    fn advance_creation_at(&mut self, now: Instant) {
        let Status::Creating { started } = self.status else {
            return;
        };
        if now.saturating_duration_since(started) < CREATION_DELAY {
            return;
        }
        self.password.reset();
        self.confirm_password.reset();
        self.status = Status::Success;
    }

    fn clear_transient_status(&mut self) {
        if matches!(self.status, Status::Error(_) | Status::Success) {
            self.status = Status::Idle;
        }
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
    }

    fn viewport_is_valid(&self) -> bool {
        self.terminal_size.0 >= MIN_WIDTH && self.terminal_size.1 >= MIN_HEIGHT
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.terminal_size = (area.width, area.height);
        self.advance_creation_at(Instant::now());
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
        .split(area.inner(ratatui::layout::Margin {
            horizontal: 2,
            vertical: 1,
        }));

        self.render_top_bar(frame, rows[0], theme);
        self.render_form(frame, rows[1], theme);
        frame.render_widget(self.status_line(theme, Instant::now()), rows[2]);
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
            Paragraph::new("brisas cli · configuración inicial (v2)").style(theme.muted()),
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
        let hero = centered(area, 50, 21);
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(hero);

        frame.render_widget(
            Paragraph::new("B R I S A S   C L I")
                .style(theme.title())
                .alignment(Alignment::Center),
            rows[0],
        );
        frame.render_widget(
            Paragraph::new("CONFIGURACIÓN INICIAL · PRIMER USUARIO DEL SISTEMA")
                .style(theme.muted())
                .alignment(Alignment::Center),
            rows[1],
        );

        let disabled = matches!(self.status, Status::Creating { .. });
        render_field(
            frame,
            rows[3],
            FieldSpec {
                label: "CÉDULA",
                input: &self.identity,
                focused: !disabled && Field::ALL[self.field] == Field::Identity,
                masked: false,
                theme,
            },
        );
        render_field(
            frame,
            rows[5],
            FieldSpec {
                label: "NOMBRE",
                input: &self.name,
                focused: !disabled && Field::ALL[self.field] == Field::Name,
                masked: false,
                theme,
            },
        );
        render_field(
            frame,
            rows[7],
            FieldSpec {
                label: "CONTRASEÑA",
                input: &self.password,
                focused: !disabled && Field::ALL[self.field] == Field::Password,
                masked: true,
                theme,
            },
        );
        render_field(
            frame,
            rows[9],
            FieldSpec {
                label: "CONFIRMAR CONTRASEÑA",
                input: &self.confirm_password,
                focused: !disabled && Field::ALL[self.field] == Field::ConfirmPassword,
                masked: true,
                theme,
            },
        );

        frame.render_widget(
            Paragraph::new("Este usuario será creado activo y con rol ROOT.")
                .style(theme.warning())
                .alignment(Alignment::Center),
            rows[11],
        );
    }

    fn status_line(&self, theme: Theme, now: Instant) -> Line<'static> {
        match &self.status {
            Status::Idle => Line::from(""),
            Status::Creating { started } => {
                let elapsed = now.saturating_duration_since(*started);
                if elapsed < SPINNER_APPEAR_THRESHOLD {
                    Line::from("")
                } else {
                    let frame_index = (elapsed.as_millis() / SPINNER_FRAME_INTERVAL.as_millis()) as usize;
                    Line::from(format!("{} Creando usuario ROOT…", SPINNER[frame_index % SPINNER.len()]))
                        .style(theme.warning())
                }
            }
            Status::Error(message) => Line::from(format!("✕ {message}")).style(theme.danger()),
            Status::Success => {
                Line::from("✓ Usuario ROOT creado · continúe al inicio de sesión").style(theme.success())
            }
        }
    }

    fn hint_lines(&self, theme: Theme) -> Vec<Line<'static>> {
        let mut lines = vec![Line::from(vec![
            Span::styled("TAB", theme.accent()),
            Span::styled(" campo   ", theme.base()),
            Span::styled("ENTER", theme.accent()),
            Span::styled(" continuar/crear   ", theme.base()),
            Span::styled("ESC", theme.accent()),
            Span::styled(" salir   ", theme.base()),
            Span::styled("F1", theme.accent()),
            Span::styled(
                if self.help_expanded { " cerrar ayuda" } else { " más" },
                theme.base(),
            ),
        ])];
        if self.help_expanded {
            lines.push(Line::from(vec![
                Span::styled("F7", theme.accent()),
                Span::styled(" tema   ", theme.base()),
                Span::styled("CTRL+C", theme.accent()),
                Span::styled(" salida de emergencia", theme.base()),
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
    masked: bool,
    theme: Theme,
}

/// Misma silueta enfocado o no: etiqueta, valor, línea. Sólo cambia color.
fn render_field(frame: &mut Frame, area: Rect, spec: FieldSpec<'_>) {
    let FieldSpec {
        label,
        input,
        focused,
        masked,
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
    let displayed = if masked {
        "•".repeat(input.value().chars().count())
    } else {
        input.value().to_owned()
    };
    let visual_cursor = if masked { input.value().chars().count() } else { input.visual_cursor() };
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
    if focused {
        let column = visual_cursor.saturating_sub(scroll).min(viewport_width);
        frame.set_cursor_position((area.x + column as u16, value_y));
    }
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

    use super::{ConfiguracionInicialV2Pilot, Field, Status};

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
    fn empieza_en_cedula_con_los_cuatro_campos_disponibles() {
        let app = ConfiguracionInicialV2Pilot::default();
        assert_eq!(app.field, 0);
        assert_eq!(Field::ALL[app.field], Field::Identity);
    }

    #[test]
    fn tab_da_la_vuelta_entre_los_cuatro_campos() {
        let mut app = ConfiguracionInicialV2Pilot::default();
        for _ in 0..4 {
            app.handle_event(&key(KeyCode::Tab));
        }
        assert_eq!(app.field, 0);

        app.handle_event(&key(KeyCode::BackTab));
        assert_eq!(app.field, 3);
    }

    #[test]
    fn enter_avanza_de_campo_en_campo_y_finalmente_intenta_crear() {
        let mut app = ConfiguracionInicialV2Pilot::default();
        app.handle_event(&Event::Paste("1-1234-5678".into()));
        app.handle_event(&key(KeyCode::Enter));
        assert_eq!(Field::ALL[app.field], Field::Name);

        app.handle_event(&Event::Paste("Root Inicial".into()));
        app.handle_event(&key(KeyCode::Enter));
        assert_eq!(Field::ALL[app.field], Field::Password);

        app.handle_event(&key(KeyCode::Enter));
        // Enter simplemente avanza hasta el último campo, sin validar antes.
        assert_eq!(Field::ALL[app.field], Field::ConfirmPassword);

        app.handle_event(&key(KeyCode::Enter));
        // En el último campo, Enter sí intenta crear: sin contraseña, falla.
        assert_eq!(Field::ALL[app.field], Field::Password);
        assert!(matches!(app.status, Status::Error(_)));
    }

    #[test]
    fn contrasena_corta_muestra_error_y_no_avanza_a_creando() {
        let mut app = ConfiguracionInicialV2Pilot::default();
        app.handle_event(&Event::Paste("1-1234-5678".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("Root Inicial".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("corta".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("corta".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(
            matches!(&app.status, Status::Error(message) if message.contains("8 caracteres"))
        );
    }

    #[test]
    fn contrasenas_que_no_coinciden_fallan() {
        let mut app = ConfiguracionInicialV2Pilot::default();
        app.handle_event(&Event::Paste("1-1234-5678".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("Root Inicial".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("contraseñaUno".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("contraseñaDos".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(
            matches!(&app.status, Status::Error(message) if message.contains("no coinciden"))
        );
    }

    #[test]
    fn crear_con_datos_validos_muestra_spinner_tras_el_umbral_y_termina_en_exito() {
        let mut app = ConfiguracionInicialV2Pilot::default();
        app.handle_event(&Event::Paste("1-1234-5678".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("Root Inicial".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("contraseñaSegura".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("contraseñaSegura".into()));
        app.handle_event(&key(KeyCode::Enter));

        let Status::Creating { started } = app.status else {
            panic!("debe quedar creando el usuario ROOT");
        };
        let temprano = app.status_line(super::ThemePreset::Classic.theme(), started + Duration::from_millis(50));
        assert_eq!(temprano.to_string(), "");
        let tarde = app.status_line(super::ThemePreset::Classic.theme(), started + Duration::from_millis(300));
        assert!(tarde.to_string().contains("Creando"));

        app.advance_creation_at(started + Duration::from_millis(700));
        assert!(matches!(app.status, Status::Success));
        assert!(app.password.value().is_empty());
        assert!(app.confirm_password.value().is_empty());
    }

    #[test]
    fn escribir_de_nuevo_limpia_el_error_anterior() {
        let mut app = ConfiguracionInicialV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter)); // avanza a Nombre
        app.handle_event(&key(KeyCode::Enter)); // avanza a Password
        app.handle_event(&key(KeyCode::Enter)); // avanza a Confirmar Contraseña
        app.handle_event(&key(KeyCode::Enter)); // último campo: intenta crear, sin contraseña falla
        assert!(matches!(app.status, Status::Error(_)));

        app.handle_event(&Event::Paste("x".into()));
        assert!(matches!(app.status, Status::Idle));
    }

    #[test]
    fn renderiza_sin_dejar_parches_de_fondo_sin_pintar() {
        let mut app = ConfiguracionInicialV2Pilot::default();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());
        assert!(rendered.contains("ROOT"));
        assert!(terminal.backend().cursor_visible());

        let theme = super::ThemePreset::Classic.theme();
        let buffer = terminal.backend().buffer();
        for (x, y) in [(0, 0), (99, 0), (0, 29), (99, 29)] {
            assert_eq!(
                buffer[(x, y)].bg,
                theme.background,
                "celda ({x},{y}) quedó con el fondo por defecto de la terminal"
            );
        }
    }

    #[test]
    fn terminal_pequena_no_pierde_estado() {
        let mut app = ConfiguracionInicialV2Pilot::default();
        app.handle_event(&Event::Resize(40, 10));
        app.handle_event(&Event::Paste("oculto".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(app.identity.value().is_empty());
        assert_eq!(app.field, 0);
    }
}
