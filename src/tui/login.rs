use std::time::{Duration, Instant};

use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::theme;

const DURACION_VALIDACION: Duration = Duration::from_millis(800);
const DURACION_FRAME: Duration = Duration::from_millis(90);
const DURACION_PARPADEO: Duration = Duration::from_millis(500);
const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampoLogin {
    Cedula,
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoLogin {
    Normal,
    Validando { iniciado: Instant },
    Error(String),
    Exito,
}

#[derive(Debug)]
pub struct LoginState {
    cedula: String,
    password: String,
    campo_activo: CampoLogin,
    estado: EstadoLogin,
    spinner_frame: usize,
    cursor_iniciado: Instant,
    cursor_visible: bool,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            cedula: String::new(),
            password: String::new(),
            campo_activo: CampoLogin::Cedula,
            estado: EstadoLogin::Normal,
            spinner_frame: 0,
            cursor_iniciado: Instant::now(),
            cursor_visible: true,
        }
    }
}

impl LoginState {
    pub fn handle_key(&mut self, key: KeyEvent) {
        if matches!(self.estado, EstadoLogin::Validando { .. }) {
            return;
        }

        match key.code {
            KeyCode::Tab | KeyCode::Down => self.siguiente_campo(),
            KeyCode::BackTab | KeyCode::Up => self.campo_anterior(),
            KeyCode::Enter => self.enter(Instant::now()),
            KeyCode::Backspace => self.borrar(),
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.escribir(character);
            }
            _ => {}
        }
    }

    pub fn tick(&mut self, ahora: Instant) {
        let ciclos = ahora
            .saturating_duration_since(self.cursor_iniciado)
            .as_millis()
            / DURACION_PARPADEO.as_millis();
        self.cursor_visible = ciclos.is_multiple_of(2);

        let EstadoLogin::Validando { iniciado } = self.estado else {
            return;
        };

        let transcurrido = ahora.saturating_duration_since(iniciado);
        self.spinner_frame = (transcurrido.as_millis() / DURACION_FRAME.as_millis()) as usize;
        if transcurrido >= DURACION_VALIDACION {
            self.estado = EstadoLogin::Exito;
        }
    }

    pub fn password_enmascarado(&self) -> String {
        "•".repeat(self.password.chars().count())
    }

    pub fn campo_activo(&self) -> CampoLogin {
        self.campo_activo
    }

    pub fn estado(&self) -> &EstadoLogin {
        &self.estado
    }

    pub fn acceso_simulado_exitoso(&self) -> bool {
        matches!(self.estado, EstadoLogin::Exito)
    }

    fn siguiente_campo(&mut self) {
        self.campo_activo = match self.campo_activo {
            CampoLogin::Cedula => CampoLogin::Password,
            CampoLogin::Password => CampoLogin::Cedula,
        };
        self.reiniciar_cursor();
    }

    fn campo_anterior(&mut self) {
        self.siguiente_campo();
    }

    fn enter(&mut self, ahora: Instant) {
        if self.campo_activo == CampoLogin::Cedula {
            self.campo_activo = CampoLogin::Password;
            return;
        }

        if self.cedula.trim().is_empty() || self.password.is_empty() {
            self.estado = EstadoLogin::Error("Complete cédula y contraseña".to_owned());
            return;
        }

        self.estado = EstadoLogin::Validando { iniciado: ahora };
        self.spinner_frame = 0;
    }

    fn escribir(&mut self, character: char) {
        self.limpiar_estado_transitorio();
        self.reiniciar_cursor();
        match self.campo_activo {
            CampoLogin::Cedula => self.cedula.push(character),
            CampoLogin::Password => self.password.push(character),
        }
    }

    fn borrar(&mut self) {
        self.limpiar_estado_transitorio();
        self.reiniciar_cursor();
        match self.campo_activo {
            CampoLogin::Cedula => {
                self.cedula.pop();
            }
            CampoLogin::Password => {
                self.password.pop();
            }
        }
    }

    fn limpiar_estado_transitorio(&mut self) {
        if matches!(self.estado, EstadoLogin::Error(_) | EstadoLogin::Exito) {
            self.estado = EstadoLogin::Normal;
        }
    }

    fn reiniciar_cursor(&mut self) {
        self.cursor_iniciado = Instant::now();
        self.cursor_visible = true;
    }

    fn spinner(&self) -> char {
        SPINNER[self.spinner_frame % SPINNER.len()]
    }

    #[cfg(test)]
    fn iniciar_validacion_en(&mut self, ahora: Instant) {
        self.campo_activo = CampoLogin::Password;
        self.enter(ahora);
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &LoginState) {
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::FONDO)),
        area,
    );

    if area.width < 60 || area.height < 22 {
        render_terminal_pequena(frame, area);
        return;
    }

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(7),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(area);

    render_encabezado(frame, vertical[1]);
    render_formulario(frame, vertical[2], state);
    render_pie(frame, vertical[3]);
}

