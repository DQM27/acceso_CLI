use chrono::Local;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
};

use super::*;
use crate::tui::{layout, mock::IngresoActivoMock, theme};

pub fn render(frame: &mut Frame, area: Rect, state: &ActivosState) {
    frame.render_widget(Block::default().style(theme::texto_normal()), area);
    if area.width < 60 || area.height < 22 {
        layout::render_terminal_pequena(frame, area);
        return;
    }
    let contenido = Rect::new(
        area.x.saturating_add(2),
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
    render_cabecera(frame, zonas[0]);
    render_estado(frame, zonas[1], state);
    render_tabla(frame, zonas[2], state);
    render_pie(frame, zonas[3], state);
    render_modo(frame, contenido, state);
}

fn render_cabecera(frame: &mut Frame, area: Rect) {
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
    let centro = |area: Rect| Rect::new(area.x, area.y + area.height / 2, area.width, 1);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(" B R I S A S   C L I").style(theme::titulo()),
            Line::from(" CONTROL DE ACCESO").style(theme::texto_secundario()),
        ]),
        Rect::new(
            columnas[0].x,
            columnas[0].y + columnas[0].height.saturating_sub(2) / 2,
            columnas[0].width,
            2,
        ),
    );
    frame.render_widget(
        Paragraph::new("INGRESOS ACTIVOS")
            .style(theme::foco())
            .alignment(Alignment::Center),
        centro(columnas[1]),
    );
    frame.render_widget(
        Paragraph::new("Usuario: Quintana ")
            .style(theme::texto_normal())
            .alignment(Alignment::Right),
        centro(columnas[2]),
    );
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &ActivosState) {
    let columnas: Vec<_> = state
        .columnas
        .iter()
        .filter_map(|(columna, visible)| visible.then_some(*columna))
        .collect();
    let marco = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::borde());
    let interior = marco.inner(area);
    frame.render_widget(marco, area);
    let anchos: Vec<_> = columnas
        .iter()
        .map(|columna| columna.constraint())
        .collect();
    let indices = state.indices_filtrados();
    let capacidad = interior.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(indices.len());
    let filas = indices.iter().skip(inicio).take(capacidad).enumerate().map(
        |(visible, indice_registro)| {
            let registro = &state.registros[*indice_registro];
            let seleccionado = state.seleccion == Some(inicio + visible);
            let celdas = columnas.iter().map(|columna| {
                let estilo = if seleccionado {
                    theme::seleccionado()
                } else if *columna == Columna::Tipo && registro.advertencia.is_some() {
                    theme::advertencia()
                } else {
                    theme::texto_normal()
                };
                Cell::from(valor_columna(registro, *columna)).style(estilo)
            });
            Row::new(celdas).style(if seleccionado {
                theme::seleccionado()
            } else {
                theme::texto_normal()
            })
        },
    );
    let encabezado = Row::new(columnas.iter().map(|columna| columna.titulo()))
        .style(theme::foco())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(filas, anchos)
            .header(encabezado)
            .column_spacing(1),
        interior,
    );
    if indices.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay ingresos activos que coincidan con la búsqueda.")
                .style(theme::advertencia())
                .alignment(Alignment::Center),
            interior,
        );
    }
}

fn valor_columna(registro: &IngresoActivoMock, columna: Columna) -> String {
    match columna {
        Columna::Cedula => registro.cedula.clone(),
        Columna::Nombre => registro.nombre.clone(),
        Columna::Empresa => registro.empresa.clone(),
        Columna::Tipo => format!(
            "{}{}",
            registro.tipo,
            if registro.advertencia.is_some() {
                " !"
            } else {
                ""
            }
        ),
        Columna::Hora => registro.hora_ingreso.clone(),
        Columna::Gafete => registro
            .gafete
            .map_or_else(|| "S/G".to_owned(), |gafete| format!("{gafete:02}")),
        Columna::Medio => registro.medio.clone(),
        Columna::Usuario => registro.usuario_ingreso.clone(),
    }
}

fn render_estado(frame: &mut Frame, area: Rect, state: &ActivosState) {
    let linea = match &state.modo {
        ModoActivos::Busqueda { texto } => Line::from(vec![
            Span::styled("BUSCAR ACTIVOS: ", theme::foco()),
            Span::styled(format!("{texto}_"), theme::texto_normal()),
        ]),
        _ if !state.filtro.is_empty() => Line::from(vec![
            Span::styled("FILTRO ACTIVO: ", theme::foco()),
            Span::styled(&state.filtro, theme::texto_normal()),
            Span::styled(
                format!("    {} resultados    ", state.indices_filtrados().len()),
                theme::texto_secundario(),
            ),
            Span::styled("ESC ", theme::ayuda_tecla()),
            Span::styled("Limpiar", theme::texto_normal()),
        ]),
        _ => {
            let texto = state.mensaje.clone().unwrap_or_default();
            let estilo = if texto.starts_with('✓') {
                theme::exito()
            } else {
                theme::foco()
            };
            Line::from(texto).style(estilo)
        }
    };
    frame.render_widget(Paragraph::new(linea).alignment(Alignment::Center), area);
}

