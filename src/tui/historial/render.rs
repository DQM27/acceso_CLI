use super::*;
use crate::{
    database::queries::ingresos::{EstadoMovimiento, MovimientoIngresoResumen},
    services::autenticacion_service::UsuarioSesion,
    tiempo::{a_costa_rica, hora_actual_texto},
    tui::ui_kit::{
        CommandHint, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, ScreenShell, StatusKind, Theme,
        auxiliary_panel, centered_rect, detail_line, identidad_sesion, master_detail_areas,
        render_form_field, render_separator, render_terminal_too_small,
    },
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Cell, Clear, Paragraph, Row, Table},
};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &HistorialState,
    sesion: &UsuarioSesion,
    theme: Theme,
) {
    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        render_terminal_too_small(
            frame,
            area,
            MIN_TERMINAL_WIDTH,
            MIN_TERMINAL_HEIGHT,
            "ESC volver",
            theme,
        );
        return;
    }

    let hora = hora_actual_texto();
    let contexto = identidad_sesion(sesion);
    let (estado_texto_linea, estado_tipo) = estado_shell(state);
    let comandos = comandos_para(state);

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: &format!("HISTORIAL · {}", state.vista.label()),
        context: &contexto,
        clock: &hora,
        status: &estado_texto_linea,
        status_kind: estado_tipo,
        commands: &comandos,
        authenticated: true,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: Some("Claves: empresa, tipo, estado, gafete, desde, hasta, ingreso, salida"),
    };
    let areas = shell.render(frame, area, theme);

    render_cuerpo(frame, areas.body, state, theme);

    match &state.modo {
        ModoHistorial::Columnas {
            seleccion,
            proposito,
        } => render_columnas_editor(frame, area, state, *seleccion, *proposito, theme),
        ModoHistorial::RutaExportacion { destino } => {
            render_ruta_exportacion(frame, area, destino, theme)
        }
        ModoHistorial::Normal => {}
    }
}

fn comandos_para(state: &HistorialState) -> Vec<CommandHint<'static>> {
    let etiqueta_vista: &'static str = state.vista.next().label();
    match &state.modo {
        ModoHistorial::Columnas { proposito, .. } => {
            let ultimo = if *proposito == PropositoColumnas::Exportacion {
                CommandHint::new("ENTER", "Continuar")
            } else {
                CommandHint::new("ESC/F4", "Cerrar")
            };
            return vec![
                CommandHint::new("↑↓", "Mover"),
                CommandHint::new("SPACE", "Mostrar/Ocultar"),
                ultimo,
            ];
        }
        ModoHistorial::RutaExportacion { .. } => {
            return vec![
                CommandHint::new("ENTER", "Exportar"),
                CommandHint::new("ESC", "Cancelar"),
            ];
        }
        ModoHistorial::Normal => {}
    }
    match state.vista {
        ViewMode::Timeline => vec![
            CommandHint::new("↑↓", "Mover"),
            CommandHint::new("PGUP/PGDN", "Página"),
            CommandHint::new("F3", etiqueta_vista),
            CommandHint::new("F5", "Exportar"),
            CommandHint::new("ESC", "Volver"),
        ],
        ViewMode::Classic => vec![
            CommandHint::new("↑↓", "Mover"),
            CommandHint::new("PGUP/PGDN", "Página"),
            CommandHint::new("F3", etiqueta_vista),
            CommandHint::new("F4", "Columnas"),
            CommandHint::new("F5", "Exportar"),
            CommandHint::new("ESC", "Volver"),
        ],
    }
}

fn estado_shell(state: &HistorialState) -> (String, StatusKind) {
    if let Some(m) = &state.mensaje {
        let tipo = if m.starts_with('✓') {
            StatusKind::Success
        } else if m.contains("Exportando") {
            StatusKind::Warning
        } else {
            StatusKind::Error
        };
        return (m.clone(), tipo);
    }
    (String::new(), StatusKind::Normal)
}

