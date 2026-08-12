use chrono::Local;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use super::*;
use crate::tui::{layout, theme};

pub fn render(frame: &mut Frame, area: Rect, state: &HistorialState) {
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
            Constraint::Length(2),
            Constraint::Length(3),
        ])
        .split(contenido);
    render_cabecera(frame, zonas[0]);
    render_rango(frame, zonas[1], state);
    render_tabla(frame, zonas[2], state);
    render_estado(frame, zonas[3], state);
    render_pie(frame, zonas[4], state);
    render_modo(frame, contenido, state);
}

fn render_cabecera(frame: &mut Frame, area: Rect) {
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(interior);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(" B R I S A S   C L I").style(theme::titulo()),
            Line::from(" CONTROL DE ACCESO").style(theme::texto_secundario()),
        ]),
        Rect::new(cols[0].x, cols[0].y, cols[0].width, 2),
    );
    let centro = |a: Rect| Rect::new(a.x, a.y + a.height / 2, a.width, 1);
    frame.render_widget(
        Paragraph::new("HISTORIAL DE INGRESOS")
            .style(theme::foco())
            .alignment(Alignment::Center),
        centro(cols[1]),
    );
    frame.render_widget(
        Paragraph::new("Usuario: Quintana ")
            .style(theme::texto_normal())
            .alignment(Alignment::Right),
        centro(cols[2]),
    );
}

fn render_rango(frame: &mut Frame, area: Rect, state: &HistorialState) {
    let n = state.indices_filtrados().len();
    let mut filtros = Vec::new();
    if state.filtro_aplicado.empresa != "Todas" {
        filtros.push(format!("Empresa: {}", state.filtro_aplicado.empresa));
    }
    if state.filtro_aplicado.tipo != "Todos" {
        filtros.push(format!("Tipo: {}", state.filtro_aplicado.tipo));
    }
    if state.filtro_aplicado.estado != EstadoFiltro::Todos {
        filtros.push(format!("Estado: {:?}", state.filtro_aplicado.estado));
    }
    let resumen = match filtros.len() {
        0 => String::new(),
        1 | 2 => format!("    {}", filtros.join(" │ ")),
        cantidad => format!("    Filtros activos: {cantidad}"),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Desde: ", theme::foco()),
            Span::styled(&state.filtro_aplicado.desde, theme::texto_normal()),
            Span::raw("    "),
            Span::styled("Hasta: ", theme::foco()),
            Span::styled(&state.filtro_aplicado.hasta, theme::texto_normal()),
            Span::styled(
                format!("    {n} resultados{resumen}"),
                theme::texto_secundario(),
            ),
        ]))
        .alignment(Alignment::Center),
        area,
    );
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &HistorialState) {
    let cols: Vec<_> = state
        .columnas
        .iter()
        .filter_map(|(c, v)| v.then_some(*c))
        .collect();
    let marco = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = marco.inner(area);
    frame.render_widget(marco, area);
    let indices = state.indices_filtrados();
    let capacidad = interior.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(indices.len());
    let filas = indices
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(v, i)| {
            let r = &state.registros[*i];
            let sel = state.seleccion == Some(inicio + v);
            Row::new(cols.iter().map(|c| Cell::from(valor(r, *c)))).style(if sel {
                theme::seleccionado()
            } else {
                theme::texto_normal()
            })
        });
    let header = Row::new(cols.iter().map(|c| c.titulo()))
        .style(theme::foco())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(filas, cols.iter().map(|c| c.constraint()))
            .header(header)
            .column_spacing(1),
        interior,
    );
    if indices.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin registros para los filtros seleccionados")
                .style(theme::advertencia())
                .alignment(Alignment::Center),
            interior,
        );
    }
}

