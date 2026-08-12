use chrono::Local;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::*;
use crate::tui::{layout, theme};

pub fn render(frame: &mut Frame, area: Rect, state: &LoginState) {
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::FONDO)),
        area,
    );

    if area.width < 60 || area.height < 22 {
        layout::render_terminal_pequena(frame, area);
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

fn centrar(area: Rect, ancho: u16, alto: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto,
    )
}
