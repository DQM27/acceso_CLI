use chrono::NaiveDate;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Cell, Paragraph, Row, Table},
};

use super::*;
use crate::{
    database::queries::contratistas::ContratistaResumen,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, render_terminal_too_small,
    },
};

const ANCHO_MINIMO: u16 = 60;
const ALTO_MINIMO: u16 = 22;
const ANCHO_PANEL_LATERAL: u16 = 100;

const COMANDOS_NORMAL: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Editar"),
    CommandHint::new("N", "Nuevo"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("F4", "Columnas"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_BUSQUEDA: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Aplicar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_FORMULARIO: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓/TAB", "Navegar"),
    CommandHint::new("SPACE", "Cambiar"),
    CommandHint::new("ENTER", "Guardar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_DESPLEGABLE: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Aceptar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_COLUMNAS: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("SPACE", "Mostrar/Ocultar"),
    CommandHint::new("ESC", "Cerrar"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &ContratistasState, theme: Theme) {

    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "ESC salir", theme);
        return;
    }

    let hora = hora_actual_texto();
    let contexto = format!("Usuario: {}", state.usuario_nombre);
    let (estado_texto, estado_tipo) = estado_shell(state);
    let comandos = match &state.modo {
        ModoContratistas::Normal => COMANDOS_NORMAL,
        ModoContratistas::Busqueda { .. } => COMANDOS_BUSQUEDA,
        ModoContratistas::Formulario(f) if f.desplegable.is_some() => COMANDOS_DESPLEGABLE,
        ModoContratistas::Formulario(_) => COMANDOS_FORMULARIO,
        ModoContratistas::Columnas { .. } => COMANDOS_COLUMNAS,
    };

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "CONTRATISTAS",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: Some("Claves: empresa, tipo, praind, ruta, acceso"),
    };
    let areas = shell.render(frame, area, theme);

    render_cuerpo(frame, areas.body, state, theme);
}

fn estado_shell(state: &ContratistasState) -> (String, StatusKind) {
    if let ModoContratistas::Formulario(f) = &state.modo
        && let Some(error) = &f.error
    {
        return (format!("✕ {error}"), StatusKind::Error);
    }
    if let Some(error) = &state.error_carga {
        return (error.clone(), StatusKind::Error);
    }
    if let Some(mensaje) = &state.mensaje {
        let tipo = if mensaje.starts_with('✓') {
            StatusKind::Success
        } else {
            StatusKind::Warning
        };
        return (mensaje.clone(), tipo);
    }
    (String::new(), StatusKind::Normal)
}

fn render_cuerpo(frame: &mut Frame, area: Rect, state: &ContratistasState, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);

    let enfocado_busqueda = matches!(state.modo, ModoContratistas::Busqueda { .. });
    let area_busqueda = render_campo(
        frame,
        filas[0],
        &format!("BUSCAR · {} RESULTADOS", state.registros.len()),
        &state.filtro,
        enfocado_busqueda,
        theme,
    );
    if enfocado_busqueda {
        posicionar_cursor(frame, area_busqueda, &state.filtro);
    }

    let (area_tabla, area_panel) = if area.width >= ANCHO_PANEL_LATERAL {
        let columnas = Layout::horizontal([
            Constraint::Percentage(60),
            Constraint::Length(1),
            Constraint::Percentage(39),
        ])
        .split(filas[1]);
        render_separador_vertical(frame, columnas[1], theme);
        (columnas[0], columnas[2])
    } else {
        let alto_panel = altura_panel(state);
        let filas_apiladas = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(alto_panel.min(filas[1].height.saturating_sub(5))),
        ])
        .split(filas[1]);
        render_separador_horizontal(frame, filas_apiladas[1], theme);
        (filas_apiladas[0], filas_apiladas[2])
    };
    render_tabla(frame, area_tabla, state, theme);
    render_panel(frame, area_panel, state, theme);
}

