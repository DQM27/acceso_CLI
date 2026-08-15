use chrono::Utc;
use chrono_tz::America::Costa_Rica;
use control_acceso::tui::ui_kit::{
    CommandHint, HELP_HINT, ScreenShell, StandardCommand, StatusKind, THEME_HINT, Theme,
    ThemePreset, auxiliary_panel, centered_content, centered_rect, render_terminal_too_small,
    standard_command,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Clear, Paragraph},
};

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuGroup {
    Operation,
    Management,
    Session,
}

impl MenuGroup {
    const fn label(self) -> &'static str {
        match self {
            Self::Operation => "OPERACIÓN",
            Self::Management => "GESTIÓN",
            Self::Session => "SESIÓN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuItem {
    NewEntry,
    ActiveEntries,
    History,
    Contractors,
    Companies,
    Users,
    Logout,
    Quit,
}

impl MenuItem {
    const ALL: [Self; 8] = [
        Self::NewEntry,
        Self::ActiveEntries,
        Self::History,
        Self::Contractors,
        Self::Companies,
        Self::Users,
        Self::Logout,
        Self::Quit,
    ];

    const fn group(self) -> MenuGroup {
        match self {
            Self::NewEntry | Self::ActiveEntries | Self::History => MenuGroup::Operation,
            Self::Contractors | Self::Companies | Self::Users => MenuGroup::Management,
            Self::Logout | Self::Quit => MenuGroup::Session,
        }
    }

    const fn shortcut(self) -> &'static str {
        match self {
            Self::NewEntry => "1",
            Self::ActiveEntries => "2",
            Self::History => "3",
            Self::Contractors => "4",
            Self::Companies => "5",
            Self::Users => "6",
            Self::Logout => "L",
            Self::Quit => "Q",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::NewEntry => "Registrar ingreso",
            Self::ActiveEntries => "Ingresos activos y salidas",
            Self::History => "Consultar historial",
            Self::Contractors => "Contratistas",
            Self::Companies => "Empresas",
            Self::Users => "Usuarios",
            Self::Logout => "Cerrar sesión",
            Self::Quit => "Salir",
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::NewEntry => "Busca un contratista y registra su entrada.",
            Self::ActiveEntries => "Consulta quién permanece dentro y registra su salida.",
            Self::History => "Consulta movimientos anteriores con filtros y paginación.",
            Self::Contractors => "Consulta, registra y actualiza contratistas.",
            Self::Companies => "Consulta, registra y actualiza empresas.",
            Self::Users => "Consulta y administra usuarios del sistema.",
            Self::Logout => "Regresa al inicio de sesión de forma segura.",
            Self::Quit => "Cierra BRISAS CLI de forma segura.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confirmation {
    Logout,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Main,
    Confirmation,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatOrigin {
    code: KeyCode,
    modifiers: KeyModifiers,
    layer: Layer,
}

#[derive(Debug)]
pub struct MenuPilot {
    selected: usize,
    confirmation: Option<Confirmation>,
    help_visible: bool,
    theme: ThemePreset,
    status: String,
    status_kind: StatusKind,
    running: bool,
    terminal_size: (u16, u16),
    repeat_origin: Option<RepeatOrigin>,
}

impl Default for MenuPilot {
    fn default() -> Self {
        Self {
            selected: 0,
            confirmation: None,
            help_visible: false,
            theme: ThemePreset::Classic,
            status: MenuItem::NewEntry.description().into(),
            status_kind: StatusKind::Normal,
            running: true,
            terminal_size: (u16::MAX, u16::MAX),
            repeat_origin: None,
        }
    }
}

impl MenuPilot {
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

        let Event::Key(key) = event else {
            return false;
        };
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return false;
        }
        if standard_command(*key) == Some(StandardCommand::EmergencyExit) {
            self.running = false;
            return true;
        }
        if !self.viewport_is_valid() {
            if key.kind == KeyEventKind::Press
                && (standard_command(*key) == Some(StandardCommand::Cancel) || key_char(*key, 'q'))
            {
                self.running = false;
                return true;
            }
            return false;
        }

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

        match self.layer() {
            Layer::Help => self.handle_help_key(*key),
            Layer::Confirmation => self.handle_confirmation_key(*key),
            Layer::Main => self.handle_main_key(*key),
        }
    }