fn etiqueta_busqueda(state: &HistorialState) -> String {
    let (pagina, total_paginas) = state.pagina();
    let mut resumen = vec![format!("{} resultados", state.total)];
    if pagina > 0 {
        resumen.push(format!("página {pagina}/{total_paginas}"));
    }
    if state.filtro_aplicado.empresa_id.is_some() {
        resumen.push(format!(
            "empresa: {}",
            empresa_texto(state.filtro_aplicado.empresa_id, &state.empresas)
        ));
    }
    if state.filtro_aplicado.estado != EstadoMovimiento::Todos {
        resumen.push(format!(
            "estado: {}",
            estado_texto(state.filtro_aplicado.estado)
        ));
    }
    if let Some(tipos) = &state.filtro_aplicado.tipos {
        resumen.push(format!("tipo: {}", tipos_texto(Some(tipos))));
    }
    if !state.filtro_aplicado.usuario_ingreso.is_empty() {
        let signo = if state.filtro_aplicado.usuario_ingreso_negado {
            "≠"
        } else {
            ""
        };
        resumen.push(format!(
            "ingreso: {signo}{}",
            state.filtro_aplicado.usuario_ingreso
        ));
    }
    if !state.filtro_aplicado.usuario_salida.is_empty() {
        let signo = if state.filtro_aplicado.usuario_salida_negado {
            "≠"
        } else {
            ""
        };
        resumen.push(format!(
            "salida: {signo}{}",
            state.filtro_aplicado.usuario_salida
        ));
    }
    format!(
        "BUSCAR · CLAVE:VALOR O TEXTO LIBRE · {}",
        resumen.join(" · ")
    )
}

fn render_cuerpo(frame: &mut Frame, area: Rect, state: &HistorialState, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);

    let etiqueta = etiqueta_busqueda(state);
    let area_busqueda = render_campo(
        frame,
        filas[0],
        &etiqueta,
        state.busqueda.value(),
        true,
        theme,
    );
    let antes_del_cursor: String = state
        .busqueda
        .value()
        .chars()
        .take(state.busqueda.cursor())
        .collect();
    let ancho_visible = Line::from(antes_del_cursor).width() as u16;
    let x = area_busqueda
        .x
        .saturating_add(ancho_visible.min(area_busqueda.width));
    frame.set_cursor_position((x, area_busqueda.y));

    match state.vista {
        ViewMode::Timeline => render_vista_timeline(frame, filas[1], state, theme),
        ViewMode::Classic => render_tabla_clasica(frame, filas[1], state, theme),
    }
}

/// El timeline agrupado + panel de detalle — el enfoque "curado".
fn render_vista_timeline(frame: &mut Frame, area: Rect, state: &HistorialState, theme: Theme) {
    let areas = master_detail_areas(area, 63, 15);
    render_separator(frame, areas.separator, areas.orientation, theme);
    let (area_timeline, area_panel) = (areas.master, areas.detail);
    render_timeline(frame, area_timeline, state, theme);
    match state.seleccionado() {
        Some(r) => render_detalle(frame, area_panel, r, theme),
        None => frame.render_widget(
            Paragraph::new("No hay un registro seleccionado.").style(theme.muted()),
            area_panel,
        ),
    }
}

/// El campo de búsqueda está siempre activo en las dos vistas de Historial.
fn render_campo(
    frame: &mut Frame,
    area: Rect,
    etiqueta: &str,
    valor: &str,
    activo: bool,
    theme: Theme,
) -> Rect {
    render_form_field(frame, area, etiqueta, valor, activo, theme)
}

