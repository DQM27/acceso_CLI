use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use super::*;
use crate::tui::ui_kit::{
    MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, TextInputFocus, Theme, render_terminal_too_small,
};

const ANCHO_PANEL: u16 = 56;
const ALTO_PANEL: u16 = 16;
const ALTO_CAMPO: u16 = 3;

pub fn render(frame: &mut Frame, area: Rect, state: &LoginState, theme: Theme) {
    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        render_terminal_too_small(
            frame,
            area,
            MIN_TERMINAL_WIDTH,
            MIN_TERMINAL_HEIGHT,
            "ESC/Ctrl+C salir",
            theme,
        );
        return;
    }

    frame.render_widget(Block::default().style(theme.base()), area);

    let validando = matches!(state.estado(), EstadoLogin::Validando { .. });
    let ancho = ANCHO_PANEL.min(area.width.saturating_sub(4));
    let alto = ALTO_PANEL.min(area.height.saturating_sub(2));
    let panel = centrar(area, ancho, alto);

    frame.render_widget(Clear, panel);
    let borde = if validando {
        theme.warning()
    } else {
        theme.border()
    };
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_style(borde)
        .style(theme.base());
    let interior = bloque.inner(panel);
    frame.render_widget(bloque, panel);
    let contenido = Rect::new(
        interior.x.saturating_add(1),
        interior.y,
        interior.width.saturating_sub(2),
        interior.height,
    );

    let filas = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(ALTO_CAMPO),
        Constraint::Length(ALTO_CAMPO),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(contenido);

    render_encabezado(frame, filas[0], theme);
    render_pasos(frame, filas[1], state.campo_activo(), validando, theme);

    state.cedula.render_with_cursor(
        frame,
        filas[2],
        "CEDULA",
        "1-1111-1111",
        TextInputFocus::new(
            state.campo_activo() == CampoLogin::Cedula && !validando,
            state.cursor_visible,
        ),
        theme,
    );
    state.password.render_masked_with_cursor(
        frame,
        filas[3],
        "CONTRASENA",
        "Clave de acceso",
        TextInputFocus::new(
            state.campo_activo() == CampoLogin::Password && !validando,
            state.cursor_visible,
        ),
        theme,
    );

    render_estado(frame, filas[4], state, theme);
    render_ayuda(frame, filas[5], state, theme);
}

fn render_estado(frame: &mut Frame, area: Rect, state: &LoginState, theme: Theme) {
    let (texto, estilo) = match state.estado() {
        EstadoLogin::Normal => ("Enter avanza o confirma".to_owned(), theme.muted()),
        EstadoLogin::Exito => ("Acceso concedido".to_owned(), theme.success()),
        EstadoLogin::Validando { .. } => (
            format!("{} Verificando credenciales...", state.spinner()),
            theme.warning(),
        ),
        EstadoLogin::Error(mensaje) => (format!("Error: {mensaje}"), theme.danger()),
    };
    frame.render_widget(
        Paragraph::new(texto)
            .style(estilo)
            .alignment(Alignment::Center),
        area,
    );
}

fn render_encabezado(frame: &mut Frame, area: Rect, theme: Theme) {
    let lineas = vec![
        Line::from("CONTROL DE ACCESO").style(theme.title()),
        Line::from("Inicio de sesion local").style(theme.muted()),
    ];
    frame.render_widget(Paragraph::new(lineas).alignment(Alignment::Center), area);
}

fn render_pasos(
    frame: &mut Frame,
    area: Rect,
    campo_activo: CampoLogin,
    validando: bool,
    theme: Theme,
) {
    let paso_cedula = estilo_paso(
        "1 CEDULA",
        campo_activo == CampoLogin::Cedula && !validando,
        theme,
    );
    let paso_password = estilo_paso(
        "2 CONTRASENA",
        campo_activo == CampoLogin::Password && !validando,
        theme,
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            paso_cedula,
            Span::styled("  >  ", theme.muted()),
            paso_password,
        ]))
        .alignment(Alignment::Center),
        area,
    );
}

fn estilo_paso(texto: &'static str, activo: bool, theme: Theme) -> Span<'static> {
    if activo {
        Span::styled(texto, theme.accent())
    } else {
        Span::styled(texto, theme.muted())
    }
}

fn render_ayuda(frame: &mut Frame, area: Rect, state: &LoginState, theme: Theme) {
    if area.height == 0 {
        return;
    }

    let texto = if state.ayuda_expandida {
        "Tab cambia de campo. Enter avanza o inicia sesion. Esc sale."
    } else {
        "Tab cambiar campo  |  Enter continuar  |  ? ayuda  |  Esc salir"
    };
    frame.render_widget(
        Paragraph::new(texto)
            .style(theme.muted())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
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
