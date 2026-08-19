use super::*;
use crate::{
    database::queries::contratistas::ContratistaResumen,
    services::autenticacion_service::UsuarioSesion,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, identidad_sesion, render_terminal_too_small,
    },
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table},
};

const ANCHO_MINIMO: u16 = 60;
const ALTO_MINIMO: u16 = 22;
const ANCHO_PANEL_LATERAL: u16 = 100;

const COMANDOS_BUSCAR: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Preparar"),
    CommandHint::new("ESC", "Limpiar/Volver"),
];
const COMANDOS_FORMULARIO: &[CommandHint<'static>] = &[
    CommandHint::new("TAB", "Campo"),
    CommandHint::new("←→", "Cambiar medio"),
    CommandHint::new("ENTER", "Registrar"),
    CommandHint::new("ESC", "Volver"),
];

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &NuevoIngresoState,
    sesion: &UsuarioSesion,
    theme: Theme,
) {
    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "ESC salir", theme);
        return;
    }

    let hora = hora_actual_texto();
    let contexto = identidad_sesion(sesion);
    let (estado_texto, estado_tipo) = estado_shell(state);
    let comandos = match state.etapa {
        EtapaNuevoIngreso::Buscar => COMANDOS_BUSCAR,
        EtapaNuevoIngreso::Formulario => COMANDOS_FORMULARIO,
    };

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "NUEVO INGRESO",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);

    render_cuerpo(frame, areas.body, state, theme);
}

fn estado_shell(state: &NuevoIngresoState) -> (String, StatusKind) {
    if let Some(error) = &state.error {
        return (error.clone(), StatusKind::Error);
    }
    match state.etapa {
        EtapaNuevoIngreso::Buscar => (String::new(), StatusKind::Normal),
        EtapaNuevoIngreso::Formulario => {
            let Some(p) = &state.preparacion else {
                return (String::new(), StatusKind::Normal);
            };
            match &p.resultado_acceso {
                ResultadoAcceso::PermitidoConAdvertencia => (
                    "PERMITIDO CON ADVERTENCIA · PRAIND próximo a vencer".to_owned(),
                    StatusKind::Warning,
                ),
                _ => (String::new(), StatusKind::Normal),
            }
        }
    }
}

fn render_cuerpo(frame: &mut Frame, area: Rect, state: &NuevoIngresoState, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);

    let enfocado_busqueda = matches!(state.etapa, EtapaNuevoIngreso::Buscar);
    let area_busqueda = render_campo(
        frame,
        filas[0],
        &format!("BUSCAR CONTRATISTA · {} RESULTADOS", state.contratistas.len()),
        &state.busqueda,
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
            Constraint::Length(10.min(filas[1].height.saturating_sub(5))),
        ])
        .split(filas[1]);
        render_separador_horizontal(frame, filas_apiladas[1], theme);
        (filas_apiladas[0], filas_apiladas[2])
    };
    render_tabla(frame, area_tabla, state, theme);
    let area_gafete = render_panel(frame, area_panel, state, theme);

    if enfocado_busqueda {
        frame.set_cursor_position((area_busqueda.x, area_busqueda.y));
    } else if state.campo_es_gafete()
        && let Some(area_campo) = area_gafete
    {
        let ancho_visible = Line::from(state.gafete_texto.as_str()).width() as u16;
        let x = area_campo.x.saturating_add(ancho_visible.min(area_campo.width));
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

fn render_opcion(frame: &mut Frame, area: Rect, etiqueta: &str, valor: &str, activo: bool, theme: Theme) {
    let marcador = if activo { ">" } else { " " };
    let estilo = if activo { theme.accent() } else { theme.base() };
    let flechas = if activo { (" ◀ ", " ▶") } else { ("  ", " ") };
    frame.render_widget(
        Paragraph::new(Line::from(format!(
            "{marcador} {etiqueta:<18}{}{valor}{}",
            flechas.0, flechas.1
        )))
        .style(estilo),
        area,
    );
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &NuevoIngresoState, theme: Theme) {
    let capacidad = area.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.contratistas.len());
    let filas = state
        .contratistas
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, c): (usize, &ContratistaResumen)| {
            let seleccionada = state.seleccion == Some(inicio + visible);
            Row::new([
                Cell::from(c.cedula.clone()),
                Cell::from(c.nombre.clone()),
                Cell::from(c.empresa_nombre.clone()),
                Cell::from(texto_tipo(c.tipo_ingreso)),
            ])
            .style(if seleccionada {
                theme.selected()
            } else {
                theme.base()
            })
        });
    let encabezado = Row::new(["CÉDULA", "NOMBRE", "EMPRESA", "TIPO"])
        .style(theme.muted())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(
            filas,
            [
                Constraint::Length(14),
                Constraint::Fill(3),
                Constraint::Fill(2),
                Constraint::Length(12),
            ],
        )
        .header(encabezado)
        .column_spacing(1),
        area,
    );
    if state.contratistas.is_empty() {
        frame.render_widget(
            Paragraph::new(if state.busqueda.is_empty() {
                "Escriba para buscar un contratista."
            } else {
                "Sin resultados para la búsqueda."
            })
            .style(theme.warning())
            .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
    }
}