    fn handle_main_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
        );
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary | StandardCommand::Activate) => self.activate_selected(),
            Some(StandardCommand::Cancel) => {
                self.select_item(MenuItem::Quit);
                self.activate_selected();
            }
            Some(StandardCommand::Help) => self.help_visible = true,
            Some(StandardCommand::Theme) => self.toggle_theme(),
            _ => match key.code {
                KeyCode::Up => self.move_selection(-1),
                KeyCode::Down => self.move_selection(1),
                KeyCode::Home => self.select(0),
                KeyCode::End => self.select(MenuItem::ALL.len().saturating_sub(1)),
                KeyCode::Char(character @ '1'..='6') => {
                    self.select((character as usize).saturating_sub('1' as usize));
                    self.activate_selected();
                }
                KeyCode::Char(character) if character.eq_ignore_ascii_case(&'l') => {
                    self.select_item(MenuItem::Logout);
                    self.activate_selected();
                }
                KeyCode::Char(character) if character.eq_ignore_ascii_case(&'q') => {
                    self.select_item(MenuItem::Quit);
                    self.activate_selected();
                }
                _ => return false,
            },
        }
        true
    }

    fn handle_confirmation_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Repeat {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => {
                let confirmation = self.confirmation.take();
                match confirmation {
                    Some(Confirmation::Quit) => self.running = false,
                    Some(Confirmation::Logout) => {
                        self.status = "Cierre de sesión confirmado · simulación del piloto".into();
                        self.status_kind = StatusKind::Success;
                        self.select(0);
                    }
                    None => return false,
                }
            }
            Some(StandardCommand::Cancel) => self.cancel_confirmation(),
            Some(StandardCommand::Theme) => self.toggle_theme(),
            _ => return false,
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

    fn activate_selected(&mut self) {
        match self.selected_item() {
            MenuItem::Logout => self.confirmation = Some(Confirmation::Logout),
            MenuItem::Quit => self.confirmation = Some(Confirmation::Quit),
            item => {
                self.status = format!(
                    "{} preparado · el piloto no abre la lógica productiva",
                    item.label()
                );
                self.status_kind = StatusKind::Success;
            }
        }
    }

    fn cancel_confirmation(&mut self) {
        self.confirmation = None;
        self.status = "Acción cancelada · no hubo cambios".into();
        self.status_kind = StatusKind::Warning;
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
        self.status = format!("Tema {} aplicado", self.theme.name());
        self.status_kind = StatusKind::Success;
    }

    fn move_selection(&mut self, delta: isize) {
        self.select(
            self.selected
                .saturating_add_signed(delta)
                .min(MenuItem::ALL.len().saturating_sub(1)),
        );
    }

    fn select(&mut self, index: usize) {
        self.selected = index.min(MenuItem::ALL.len().saturating_sub(1));
        self.status = self.selected_item().description().into();
        self.status_kind = StatusKind::Normal;
    }

    fn select_item(&mut self, item: MenuItem) {
        if let Some(index) = MenuItem::ALL
            .iter()
            .position(|candidate| *candidate == item)
        {
            self.select(index);
        }
    }

    fn selected_item(&self) -> MenuItem {
        MenuItem::ALL[self.selected]
    }

    fn layer(&self) -> Layer {
        if self.help_visible {
            Layer::Help
        } else if self.confirmation.is_some() {
            Layer::Confirmation
        } else {
            Layer::Main
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

        let clock = Utc::now()
            .with_timezone(&Costa_Rica)
            .format("%d/%m/%Y  %H:%M:%S")
            .to_string();
        let context = format!("DANIEL QUINTANA · PILOTO · {}", self.theme.name());
        let commands = self.command_hints();
        let shell = ScreenShell {
            product: "B R I S A S   C L I",
            screen: "MENÚ PRINCIPAL",
            context: &context,
            clock: &clock,
            status: &self.status,
            status_kind: self.status_kind,
            commands: &commands,
        };
        let areas = shell.render(frame, area, theme);
        self.render_body(frame, areas.body, theme);

        if let Some(confirmation) = self.confirmation {
            self.render_confirmation(frame, area, confirmation, theme);
        }
        if self.help_visible {
            self.render_help(frame, area, theme);
        }
    }

    fn command_hints(&self) -> Vec<CommandHint<'static>> {
        match self.layer() {
            Layer::Main => vec![
                CommandHint::new("↑↓", "Mover"),
                CommandHint::new("ENTER", "Abrir"),
                CommandHint::new("1–6", "Acceso directo"),
                CommandHint::new("L", "Cerrar sesión"),
                CommandHint::new("Q/ESC", "Salir"),
                HELP_HINT,
                THEME_HINT,
            ],
            Layer::Confirmation => vec![
                CommandHint::new("ENTER", "Confirmar"),
                CommandHint::new("ESC", "Cancelar"),
                THEME_HINT,
            ],
            Layer::Help => vec![CommandHint::new("ESC/F1", "Cerrar ayuda"), THEME_HINT],
        }
    }

    fn render_body(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let menu = centered_rect(area, 56, MenuItem::ALL.len() as u16 + 3);
        self.render_options(frame, menu, theme);
    }

    fn render_options(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let mut lines = Vec::with_capacity(MenuItem::ALL.len() + 3);
        let mut previous_group = None;
        for (index, item) in MenuItem::ALL.iter().copied().enumerate() {
            if previous_group != Some(item.group()) {
                lines.push(Line::from(item.group().label()).style(theme.muted()));
                previous_group = Some(item.group());
            }
            let marker = if index == self.selected { ">" } else { " " };
            let row = format!(
                "{marker}  {:<3} {:<width$}",
                item.shortcut(),
                item.label(),
                width = area.width.saturating_sub(8) as usize
            );
            lines.push(Line::from(row).style(if index == self.selected {
                theme.selected()
            } else {
                theme.base()
            }));
        }
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_confirmation(
        &self,
        frame: &mut Frame,
        area: Rect,
        confirmation: Confirmation,
        theme: Theme,
    ) {
        let (title, question) = match confirmation {
            Confirmation::Logout => ("CERRAR SESIÓN", "¿Cerrar la sesión de Daniel Quintana?"),
            Confirmation::Quit => ("SALIR DE BRISAS CLI", "¿Cerrar la aplicación?"),
        };
        let popup = centered_rect(area, 58, 9);
        frame.render_widget(Clear, popup);
        let block = auxiliary_panel(title, theme, true);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let content = centered_content(inner, 44);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(question),
                Line::from(""),
                Line::from("ENTER  Confirmar"),
                Line::from("ESC    Cancelar").style(theme.muted()),
            ]),
            content,
        );
    }

    fn render_help(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let popup = centered_rect(area, 68, 14);
        frame.render_widget(Clear, popup);
        let block = auxiliary_panel("AYUDA DEL MENÚ PRINCIPAL", theme, true);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let content = centered_content(inner, 56);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("Use ↑/↓ para recorrer las opciones."),
                Line::from("Enter ejecuta la opción seleccionada."),
                Line::from("1–6 abre directamente una sección operativa."),
                Line::from("L solicita cerrar la sesión."),
                Line::from("Q o Esc solicita cerrar la aplicación."),
                Line::from(""),
                Line::from("El piloto no consulta ni modifica la base de datos.")
                    .style(theme.warning()),
                Line::from(""),
                Line::from("ESC / F1  Cerrar ayuda").style(theme.accent()),
            ]),
            content,
        );
    }
}

