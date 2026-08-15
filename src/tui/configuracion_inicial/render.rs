use crate::tiempo::hora_actual_texto;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use super::*;
use crate::tui::{layout, theme};

pub fn render(frame: &mut Frame, area: Rect, state: &ConfiguracionInicialState) {
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::FONDO)),
        area,
    );
    if area.width < 60 || area.height < 22 {
        layout::render_terminal_pequena(frame, area);
        return;
    }

    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(7),
        Constraint::Min(11),
        Constraint::Length(3),
    ])
    .split(area);
    render_encabezado(frame, vertical[1]);
    render_formulario(frame, vertical[2], state);
    render_pie(frame, vertical[3]);
}

fn render_encabezado(frame: &mut Frame, area: Rect) {
    let encabezado = centrar(area, area.width.saturating_sub(4).min(96), area.height);
    let contenido = vec![
        Line::from(vec![
            Span::styled("B R I S A S   C L I", theme::titulo()),
            Span::raw("                 "),
            Span::styled("CONFIGURACIÓN INICIAL", theme::foco()),
        ]),
        Line::from(vec![
            Span::styled("·····  ", theme::texto_secundario()),
            Span::styled("────────────────── ◆ ──────────────────", theme::foco()),
            Span::styled("  ·····", theme::texto_secundario()),
        ]),
        Line::from("CONTROL DE ACCESO").style(theme::subtitulo()),
    ];
    frame.render_widget(
        Paragraph::new(contenido)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .border_style(theme::foco())
                    .padding(ratatui::widgets::Padding::vertical(1)),
            ),
        encabezado,
    );
}

fn render_formulario(frame: &mut Frame, area: Rect, state: &ConfiguracionInicialState) {
    let formulario = centrar(
        area,
        area.width.saturating_sub(6).min(76),
        11.min(area.height),
    );
    let filas = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(formulario);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("────────── ", theme::borde()),
            Span::styled("◆ PRIMER USUARIO DEL SISTEMA ◆", theme::foco()),
            Span::styled(" ──────────", theme::borde()),
        ]))
        .alignment(Alignment::Center),
        filas[0],
    );

    let cedula = render_campo(
        frame,
        filas[2],
        "CÉDULA",
        &state.cedula,
        state.campo_activo == CampoConfiguracion::Cedula,
    );
    let nombre = render_campo(
        frame,
        filas[3],
        "NOMBRE",
        &state.nombre,
        state.campo_activo == CampoConfiguracion::Nombre,
    );
    let password_mascara = state.password_enmascarado();
    let password = render_campo(
        frame,
        filas[4],
        "CONTRASEÑA",
        &password_mascara,
        state.campo_activo == CampoConfiguracion::Password,
    );
    let confirmacion_mascara = state.confirmacion_enmascarada();
    let confirmar = render_campo(
        frame,
        filas[5],
        "CONFIRMAR",
        &confirmacion_mascara,
        state.campo_activo == CampoConfiguracion::ConfirmarPassword,
    );

    frame.render_widget(
        Paragraph::new("Este usuario será creado activo y con rol ROOT.")
            .style(theme::advertencia())
            .alignment(Alignment::Center),
        filas[7],
    );
    render_estado(frame, filas[9], state);

    if state.estado != EstadoConfiguracion::Creando && state.cursor_visible {
        let (campo, longitud) = match state.campo_activo {
            CampoConfiguracion::Cedula => (cedula, Line::from(state.cedula.as_str()).width()),
            CampoConfiguracion::Nombre => (nombre, Line::from(state.nombre.as_str()).width()),
            CampoConfiguracion::Password => (password, Line::from(password_mascara).width()),
            CampoConfiguracion::ConfirmarPassword => {
                (confirmar, Line::from(confirmacion_mascara).width())
            }
        };
        frame.set_cursor_position((
            campo
                .x
                .saturating_add((longitud as u16).min(campo.width.saturating_sub(1))),
            campo.y,
        ));
    }
}

fn render_campo(frame: &mut Frame, area: Rect, etiqueta: &str, valor: &str, activo: bool) -> Rect {
    let columnas = Layout::horizontal([Constraint::Length(18), Constraint::Min(1)]).split(area);
    let estilo = if activo {
        theme::foco()
    } else {
        theme::texto_secundario()
    };
    frame.render_widget(
        Paragraph::new(etiqueta)
            .style(estilo)
            .alignment(Alignment::Right),
        columnas[0],
    );
    let valor_area = Rect::new(
        columnas[1].x.saturating_add(2),
        columnas[1].y,
        columnas[1].width.saturating_sub(2),
        1,
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("│ ", estilo),
            Span::styled(valor, theme::texto_normal()),
        ])),
        columnas[1],
    );
    valor_area
}

fn render_estado(frame: &mut Frame, area: Rect, state: &ConfiguracionInicialState) {
    let (texto, estilo) = match &state.estado {
        EstadoConfiguracion::Editando => (String::new(), theme::texto_secundario()),
        EstadoConfiguracion::Creando => {
            ("⠋ Creando usuario ROOT...".to_owned(), theme::advertencia())
        }
        EstadoConfiguracion::Error(error) => (format!("✕ {error}"), theme::error()),
    };
    frame.render_widget(
        Paragraph::new(texto)
            .style(estilo)
            .alignment(Alignment::Center),
        area,
    );
}

fn render_pie(frame: &mut Frame, area: Rect) {
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let columnas = Layout::horizontal([
        Constraint::Length(25),
        Constraint::Min(20),
        Constraint::Length(10),
    ])
    .split(interior);
    frame.render_widget(
        Paragraph::new(" Configuración requerida").style(theme::advertencia()),
        columnas[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("TAB/↑↓ ", theme::ayuda_tecla()),
            Span::styled("Campo  │  ", theme::texto_normal()),
            Span::styled("G/ENTER ", theme::ayuda_tecla()),
            Span::styled("Crear ROOT  │  ", theme::texto_normal()),
            Span::styled("ESC ", theme::ayuda_tecla()),
            Span::styled("Salir", theme::texto_normal()),
        ]))
        .alignment(Alignment::Center),
        columnas[1],
    );
    frame.render_widget(
        Paragraph::new(hora_actual_texto())
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
