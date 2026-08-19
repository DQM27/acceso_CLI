use super::*;
use crate::{
    database::queries::ingresos::{EstadoMovimiento, MovimientoIngresoResumen},
    services::autenticacion_service::UsuarioSesion,
    tiempo::{a_costa_rica, hora_actual_texto},
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, auxiliary_panel, centered_rect,
        identidad_sesion, render_terminal_too_small,
    },
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Cell, Clear, Paragraph, Row, Table},
};

const ANCHO_MINIMO: u16 = 60;
const ALTO_MINIMO: u16 = 22;
const ANCHO_PANEL_LATERAL: u16 = 100;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &HistorialState,
    sesion: &UsuarioSesion,
    theme: Theme,
) {
    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "ESC salir", theme);
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
        help_expanded: state.ayuda_expandida,
        ayuda_extra: Some("Claves: empresa, tipo, estado, gafete, desde, hasta, ingreso, salida"),
    };
    let areas = shell.render(frame, area, theme);

    render_cuerpo(frame, areas.body, state, theme);

    if let ModoHistorial::Columnas { seleccion } = state.modo {
        render_columnas_editor(frame, area, state, seleccion, theme);
    }
}

fn comandos_para(state: &HistorialState) -> Vec<CommandHint<'static>> {
    let etiqueta_vista: &'static str = state.vista.next().label();
    if let ModoHistorial::Columnas { .. } = state.modo {
        return vec![
            CommandHint::new("↑↓", "Mover"),
            CommandHint::new("SPACE", "Mostrar/Ocultar"),
            CommandHint::new("ESC/F4", "Cerrar"),
        ];
    }
    match state.vista {
        ViewMode::Timeline => vec![
            CommandHint::new("↑↓", "Mover"),
            CommandHint::new("PGUP/PGDN", "Página"),
            CommandHint::new("F3", etiqueta_vista),
            CommandHint::new("ESC", "Volver"),
        ],
        ViewMode::Classic => vec![
            CommandHint::new("↑↓", "Mover"),
            CommandHint::new("PGUP/PGDN", "Página"),
            CommandHint::new("F3", etiqueta_vista),
            CommandHint::new("F4", "Columnas"),
            CommandHint::new("ESC", "Volver"),
        ],
        ViewMode::Heatmap => vec![
            CommandHint::new("↑↓", "Semana"),
            CommandHint::new("TAB", "Día"),
            CommandHint::new("ENTER", "Ver ese día"),
            CommandHint::new("F3", etiqueta_vista),
            CommandHint::new("ESC", "Volver"),
        ],
    }
}

fn estado_shell(state: &HistorialState) -> (String, StatusKind) {
    if let Some(m) = &state.mensaje {
        return (m.clone(), StatusKind::Error);
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
        resumen.push(format!("ingreso: {}", state.filtro_aplicado.usuario_ingreso));
    }
    if !state.filtro_aplicado.usuario_salida.is_empty() {
        resumen.push(format!("salida: {}", state.filtro_aplicado.usuario_salida));
    }
    format!("BUSCAR · CLAVE:VALOR O TEXTO LIBRE · {}", resumen.join(" · "))
}

fn render_cuerpo(frame: &mut Frame, area: Rect, state: &HistorialState, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);

    let etiqueta = etiqueta_busqueda(state);
    let area_busqueda = render_campo(frame, filas[0], &etiqueta, state.busqueda.value(), true, theme);
    if !matches!(state.vista, ViewMode::Heatmap) {
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
    }

    match state.vista {
        ViewMode::Timeline => render_vista_timeline(frame, filas[1], state, theme),
        ViewMode::Classic => render_tabla_clasica(frame, filas[1], state, theme),
        ViewMode::Heatmap => render_mapa_calor(frame, filas[1], state, theme),
    }
}

