use chrono::{Local, NaiveDate};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
};

use super::*;
use crate::tui::{contratistas_mock::ContratistaMock, layout, theme};

const HOY_MOCK: &str = "12/08/2026";

pub fn render(frame: &mut Frame, area: Rect, state: &ContratistasState) {
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
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(interior);
    let centro = |a: Rect| Rect::new(a.x, a.y + a.height / 2, a.width, 1);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(" B R I S A S   C L I").style(theme::titulo()),
            Line::from(" CONTROL DE ACCESO").style(theme::texto_secundario()),
        ]),
        Rect::new(
            cols[0].x,
            cols[0].y + cols[0].height.saturating_sub(2) / 2,
            cols[0].width,
            2,
        ),
    );
    frame.render_widget(
        Paragraph::new("BASE DE CONTRATISTAS")
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

fn render_tabla(frame: &mut Frame, area: Rect, state: &ContratistasState) {
    let columnas: Vec<_> = state
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
        .map(|(visible, i)| {
            let c = &state.registros[*i];
            let seleccionado = state.seleccion == Some(inicio + visible);
            let celdas = columnas.iter().map(|col| {
                Cell::from(valor(c, *col)).style(if seleccionado {
                    theme::seleccionado()
                } else {
                    estilo(c, *col)
                })
            });
            Row::new(celdas).style(if seleccionado {
                theme::seleccionado()
            } else {
                theme::texto_normal()
            })
        });
    let encabezado = Row::new(columnas.iter().map(|c| c.titulo()))
        .style(theme::foco())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(
            filas,
            columnas.iter().map(|c| c.constraint()).collect::<Vec<_>>(),
        )
        .header(encabezado)
        .column_spacing(1),
        interior,
    );
    if indices.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay contratistas que coincidan con la búsqueda.")
                .style(theme::advertencia())
                .alignment(Alignment::Center),
            interior,
        );
    }
}

fn valor(c: &ContratistaMock, col: Columna) -> String {
    match col {
        Columna::Cedula => c.cedula.clone(),
        Columna::Nombre => c.nombre.clone(),
        Columna::Empresa => c.empresa.clone(),
        Columna::Tipo => c.tipo_ingreso.texto().into(),
        Columna::Praind => c.fecha_praind.clone().unwrap_or_else(|| "--".into()),
        Columna::Ruta => si_no(c.personal_ruta).into(),
        Columna::Acceso => si_no(c.tiene_acceso).into(),
    }
}
fn si_no(v: bool) -> &'static str {
    if v { "SÍ" } else { "NO" }
}
fn estilo(c: &ContratistaMock, col: Columna) -> ratatui::style::Style {
    match col {
        Columna::Acceso if c.tiene_acceso => theme::exito(),
        Columna::Acceso => theme::error(),
        Columna::Praind => estilo_fecha(c.fecha_praind.as_deref()),
        _ => theme::texto_normal(),
    }
}
fn estilo_fecha(fecha: Option<&str>) -> ratatui::style::Style {
    let Some(fecha) = fecha.and_then(|f| NaiveDate::parse_from_str(f, "%d/%m/%Y").ok()) else {
        return theme::texto_secundario();
    };
    let hoy = NaiveDate::parse_from_str(HOY_MOCK, "%d/%m/%Y").expect("fecha mock válida");
    let dias = (fecha - hoy).num_days();
    if dias < 0 {
        theme::error()
    } else if dias <= 30 {
        theme::advertencia()
    } else {
        theme::exito()
    }
}

fn render_estado(frame: &mut Frame, area: Rect, state: &ContratistasState) {
    let linea = match &state.modo {
        ModoContratistas::Busqueda { texto } => Line::from(vec![
            Span::styled("BUSCAR CONTRATISTAS: ", theme::foco()),
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
        _ => Line::from(state.mensaje.clone().unwrap_or_default()).style(theme::exito()),
    };
    frame.render_widget(Paragraph::new(linea).alignment(Alignment::Center), area);
}
fn render_pie(frame: &mut Frame, area: Rect, state: &ContratistasState) {
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(42),
            Constraint::Min(20),
            Constraint::Length(12),
        ])
        .split(interior);
    let posicion = state.seleccion.map_or_else(
        || "—/—".into(),
        |i| format!("{}/{}", i + 1, state.indices_filtrados().len()),
    );
    frame.render_widget(
        Paragraph::new(format!(
            " {} contratistas │ Registro {posicion}",
            state.registros.len()
        )),
        cols[0],
    );
    frame.render_widget(Paragraph::new("↑↓ Seleccionar │ ENTER Detalle │ N Nuevo │ E Editar │ / Buscar │ C Columnas │ ESC Volver").style(theme::foco()).alignment(Alignment::Center), cols[1]);
    frame.render_widget(
        Paragraph::new(Local::now().format("%H:%M:%S").to_string())
            .style(theme::advertencia())
            .alignment(Alignment::Right)
            .block(Block::default().padding(ratatui::widgets::Padding::right(1))),
        cols[2],
    );
}