fn render_timeline(frame: &mut Frame, area: Rect, state: &HistorialState, theme: Theme) {
    if state.registros.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin registros para los filtros seleccionados")
                .style(theme.warning())
                .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
        return;
    }

    let secciones = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    frame.render_widget(encabezado_timeline(theme), secciones[0]);
    let area_scroll = secciones[1];

    let mut filas: Vec<Line<'static>> = Vec::with_capacity(state.registros.len() * 2);
    let mut fila_seleccionada = 0usize;
    let mut ultima_fecha: Option<String> = None;
    for (indice, registro) in state.registros.iter().enumerate() {
        let fecha = a_costa_rica(registro.fecha_hora_ingreso)
            .format("%d/%m/%Y")
            .to_string();
        if ultima_fecha.as_deref() != Some(fecha.as_str()) {
            let cantidad = state
                .registros
                .iter()
                .filter(|r| {
                    a_costa_rica(r.fecha_hora_ingreso)
                        .format("%d/%m/%Y")
                        .to_string()
                        == fecha
                })
                .count();
            filas.push(
                Line::from(format!("{fecha} · {cantidad} movimientos"))
                    .style(theme.muted().add_modifier(Modifier::BOLD)),
            );
            ultima_fecha = Some(fecha);
        }
        if state.seleccion == Some(indice) {
            fila_seleccionada = filas.len();
        }
        filas.push(fila_movimiento(
            registro,
            state.seleccion == Some(indice),
            theme,
        ));
    }

    let capacidad = area_scroll.height as usize;
    let inicio = fila_seleccionada.saturating_sub(capacidad.saturating_sub(1));
    let visibles: Vec<Line<'static>> = filas.into_iter().skip(inicio).take(capacidad).collect();
    frame.render_widget(Paragraph::new(visibles), area_scroll);
}

fn encabezado_timeline(theme: Theme) -> Line<'static> {
    Line::from(format!(
        "    {:<5} → {:<5}  {:<20} {:<18} {}",
        "ENTRA", "SALE", "NOMBRE", "EMPRESA", "TIPO"
    ))
    .style(theme.muted().add_modifier(Modifier::BOLD))
}

fn fila_movimiento(
    r: &MovimientoIngresoResumen,
    seleccionada: bool,
    theme: Theme,
) -> Line<'static> {
    let activo = r.fecha_hora_salida.is_none();
    let glifo = if activo { "●" } else { "○" };
    let estilo_glifo = if activo {
        theme.warning()
    } else {
        theme.muted()
    };
    let salida = r.fecha_hora_salida.map_or_else(
        || "ahora".to_owned(),
        |f| a_costa_rica(f).format("%H:%M").to_string(),
    );
    let entrada = a_costa_rica(r.fecha_hora_ingreso)
        .format("%H:%M")
        .to_string();
    let marcador = if seleccionada { ">" } else { " " };
    let estilo_fila = if seleccionada {
        theme.selected()
    } else {
        theme.base()
    };
    Line::from(vec![
        Span::styled(format!("{marcador} "), estilo_fila),
        Span::styled(
            format!("{glifo} "),
            if seleccionada {
                estilo_fila
            } else {
                estilo_glifo
            },
        ),
        Span::styled(format!("{entrada:<5} → {salida:<5}  "), estilo_fila),
        Span::styled(format!("{:<20.20} ", r.contratista_nombre), estilo_fila),
        Span::styled(format!("{:<18.18} ", r.empresa_nombre), estilo_fila),
        Span::styled(tipo_texto(r.tipo_ingreso).to_owned(), estilo_fila),
    ])
}