/// El timeline agrupado + panel de detalle — el enfoque "curado".
fn render_vista_timeline(frame: &mut Frame, area: Rect, state: &HistorialState, theme: Theme) {
    let (area_timeline, area_panel) = if area.width >= ANCHO_PANEL_LATERAL {
        let columnas = Layout::horizontal([
            Constraint::Percentage(63),
            Constraint::Length(1),
            Constraint::Percentage(36),
        ])
        .split(area);
        render_separador_vertical(frame, columnas[1], theme);
        (columnas[0], columnas[2])
    } else {
        let filas_apiladas = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(15.min(area.height.saturating_sub(5))),
        ])
        .split(area);
        render_separador_horizontal(frame, filas_apiladas[1], theme);
        (filas_apiladas[0], filas_apiladas[2])
    };
    render_timeline(frame, area_timeline, state, theme);
    match state.seleccionado() {
        Some(r) => render_detalle(frame, area_panel, r, theme),
        None => frame.render_widget(
            Paragraph::new("No hay un registro seleccionado.").style(theme.muted()),
            area_panel,
        ),
    }
}

fn render_separador_vertical(frame: &mut Frame, area: Rect, theme: Theme) {
    let lineas: Vec<Line<'static>> = (0..area.height).map(|_| Line::from("│")).collect();
    frame.render_widget(Paragraph::new(lineas).style(theme.border()), area);
}

fn render_separador_horizontal(frame: &mut Frame, area: Rect, theme: Theme) {
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(theme.border()),
        area,
    );
}

