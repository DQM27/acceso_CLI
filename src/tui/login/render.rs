use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use super::*;
use crate::tui::ui_kit::{MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, Theme, render_terminal_too_small};

const ANCHO_CAJA: u16 = 34;
const ALTO_CAJA: u16 = 3;

/// Login minimalista, estilo consola: sólo un par de cajas delgadas, blancas
/// y centradas, sin ningún texto — ni título, ni etiquetas, ni formulario.
/// La cédula en una, Enter, y la contraseña aparece debajo en otra
/// (enmascarada). El estado (error, verificando) se avisa aparte, debajo de
/// las cajas, para no meterle texto adentro.
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
    let muestra_password = state.campo_activo() == CampoLogin::Password || validando;

    let ancho = ANCHO_CAJA.min(area.width.saturating_sub(4));
    let alto_bloque = if muestra_password {
        ALTO_CAJA * 2 + 1
    } else {
        ALTO_CAJA
    };
    let bloque = centrar(area, ancho, alto_bloque);

    if muestra_password {
        let filas = Layout::vertical([
            Constraint::Length(ALTO_CAJA),
            Constraint::Length(1),
            Constraint::Length(ALTO_CAJA),
        ])
        .split(bloque);
        render_caja(frame, filas[0], state.cedula.value(), state.cedula.cursor(), false);
        render_caja(
            frame,
            filas[2],
            &state.password_enmascarado(),
            state.password.cursor(),
            !validando,
        );
        render_estado(
            frame,
            Rect::new(bloque.x, filas[2].y + ALTO_CAJA, ancho, 1),
            state,
            theme,
        );
    } else {
        let filas = Layout::vertical([Constraint::Length(ALTO_CAJA)]).split(bloque);
        render_caja(frame, filas[0], state.cedula.value(), state.cedula.cursor(), true);
        render_estado(
            frame,
            Rect::new(bloque.x, filas[0].y + ALTO_CAJA, ancho, 1),
            state,
            theme,
        );
    }
}

/// Caja blanca sin título ni etiqueta — la activa en blanco pleno, la ya
/// completada (cédula, una vez que el foco pasó a contraseña) atenuada para
/// marcar cuál sigue editable sin dejar de ser "sólo la caja".
fn render_caja(frame: &mut Frame, area: Rect, valor: &str, cursor: usize, activa: bool) {
    let color = if activa { Color::White } else { Color::Gray };
    let estilo = Style::default().fg(color);
    let bloque = Block::default().borders(Borders::ALL).border_style(estilo);
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);

    let viewport = interior.width.saturating_sub(1) as usize;
    let scroll = cursor.saturating_sub(viewport);
    let texto: String = valor.chars().skip(scroll).take(viewport).collect();
    frame.render_widget(Paragraph::new(Line::from(texto).style(estilo)), interior);

    if activa {
        let columna = cursor.saturating_sub(scroll).min(viewport) as u16;
        frame.set_cursor_position((interior.x + columna, interior.y));
    }
}

/// Única línea de texto de toda la pantalla, y sólo cuando hace falta:
/// verificando o un error. Sin eso, no hay nada debajo de las cajas.
fn render_estado(frame: &mut Frame, area: Rect, state: &LoginState, theme: Theme) {
    let (texto, estilo) = match state.estado() {
        EstadoLogin::Normal | EstadoLogin::Exito => return,
        EstadoLogin::Validando { .. } => {
            (format!("{} Verificando…", state.spinner()), theme.warning())
        }
        EstadoLogin::Error(mensaje) => (format!("✗ {mensaje}"), theme.danger()),
    };
    frame.render_widget(
        Paragraph::new(texto)
            .style(estilo)
            .alignment(Alignment::Center),
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