fn altura_fila(campo: CampoFormulario, f: &FormularioContratista) -> u16 {
    match campo {
        CampoFormulario::Cedula => {
            if matches!(f.modo, ModoFormulario::Crear) {
                3
            } else {
                1
            }
        }
        CampoFormulario::Nombre => 3,
        CampoFormulario::FechaPraind => {
            if f.requiere_praind() {
                3
            } else {
                1
            }
        }
        CampoFormulario::Empresa | CampoFormulario::Tipo | CampoFormulario::Ruta | CampoFormulario::Acceso => 1,
    }
}

fn altura_panel(state: &ContratistasState) -> u16 {
    match &state.modo {
        ModoContratistas::Formulario(f) => {
            let mut total: u16 = CampoFormulario::TODOS
                .iter()
                .map(|c| altura_fila(*c, f))
                .sum();
            if let Some((tipo, _)) = f.desplegable {
                total += match tipo {
                    Desplegable::Empresa => state.empresas.len().max(1) as u16,
                    Desplegable::Tipo => tipos().len() as u16,
                };
            }
            total + 1
        }
        ModoContratistas::Columnas { .. } => state.columnas.len() as u16 + 1,
        _ => 9,
    }
}

/// Misma silueta con foco o sin él (etiqueta, valor, línea); sólo cambian
/// color y peso.
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
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(estilo_linea),
        Rect::new(area.x, linea_y, area.width, 1),
    );

    Rect::new(area.x, valor_y, area.width, 1)
}

fn render_opcion(frame: &mut Frame, area: Rect, etiqueta: &str, valor: &str, activo: bool, theme: Theme) {
    let marcador = if activo { ">" } else { " " };
    let estilo = if activo { theme.accent() } else { theme.base() };
    frame.render_widget(
        Paragraph::new(Line::from(format!("{marcador} {etiqueta:<20} {valor}"))).style(estilo),
        area,
    );
}

fn posicionar_cursor(frame: &mut Frame, area: Rect, contenido: &str) {
    let ancho_visible = Line::from(contenido).width() as u16;
    let x = area.x.saturating_add(ancho_visible.min(area.width));
    frame.set_cursor_position((x, area.y));
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

fn render_tabla(frame: &mut Frame, area: Rect, state: &ContratistasState, theme: Theme) {
    let columnas: Vec<_> = state
        .columnas
        .iter()
        .filter_map(|(c, v)| v.then_some(*c))
        .collect();
    let capacidad = area.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.registros.len());
    let filas = state
        .registros
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, c)| {
            let seleccionado = state.seleccion == Some(inicio + visible);
            let estilo_fila = if seleccionado { theme.selected() } else { theme.base() };
            let celdas = columnas.iter().map(|col| {
                Cell::from(valor(c, *col)).style(if seleccionado {
                    estilo_fila
                } else {
                    estilo(c, *col, state.hoy, theme)
                })
            });
            Row::new(celdas).style(estilo_fila)
        });
    let encabezado = Row::new(columnas.iter().map(|c| c.titulo()))
        .style(theme.muted())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(
            filas,
            columnas.iter().map(|c| c.constraint()).collect::<Vec<_>>(),
        )
        .header(encabezado)
        .column_spacing(1),
        area,
    );
    if state.registros.is_empty() {
        frame.render_widget(
            Paragraph::new(if state.filtro.is_empty() {
                "Sin contratistas registrados"
            } else {
                "No hay contratistas que coincidan con la búsqueda."
            })
            .style(theme.warning())
            .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
    }
}

fn valor(c: &ContratistaResumen, col: Columna) -> String {
    match col {
        Columna::Cedula => c.cedula.clone(),
        Columna::Nombre => c.nombre.clone(),
        Columna::Empresa => c.empresa_nombre.clone(),
        Columna::Tipo => texto_tipo(c.tipo_ingreso).into(),
        Columna::Praind => c
            .fecha_vencimiento_praind
            .map(|f| f.format("%d/%m/%Y").to_string())
            .unwrap_or_else(|| "--".into()),
        Columna::Ruta => si_no(c.es_personal_ruta).into(),
        Columna::Acceso => si_no(c.tiene_acceso).into(),
    }
}