/// El campo de búsqueda está siempre activo: no hay otro modo que le
/// dispute el teclado (salvo el mapa de calor, que no tiene texto libre),
/// así que se dibuja siempre en estado activo.
fn render_campo(
    frame: &mut Frame,
    area: Rect,
    etiqueta: &str,
    valor: &str,
    activo: bool,
    theme: Theme,
) -> Rect {
    let estilo_etiqueta = if activo { theme.accent() } else { theme.muted() };
    let estilo_linea = if activo { theme.accent() } else { theme.border() };
    let valor_y = area.y.saturating_add(1);
    let linea_y = area.y.saturating_add(2);

    frame.render_widget(
        Paragraph::new(etiqueta).style(estilo_etiqueta),
        Rect::new(area.x, area.y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(valor).style(theme.base())),
        Rect::new(area.x, valor_y, area.width, 1),
    );
    if area.height > 2 {
        frame.render_widget(
            Paragraph::new("─".repeat(area.width as usize)).style(estilo_linea),
            Rect::new(area.x, linea_y, area.width, 1),
        );
    }

    Rect::new(area.x, valor_y, area.width, 1)
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
        let fecha = a_costa_rica(registro.fecha_hora_ingreso).format("%d/%m/%Y").to_string();
        if ultima_fecha.as_deref() != Some(fecha.as_str()) {
            let cantidad = state
                .registros
                .iter()
                .filter(|r| a_costa_rica(r.fecha_hora_ingreso).format("%d/%m/%Y").to_string() == fecha)
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
        filas.push(fila_movimiento(registro, state.seleccion == Some(indice), theme));
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

fn fila_movimiento(r: &MovimientoIngresoResumen, seleccionada: bool, theme: Theme) -> Line<'static> {
    let activo = r.fecha_hora_salida.is_none();
    let glifo = if activo { "●" } else { "○" };
    let estilo_glifo = if activo { theme.warning() } else { theme.muted() };
    let salida = r
        .fecha_hora_salida
        .map_or_else(|| "ahora".to_owned(), |f| a_costa_rica(f).format("%H:%M").to_string());
    let entrada = a_costa_rica(r.fecha_hora_ingreso).format("%H:%M").to_string();
    let marcador = if seleccionada { ">" } else { " " };
    let estilo_fila = if seleccionada { theme.selected() } else { theme.base() };
    Line::from(vec![
        Span::styled(format!("{marcador} "), estilo_fila),
        Span::styled(format!("{glifo} "), if seleccionada { estilo_fila } else { estilo_glifo }),
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
        Line::from(r.cedula.clone()).style(theme.muted()),
        Line::from(""),
        Line::from(format!("Empresa: {}", r.empresa_nombre)).style(theme.base()),
        Line::from(format!("Tipo de ingreso: {}", tipo_texto(r.tipo_ingreso))).style(theme.base()),
        Line::from(format!("Medio: {}", texto_medio(r.medio_ingreso))).style(theme.base()),
        Line::from(format!("Gafete: {gafete_texto}")).style(theme.base()),
        Line::from(""),
        Line::from(format!(
            "Fecha: {}",
            a_costa_rica(r.fecha_hora_ingreso).format("%d/%m/%Y")
        ))
        .style(theme.base()),
        Line::from(format!(
            "Entrada: {}",
            a_costa_rica(r.fecha_hora_ingreso).format("%H:%M")
        ))
        .style(theme.base()),
    ];
    match r.fecha_hora_salida {
        Some(salida) => {
            let duracion = salida - r.fecha_hora_ingreso;
            lineas.push(
                Line::from(format!("Salida: {}", a_costa_rica(salida).format("%H:%M")))
                    .style(theme.base()),
            );
            lineas.push(
                Line::from(format!(
                    "Duración: {}h{:02}m",
                    duracion.num_minutes() / 60,
                    duracion.num_minutes() % 60,
                ))
                .style(theme.base()),
            );
        }
        None => lineas.push(Line::from("Estado: activo, aún dentro").style(theme.warning())),
    }
    lineas.push(Line::from(""));
    lineas.push(
        Line::from(format!("Ingreso registrado por: {}", r.usuario_ingreso_nombre))
            .style(theme.base()),
    );
    if let Some(operador) = &r.usuario_salida_nombre {
        lineas
            .push(Line::from(format!("Salida registrada por: {operador}")).style(theme.base()));
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
    let columnas_visibles: Vec<ClassicColumn> = state
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
            Row::new(
                columnas_visibles
                    .iter()
                    .map(|c| Cell::from(valor_columna_clasica(r, *c))),
            )
            .style(if seleccionada { theme.selected() } else { theme.base() })
        });
    let encabezado = Row::new(columnas_visibles.iter().map(|c| c.label()))
        .style(theme.muted().add_modifier(Modifier::BOLD))
        .bottom_margin(1);
    let anchos: Vec<_> = columnas_visibles.iter().map(|c| c.constraint()).collect();
    frame.render_widget(
        Table::new(filas, anchos).header(encabezado).column_spacing(1),
        area,
    );
}

fn valor_columna_clasica(r: &MovimientoIngresoResumen, c: ClassicColumn) -> String {
    match c {
        ClassicColumn::Fecha => a_costa_rica(r.fecha_hora_ingreso).format("%d/%m/%Y").to_string(),
        ClassicColumn::Cedula => r.cedula.clone(),
        ClassicColumn::Nombre => r.contratista_nombre.clone(),
        ClassicColumn::Empresa => r.empresa_nombre.clone(),
        ClassicColumn::Tipo => tipo_texto(r.tipo_ingreso).into(),
        ClassicColumn::Entrada => a_costa_rica(r.fecha_hora_ingreso).format("%H:%M").to_string(),
        ClassicColumn::Salida => r.fecha_hora_salida.map_or_else(
            || "Activo".into(),
            |f| a_costa_rica(f).format("%H:%M").to_string(),
        ),
        ClassicColumn::Gafete => r.gafete_numero.map_or_else(|| "S/G".into(), |g| g.to_string()),
        ClassicColumn::Medio => texto_medio(r.medio_ingreso).into(),
        ClassicColumn::Ingreso => r.usuario_ingreso_nombre.clone(),
        ClassicColumn::Egreso => r.usuario_salida_nombre.clone().unwrap_or_else(|| "—".into()),
    }
}

fn render_columnas_editor(frame: &mut Frame, area: Rect, state: &HistorialState, seleccion: usize, theme: Theme) {
    let franja_superior = Rect::new(area.x, area.y, area.width, area.height.min(20));
    let popup = centered_rect(
        franja_superior,
        44.min(area.width),
        (ClassicColumn::ALL.len() as u16 + 4).min(franja_superior.height),
    );
    frame.render_widget(Clear, popup);
    let block = auxiliary_panel("COLUMNAS VISIBLES", theme, true);
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
    frame.render_widget(
        Line::from("↑↓ mover · ESPACIO mostrar/ocultar · F4/ESC cerrar").style(theme.muted()),
        filas[1],
    );
}

/// Grilla semanal estilo "contribuciones de GitHub": responde "¿cuándo hubo
/// más actividad?", algo que el timeline y la vista clásica no contestan de
/// un vistazo porque ambas están pensadas para navegar registros uno por
/// uno, no para ver el patrón completo.
fn render_mapa_calor(frame: &mut Frame, area: Rect, state: &HistorialState, theme: Theme) {
    let conteo = state.conteo_por_dia();
    if conteo.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin registros para los filtros seleccionados")
                .style(theme.warning())
                .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
        return;
    }

    let maximo = *conteo.values().max().unwrap_or(&1);
    let fecha_min = *conteo.keys().min().expect("conteo no está vacío");
    let fecha_max = *conteo.keys().max().expect("conteo no está vacío");
    let seleccion = state.heatmap_seleccion.clamp(fecha_min, fecha_max);

    let mut lineas: Vec<Line<'static>> = vec![
        Line::from("            L  M  M  J  V  S  D").style(theme.muted().add_modifier(Modifier::BOLD)),
    ];
    let mut inicio_semana = start_of_week(fecha_min);
    let fin_grilla = end_of_week(fecha_max);
    while inicio_semana <= fin_grilla {
        let mut spans = vec![Span::styled(format!("{}  ", inicio_semana.format("%d/%m")), theme.muted())];
        for offset in 0..7 {
            let dia = inicio_semana + Duration::days(offset);
            let en_rango = dia >= fecha_min && dia <= fecha_max;
            let cantidad = conteo.get(&dia).copied().unwrap_or(0);
            let seleccionado = dia == seleccion;
            let glifo = bucket_glyph(cantidad, maximo);
            let estilo = if !en_rango {
                theme.muted()
            } else if seleccionado {
                theme.selected()
            } else if cantidad == 0 {
                theme.muted()
            } else {
                let ratio = cantidad as f64 / maximo.max(1) as f64;
                if ratio >= 0.99 {
                    theme.warning()
                } else if ratio >= 0.66 {
                    theme.accent()
                } else if ratio >= 0.33 {
                    theme.base()
                } else {
                    theme.muted()
                }
            };
            let texto = if seleccionado { format!("[{glifo}]") } else { format!(" {glifo} ") };
            spans.push(Span::styled(texto, estilo));
        }
        lineas.push(Line::from(spans));
        inicio_semana += Duration::days(7);
    }

    lineas.push(Line::from(""));
    let cantidad_seleccion = conteo.get(&seleccion).copied().unwrap_or(0);
    lineas.push(
        Line::from(format!(
            "{} · {cantidad_seleccion} movimientos · ENTER ver en Línea de tiempo",
            seleccion.format("%d/%m/%Y")
        ))
        .style(theme.accent()),
    );
    lineas.push(Line::from("█ mucho   ▓ ▒ ░ menos   · sin datos").style(theme.muted()));

    frame.render_widget(Paragraph::new(lineas), area);
}

fn texto_medio(m: crate::models::medio_ingreso::MedioIngreso) -> &'static str {
    match m {
        crate::models::medio_ingreso::MedioIngreso::Caminando => "Caminando",
        crate::models::medio_ingreso::MedioIngreso::Vehiculo => "Vehículo",
    }
}