fn render_encabezado(frame: &mut Frame, area: Rect) {
    let contenido = vec![
        Line::from("B R I S A S   C L I").style(theme::titulo()),
        Line::from(vec![
            Span::styled("·····  ", theme::texto_secundario()),
            Span::styled("────────────────── ◆ ──────────────────", theme::foco()),
            Span::styled("  ·····", theme::texto_secundario()),
        ]),
        Line::from("CONTROL DE ACCESO").style(theme::subtitulo()),
    ];
    let encabezado = centrar(area, area.width.saturating_sub(4).min(96), area.height);
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::foco())
        .padding(ratatui::widgets::Padding::vertical(1));
    frame.render_widget(
        Paragraph::new(contenido)
            .block(bloque)
            .alignment(Alignment::Center),
        encabezado,
    );
}

fn render_formulario(frame: &mut Frame, area: Rect, state: &LoginState) {
    let ancho = area.width.saturating_sub(4).min(70);
    let alto = 14.min(area.height);
    let formulario = centrar(area, ancho, alto);
    let filas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(formulario);

    let longitud_linea = ancho.saturating_sub(20).min(20) as usize;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("─".repeat(longitud_linea), theme::borde()),
            Span::styled(" ◆ INICIAR SESIÓN ◆ ", theme::foco()),
            Span::styled("─".repeat(longitud_linea), theme::borde()),
        ]))
        .alignment(Alignment::Center),
        filas[0],
    );

    render_etiqueta(
        frame,
        filas[1],
        "CÉDULA",
        state.campo_activo == CampoLogin::Cedula,
    );
    let area_cedula = render_campo(
        frame,
        filas[2],
        "●",
        &state.cedula,
        state.campo_activo == CampoLogin::Cedula,
    );
    render_etiqueta(
        frame,
        filas[4],
        "CONTRASEÑA",
        state.campo_activo == CampoLogin::Password,
    );
    let area_password = render_campo(
        frame,
        filas[5],
        "▣",
        &state.password_enmascarado(),
        state.campo_activo == CampoLogin::Password,
    );
    render_estado(frame, filas[7], state);

    if !matches!(state.estado, EstadoLogin::Validando { .. }) && state.cursor_visible {
        let password_enmascarado = state.password_enmascarado();
        let (area_campo, contenido) = match state.campo_activo {
            CampoLogin::Cedula => (area_cedula, state.cedula.as_str()),
            CampoLogin::Password => (area_password, password_enmascarado.as_str()),
        };
        let ancho_visible = Line::from(contenido).width() as u16;
        let x = area_campo
            .x
            .saturating_add(1)
            .saturating_add(ancho_visible.min(area_campo.width.saturating_sub(2)));
        let y = area_campo.y.saturating_add(1);
        frame.set_cursor_position((x, y));
    }
}

fn render_etiqueta(frame: &mut Frame, area: Rect, etiqueta: &str, activo: bool) {
    let estilo = if activo {
        theme::foco()
    } else {
        theme::texto_secundario()
    };
    let etiqueta_area = Rect::new(
        area.x.saturating_add(5),
        area.y,
        area.width.saturating_sub(5),
        area.height,
    );
    frame.render_widget(Paragraph::new(etiqueta).style(estilo), etiqueta_area);
}

fn render_campo(frame: &mut Frame, area: Rect, icono: &str, valor: &str, activo: bool) -> Rect {
    let estilo = if activo {
        theme::foco()
    } else {
        theme::borde()
    };
    let columnas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(4), Constraint::Min(1)])
        .split(area);
    frame.render_widget(
        Paragraph::new(icono)
            .style(estilo)
            .alignment(Alignment::Center)
            .block(Block::default().padding(ratatui::widgets::Padding::top(1))),
        columnas[0],
    );
    let borde = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(estilo);
    frame.render_widget(
        Paragraph::new(valor)
            .style(theme::texto_normal())
            .block(borde),
        columnas[1],
    );
    columnas[1]
}

fn render_estado(frame: &mut Frame, area: Rect, state: &LoginState) {
    let (texto, estilo) = match &state.estado {
        EstadoLogin::Normal => (String::new(), theme::texto_secundario()),
        EstadoLogin::Validando { .. } => (
            format!("{} Verificando credenciales...", state.spinner()),
            theme::advertencia(),
        ),
        EstadoLogin::Error(mensaje) => (format!("✕ {mensaje}"), theme::error()),
        EstadoLogin::Exito => ("✓ Acceso autorizado".to_owned(), theme::exito()),
    };
    frame.render_widget(
        Paragraph::new(texto)
            .style(estilo)
            .alignment(Alignment::Center),
        area,
    );
}

