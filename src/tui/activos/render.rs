use chrono::Local;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
};

use super::*;
use crate::{
    services::registro_ingreso_service::IngresoActivoResumen,
    tui::{layout, theme},
};

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
    let capacidad = interior.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.registros.len());
    let filas = state
        .registros
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, registro)| {
            let seleccionado = state.seleccion == Some(inicio + visible);
            let celdas = columnas.iter().map(|columna| {
                let estilo = if seleccionado {
                    theme::seleccionado()
                } else if *columna == Columna::Tipo
                    && matches!(
                        registro.resultado_acceso,
                        crate::domain::resultado_acceso::ResultadoAcceso::PermitidoConAdvertencia
                            | crate::domain::resultado_acceso::ResultadoAcceso::Denegado(_)
                    )
                {
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
        });
    let encabezado = Row::new(columnas.iter().map(|columna| columna.titulo()))
        .style(theme::foco())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(filas, anchos)
            .header(encabezado)
            .column_spacing(1),
        interior,
    );
    if state.registros.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay ingresos activos que coincidan con la búsqueda.")
                .style(theme::advertencia())
                .alignment(Alignment::Center),
            interior,
        );
    }
}

fn valor_columna(registro: &IngresoActivoResumen, columna: Columna) -> String {
    match columna {
        Columna::Cedula => registro.cedula.clone(),
        Columna::Nombre => registro.contratista_nombre.clone(),
        Columna::Empresa => registro.empresa_nombre.clone(),
        Columna::Tipo => format!(
            "{}{}",
            texto_tipo(registro.tipo_ingreso),
            if !matches!(
                registro.resultado_acceso,
                crate::domain::resultado_acceso::ResultadoAcceso::Permitido
            ) {
                " !"
            } else {
                ""
            }
        ),
        Columna::Hora => registro.fecha_hora_ingreso.format("%H:%M").to_string(),
        Columna::Gafete => registro
            .gafete_numero
            .map_or_else(|| "S/G".to_owned(), |gafete| format!("{gafete:02}")),
        Columna::Medio => texto_medio(registro.medio_ingreso).into(),
        Columna::Usuario => registro.usuario_ingreso_nombre.clone(),
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
                format!(
                    "    {} resultados de {} personas dentro    ",
                    state.cantidad(),
                    state.total_activos()
                ),
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
            " {} personas dentro │ Registro {posicion}",
            state.total_activos()
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
        ModoActivos::SalidaPorGafete(estado) => render_gafete(frame, area, estado),
        ModoActivos::Columnas { seleccion } => render_columnas(frame, area, state, *seleccion),
        ModoActivos::Normal | ModoActivos::Busqueda { .. } => {}
    }
}

fn texto_posicion(state: &ActivosState) -> String {
    state.seleccion.map_or_else(
        || "—/—".to_owned(),
        |indice| format!("{}/{}", indice + 1, state.registros.len()),
    )
}

fn render_confirmacion(frame: &mut Frame, area: Rect, registro: &IngresoActivoResumen) {
    layout::render_overlay(
        frame,
        area,
        56,
        11,
        4,
        "REGISTRAR SALIDA",
        vec![
            Line::from(registro.contratista_nombre.clone()).style(theme::titulo()),
            Line::from(registro.empresa_nombre.clone()),
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

fn render_detalle(frame: &mut Frame, area: Rect, registro: &IngresoActivoResumen) {
    layout::render_overlay(
        frame,
        area,
        62,
        16,
        4,
        "DETALLE",
        vec![
            Line::from(registro.contratista_nombre.clone()).style(theme::titulo()),
            Line::from(registro.cedula.clone()).style(theme::texto_secundario()),
            Line::from(""),
            Line::from(format!("Empresa          {}", registro.empresa_nombre)),
            Line::from(format!(
                "Tipo             {}",
                texto_tipo(registro.tipo_ingreso)
            )),
            Line::from(format!(
                "Medio            {}",
                texto_medio(registro.medio_ingreso)
            )),
            Line::from(format!(
                "Ingreso          {}",
                registro.fecha_hora_ingreso.format("%d/%m/%Y %H:%M")
            )),
            Line::from(format!(
                "Gafete           {}",
                valor_columna(registro, Columna::Gafete)
            )),
            Line::from(format!(
                "Registrado por   {}",
                registro.usuario_ingreso_nombre
            )),
            Line::from(
                if matches!(
                    registro.resultado_acceso,
                    crate::domain::resultado_acceso::ResultadoAcceso::Permitido
                ) {
                    ""
                } else {
                    "Condición de acceso actual requiere atención"
                },
            )
            .style(theme::advertencia()),
            Line::from(""),
            Line::from("S Salida                         ESC Cerrar").style(theme::foco()),
        ],
    );
}

fn render_gafete(frame: &mut Frame, area: Rect, estado: &SalidaGafete) {
    let lineas = match estado {
        SalidaGafete::Capturando { numero, error } => vec![
            Line::from(format!("Gafete: {numero}_")).style(theme::foco()),
            Line::from(""),
            Line::from(error.clone().unwrap_or_default()).style(theme::error()),
            Line::from(""),
            Line::from("ENTER Buscar                  ESC Cancelar").style(theme::foco()),
        ],
        SalidaGafete::Encontrado { registro } => vec![
            Line::from(registro.contratista_nombre.clone()).style(theme::titulo()),
            Line::from(registro.empresa_nombre.clone()),
            Line::from(format!(
                "Gafete: {}",
                valor_columna(registro, Columna::Gafete)
            )),
            Line::from(format!(
                "Ingreso: {}",
                registro.fecha_hora_ingreso.format("%H:%M")
            )),
            Line::from(""),
            Line::from("¿Registrar salida?"),
            Line::from("Y Sí        N No        ESC Cancelar").style(theme::foco()),
        ],
    };
    layout::render_overlay(frame, area, 58, 11, 4, "SALIDA POR GAFETE", lineas);
}
fn texto_tipo(t: crate::models::tipo_ingreso::TipoIngreso) -> &'static str {
    match t {
        crate::models::tipo_ingreso::TipoIngreso::Praind => "PRAIND",
        crate::models::tipo_ingreso::TipoIngreso::InHouse => "IN HOUSE",
        crate::models::tipo_ingreso::TipoIngreso::PorCorreo => "POR CORREO",
        crate::models::tipo_ingreso::TipoIngreso::Swat => "SWAT",
    }
}
fn texto_medio(m: crate::models::medio_ingreso::MedioIngreso) -> &'static str {
    match m {
        crate::models::medio_ingreso::MedioIngreso::Caminando => "Caminando",
        crate::models::medio_ingreso::MedioIngreso::Vehiculo => "Vehículo",
    }
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