fn key_char(key: KeyEvent, expected: char) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        && matches!(key.code, KeyCode::Char(actual) if actual.eq_ignore_ascii_case(&expected))
}

#[cfg(test)]
mod tests {
    use control_acceso::tui::ui_kit::THEME_KEY;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{Confirmation, MenuItem, MenuPilot, ThemePreset};

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
    fn navegacion_y_repeticion_respetan_los_limites() {
        let mut app = MenuPilot::default();

        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&repeat(KeyCode::Down));
        assert_eq!(app.selected_item(), MenuItem::History);

        app.handle_event(&key(KeyCode::Home));
        app.handle_event(&repeat(KeyCode::Home));
        assert_eq!(app.selected_item(), MenuItem::NewEntry);
    }

    #[test]
    fn acceso_directo_prepara_la_opcion_sin_abrir_logica_real() {
        let mut app = MenuPilot::default();

        app.handle_event(&key(KeyCode::Char('4')));

        assert_eq!(app.selected_item(), MenuItem::Contractors);
        assert!(app.status.contains("Contratistas preparado"));
        assert!(app.confirmation.is_none());
    }

    #[test]
    fn salir_exige_confirmacion_y_escape_la_cancela() {
        let mut app = MenuPilot::default();

        app.handle_event(&key(KeyCode::Char('q')));
        app.handle_event(&repeat(KeyCode::Char('q')));
        assert_eq!(app.confirmation, Some(Confirmation::Quit));
        assert!(app.is_running());

        app.handle_event(&key(KeyCode::Esc));
        assert!(app.confirmation.is_none());
        assert!(app.is_running());
    }

    #[test]
    fn enter_ejecuta_la_accion_principal_de_una_confirmacion() {
        let mut app = MenuPilot::default();

        app.handle_event(&key(KeyCode::Char('q')));
        app.handle_event(&key(KeyCode::Enter));

        assert!(!app.is_running());
    }

    #[test]
    fn tema_cambia_una_sola_vez_al_mantener_la_tecla() {
        let mut app = MenuPilot::default();

        app.handle_event(&key(THEME_KEY));
        app.handle_event(&repeat(THEME_KEY));

        assert_eq!(app.theme, ThemePreset::Brisas);
    }

    #[test]
    fn renderiza_menu_compacto_sin_panel_lateral() {
        for (width, height) in [(60, 20), (80, 24), (120, 30)] {
            let mut app = MenuPilot::default();
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("backend de prueba");

            terminal
                .draw(|frame| app.render(frame))
                .expect("el menú debe renderizar");

            let rendered = buffer_text(terminal.backend());
            assert!(rendered.contains("MENÚ PRINCIPAL"), "ancho {width}");
            assert!(rendered.contains("Registrar ingreso"), "ancho {width}");
            assert!(rendered.contains("F7 Tema"), "ancho {width}");
            assert!(!rendered.contains("SELECCIÓN"), "ancho {width}");
            assert!(!rendered.contains("ACCIONES"), "ancho {width}");
            assert!(rendered.contains("Busca un contratista"), "ancho {width}");
            assert!(!terminal.backend().cursor_visible());
        }
    }

    #[test]
    fn terminal_pequena_bloquea_acciones_ocultas() {
        let mut app = MenuPilot::default();
        app.handle_event(&Event::Resize(40, 10));

        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.selected_item(), MenuItem::NewEntry);
        assert!(app.confirmation.is_none());
    }
}
