use crate::tiempo::hora_actual_texto;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
};

use super::*;
use crate::{
    database::queries::empresas::EmpresaResumen,
    tui::{layout, theme},
};

pub fn render(frame: &mut Frame, area: Rect, state: &EmpresasState) {
    frame.render_widget(Block::default().style(theme::texto_normal()), area);
    if area.width < 60 || area.height < 22 {
        layout::render_terminal_pequena(frame, area);
        return;
    }
    let contenido = Rect::new(
        area.x + 2,
        area.y,
        area.width.saturating_sub(4),
        area.height,
    );
    let zonas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(2),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(contenido);
    render_cabecera(frame, zonas[0], state);
    render_estado(frame, zonas[1], state);
    render_tabla(frame, zonas[2], state);
    render_pie(frame, zonas[3], state);
    render_modo(frame, contenido, state);
}

fn render_cabecera(frame: &mut Frame, area: Rect, state: &EmpresasState) {
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let columnas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(interior);
    let centro = |rect: Rect| Rect::new(rect.x, rect.y + rect.height / 2, rect.width, 1);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(" B R I S A S   C L I").style(theme::titulo()),
            Line::from(" CONTROL DE ACCESO").style(theme::texto_secundario()),
        ]),
        Rect::new(columnas[0].x, columnas[0].y, columnas[0].width, 2),
    );
    frame.render_widget(
        Paragraph::new("BASE DE EMPRESAS")
            .style(theme::foco())
            .alignment(Alignment::Center),
        centro(columnas[1]),
    );
    frame.render_widget(
        Paragraph::new(format!("Usuario: {} ", state.usuario_nombre)).alignment(Alignment::Right),
        centro(columnas[2]),
    );
}

fn render_estado(frame: &mut Frame, area: Rect, state: &EmpresasState) {
    let linea = match &state.modo {
        ModoEmpresas::Busqueda { texto } => Line::from(vec![
            Span::styled("BUSCAR EMPRESAS: ", theme::foco()),
            Span::styled(format!("{texto}_"), theme::texto_normal()),
        ]),
        _ if !state.filtro.is_empty() => Line::from(vec![
            Span::styled("FILTRO ACTIVO: ", theme::foco()),
            Span::styled(&state.filtro, theme::texto_normal()),
            Span::styled(
                format!("    {} resultados    ", state.empresas.len()),
                theme::texto_secundario(),
            ),
            Span::styled("ESC ", theme::ayuda_tecla()),
            Span::styled("Limpiar", theme::texto_normal()),
        ]),
        _ if state.error_carga.is_some() => {
            Line::from(state.error_carga.clone().unwrap_or_default()).style(theme::error())
        }
        _ => Line::from(state.mensaje.clone().unwrap_or_default()).style(theme::exito()),
    };
    frame.render_widget(Paragraph::new(linea).alignment(Alignment::Center), area);
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &EmpresasState) {
    let marco = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = marco.inner(area);
    frame.render_widget(marco, area);
    let capacidad = interior.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.empresas.len());
    let filas = state
        .empresas
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, empresa)| {
            let seleccionada = state.seleccion == Some(inicio + visible);
            Row::new([
                Cell::from(empresa.nombre.clone()),
                Cell::from(empresa.contratistas.to_string()),
            ])
            .style(if seleccionada {
                theme::seleccionado()
            } else {
                theme::texto_normal()
            })
        });
    let encabezado = Row::new(["NOMBRE", "CONTRATISTAS"])
        .style(theme::foco())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(filas, [Constraint::Fill(4), Constraint::Length(16)])
            .header(encabezado)
            .column_spacing(2),
        interior,
    );
    if state.empresas.is_empty() {
        frame.render_widget(
            Paragraph::new(if state.filtro.is_empty() {
                "Sin empresas registradas"
            } else {
                "No hay empresas que coincidan con la búsqueda."
            })
            .style(theme::advertencia())
            .alignment(Alignment::Center),
            interior,
        );
    }
}

fn render_pie(frame: &mut Frame, area: Rect, state: &EmpresasState) {
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let columnas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(36),
            Constraint::Min(20),
            Constraint::Length(12),
        ])
        .split(interior);
    let posicion = state.seleccion.map_or_else(
        || "—/—".to_owned(),
        |indice| format!("{}/{}", indice + 1, state.empresas.len()),
    );
    frame.render_widget(
        Paragraph::new(format!(
            " {} empresas │ Registro {posicion}",
            state.empresas.len()
        )),
        columnas[0],
    );
    frame.render_widget(
        Paragraph::new(
            "↑↓ Seleccionar │ ENTER Detalle │ N Nueva │ E Editar │ / Buscar │ ESC Volver",
        )
        .style(theme::foco())
        .alignment(Alignment::Center),
        columnas[1],
    );
    frame.render_widget(
        Paragraph::new(hora_actual_texto())
            .style(theme::advertencia())
            .alignment(Alignment::Right)
            .block(Block::default().padding(ratatui::widgets::Padding::right(1))),
        columnas[2],
    );
}

fn render_modo(frame: &mut Frame, area: Rect, state: &EmpresasState) {
    match &state.modo {
        ModoEmpresas::Detalle { id } => {
            if let Some(empresa) = state.empresa(*id) {
                render_detalle(frame, area, empresa);
            }
        }
        ModoEmpresas::Formulario(formulario) => render_formulario(frame, area, formulario),
        _ => {}
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, empresa: &EmpresaResumen) {
    layout::render_overlay(
        frame,
        area,
        58,
        11,
        4,
        "DETALLE DE LA EMPRESA",
        vec![
            Line::from(empresa.nombre.clone()).style(theme::titulo()),
            Line::from(""),
            Line::from(format!("Nombre                   {}", empresa.nombre)),
            Line::from(format!("Contratistas asociados  {}", empresa.contratistas)),
            Line::from(""),
            Line::from("E Editar                         ESC Cerrar").style(theme::foco()),
        ],
    );
}

fn render_formulario(frame: &mut Frame, area: Rect, formulario: &FormularioEmpresa) {
    let titulo = match formulario.modo {
        ModoFormularioEmpresa::Crear => "NUEVA EMPRESA",
        ModoFormularioEmpresa::Editar { .. } => "EDITAR EMPRESA",
    };
    layout::render_overlay(
        frame,
        area,
        64,
        10,
        4,
        titulo,
        vec![
            Line::from(""),
            Line::from(format!("Nombre      {}_", formulario.nombre)).style(theme::foco()),
            Line::from(""),
            Line::from(formulario.error.clone().unwrap_or_default()).style(theme::error()),
            Line::from("G Guardar                         ESC Cancelar").style(theme::foco()),
        ],
    );
    let modal_x = area.x
        + area
            .width
            .saturating_sub(64.min(area.width.saturating_sub(4)))
            / 2;
    let modal_y = area.y + area.height.saturating_sub(10) / 2;
    let cursor_x = modal_x
        .saturating_add(14)
        .saturating_add(formulario.nombre.chars().count() as u16);
    frame.set_cursor_position((cursor_x, modal_y.saturating_add(3)));
}