/// Sigue la misma convención "Etiqueta: valor" que usan los paneles de
/// detalle de Contratistas/Activos, agrupada en tres bloques: identidad
/// (nombre/cédula), clasificación (empresa/tipo/medio/gafete) y cronología
/// (fecha/entrada/salida/duración), cerrando con la trazabilidad de quién
/// registró cada movimiento.
fn render_detalle(frame: &mut Frame, area: Rect, r: &MovimientoIngresoResumen, theme: Theme) {
    let gafete_texto = r
        .gafete_numero
        .map_or_else(|| "Sin gafete".to_owned(), |g| g.to_string());
    let mut lineas = vec![
        Line::from(r.contratista_nombre.clone()).style(theme.title()),
        detail_line("Cédula", r.cedula.clone(), theme),
        Line::from(""),
        detail_line("Empresa", r.empresa_nombre.clone(), theme),
        detail_line("Tipo", tipo_texto(r.tipo_ingreso), theme),
        detail_line("Medio", texto_medio(r.medio_ingreso), theme),
        detail_line("Gafete", gafete_texto, theme),
        Line::from(""),
        detail_line(
            "Fecha",
            a_costa_rica(r.fecha_hora_ingreso)
                .format("%d/%m/%Y")
                .to_string(),
            theme,
        ),
        detail_line(
            "Entrada",
            a_costa_rica(r.fecha_hora_ingreso)
                .format("%H:%M")
                .to_string(),
            theme,
        ),
    ];
    match r.fecha_hora_salida {
        Some(salida) => {
            let duracion = salida - r.fecha_hora_ingreso;
            lineas.push(detail_line(
                "Salida",
                a_costa_rica(salida).format("%H:%M").to_string(),
                theme,
            ));
            lineas.push(detail_line(
                "Duración",
                format!(
                    "{}h{:02}m",
                    duracion.num_minutes() / 60,
                    duracion.num_minutes() % 60,
                ),
                theme,
            ));
        }
        None => lineas.push(detail_line("Estado", "Activo, aún dentro", theme)),
    }
    lineas.push(Line::from(""));
    lineas.push(detail_line(
        "Ingreso registrado por",
        r.usuario_ingreso_nombre.clone(),
        theme,
    ));
    if let Some(operador) = &r.usuario_salida_nombre {
        lineas.push(detail_line(
            "Salida registrada por",
            operador.clone(),
            theme,
        ));
    }
    frame.render_widget(Paragraph::new(lineas), area);
}

/// Una línea por movimiento con todos los datos a la vista — sin panel,
/// para quien prefiera el formato de planilla.
fn render_tabla_clasica(frame: &mut Frame, area: Rect, state: &HistorialState, theme: Theme) {
    if state.registros.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin registros para los filtros seleccionados")
                .style(theme.warning())
                .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
        return;
    }
    let columnas_visibles: Vec<ColumnaHistorial> = state
        .columnas_clasica
        .iter()
        .filter(|(_, visible)| *visible)
        .map(|(c, _)| *c)
        .collect();
    let capacidad = area.height.saturating_sub(2) as usize;
    let inicio = state
        .seleccion
        .unwrap_or(0)
        .saturating_sub(capacidad.saturating_sub(1))
        .min(state.registros.len());
    let filas = state
        .registros
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, r)| {
            let seleccionada = state.seleccion == Some(inicio + visible);
            Row::new(columnas_visibles.iter().enumerate().map(|(indice, c)| {
                let valor = valor_columna_clasica(r, *c);
                Cell::from(if indice == 0 {
                    format!("{} {valor}", if seleccionada { ">" } else { " " })
                } else {
                    valor
                })
            }))
            .style(if seleccionada {
                theme.selected()
            } else {
                theme.base()
            })
        });
    let encabezado = Row::new(columnas_visibles.iter().map(|c| c.label()))
        .style(theme.muted().add_modifier(Modifier::BOLD))
        .bottom_margin(1);
    let anchos: Vec<_> = columnas_visibles.iter().map(|c| c.constraint()).collect();
    frame.render_widget(
        Table::new(filas, anchos)
            .header(encabezado)
            .column_spacing(1),
        area,
    );
}

