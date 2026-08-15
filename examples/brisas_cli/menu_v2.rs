//! Mockup de fluidez para el menú principal: mismos principios de `login_v2`
//! (lienzo pintado por completo, ayuda expandible sin bloquear, sin modal
//! flotante) aplicados a una lista con confirmación de acciones sensibles.
//!
//! cargo run --example brisas_cli -- menu       (piloto actual)
//! cargo run --example brisas_cli -- menu-v2    (esta propuesta)

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
    widgets::{Block, Paragraph},
};

use super::quick_exit::{QuickExitOutcome, QuickExitOverlay};

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 18;
/// Operador con sesión simulada: cada pantalla lo muestra en la barra
/// superior y lo estampa al registrar, para que quede trazable quién hizo
/// cada acción. En la app real vendría de la sesión autenticada.
const CURRENT_OPERATOR: &str = "Daniel Quintana";

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

impl Confirmation {
    const fn question(self) -> &'static str {
        match self {
            Self::Logout => "¿Cerrar la sesión de Daniel Quintana?",
            Self::Quit => "¿Cerrar BRISAS CLI?",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatOrigin {
    code: KeyCode,
    modifiers: KeyModifiers,
    confirming: bool,
}

#[derive(Debug)]
pub struct MenuV2Pilot {
    selected: usize,
    confirmation: Option<Confirmation>,
    prepared_message: Option<String>,
    help_expanded: bool,
    theme: ThemePreset,
    running: bool,
    terminal_size: (u16, u16),
    repeat_origin: Option<RepeatOrigin>,
    quick_exit: QuickExitOverlay,
}

impl Default for MenuV2Pilot {
    fn default() -> Self {
        Self {
            selected: 0,
            confirmation: None,
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

impl MenuV2Pilot {
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
                key.kind == KeyEventKind::Press
                    && (standard_command(key) == Some(StandardCommand::Cancel) || key_char(key, 'q'))
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
            return false;
        };

        let origin = RepeatOrigin {
            code: key.code,
            modifiers: key.modifiers,
            confirming: self.confirmation.is_some(),
        };
        match key.kind {
            KeyEventKind::Press => self.repeat_origin = Some(origin),
            KeyEventKind::Repeat if self.repeat_origin != Some(origin) => return false,
            KeyEventKind::Repeat | KeyEventKind::Release => {}
        }

        if self.confirmation.is_some() {
            self.handle_confirmation_key(key)
        } else {
            self.handle_main_key(key)
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
            Some(StandardCommand::Help) => self.help_expanded = !self.help_expanded,
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
        match standard_command(key) {
            Some(StandardCommand::Primary) => match self.confirmation.take() {
                Some(Confirmation::Quit) => self.running = false,
                Some(Confirmation::Logout) => {
                    self.prepared_message =
                        Some("Cierre de sesión confirmado · simulación del piloto".into());
                    self.select(0);
                }
                None => return false,
            },
            Some(StandardCommand::Cancel) => {
                self.confirmation = None;
                self.prepared_message = Some("Acción cancelada · no hubo cambios".into());
            }
            Some(StandardCommand::Help) => self.help_expanded = !self.help_expanded,
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
                self.prepared_message = Some(format!(
                    "{} preparado · el piloto no abre la lógica productiva",
                    item.label()
                ));
            }
        }
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
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
        self.prepared_message = None;
    }

    fn select_item(&mut self, item: MenuItem) {
        if let Some(index) = MenuItem::ALL.iter().position(|candidate| *candidate == item) {
            self.select(index);
        }
    }

    fn selected_item(&self) -> MenuItem {
        MenuItem::ALL[self.selected]
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

        // Lienzo completo primero: evita parches de fondo entre widgets.
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
        self.render_list(frame, rows[1], theme);
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
            Paragraph::new("brisas cli · menú principal (v2)").style(theme.muted()),
            columns[0],
        );
        frame.render_widget(
            Paragraph::new(format!("{CURRENT_OPERATOR} · {clock}"))
                .style(theme.muted())
                .alignment(Alignment::Right),
            columns[1],
        );
    }

    fn render_list(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let list_area = centered(area, 56, MenuItem::ALL.len() as u16 + 3);
        let mut lines = Vec::with_capacity(MenuItem::ALL.len() + 3);
        let mut previous_group = None;
        for (index, item) in MenuItem::ALL.iter().copied().enumerate() {
            if previous_group != Some(item.group()) {
                lines.push(Line::from(item.group().label()).style(theme.muted()));
                previous_group = Some(item.group());
            }
            let selected = index == self.selected;
            let marker = if selected { ">" } else { " " };
            let row = format!("{marker} {:<3} {}", item.shortcut(), item.label());
            lines.push(Line::from(row).style(if selected {
                theme.selected()
            } else {
                theme.base()
            }));
        }
        frame.render_widget(Paragraph::new(lines), list_area);
    }

    fn status_line(&self, theme: Theme) -> Line<'static> {
        if let Some(confirmation) = self.confirmation {
            return Line::from(confirmation.question()).style(theme.warning());
        }
        if let Some(message) = &self.prepared_message {
            return Line::from(message.clone()).style(theme.success());
        }
        Line::from(self.selected_item().description()).style(theme.muted())
    }

    fn hint_lines(&self, theme: Theme) -> Vec<Line<'static>> {
        if self.confirmation.is_some() {
            return vec![Line::from(vec![
                Span::styled("ENTER", theme.accent()),
                Span::styled(" confirmar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cancelar   ", theme.base()),
                Span::styled("F7", theme.accent()),
                Span::styled(" tema", theme.base()),
            ])];
        }
        let mut lines = vec![Line::from(vec![
            Span::styled("ENTER", theme.accent()),
            Span::styled(" abrir   ", theme.base()),
            Span::styled("↑↓", theme.accent()),
            Span::styled(" mover   ", theme.base()),
            Span::styled("F1", theme.accent()),
            Span::styled(
                if self.help_expanded { " cerrar ayuda" } else { " más" },
                theme.base(),
            ),
        ])];
        if self.help_expanded {
            lines.push(Line::from(vec![
                Span::styled("1–6", theme.accent()),
                Span::styled(" acceso directo   ", theme.base()),
                Span::styled("L", theme.accent()),
                Span::styled(" cerrar sesión   ", theme.base()),
                Span::styled("Q/ESC", theme.accent()),
                Span::styled(" salir   ", theme.base()),
                Span::styled(QUICK_EXIT_HINT.key, theme.accent()),
                Span::styled(format!(" {}   ", QUICK_EXIT_HINT.label), theme.base()),
                Span::styled("F7", theme.accent()),
                Span::styled(" tema", theme.base()),
            ]));
        }
        lines
    }
}