pub(super) fn valor(r: &MovimientoHistorialMock, c: ColumnaHistorial) -> String {
    match c {
        ColumnaHistorial::Fecha => r.fecha.format("%d/%m/%y").to_string(),
        ColumnaHistorial::Cedula => r.cedula.clone(),
        ColumnaHistorial::Nombre => r.nombre.clone(),
        ColumnaHistorial::Empresa => r.empresa.clone(),
        ColumnaHistorial::Tipo => r.tipo.clone(),
        ColumnaHistorial::Entrada => r.entrada.clone(),
        ColumnaHistorial::Salida => r.salida.clone().unwrap_or_else(|| "--".into()),
        ColumnaHistorial::Gafete => r.gafete.map_or_else(|| "S/G".into(), |g| format!("{g:02}")),
        ColumnaHistorial::Medio => r.medio.clone(),
        ColumnaHistorial::UsuarioIngreso => r.usuario_ingreso.clone(),
        ColumnaHistorial::UsuarioSalida => r.usuario_salida.clone().unwrap_or_else(|| "--".into()),
    }
}

fn render_estado(frame: &mut Frame, area: Rect, state: &HistorialState) {
    let linea = match &state.modo {
        ModoHistorial::Busqueda { texto } => Line::from(vec![
            Span::styled("BUSCAR HISTORIAL: ", theme::foco()),
            Span::styled(format!("{texto}_"), theme::texto_normal()),
        ]),
        _ if !state.busqueda.is_empty() => Line::from(vec![
            Span::styled("BÚSQUEDA ACTIVA: ", theme::foco()),
            Span::styled(&state.busqueda, theme::texto_normal()),
            Span::styled(
                format!("    {} resultados    ", state.indices_filtrados().len()),
                theme::texto_secundario(),
            ),
            Span::styled("ESC ", theme::ayuda_tecla()),
            Span::styled("Limpiar", theme::texto_normal()),
        ]),
        _ => Line::from(state.mensaje.clone().unwrap_or_default()),
    };
    frame.render_widget(Paragraph::new(linea).alignment(Alignment::Center), area);
}

fn render_pie(frame: &mut Frame, area: Rect, state: &HistorialState) {
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(35),
            Constraint::Min(20),
            Constraint::Length(12),
        ])
        .split(interior);
    let n = state.indices_filtrados().len();
    let pos = state
        .seleccion
        .map_or_else(|| "—/—".into(), |i| format!("{}/{}", i + 1, n));
    frame.render_widget(
        Paragraph::new(format!(" {n} registros │ Registro {pos}")).style(theme::texto_normal()),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(
            "↑↓ Seleccionar │ ENTER Detalle │ / Buscar │ F Filtros │ C Columnas │ ESC Volver",
        )
        .style(theme::foco())
        .alignment(Alignment::Center),
        cols[1],
    );
    frame.render_widget(
        Paragraph::new(Local::now().format("%H:%M:%S").to_string())
            .style(theme::advertencia())
            .alignment(Alignment::Right)
            .block(Block::default().padding(ratatui::widgets::Padding::right(1))),
        cols[2],
    );
}

fn render_modo(frame: &mut Frame, area: Rect, state: &HistorialState) {
    match &state.modo {
        ModoHistorial::Detalle { id } => {
            if let Some(r) = state.registro(*id) {
                layout::render_overlay(
                    frame,
                    area,
                    64,
                    17,
                    2,
                    "DETALLE DE MOVIMIENTO",
                    vec![
                        Line::from(r.nombre.clone()).style(theme::titulo()),
                        Line::from(r.cedula.clone()).style(theme::texto_secundario()),
                        Line::from(""),
                        Line::from(format!("Empresa          {}", r.empresa)),
                        Line::from(format!("Tipo             {}", r.tipo)),
                        Line::from(format!("Medio            {}", r.medio)),
                        Line::from(format!(
                            "Entrada          {} {}",
                            r.fecha.format("%d/%m/%Y"),
                            r.entrada
                        )),
                        Line::from(format!("Usuario ingreso  {}", r.usuario_ingreso)),
                        Line::from(format!(
                            "Salida           {}",
                            r.salida.as_ref().map_or("--".into(), |s| format!(
                                "{} {s}",
                                r.fecha.format("%d/%m/%Y")
                            ))
                        )),
                        Line::from(format!(
                            "Usuario salida   {}",
                            r.usuario_salida.as_deref().unwrap_or("--")
                        )),
                        Line::from(format!(
                            "Gafete           {}",
                            r.gafete.map_or_else(|| "S/G".into(), |g| format!("{g:02}"))
                        )),
                        Line::from(""),
                        Line::from("ESC Cerrar").style(theme::foco()),
                    ],
                );
            }
        }
        ModoHistorial::Filtros {
            seleccion,
            editando,
        } => render_filtros(frame, area, state, *seleccion, *editando),
        ModoHistorial::Desplegable {
            campo,
            seleccion_filtro,
            opcion,
        } => {
            render_filtros(frame, area, state, *seleccion_filtro, false);
            render_desplegable(frame, area, *campo, *opcion);
        }
        ModoHistorial::Columnas { seleccion } => render_columnas(frame, area, state, *seleccion),
        _ => {}
    }
}