fn si_no(v: bool) -> &'static str {
    if v { "SÍ" } else { "NO" }
}

fn estilo(c: &ContratistaResumen, col: Columna, hoy: NaiveDate, theme: Theme) -> Style {
    match col {
        Columna::Acceso if c.tiene_acceso => theme.success(),
        Columna::Acceso => theme.danger(),
        Columna::Praind => estilo_fecha(c.fecha_vencimiento_praind, hoy, theme),
        _ => theme.base(),
    }
}

fn estilo_fecha(fecha: Option<NaiveDate>, hoy: NaiveDate, theme: Theme) -> Style {
    let Some(fecha) = fecha else {
        return theme.muted();
    };
    let dias = (fecha - hoy).num_days();
    if dias < 0 {
        theme.danger()
    } else if dias <= 30 {
        theme.warning()
    } else {
        theme.success()
    }
}

/// Dibuja el panel lateral según el modo y, cuando corresponde, posiciona
/// el cursor sobre el campo de texto enfocado.
fn render_panel(frame: &mut Frame, area: Rect, state: &ContratistasState, theme: Theme) {
    match &state.modo {
        ModoContratistas::Formulario(f) => render_formulario(frame, area, state, f, theme),
        ModoContratistas::Columnas { seleccion } => render_columnas(frame, area, state, *seleccion, theme),
        ModoContratistas::Normal | ModoContratistas::Busqueda { .. } => match state.seleccionado() {
            Some(c) => render_detalle(frame, area, c, state.hoy, theme),
            None => frame.render_widget(
                Paragraph::new("No hay un registro seleccionado.").style(theme.muted()),
                area,
            ),
        },
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, c: &ContratistaResumen, hoy: NaiveDate, theme: Theme) {
    let estilo_praind = estilo_fecha(c.fecha_vencimiento_praind, hoy, theme);
    let estilo_acceso = if c.tiene_acceso { theme.success() } else { theme.danger() };
    let lineas = vec![
        Line::from(c.nombre.as_str()).style(theme.title()),
        Line::from(c.cedula.as_str()).style(theme.muted()),
        Line::from(""),
        Line::from(format!("Empresa: {}", c.empresa_nombre)).style(theme.base()),
        Line::from(format!("Tipo de ingreso: {}", texto_tipo(c.tipo_ingreso))).style(theme.base()),
        Line::from(format!(
            "PRAIND: {}",
            c.fecha_vencimiento_praind
                .map(|f| f.format("%d/%m/%Y").to_string())
                .unwrap_or_else(|| "No requerida".into())
        ))
        .style(estilo_praind),
        Line::from(format!("Personal de ruta: {}", si_no(c.es_personal_ruta))).style(theme.base()),
        Line::from(if c.tiene_acceso { "ACCESO PERMITIDO" } else { "SIN ACCESO" }).style(estilo_acceso),
    ];
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_formulario(
    frame: &mut Frame,
    area: Rect,
    state: &ContratistasState,
    f: &FormularioContratista,
    theme: Theme,
) {
    let campos = CampoFormulario::TODOS;
    let mut restricciones: Vec<Constraint> = campos
        .iter()
        .map(|c| Constraint::Length(altura_fila(*c, f)))
        .collect();
    if let Some((tipo_desplegable, _)) = f.desplegable {
        let objetivo = match tipo_desplegable {
            Desplegable::Empresa => CampoFormulario::Empresa,
            Desplegable::Tipo => CampoFormulario::Tipo,
        };
        let indice = campos.iter().position(|c| *c == objetivo).unwrap_or(0);
        let alto = match tipo_desplegable {
            Desplegable::Empresa => state.empresas.len().max(1) as u16,
            Desplegable::Tipo => tipos().len() as u16,
        };
        restricciones.insert(indice + 1, Constraint::Length(alto));
    }
    restricciones.push(Constraint::Length(1));
    let filas = Layout::vertical(restricciones).split(area);

    let mut fila = 0usize;
    for (indice, campo) in campos.iter().enumerate() {
        let enfocado = f.campo == indice;
        match campo {
            CampoFormulario::Cedula => {
                if matches!(f.modo, ModoFormulario::Crear) {
                    let r = render_campo(frame, filas[fila], "CÉDULA", &f.cedula, enfocado, theme);
                    if enfocado {
                        posicionar_cursor(frame, r, &f.cedula);
                    }
                } else {
                    render_opcion(
                        frame,
                        filas[fila],
                        "CÉDULA",
                        &format!("{} (no editable)", f.cedula),
                        false,
                        theme,
                    );
                }
            }
            CampoFormulario::Nombre => {
                let r = render_campo(frame, filas[fila], "NOMBRE", &f.nombre, enfocado, theme);
                if enfocado {
                    posicionar_cursor(frame, r, &f.nombre);
                }
            }
            CampoFormulario::Empresa => {
                let valor = state
                    .empresas
                    .get(f.empresa)
                    .map_or("Sin empresas", |e| e.nombre.as_str());
                render_opcion(frame, filas[fila], "EMPRESA", valor, enfocado, theme);
                if let Some((Desplegable::Empresa, resaltado)) = f.desplegable {
                    fila += 1;
                    render_lista_desplegable(
                        frame,
                        filas[fila],
                        state.empresas.iter().map(|e| e.nombre.as_str()),
                        resaltado,
                        theme,
                    );
                }
            }
            CampoFormulario::Tipo => {
                render_opcion(frame, filas[fila], "TIPO DE INGRESO", texto_tipo(f.tipo), enfocado, theme);
                if let Some((Desplegable::Tipo, resaltado)) = f.desplegable {
                    fila += 1;
                    render_lista_desplegable(
                        frame,
                        filas[fila],
                        tipos().iter().map(|t| texto_tipo(*t)),
                        resaltado,
                        theme,
                    );
                }
            }
            CampoFormulario::FechaPraind => {
                if f.requiere_praind() {
                    let r = render_campo(
                        frame,
                        filas[fila],
                        "FECHA PRAIND",
                        &f.fecha_praind,
                        enfocado,
                        theme,
                    );
                    if enfocado {
                        posicionar_cursor(frame, r, &f.fecha_praind);
                    }
                } else {
                    render_opcion(frame, filas[fila], "FECHA PRAIND", "No requerida", false, theme);
                }
            }
            CampoFormulario::Ruta => {
                render_opcion(frame, filas[fila], "PERSONAL DE RUTA", si_no(f.personal_ruta), enfocado, theme);
            }
            CampoFormulario::Acceso => {
                render_opcion(frame, filas[fila], "TIENE ACCESO", si_no(f.tiene_acceso), enfocado, theme);
            }
        }
        fila += 1;
    }
    frame.render_widget(
        Paragraph::new(f.error.as_deref().unwrap_or_default()).style(theme.danger()),
        filas[fila],
    );
}

fn render_lista_desplegable<'a>(
    frame: &mut Frame,
    area: Rect,
    opciones: impl Iterator<Item = &'a str>,
    resaltado: usize,
    theme: Theme,
) {
    let lineas: Vec<Line<'_>> = opciones
        .enumerate()
        .map(|(indice, opcion)| {
            let seleccionado = indice == resaltado;
            let marcador = if seleccionado { "  >" } else { "   " };
            Line::from(format!("{marcador} {opcion}"))
                .style(if seleccionado { theme.selected() } else { theme.muted() })
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_columnas(frame: &mut Frame, area: Rect, state: &ContratistasState, seleccion: usize, theme: Theme) {
    let lineas: Vec<Line<'_>> = state
        .columnas
        .iter()
        .enumerate()
        .map(|(indice, (c, visible))| {
            let activo = indice == seleccion;
            let estilo = if activo { theme.accent() } else { theme.base() };
            Line::from(format!(
                "{} [{}] {}",
                if activo { ">" } else { " " },
                if *visible { "x" } else { " " },
                c.titulo()
            ))
            .style(estilo)
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), area);
}
