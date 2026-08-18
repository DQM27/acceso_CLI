use std::time::{Duration, Instant};

use chrono::Utc;
use chrono_tz::America::Costa_Rica;
use control_acceso::tui::ui_kit::{
    CommandHint, HELP_HINT, ScreenShell, StandardCommand, StatusKind, THEME_HINT, TextInput,
    TextInputFocus, Theme, ThemePreset, auxiliary_panel, centered_content, centered_rect,
    render_terminal_too_small, standard_command,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::Paragraph;
use ratatui::{
    Frame, layout::Constraint, layout::Layout, layout::Rect, text::Line, widgets::Clear,
};

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 20;
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginField {
    Identity,
    Password,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Form,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatOrigin {
    code: KeyCode,
    modifiers: KeyModifiers,
    layer: Layer,
}

#[derive(Debug)]
pub struct LoginPilot {
    identity: TextInput,
    password: TextInput,
    field: LoginField,
    help_visible: bool,
    theme: ThemePreset,
    status: String,
    status_kind: StatusKind,
    running: bool,
    terminal_size: (u16, u16),
    repeat_origin: Option<RepeatOrigin>,
    cursor_started: Instant,
}

impl Default for LoginPilot {
    fn default() -> Self {
        Self {
            identity: TextInput::default().with_max_chars(30),
            password: TextInput::default().with_max_chars(128),
            field: LoginField::Identity,
            help_visible: false,
            theme: ThemePreset::Classic,
            status: "Ingrese sus credenciales para continuar.".into(),
            status_kind: StatusKind::Normal,
            running: true,
            terminal_size: (u16::MAX, u16::MAX),
            repeat_origin: None,
            cursor_started: Instant::now(),
        }
    }
}

impl LoginPilot {
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
            self.password.clear();
            self.running = false;
            return true;
        }
        if !self.viewport_is_valid() {
            if key.is_some_and(|key| {
                key.kind == KeyEventKind::Press
                    && standard_command(key) == Some(StandardCommand::Cancel)
            }) {
                self.password.clear();
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

        if self.help_visible {
            return self.handle_help_key(key);
        }
        if key.kind == KeyEventKind::Press {
            match standard_command(key) {
                Some(StandardCommand::FocusNext) => return self.change_field(false),
                Some(StandardCommand::FocusPrevious) => return self.change_field(true),
                Some(StandardCommand::Primary) => return self.continue_form(),
                Some(StandardCommand::Cancel) => {
                    self.password.clear();
                    self.running = false;
                    return true;
                }
                Some(StandardCommand::Help) => {
                    self.help_visible = true;
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
        let changed = match self.field {
            LoginField::Identity => self.identity.handle_event(event),
            LoginField::Password => self.password.handle_event(event),
        };
        if changed {
            self.status = "Ingrese sus credenciales para continuar.".into();
            self.status_kind = StatusKind::Normal;
            self.restart_cursor();
        }
        changed
    }

    fn change_field(&mut self, backwards: bool) -> bool {
        self.field = match (self.field, backwards) {
            (LoginField::Identity, false) | (LoginField::Password, true) => LoginField::Password,
            (LoginField::Password, false) | (LoginField::Identity, true) => LoginField::Identity,
        };
        self.restart_cursor();
        true
    }

    fn continue_form(&mut self) -> bool {
        if self.field == LoginField::Identity {
            self.field = LoginField::Password;
            self.restart_cursor();
            return true;
        }
        if self.identity.value().trim().is_empty() {
            self.field = LoginField::Identity;
            self.status = "La cédula es obligatoria.".into();
            self.status_kind = StatusKind::Error;
            self.restart_cursor();
            return true;
        }
        if self.password.value().is_empty() {
            self.status = "La contraseña es obligatoria.".into();
            self.status_kind = StatusKind::Error;
            self.restart_cursor();
            return true;
        }

        self.password.clear();
        self.field = LoginField::Identity;
        self.status = "Acceso simulado autorizado · el piloto no autentica usuarios.".into();
        self.status_kind = StatusKind::Success;
        self.restart_cursor();
        true
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
        self.status = format!("Tema {} aplicado", self.theme.name());
        self.status_kind = StatusKind::Success;
    }

    fn layer(&self) -> Layer {
        if self.help_visible {
            Layer::Help
        } else {
            Layer::Form
        }
    }

    fn viewport_is_valid(&self) -> bool {
        self.terminal_size.0 >= MIN_WIDTH && self.terminal_size.1 >= MIN_HEIGHT
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
        let context = "";
        let commands = self.command_hints();
        let shell = ScreenShell {
            product: "B R I S A S   C L I",
            screen: "",
            context,
            clock: &clock,
            status: &self.status,
            status_kind: self.status_kind,
            commands: &commands,
            help_expanded: false,
            ayuda_extra: None,
        };
        let areas = shell.render(frame, area, theme);
        self.render_form(frame, areas.body, theme);
        if self.help_visible {
            self.render_help(frame, area, theme);
        }
    }

    fn command_hints(&self) -> Vec<CommandHint<'static>> {
        if self.help_visible {
            vec![CommandHint::new("ESC/F1", "Cerrar ayuda"), THEME_HINT]
        } else {
            vec![
                CommandHint::new("TAB/SHIFT+TAB", "Campo"),
                CommandHint::new("ENTER", "Continuar"),
                CommandHint::new("ESC", "Salir"),
                HELP_HINT,
                THEME_HINT,
            ]
        }
    }

    fn render_form(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let form = centered_rect(area, 56, 9);
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(form);
        let focus_enabled = !self.help_visible;
        let cursor_visible = focus_enabled && self.cursor_visible_at(Instant::now());
        frame.render_widget(
            Paragraph::new("INICIAR SESIÓN")
                .style(theme.title())
                .centered(),
            rows[0],
        );
        self.identity.render_with_cursor(
            frame,
            rows[2],
            "CÉDULA",
            "Identificador del usuario",
            TextInputFocus::new(
                focus_enabled && self.field == LoginField::Identity,
                cursor_visible,
            ),
            theme,
        );
        self.password.render_masked_with_cursor(
            frame,
            rows[4],
            "CONTRASEÑA",
            "Contraseña",
            TextInputFocus::new(
                focus_enabled && self.field == LoginField::Password,
                cursor_visible,
            ),
            theme,
        );
    }

    fn render_help(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let popup = centered_rect(area, 68, 13);
        frame.render_widget(Clear, popup);
        let block = auxiliary_panel("AYUDA DE INICIO DE SESIÓN", theme, true);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let content = centered_content(inner, 56);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("Escriba la cédula asignada al usuario."),
                Line::from("Tab y Shift+Tab cambian entre los dos campos."),
                Line::from("Enter avanza o intenta iniciar la sesión."),
                Line::from("Esc cierra el piloto desde el formulario."),
                Line::from(""),
                Line::from("La contraseña nunca se muestra en pantalla.").style(theme.success()),
                Line::from("Esta vista no consulta la base de datos.").style(theme.warning()),
                Line::from(""),
                Line::from("ESC / F1  Cerrar ayuda").style(theme.accent()),
            ]),
            content,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use control_acceso::tui::ui_kit::THEME_KEY;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{LoginField, LoginPilot, ThemePreset};

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
    fn enter_avanza_y_valida_los_dos_campos() {
        let mut app = LoginPilot::default();

        app.handle_event(&key(KeyCode::Enter));
        assert_eq!(app.field, LoginField::Password);
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.field, LoginField::Identity);
        assert!(app.status.contains("cédula"));
    }

    #[test]
    fn flechas_no_reemplazan_la_navegacion_estandar_con_tab() {
        let mut app = LoginPilot::default();

        app.handle_event(&key(KeyCode::Down));
        assert_eq!(app.field, LoginField::Identity);

        app.handle_event(&key(KeyCode::Tab));
        assert_eq!(app.field, LoginField::Password);
    }

    #[test]
    fn abre_con_cedula_enfocada_y_cursor_parpadeante() {
        let app = LoginPilot::default();
        let inicio = app.cursor_started;

        assert_eq!(app.field, LoginField::Identity);
        assert!(app.cursor_visible_at(inicio));
        assert!(app.cursor_visible_at(inicio + Duration::from_millis(499)));
        assert!(!app.cursor_visible_at(inicio + Duration::from_millis(500)));
        assert!(app.cursor_visible_at(inicio + Duration::from_millis(1_000)));
    }

    #[test]
    fn envio_valido_elimina_la_password_y_no_la_expone_en_debug() {
        let mut app = LoginPilot::default();
        app.handle_event(&Event::Paste("1-1234-5678".into()));
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&Event::Paste("secreto".into()));

        assert!(!format!("{app:?}").contains("secreto"));
        app.handle_event(&key(KeyCode::Enter));

        assert!(app.password.value().is_empty());
        assert!(app.status.contains("Acceso simulado autorizado"));
    }

    #[test]
    fn tema_cambia_una_sola_vez_al_mantener_la_tecla() {
        let mut app = LoginPilot::default();

        app.handle_event(&key(THEME_KEY));
        app.handle_event(&repeat(THEME_KEY));

        assert_eq!(app.theme, ThemePreset::Brisas);
    }

    #[test]
    fn q_es_texto_del_formulario_y_no_un_atajo_de_salida() {
        let mut app = LoginPilot::default();

        app.handle_event(&key(KeyCode::Char('q')));

        assert_eq!(app.identity.value(), "q");
        assert!(app.is_running());
    }

    #[test]
    fn renderiza_password_enmascarada_y_cursor_real() {
        let mut app = LoginPilot::default();
        app.handle_event(&Event::Paste("1-1234".into()));
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&Event::Paste("secreto".into()));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| app.render(frame))
            .expect("el login debe renderizar");

        let rendered = buffer_text(terminal.backend());
        assert!(rendered.contains("INICIAR SESIÓN"));
        assert!(rendered.contains("•••••••"));
        assert!(!rendered.contains("secreto"));
        assert!(terminal.backend().cursor_visible());
    }

    #[test]
    fn ayuda_oculta_el_cursor_y_descarta_pegado() {
        let mut app = LoginPilot::default();
        app.handle_event(&key(KeyCode::F(1)));
        app.handle_event(&Event::Paste("no debe escribirse".into()));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| app.render(frame))
            .expect("la ayuda debe renderizar");

        assert!(app.identity.value().is_empty());
        assert!(!terminal.backend().cursor_visible());
    }

    #[test]
    fn renderiza_los_tamanos_objetivo_sin_perder_campos_ni_comandos() {
        for (width, height) in [(60, 20), (80, 24), (120, 30)] {
            let mut app = LoginPilot::default();
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("backend de prueba");

            terminal
                .draw(|frame| app.render(frame))
                .expect("el login debe renderizar");

            let rendered = buffer_text(terminal.backend());
            assert!(rendered.contains("INICIAR SESIÓN"), "ancho {width}");
            assert!(rendered.contains("CÉDULA"), "ancho {width}");
            assert!(rendered.contains("CONTRASEÑA"), "ancho {width}");
            assert!(rendered.contains("F7 Tema"), "ancho {width}");
            assert!(!rendered.contains("ACCESO LOCAL"), "ancho {width}");
            assert!(!rendered.contains("PILOTO"), "ancho {width}");
            assert!(
                rendered.find("INICIAR SESIÓN") < rendered.find("CÉDULA"),
                "ancho {width}"
            );
            let cursor = terminal.backend().cursor_position();
            assert!(terminal.backend().cursor_visible());
            assert!(cursor.x < width && cursor.y < height);
        }
    }

    #[test]
    fn terminal_pequena_bloquea_escritura_invisible() {
        let mut app = LoginPilot::default();
        app.handle_event(&Event::Resize(40, 10));

        app.handle_event(&Event::Paste("oculto".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(app.identity.value().is_empty());
        assert_eq!(app.field, LoginField::Identity);
    }
}