fn render_pie(frame: &mut Frame, area: Rect, state: &ActivosState) {
    let hora = Local::now().format("%H:%M:%S").to_string();
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let columnas = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(40),
            Constraint::Min(20),
            Constraint::Length(12),
        ])
        .split(interior);
    let posicion = texto_posicion(state);
    frame.render_widget(
        Paragraph::new(format!(
            " {} ingresos activos │ Registro {posicion}",
            state.cantidad()
        ))
        .style(theme::texto_normal()),
        columnas[0],
    );
    let ayuda = if area.width >= 105 {
        "↑↓ Seleccionar │ ENTER Detalle │ S Salida │ F2 Gafete │ / Buscar │ C Columnas │ ESC Volver"
    } else {
        "↑↓ Mover │ ENTER Ver │ S Salida │ F2 Gafete │ / Buscar │ C Columnas"
    };
    frame.render_widget(
        Paragraph::new(ayuda)
            .style(theme::foco())
            .alignment(Alignment::Center),
        columnas[1],
    );
    frame.render_widget(
        Paragraph::new(hora)
            .style(theme::advertencia())
            .alignment(Alignment::Right)
            .block(Block::default().padding(ratatui::widgets::Padding::right(1))),
        columnas[2],
    );
}

fn render_modo(frame: &mut Frame, area: Rect, state: &ActivosState) {
    match &state.modo {
        ModoActivos::Detalle { id } => {
            if let Some(registro) = state.registro(*id) {
                render_detalle(frame, area, registro);
            }
        }
        ModoActivos::ConfirmarSalida { id } => {
            if let Some(registro) = state.registro(*id) {
                render_confirmacion(frame, area, registro);
            }
        }
        ModoActivos::SalidaPorGafete(estado) => render_gafete(frame, area, state, estado),
        ModoActivos::Columnas { seleccion } => render_columnas(frame, area, state, *seleccion),
        ModoActivos::Normal | ModoActivos::Busqueda { .. } => {}
    }
}

fn texto_posicion(state: &ActivosState) -> String {
    state.seleccion.map_or_else(
        || "—/—".to_owned(),
        |indice| format!("{}/{}", indice + 1, state.indices_filtrados().len()),
    )
}

fn render_confirmacion(frame: &mut Frame, area: Rect, registro: &IngresoActivoMock) {
    layout::render_overlay(
        frame,
        area,
        56,
        11,
        4,
        "REGISTRAR SALIDA",
        vec![
            Line::from(registro.nombre.clone()).style(theme::titulo()),
            Line::from(registro.empresa.clone()),
            Line::from(format!(
                "Gafete: {}",
                valor_columna(registro, Columna::Gafete)
            )),
            Line::from(""),
            Line::from("¿Desea registrar la salida?"),
            Line::from(""),
            Line::from("Y Sí        N No        ESC Cancelar").style(theme::foco()),
        ],
    );
}

fn render_detalle(frame: &mut Frame, area: Rect, registro: &IngresoActivoMock) {
    layout::render_overlay(
        frame,
        area,
        62,
        16,
        4,
        "DETALLE",
        vec![
            Line::from(registro.nombre.clone()).style(theme::titulo()),
            Line::from(registro.cedula.clone()).style(theme::texto_secundario()),
            Line::from(""),
            Line::from(format!("Empresa          {}", registro.empresa)),
            Line::from(format!("Tipo             {}", registro.tipo)),
            Line::from(format!("Medio            {}", registro.medio)),
            Line::from(format!(
                "Ingreso          12/08/2026 {}",
                registro.hora_ingreso
            )),
            Line::from(format!(
                "Gafete           {}",
                valor_columna(registro, Columna::Gafete)
            )),
            Line::from(format!("Registrado por   {}", registro.usuario_ingreso)),
            Line::from(registro.advertencia.clone().unwrap_or_default())
                .style(theme::advertencia()),
            Line::from(""),
            Line::from("S Salida                         ESC Cerrar").style(theme::foco()),
        ],
    );
}

fn render_gafete(frame: &mut Frame, area: Rect, state: &ActivosState, estado: &SalidaGafete) {
    let lineas = match estado {
        SalidaGafete::Capturando { numero, error } => vec![
            Line::from(format!("Gafete: {numero}_")).style(theme::foco()),
            Line::from(""),
            Line::from(error.clone().unwrap_or_default()).style(theme::error()),
            Line::from(""),
            Line::from("ENTER Buscar                  ESC Cancelar").style(theme::foco()),
        ],
        SalidaGafete::Encontrado { id } => {
            let Some(registro) = state.registro(*id) else {
                return;
            };
            vec![
                Line::from(registro.nombre.clone()).style(theme::titulo()),
                Line::from(registro.empresa.clone()),
                Line::from(format!(
                    "Gafete: {}",
                    valor_columna(registro, Columna::Gafete)
                )),
                Line::from(format!("Ingreso: {}", registro.hora_ingreso)),
                Line::from(""),
                Line::from("¿Registrar salida?"),
                Line::from("Y Sí        N No        ESC Cancelar").style(theme::foco()),
            ]
        }
    };
    layout::render_overlay(frame, area, 58, 11, 4, "SALIDA POR GAFETE", lineas);
}

fn render_columnas(frame: &mut Frame, area: Rect, state: &ActivosState, seleccion: usize) {
    let mut lineas = Vec::new();
    for (indice, (columna, visible)) in state.columnas.iter().enumerate() {
        lineas.push(
            Line::from(format!(
                "{} [{}] {}",
                if indice == seleccion { ">" } else { " " },
                if *visible { "x" } else { " " },
                columna.titulo()
            ))
            .style(if indice == seleccion {
                theme::foco()
            } else {
                theme::texto_normal()
            }),
        );
    }
    lineas.push(Line::from(""));
    if let Some(mensaje) = &state.mensaje {
        lineas.push(Line::from(mensaje.clone()).style(theme::advertencia()));
    }
    lineas.push(Line::from("↑↓ mover  SPACE mostrar/ocultar  ESC cerrar").style(theme::foco()));
    layout::render_overlay(frame, area, 48, 15, 4, "COLUMNAS", lineas);
}