fn render_contexto(c: &ContratistaResumen) -> Vec<Line<'static>> {
    vec![
        Line::from(c.nombre.clone()),
        Line::from(format!("{} · {}", c.cedula, c.empresa_nombre)),
        Line::from(""),
    ]
}

/// Devuelve, cuando aplica, el área del valor del campo GAFETE para que el
/// llamador posicione el cursor.
fn render_panel(
    frame: &mut Frame,
    area: Rect,
    state: &NuevoIngresoState,
    theme: Theme,
) -> Option<Rect> {
    match state.etapa {
        EtapaNuevoIngreso::Buscar => {
            frame.render_widget(
                Paragraph::new("Seleccione ENTER para preparar el ingreso.").style(theme.muted()),
                area,
            );
            None
        }
        EtapaNuevoIngreso::Formulario => render_formulario(frame, area, state, theme),
    }
}

fn render_formulario(
    frame: &mut Frame,
    area: Rect,
    state: &NuevoIngresoState,
    theme: Theme,
) -> Option<Rect> {
    let (Some(c), Some(p)) = (state.contratista(), state.preparacion()) else {
        return None;
    };
    let requiere_gafete = p.requiere_gafete;
    let mut lineas = render_contexto(c);
    lineas.push(Line::from(format!("Tipo de ingreso    {}", texto_tipo(p.tipo_ingreso))).style(theme.base()));
    lineas.push(Line::from(""));

    let filas = Layout::vertical(if requiere_gafete {
        vec![Constraint::Length(lineas.len() as u16), Constraint::Length(1), Constraint::Length(3)]
    } else {
        vec![Constraint::Length(lineas.len() as u16), Constraint::Length(1)]
    })
    .split(area);
    frame.render_widget(Paragraph::new(lineas), filas[0]);

    let medio_enfocado = !state.campo_es_gafete();
    render_opcion(
        frame,
        filas[1],
        "MEDIO DE INGRESO",
        texto_medio(state.medio_actual()),
        medio_enfocado,
        theme,
    );

    if requiere_gafete {
        let area_gafete = render_campo(
            frame,
            filas[2],
            "NÚMERO DE GAFETE",
            &state.gafete_texto,
            state.campo_es_gafete(),
            theme,
        );
        return Some(area_gafete);
    }
    None
}

fn texto_tipo(t: crate::models::tipo_ingreso::TipoIngreso) -> &'static str {
    match t {
        crate::models::tipo_ingreso::TipoIngreso::Praind => "PRAIND",
        crate::models::tipo_ingreso::TipoIngreso::InHouse => "IN HOUSE",
        crate::models::tipo_ingreso::TipoIngreso::PorCorreo => "POR CORREO",
        crate::models::tipo_ingreso::TipoIngreso::Swat => "SWAT",
    }
}
