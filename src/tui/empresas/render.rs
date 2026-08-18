use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table},
};

use super::*;
use crate::{
    database::queries::empresas::EmpresaResumen,
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
    CommandHint::new("N", "Nueva"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_BUSQUEDA: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Aplicar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_FORMULARIO: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Guardar"),
    CommandHint::new("ESC", "Cancelar"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &EmpresasState, theme: Theme) {

    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "ESC salir", theme);
        return;
    }

    let hora = hora_actual_texto();
    let contexto = format!("Usuario: {}", state.usuario_nombre);
    let (estado_texto, estado_tipo) = estado_shell(state);
    let comandos = match &state.modo {
        ModoEmpresas::Normal => COMANDOS_NORMAL,
        ModoEmpresas::Busqueda { .. } => COMANDOS_BUSQUEDA,
        ModoEmpresas::Formulario(_) => COMANDOS_FORMULARIO,
    };

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "EMPRESAS",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        help_expanded: state.ayuda_expandida,
    };
    let areas = shell.render(frame, area, theme);

    render_cuerpo(frame, areas.body, state, theme);
}

fn estado_shell(state: &EmpresasState) -> (String, StatusKind) {
    if let ModoEmpresas::Formulario(formulario) = &state.modo
        && let Some(error) = &formulario.error
    {
        return (format!("✕ {error}"), StatusKind::Error);
    }
    if let Some(error) = &state.error_carga {
        return (error.clone(), StatusKind::Error);
    }
    if let Some(mensaje) = &state.mensaje {
        return (mensaje.clone(), StatusKind::Success);
    }
    (String::new(), StatusKind::Normal)
}

fn render_cuerpo(frame: &mut Frame, area: Rect, state: &EmpresasState, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);

    let enfocado_busqueda = matches!(state.modo, ModoEmpresas::Busqueda { .. });
    let area_busqueda = render_campo(
        frame,
        filas[0],
        &format!("BUSCAR · {} RESULTADOS", state.empresas.len()),
        &state.filtro,
        enfocado_busqueda,
        theme,
    );

    let (area_tabla, area_panel) = if area.width >= ANCHO_PANEL_LATERAL {
        let columnas = Layout::horizontal([
            Constraint::Percentage(63),
            Constraint::Length(1),
            Constraint::Percentage(36),
        ])
        .split(filas[1]);
        render_separador_vertical(frame, columnas[1], theme);
        (columnas[0], columnas[2])
    } else {
        let filas_apiladas = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(7.min(filas[1].height.saturating_sub(5))),
        ])
        .split(filas[1]);
        render_separador_horizontal(frame, filas_apiladas[1], theme);
        (filas_apiladas[0], filas_apiladas[2])
    };
    render_tabla(frame, area_tabla, state, theme);
    let area_form_nombre = render_panel(frame, area_panel, state, theme);

    let cursor = match &state.modo {
        ModoEmpresas::Busqueda { .. } => Some((area_busqueda, state.filtro.as_str())),
        ModoEmpresas::Formulario(formulario) => {
            area_form_nombre.map(|area| (area, formulario.nombre.as_str()))
        }
        _ => None,
    };
    if let Some((area_campo, contenido)) = cursor {
        let ancho_visible = Line::from(contenido).width() as u16;
        let x = area_campo
            .x
            .saturating_add(ancho_visible.min(area_campo.width));
        frame.set_cursor_position((x, area_campo.y));
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

fn render_tabla(frame: &mut Frame, area: Rect, state: &EmpresasState, theme: Theme) {
    let capacidad = area.height.saturating_sub(2) as usize;
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
                theme.selected()
            } else {
                theme.base()
            })
        });
    let encabezado = Row::new(["NOMBRE", "CONTRATISTAS"])
        .style(theme.muted())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(filas, [Constraint::Fill(4), Constraint::Length(14)])
            .header(encabezado)
            .column_spacing(1),
        area,
    );
    if state.empresas.is_empty() {
        frame.render_widget(
            Paragraph::new(if state.filtro.is_empty() {
                "Sin empresas registradas"
            } else {
                "No hay empresas que coincidan con la búsqueda."
            })
            .style(theme.warning())
            .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
    }
}

/// Devuelve, cuando aplica, el área del valor del campo NOMBRE del
/// formulario para que el llamador posicione el cursor.
fn render_panel(frame: &mut Frame, area: Rect, state: &EmpresasState, theme: Theme) -> Option<Rect> {
    match &state.modo {
        ModoEmpresas::Formulario(formulario) => Some(render_formulario(frame, area, formulario, theme)),
        ModoEmpresas::Normal | ModoEmpresas::Busqueda { .. } => {
            match state.empresa_seleccionada() {
                Some(empresa) => render_detalle(frame, area, empresa, theme),
                None => frame.render_widget(
                    Paragraph::new("No hay un registro seleccionado.").style(theme.muted()),
                    area,
                ),
            }
            None
        }
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, empresa: &EmpresaResumen, theme: Theme) {
    let lineas = vec![
        Line::from(empresa.nombre.as_str()).style(theme.title()),
        Line::from(""),
        Line::from(format!("Contratistas asociados: {}", empresa.contratistas)).style(theme.base()),
    ];
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_formulario(
    frame: &mut Frame,
    area: Rect,
    formulario: &FormularioEmpresa,
    theme: Theme,
) -> Rect {
    let titulo = match formulario.modo {
        ModoFormularioEmpresa::Crear => "NOMBRE DE LA NUEVA EMPRESA",
        ModoFormularioEmpresa::Editar { .. } => "NOMBRE",
    };
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Length(1)]).split(area);
    render_campo(frame, filas[0], titulo, &formulario.nombre, true, theme)
}