fn render_pie(frame: &mut Frame, area: Rect) {
    let hora = Local::now().format("%H:%M:%S").to_string();
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);

    let columnas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),
            Constraint::Min(20),
            Constraint::Length(10),
        ])
        .split(interior);
    frame.render_widget(
        Paragraph::new(" Sistema preparado").style(theme::texto_normal()),
        columnas[0],
    );

    let ayuda = if area.width >= 88 {
        Line::from(vec![
            Span::styled("TAB/↑↓ ", theme::ayuda_tecla()),
            Span::styled("Cambiar campo  │  ", theme::texto_normal()),
            Span::styled("ENTER ", theme::ayuda_tecla()),
            Span::styled("Continuar  │  ", theme::texto_normal()),
            Span::styled("ESC/Ctrl+C ", theme::ayuda_tecla()),
            Span::styled("Salir", theme::texto_normal()),
        ])
    } else {
        Line::from(vec![
            Span::styled("TAB/↑↓ ", theme::ayuda_tecla()),
            Span::styled("Campo  │  ", theme::texto_normal()),
            Span::styled("ENTER ", theme::ayuda_tecla()),
            Span::styled("Seguir  │  ", theme::texto_normal()),
            Span::styled("ESC ", theme::ayuda_tecla()),
            Span::styled("Salir", theme::texto_normal()),
        ])
    };
    frame.render_widget(
        Paragraph::new(ayuda).alignment(Alignment::Center),
        columnas[1],
    );
    frame.render_widget(
        Paragraph::new(hora)
            .style(theme::advertencia())
            .alignment(Alignment::Right),
        columnas[2],
    );
}

fn render_terminal_pequena(frame: &mut Frame, area: Rect) {
    let mensaje = vec![
        Line::from("Terminal demasiado pequeña").style(theme::advertencia()),
        Line::from("Tamaño mínimo recomendado: 60 x 22").style(theme::texto_normal()),
        Line::from(format!("Tamaño actual: {} x {}", area.width, area.height))
            .style(theme::texto_secundario()),
    ];
    frame.render_widget(
        Paragraph::new(mensaje)
            .alignment(Alignment::Center)
            .block(Block::default().padding(ratatui::widgets::Padding::top(
                area.height.saturating_sub(3) / 2,
            ))),
        area,
    );
}

fn centrar(area: Rect, ancho: u16, alto: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tecla(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn login_completo() -> LoginState {
        let mut state = LoginState::default();
        for character in "1-1111-1111".chars() {
            state.handle_key(tecla(KeyCode::Char(character)));
        }
        state.handle_key(tecla(KeyCode::Tab));
        for character in "secreto".chars() {
            state.handle_key(tecla(KeyCode::Char(character)));
        }
        state
    }

    #[test]
    fn cambia_el_foco_en_ambas_direcciones() {
        let mut state = LoginState::default();
        state.handle_key(tecla(KeyCode::Tab));
        assert_eq!(state.campo_activo(), CampoLogin::Password);
        state.handle_key(tecla(KeyCode::BackTab));
        assert_eq!(state.campo_activo(), CampoLogin::Cedula);
    }

    #[test]
    fn enter_desde_cedula_avanza_a_password() {
        let mut state = LoginState::default();
        state.handle_key(tecla(KeyCode::Enter));
        assert_eq!(state.campo_activo(), CampoLogin::Password);
    }

    #[test]
    fn backspace_elimina_el_ultimo_caracter() {
        let mut state = LoginState::default();
        state.handle_key(tecla(KeyCode::Char('á')));
        state.handle_key(tecla(KeyCode::Char('b')));
        state.handle_key(tecla(KeyCode::Backspace));
        assert_eq!(state.cedula, "á");
    }

    #[test]
    fn campos_vacios_producen_error() {
        let mut state = LoginState::default();
        state.handle_key(tecla(KeyCode::Enter));
        state.handle_key(tecla(KeyCode::Enter));
        assert!(matches!(state.estado(), EstadoLogin::Error(_)));
    }

    #[test]
    fn campos_completos_inician_validacion() {
        let mut state = login_completo();
        state.handle_key(tecla(KeyCode::Enter));
        assert!(matches!(state.estado(), EstadoLogin::Validando { .. }));
    }

    #[test]
    fn tick_completa_la_validacion_simulada() {
        let inicio = Instant::now();
        let mut state = login_completo();
        state.iniciar_validacion_en(inicio);
        state.tick(inicio + DURACION_VALIDACION);
        assert_eq!(state.estado(), &EstadoLogin::Exito);
    }

    #[test]
    fn escribir_limpia_un_error_anterior() {
        let mut state = LoginState::default();
        state.handle_key(tecla(KeyCode::Enter));
        state.handle_key(tecla(KeyCode::Enter));
        state.handle_key(tecla(KeyCode::Char('x')));
        assert_eq!(state.estado(), &EstadoLogin::Normal);
    }

    #[test]
    fn mascara_conserva_cantidad_de_caracteres() {
        let state = login_completo();
        assert_eq!(state.password_enmascarado(), "•••••••");
    }

    #[test]
    fn representacion_para_render_no_expone_password() {
        let state = login_completo();
        let mascara = state.password_enmascarado();
        assert!(!mascara.contains("secreto"));
        assert!(!mascara.contains('s'));
    }

    #[test]
    fn cursor_parpadea_y_se_reinicia_al_escribir() {
        let inicio = Instant::now();
        let mut state = LoginState {
            cursor_iniciado: inicio,
            ..LoginState::default()
        };
        state.tick(inicio + DURACION_PARPADEO);
        assert!(!state.cursor_visible);

        state.handle_key(tecla(KeyCode::Char('1')));
        assert!(state.cursor_visible);
    }
}