fn valor_columna_clasica(r: &MovimientoIngresoResumen, c: ColumnaHistorial) -> String {
    match c {
        ColumnaHistorial::Fecha => a_costa_rica(r.fecha_hora_ingreso)
            .format("%d/%m/%Y")
            .to_string(),
        ColumnaHistorial::Cedula => r.cedula.clone(),
        ColumnaHistorial::Nombre => r.contratista_nombre.clone(),
        ColumnaHistorial::Empresa => r.empresa_nombre.clone(),
        ColumnaHistorial::Tipo => tipo_texto(r.tipo_ingreso).into(),
        ColumnaHistorial::Entrada => a_costa_rica(r.fecha_hora_ingreso)
            .format("%H:%M")
            .to_string(),
        ColumnaHistorial::Salida => r.fecha_hora_salida.map_or_else(
            || "Activo".into(),
            |f| a_costa_rica(f).format("%H:%M").to_string(),
        ),
        ColumnaHistorial::Gafete => r
            .gafete_numero
            .map_or_else(|| "S/G".into(), |g| g.to_string()),
        ColumnaHistorial::Medio => texto_medio(r.medio_ingreso).into(),
        ColumnaHistorial::Ingreso => r.usuario_ingreso_nombre.clone(),
        ColumnaHistorial::Egreso => r
            .usuario_salida_nombre
            .clone()
            .unwrap_or_else(|| "—".into()),
    }
}

fn render_columnas_editor(
    frame: &mut Frame,
    area: Rect,
    state: &HistorialState,
    seleccion: usize,
    proposito: PropositoColumnas,
    theme: Theme,
) {
    let franja_superior = Rect::new(area.x, area.y, area.width, area.height.min(20));
    let popup = centered_rect(
        franja_superior,
        44.min(area.width),
        (ColumnaHistorial::ALL.len() as u16 + 4).min(franja_superior.height),
    );
    frame.render_widget(Clear, popup);
    let titulo = if proposito == PropositoColumnas::Exportacion {
        "COLUMNAS PARA EXPORTAR"
    } else {
        "COLUMNAS VISIBLES"
    };
    let block = auxiliary_panel(titulo, theme, true);
    let interior = block.inner(popup);
    frame.render_widget(block, popup);
    if interior.height == 0 {
        return;
    }
    let filas = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(interior);
    let lineas: Vec<Line<'_>> = state
        .columnas_clasica
        .iter()
        .enumerate()
        .map(|(i, (c, visible))| {
            let seleccionado = i == seleccion;
            let marcador = if seleccionado { ">" } else { " " };
            let caja = if *visible { "[x]" } else { "[ ]" };
            Line::from(format!("{marcador} {caja} {}", c.label())).style(if seleccionado {
                theme.selected()
            } else {
                theme.base()
            })
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), filas[0]);
    let ayuda = if proposito == PropositoColumnas::Exportacion {
        "↑↓ mover · ESPACIO incluir/omitir · ENTER continuar · ESC cancelar"
    } else {
        "↑↓ mover · ESPACIO mostrar/ocultar · F4/ESC cerrar"
    };
    frame.render_widget(Line::from(ayuda).style(theme.muted()), filas[1]);
}

fn render_ruta_exportacion(frame: &mut Frame, area: Rect, destino: &TextInput, theme: Theme) {
    let popup = centered_rect(area, area.width.min(100), 7.min(area.height));
    frame.render_widget(Clear, popup);
    let block = auxiliary_panel("EXPORTAR HISTORIAL", theme, true);
    let interior = block.inner(popup);
    frame.render_widget(block, popup);
    if interior.height < 4 {
        return;
    }
    let filas = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(interior);
    frame.render_widget(
        Paragraph::new("Confirme o edite la ruta del archivo XLSX:").style(theme.base()),
        filas[0],
    );
    destino.render(frame, filas[1], "DESTINO", "historial.xlsx", true, theme);
    frame.render_widget(
        Line::from("ENTER exportar · ESC cancelar").style(theme.muted()),
        filas[2],
    );
}

fn texto_medio(m: crate::models::medio_ingreso::MedioIngreso) -> &'static str {
    match m {
        crate::models::medio_ingreso::MedioIngreso::Caminando => "Caminando",
        crate::models::medio_ingreso::MedioIngreso::Vehiculo => "Vehículo",
    }
}