fn render_modo(frame: &mut Frame, area: Rect, state: &ContratistasState) {
    match &state.modo {
        ModoContratistas::Detalle { id } => {
            if let Some(c) = state.registro(*id) {
                render_detalle(frame, area, c);
            }
        }
        ModoContratistas::Formulario(f) => render_formulario(frame, area, f),
        ModoContratistas::Columnas { seleccion } => render_columnas(frame, area, state, *seleccion),
        _ => {}
    }
}
fn render_detalle(frame: &mut Frame, area: Rect, c: &ContratistaMock) {
    layout::render_overlay(
        frame,
        area,
        66,
        17,
        4,
        "DETALLE DEL CONTRATISTA",
        vec![
            Line::from(c.nombre.clone()).style(theme::titulo()),
            Line::from(""),
            Line::from(format!("Cédula                 {}", c.cedula)),
            Line::from(format!("Empresa                {}", c.empresa)),
            Line::from(format!("Tipo de ingreso        {}", c.tipo_ingreso.texto())),
            Line::from(format!(
                "Fecha PRAIND           {}",
                c.fecha_praind.as_deref().unwrap_or("No requerida")
            )),
            Line::from(format!("Personal de ruta       {}", si_no(c.personal_ruta))),
            Line::from(format!("Tiene acceso           {}", si_no(c.tiene_acceso))),
            Line::from(""),
            Line::from("ESC Cerrar").style(theme::foco()),
        ],
    );
}
fn render_formulario(frame: &mut Frame, area: Rect, f: &FormularioContratista) {
    let titulo = match f.modo {
        ModoFormulario::Crear => "NUEVO CONTRATISTA",
        ModoFormulario::Editar { .. } => "EDITAR CONTRATISTA",
    };
    let valores = [
        format!("{}{}", f.cedula, cursor(f, 0)),
        format!("{}{}", f.nombre, cursor(f, 1)),
        EMPRESAS[f.empresa].into(),
        f.tipo().texto().into(),
        if f.requiere_praind() {
            format!("{}{}", f.fecha_praind, cursor(f, 4))
        } else {
            "No requerida".into()
        },
        si_no(f.personal_ruta).into(),
        si_no(f.tiene_acceso).into(),
    ];
    let nombres = [
        "Cédula",
        "Nombre",
        "Empresa",
        "Tipo de ingreso",
        "Fecha PRAIND",
        "Personal de ruta",
        "Tiene acceso",
    ];
    let mut lineas = vec![Line::from("")];
    for (i, nombre) in nombres.iter().enumerate() {
        lineas.push(
            Line::from(format!(
                "{} {:<21} {}",
                if f.campo == i { ">" } else { " " },
                nombre,
                valores[i]
            ))
            .style(if f.campo == i {
                theme::foco()
            } else {
                theme::texto_normal()
            }),
        );
        lineas.push(Line::from(""));
    }
    lineas.push(Line::from(f.error.clone().unwrap_or_default()).style(theme::error()));
    lineas.push(Line::from("↑↓/Tab Navegar   ENTER Seleccionar/Cambiar").style(theme::foco()));
    lineas.push(
        Line::from("G Guardar                              ESC Cancelar").style(theme::foco()),
    );
    layout::render_overlay(frame, area, 76, 24, 4, titulo, lineas);
    if let Some((tipo, opcion)) = f.desplegable {
        render_desplegable(frame, area, tipo, opcion);
    }
}
fn cursor(f: &FormularioContratista, campo: usize) -> &'static str {
    if f.campo == campo { "_" } else { "" }
}
fn render_desplegable(frame: &mut Frame, area: Rect, tipo: Desplegable, opcion: usize) {
    let (titulo, opciones): (&str, Vec<&str>) = match tipo {
        Desplegable::Empresa => ("SELECCIONAR EMPRESA", EMPRESAS.to_vec()),
        Desplegable::Tipo => (
            "SELECCIONAR TIPO",
            TipoIngresoMock::TODOS.iter().map(|t| t.texto()).collect(),
        ),
    };
    let mut lineas: Vec<_> = opciones
        .iter()
        .enumerate()
        .map(|(i, o)| {
            Line::from(format!("{} {o}", if i == opcion { ">" } else { " " })).style(
                if i == opcion {
                    theme::seleccionado()
                } else {
                    theme::texto_normal()
                },
            )
        })
        .collect();
    lineas.push(Line::from(""));
    lineas.push(Line::from("↑↓ Seleccionar   ENTER Aceptar   ESC Cancelar").style(theme::foco()));
    let alto = lineas.len() as u16 + 4;
    layout::render_overlay(frame, area, 54, alto, 4, titulo, lineas);
}
fn render_columnas(frame: &mut Frame, area: Rect, state: &ContratistasState, seleccion: usize) {
    let mut lineas: Vec<_> = state
        .columnas
        .iter()
        .enumerate()
        .map(|(i, (c, v))| {
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
            })
        })
        .collect();
    lineas.push(Line::from(""));
    if let Some(m) = &state.mensaje {
        lineas.push(Line::from(m.clone()).style(theme::advertencia()));
    }
    lineas.push(Line::from("↑↓ mover  SPACE mostrar/ocultar  ESC cerrar").style(theme::foco()));
    layout::render_overlay(frame, area, 48, 15, 4, "COLUMNAS", lineas);
}