fn render_filtros(
    frame: &mut Frame,
    area: Rect,
    state: &HistorialState,
    seleccion: usize,
    editando: bool,
) {
    let f = &state.filtro_edicion;
    let valores = [
        &f.desde,
        &f.hasta,
        &f.nombre_cedula,
        &f.empresa,
        &f.tipo,
        &f.gafete,
    ];
    let nombres = [
        "Desde",
        "Hasta",
        "Nombre/Cédula",
        "Empresa",
        "Tipo",
        "Gafete",
        "Estado",
    ];
    let mut lineas = Vec::new();
    for (i, nombre) in nombres.iter().enumerate() {
        let valor = if i < 6 {
            valores[i].as_str()
        } else {
            match f.estado {
                EstadoFiltro::Todos => "Todos",
                EstadoFiltro::Cerrados => "Cerrados",
                EstadoFiltro::Activos => "Activos",
            }
        };
        lineas.push(
            Line::from(format!(
                "{} {:<15} {}{}",
                if i == seleccion { ">" } else { " " },
                nombre,
                valor,
                if i == seleccion && editando { "_" } else { "" }
            ))
            .style(if i == seleccion {
                theme::foco()
            } else {
                theme::texto_normal()
            }),
        );
    }
    lineas.extend([
        Line::from(""),
        Line::from("↑↓ Mover   ENTER Editar/Seleccionar").style(theme::foco()),
        Line::from("A Aplicar   L Limpiar   ESC Cerrar").style(theme::foco()),
    ]);
    layout::render_overlay(frame, area, 58, 15, 2, "FILTROS", lineas);
}

fn render_desplegable(frame: &mut Frame, area: Rect, campo: CampoFiltro, opcion: usize) {
    let titulo = match campo {
        CampoFiltro::Empresa => "SELECCIONAR EMPRESA",
        CampoFiltro::Tipo => "SELECCIONAR TIPO",
        CampoFiltro::Estado => "SELECCIONAR ESTADO",
        _ => return,
    };
    let mut lineas: Vec<_> = opciones_campo(campo)
        .iter()
        .enumerate()
        .map(|(indice, valor)| {
            Line::from(format!(
                "{} {valor}",
                if indice == opcion { ">" } else { " " }
            ))
            .style(if indice == opcion {
                theme::seleccionado()
            } else {
                theme::texto_normal()
            })
        })
        .collect();
    lineas.push(Line::from(""));
    lineas.push(Line::from("↑↓ Seleccionar   ENTER Aceptar   ESC Cancelar").style(theme::foco()));
    let alto = (lineas.len() as u16 + 4).min(area.height.saturating_sub(2));
    layout::render_overlay(frame, area, 52, alto, 2, titulo, lineas);
}

fn render_columnas(frame: &mut Frame, area: Rect, state: &HistorialState, seleccion: usize) {
    let mut lineas = Vec::new();
    for (i, (c, v)) in state.columnas.iter().enumerate() {
        lineas.push(
            Line::from(format!(
                "{} [{}] {}",
                if i == seleccion { ">" } else { " " },
                if *v { "x" } else { " " },
                c.titulo()
            ))
            .style(if i == seleccion {
                theme::foco()
            } else {
                theme::texto_normal()
            }),
        );
    }
    lineas.push(Line::from("↑↓ mover  SPACE mostrar/ocultar  ESC cerrar").style(theme::foco()));
    layout::render_overlay(frame, area, 50, 17, 2, "COLUMNAS", lineas);
}
