use crate::tiempo::{a_costa_rica, hora_actual_texto};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use super::*;
use crate::{
    database::queries::ingresos::MovimientoIngresoResumen,
    tui::{layout, theme},
};

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
    let zonas = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(2),
        Constraint::Min(8),
        Constraint::Length(2),
        Constraint::Length(3),
    ])
    .split(contenido);
    render_cabecera(frame, zonas[0], state);
    render_rango(frame, zonas[1], state);
    render_tabla(frame, zonas[2], state);
    render_estado(frame, zonas[3], state);
    render_pie(frame, zonas[4], state);
    render_modo(frame, contenido, state);
}

fn render_cabecera(frame: &mut Frame, area: Rect, state: &HistorialState) {
    let bloque = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let cols = Layout::horizontal([
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
        cols[0],
    );
    let centro = |a: Rect| Rect::new(a.x, a.y + a.height / 2, a.width, 1);
    frame.render_widget(
        Paragraph::new("HISTORIAL DE INGRESOS")
            .style(theme::foco())
            .alignment(Alignment::Center),
        centro(cols[1]),
    );
    frame.render_widget(
        Paragraph::new(format!("Usuario: {} ", state.usuario_nombre)).alignment(Alignment::Right),
        centro(cols[2]),
    );
}

fn render_rango(frame: &mut Frame, area: Rect, state: &HistorialState) {
    let f = &state.filtro_aplicado;
    let mut extras = Vec::new();
    if f.empresa_id.is_some() {
        extras.push(format!(
            "Empresa: {}",
            empresa_texto(f.empresa_id, &state.empresas)
        ));
    }
    if f.tipo.is_some() {
        extras.push(format!("Tipo: {}", tipo_texto(f.tipo)));
    }
    if f.estado != EstadoMovimiento::Todos {
        extras.push(format!("Estado: {}", estado_texto(f.estado)));
    }
    let resumen = if extras.is_empty() {
        String::new()
    } else {
        format!("    {}", extras.join(" │ "))
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Desde: ", theme::foco()),
            Span::raw(&f.desde),
            Span::raw("    "),
            Span::styled("Hasta: ", theme::foco()),
            Span::raw(&f.hasta),
            Span::styled(
                format!("    {} resultados{resumen}", state.total),
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
        .filter_map(|(c, visible)| visible.then_some(*c))
        .collect();
    let marco = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::borde());
    let interior = marco.inner(area);
    frame.render_widget(marco, area);
    let capacidad = interior.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.registros.len());
    let filas = state
        .registros
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, r)| {
            let seleccionada = state.seleccion == Some(inicio + visible);
            Row::new(cols.iter().map(|c| Cell::from(valor(r, *c)))).style(if seleccionada {
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
    if state.registros.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin registros para los filtros seleccionados")
                .style(theme::advertencia())
                .alignment(Alignment::Center),
            interior,
        );
    }
}

pub(super) fn valor(r: &MovimientoIngresoResumen, c: ColumnaHistorial) -> String {
    match c {
        ColumnaHistorial::Fecha => a_costa_rica(r.fecha_hora_ingreso)
            .format("%d/%m/%y")
            .to_string(),
        ColumnaHistorial::Cedula => r.cedula.clone(),
        ColumnaHistorial::Nombre => r.contratista_nombre.clone(),
        ColumnaHistorial::Empresa => r.empresa_nombre.clone(),
        ColumnaHistorial::Tipo => tipo_texto(Some(r.tipo_ingreso)).into(),
        ColumnaHistorial::Entrada => a_costa_rica(r.fecha_hora_ingreso)
            .format("%H:%M")
            .to_string(),
        ColumnaHistorial::Salida => r.fecha_hora_salida.map_or_else(
            || "--".into(),
            |f| a_costa_rica(f).format("%H:%M").to_string(),
        ),
        ColumnaHistorial::Gafete => r
            .gafete_numero
            .map_or_else(|| "S/G".into(), |g| g.to_string()),
        ColumnaHistorial::Medio => format!("{:?}", r.medio_ingreso),
        ColumnaHistorial::UsuarioIngreso => r.usuario_ingreso_nombre.clone(),
        ColumnaHistorial::UsuarioSalida => r
            .usuario_salida_nombre
            .clone()
            .unwrap_or_else(|| "--".into()),
    }
}

fn render_estado(frame: &mut Frame, area: Rect, state: &HistorialState) {
    let linea = match &state.modo {
        ModoHistorial::Busqueda { texto } => Line::from(vec![
            Span::styled("BUSCAR HISTORIAL: ", theme::foco()),
            Span::raw(format!("{texto}_")),
        ]),
        _ if !state.busqueda.is_empty() => Line::from(vec![
            Span::styled("BÚSQUEDA ACTIVA: ", theme::foco()),
            Span::raw(&state.busqueda),
            Span::styled(
                format!("    {} resultados    ", state.total),
                theme::texto_secundario(),
            ),
            Span::styled("ESC ", theme::ayuda_tecla()),
            Span::raw("Limpiar"),
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
    let cols = Layout::horizontal([
        Constraint::Length(42),
        Constraint::Min(20),
        Constraint::Length(12),
    ])
    .split(interior);
    let pos = state.seleccion.map_or_else(
        || "—/—".into(),
        |i| format!("{}/{}", state.offset + i + 1, state.total),
    );
    let (pagina, paginas) = state.pagina();
    frame.render_widget(
        Paragraph::new(format!(
            " {} registros │ Registro {pos} │ Página {pagina}/{paginas}",
            state.total
        )),
        cols[0],
    );
    frame.render_widget(Paragraph::new("↑↓ Seleccionar │ PgUp/PgDn Página │ ENTER Detalle │ / Buscar │ F Filtros │ C Columnas │ ESC Volver").style(theme::foco()).alignment(Alignment::Center), cols[1]);
    frame.render_widget(
        Paragraph::new(hora_actual_texto())
            .style(theme::advertencia())
            .alignment(Alignment::Right),
        cols[2],
    );
}

fn render_modo(frame: &mut Frame, area: Rect, state: &HistorialState) {
    match &state.modo {
        ModoHistorial::Detalle { id } => {
            if let Some(r) = state.registro(*id) {
                let salida = r.fecha_hora_salida.map_or_else(
                    || "--".into(),
                    |f| a_costa_rica(f).format("%d/%m/%Y %H:%M").to_string(),
                );
                layout::render_overlay(
                    frame,
                    area,
                    68,
                    19,
                    2,
                    "DETALLE DE MOVIMIENTO",
                    vec![
                        Line::from(r.contratista_nombre.clone()).style(theme::titulo()),
                        Line::from(r.cedula.clone()),
                        Line::from(""),
                        Line::from(format!("Empresa          {}", r.empresa_nombre)),
                        Line::from(format!(
                            "Tipo             {}",
                            tipo_texto(Some(r.tipo_ingreso))
                        )),
                        Line::from(format!("Medio            {:?}", r.medio_ingreso)),
                        Line::from(format!(
                            "Entrada          {}",
                            a_costa_rica(r.fecha_hora_ingreso).format("%d/%m/%Y %H:%M")
                        )),
                        Line::from(format!("Usuario ingreso  {}", r.usuario_ingreso_nombre)),
                        Line::from(format!("Evaluación       {}", texto_evaluacion(r))),
                        Line::from(format!(
                            "Reglas           {}",
                            if r.reglas_version == 0 {
                                "Registro migrado".to_owned()
                            } else {
                                format!("Versión {}", r.reglas_version)
                            }
                        )),
                        Line::from(format!("Salida           {salida}")),
                        Line::from(format!(
                            "Usuario salida   {}",
                            r.usuario_salida_nombre.as_deref().unwrap_or("--")
                        )),
                        Line::from(format!(
                            "Gafete           {}",
                            r.gafete_numero
                                .map_or_else(|| "S/G".into(), |g| g.to_string())
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
            render_desplegable(frame, area, state, *campo, *opcion);
        }
        ModoHistorial::Columnas { seleccion } => render_columnas(frame, area, state, *seleccion),
        _ => {}
    }
}

fn texto_evaluacion(r: &MovimientoIngresoResumen) -> &'static str {
    match r.resultado_acceso {
        crate::models::registro_ingreso::ResultadoIngresoRegistrado::Permitido => "Permitido",
        crate::models::registro_ingreso::ResultadoIngresoRegistrado::PermitidoConAdvertencia => {
            "Permitido con advertencia PRAIND"
        }
        crate::models::registro_ingreso::ResultadoIngresoRegistrado::Migrado => {
            "Datos reconstruidos durante migración"
        }
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
        f.desde.clone(),
        f.hasta.clone(),
        f.nombre_cedula.clone(),
        empresa_texto(f.empresa_id, &state.empresas),
        tipo_texto(f.tipo).into(),
        f.gafete.clone(),
        estado_texto(f.estado).into(),
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
    let mut lineas: Vec<_> = nombres
        .iter()
        .enumerate()
        .map(|(i, nombre)| {
            Line::from(format!(
                "{} {:<15} {}{}",
                if i == seleccion { ">" } else { " " },
                nombre,
                valores[i],
                if i == seleccion && editando { "_" } else { "" }
            ))
            .style(if i == seleccion {
                theme::foco()
            } else {
                theme::texto_normal()
            })
        })
        .collect();
    lineas.extend([
        Line::from(""),
        Line::from("↑↓ Mover   ENTER Editar/Seleccionar").style(theme::foco()),
        Line::from("A Aplicar   L Limpiar   ESC Cerrar").style(theme::foco()),
    ]);
    layout::render_overlay(frame, area, 58, 15, 2, "FILTROS", lineas);
}

fn render_desplegable(
    frame: &mut Frame,
    area: Rect,
    state: &HistorialState,
    campo: CampoFiltro,
    opcion: usize,
) {
    let (titulo, opciones): (_, Vec<String>) = match campo {
        CampoFiltro::Empresa => (
            "SELECCIONAR EMPRESA",
            std::iter::once("Todas".into())
                .chain(state.empresas.iter().map(|e| e.nombre.clone()))
                .collect(),
        ),
        CampoFiltro::Tipo => (
            "SELECCIONAR TIPO",
            ["Todos", "PRAIND", "IN HOUSE", "POR CORREO", "SWAT"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        ),
        CampoFiltro::Estado => (
            "SELECCIONAR ESTADO",
            ["Todos", "Activos", "Cerrados"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        ),
        _ => return,
    };
    let mut lineas: Vec<_> = opciones
        .iter()
        .enumerate()
        .map(|(i, v)| {
            Line::from(format!("{} {v}", if i == opcion { ">" } else { " " })).style(
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
    layout::render_overlay(
        frame,
        area,
        52,
        (lineas.len() as u16 + 4).min(area.height.saturating_sub(2)),
        2,
        titulo,
        lineas,
    );
}

fn render_columnas(frame: &mut Frame, area: Rect, state: &HistorialState, seleccion: usize) {
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
    lineas.push(Line::from("↑↓ mover  SPACE mostrar/ocultar  ESC cerrar").style(theme::foco()));
    layout::render_overlay(frame, area, 50, 17, 2, "COLUMNAS", lineas);
}