fn key_char(key: KeyEvent, expected: char) -> bool {
    !key.modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        && matches!(key.code, KeyCode::Char(actual) if actual.eq_ignore_ascii_case(&expected))
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
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{Confirmation, MenuItem, MenuV2Pilot, ThemePreset};

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
        let mut app = MenuV2Pilot::default();

        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&repeat(KeyCode::Down));
        assert_eq!(app.selected_item(), MenuItem::History);

        app.handle_event(&key(KeyCode::Home));
        assert_eq!(app.selected_item(), MenuItem::NewEntry);
    }

    #[test]
    fn f2_abre_salida_rapida_desde_el_menu_sin_perder_la_seleccion() {
        let mut app = MenuV2Pilot::default();
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::F(2)));
        assert!(app.quick_exit.is_open());

        app.handle_event(&Event::Paste("Ana María".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(!app.quick_exit.is_open());
        assert_eq!(app.selected_item(), MenuItem::ActiveEntries);
        assert!(
            app.prepared_message
                .as_deref()
                .is_some_and(|message| message.contains("Ana María Solís"))
        );
    }

    #[test]
    fn salir_pide_confirmacion_sin_ocultar_la_lista() {
        let mut app = MenuV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('q')));

        assert_eq!(app.confirmation, Some(Confirmation::Quit));
        assert!(app.is_running());

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal
            .draw(|frame| app.render(frame))
            .expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        // La lista sigue en pantalla junto a la pregunta: no hay modal que la tape.
        assert!(rendered.contains("Registrar ingreso"));
        assert!(rendered.contains("¿Cerrar BRISAS CLI?"));
    }

    #[test]
    fn escape_cancela_la_confirmacion_sin_salir() {
        let mut app = MenuV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('q')));
        app.handle_event(&key(KeyCode::Esc));

        assert!(app.confirmation.is_none());
        assert!(app.is_running());
    }

    #[test]
    fn enter_confirma_la_salida() {
        let mut app = MenuV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('q')));
        app.handle_event(&key(KeyCode::Enter));

        assert!(!app.is_running());
    }

    #[test]
    fn acceso_directo_prepara_la_opcion_sin_abrir_logica_real() {
        let mut app = MenuV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('4')));

        assert_eq!(app.selected_item(), MenuItem::Contractors);
        assert!(
            app.prepared_message
                .as_deref()
                .is_some_and(|message| message.contains("Contratistas preparado"))
        );
        assert!(app.confirmation.is_none());
    }

    #[test]
    fn la_ayuda_expande_sin_interrumpir_la_navegacion() {
        let mut app = MenuV2Pilot::default();
        app.handle_event(&key(KeyCode::F(1)));
        assert!(app.help_expanded);

        app.handle_event(&key(KeyCode::Down));
        assert_eq!(app.selected_item(), MenuItem::ActiveEntries);
    }

    #[test]
    fn tema_cambia_una_sola_vez_al_mantener_la_tecla() {
        use control_acceso::tui::ui_kit::THEME_KEY;

        let mut app = MenuV2Pilot::default();
        app.handle_event(&key(THEME_KEY));
        app.handle_event(&repeat(THEME_KEY));

        assert_eq!(app.theme, ThemePreset::Brisas);
    }

    #[test]
    fn pinta_todo_el_lienzo_sin_dejar_parches_sin_color() {
        let mut app = MenuV2Pilot::default();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| app.render(frame))
            .expect("debe renderizar");

        let theme = ThemePreset::Classic.theme();
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
    fn terminal_pequena_bloquea_acciones_ocultas() {
        let mut app = MenuV2Pilot::default();
        app.handle_event(&Event::Resize(40, 10));

        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.selected_item(), MenuItem::NewEntry);
        assert!(app.confirmation.is_none());
    }
}
