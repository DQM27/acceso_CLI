use super::*;
use crate::{
    database::queries::contratistas::ContratistaResumen,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, ThemePreset, render_terminal_too_small,
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
const COMANDOS_PREPARACION: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Continuar"),
    CommandHint::new("ESC", "Cambiar contratista"),
];
const COMANDOS_MEDIO: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Continuar"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_GAFETE: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Continuar"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_CONFIRMAR: &[CommandHint<'static>] = &[
    CommandHint::new("Y", "Confirmar"),
    CommandHint::new("N/ESC", "Volver"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &NuevoIngresoState) {
    let theme = ThemePreset::Brisas.theme();

    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "ESC salir", theme);
        return;
    }

    let hora = hora_actual_texto();
    let contexto = format!("Usuario: {}", state.usuario_nombre);
    let (estado_texto, estado_tipo) = estado_shell(state);
    let comandos = match state.etapa {
        EtapaNuevoIngreso::Buscar => COMANDOS_BUSCAR,
        EtapaNuevoIngreso::Preparacion => COMANDOS_PREPARACION,
        EtapaNuevoIngreso::Medio { .. } => COMANDOS_MEDIO,
        EtapaNuevoIngreso::Gafete => COMANDOS_GAFETE,
        EtapaNuevoIngreso::Confirmar => COMANDOS_CONFIRMAR,
    };

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "NUEVO INGRESO",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
    };
    let areas = shell.render(frame, area, theme);

    render_cuerpo(frame, areas.body, state, theme);
}

fn estado_shell(state: &NuevoIngresoState) -> (String, StatusKind) {
    if let Some(error) = &state.error {
        return (error.clone(), StatusKind::Error);
    }
    match state.etapa {
        EtapaNuevoIngreso::Preparacion => {
            let Some(p) = &state.preparacion else {
                return (String::new(), StatusKind::Normal);
            };
            if p.tiene_ingreso_activo {
                return (
                    "El contratista ya tiene un ingreso activo.".to_owned(),
                    StatusKind::Error,
                );
            }
            match &p.resultado_acceso {
                ResultadoAcceso::Permitido => ("ACCESO PERMITIDO".to_owned(), StatusKind::Success),
                ResultadoAcceso::PermitidoConAdvertencia => (
                    "PERMITIDO CON ADVERTENCIA · PRAIND próximo a vencer".to_owned(),
                    StatusKind::Warning,
                ),
                ResultadoAcceso::Denegado(m) => {
                    (texto_denegacion(m.clone()), StatusKind::Error)
                }
            }
        }
        EtapaNuevoIngreso::Medio { .. } => (
            "Seleccione el medio de ingreso.".to_owned(),
            StatusKind::Normal,
        ),
        EtapaNuevoIngreso::Gafete => (
            "Ingrese el número de gafete.".to_owned(),
            StatusKind::Normal,
        ),
        EtapaNuevoIngreso::Confirmar => (
            "¿Confirma el registro de este ingreso?".to_owned(),
            StatusKind::Warning,
        ),
        EtapaNuevoIngreso::Buscar => (String::new(), StatusKind::Normal),
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
    } else if matches!(state.etapa, EtapaNuevoIngreso::Gafete)
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
        EtapaNuevoIngreso::Preparacion => {
            render_preparacion(frame, area, state, theme);
            None
        }
        EtapaNuevoIngreso::Medio { opcion } => {
            render_medio(frame, area, state, opcion, theme);
            None
        }
        EtapaNuevoIngreso::Gafete => Some(render_gafete(frame, area, state, theme)),
        EtapaNuevoIngreso::Confirmar => {
            render_confirmar(frame, area, state, theme);
            None
        }
    }
}

fn render_contexto(c: &ContratistaResumen) -> Vec<Line<'static>> {
    vec![
        Line::from(c.nombre.clone()),
        Line::from(format!("{} · {}", c.cedula, c.empresa_nombre)),
        Line::from(""),
    ]
}

fn render_preparacion(frame: &mut Frame, area: Rect, state: &NuevoIngresoState, theme: Theme) {
    let (Some(c), Some(p)) = (state.contratista(), &state.preparacion) else {
        return;
    };
    let mut lineas = render_contexto(c);
    lineas.push(Line::from(format!("Tipo de ingreso    {}", texto_tipo(p.tipo_ingreso))).style(theme.base()));
    lineas.push(
        Line::from(format!(
            "Gafete             {}",
            if p.requiere_gafete { "REQUERIDO" } else { "SIN GAFETE" }
        ))
        .style(theme.base()),
    );
    frame.render_widget(Paragraph::new(lineas).style(theme.base()), area);
}

fn render_medio(
    frame: &mut Frame,
    area: Rect,
    state: &NuevoIngresoState,
    opcion: usize,
    theme: Theme,
) {
    let Some(c) = state.contratista() else { return };
    let mut lineas = render_contexto(c);
    lineas.push(Line::from("MEDIO DE INGRESO").style(theme.muted()));
    for (i, medio) in MEDIOS.iter().enumerate() {
        let marcador = if i == opcion { ">" } else { " " };
        lineas.push(
            Line::from(format!("{marcador} {}", texto_medio(*medio))).style(if i == opcion {
                theme.selected()
            } else {
                theme.base()
            }),
        );
    }
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_gafete(frame: &mut Frame, area: Rect, state: &NuevoIngresoState, theme: Theme) -> Rect {
    let Some(c) = state.contratista() else {
        return Rect::new(area.x, area.y, area.width, 1);
    };
    let contexto = render_contexto(c);
    let filas = Layout::vertical([Constraint::Length(contexto.len() as u16), Constraint::Length(3)])
        .split(area);
    frame.render_widget(Paragraph::new(contexto), filas[0]);
    render_campo(frame, filas[1], "NÚMERO DE GAFETE", &state.gafete_texto, true, theme)
}

fn render_confirmar(frame: &mut Frame, area: Rect, state: &NuevoIngresoState, theme: Theme) {
    let (Some(c), Some(p)) = (state.contratista(), &state.preparacion) else {
        return;
    };
    let mut lineas = render_contexto(c);
    lineas.push(Line::from(format!("Empresa            {}", p.empresa_nombre)).style(theme.base()));
    lineas.push(Line::from(format!("Tipo de ingreso    {}", texto_tipo(p.tipo_ingreso))).style(theme.base()));
    lineas.push(
        Line::from(format!(
            "Medio              {}",
            state.medio.map(texto_medio).unwrap_or("—")
        ))
        .style(theme.base()),
    );
    lineas.push(
        Line::from(format!(
            "Gafete             {}",
            state.gafete.map_or("SIN GAFETE".to_owned(), |n| n.to_string())
        ))
        .style(theme.base()),
    );
    lineas.push(Line::from(format!("Registrado por     {}", state.usuario_nombre)).style(theme.base()));
    if matches!(p.resultado_acceso, ResultadoAcceso::PermitidoConAdvertencia) {
        lineas.push(Line::from(""));
        lineas.push(Line::from("PRAIND próximo a vencer").style(theme.warning()));
    }
    frame.render_widget(Paragraph::new(lineas), area);
}

fn texto_tipo(t: crate::models::tipo_ingreso::TipoIngreso) -> &'static str {
    match t {
        crate::models::tipo_ingreso::TipoIngreso::Praind => "PRAIND",
        crate::models::tipo_ingreso::TipoIngreso::InHouse => "IN HOUSE",
        crate::models::tipo_ingreso::TipoIngreso::PorCorreo => "POR CORREO",
        crate::models::tipo_ingreso::TipoIngreso::Swat => "SWAT",
    }
}

fn texto_denegacion(m: crate::domain::resultado_acceso::MotivoDenegacion) -> String {
    match m {
        crate::domain::resultado_acceso::MotivoDenegacion::SinAcceso => {
            "Acceso denegado · no tiene acceso autorizado".into()
        }
        crate::domain::resultado_acceso::MotivoDenegacion::PraindVencido => {
            "Acceso denegado · PRAIND vencido o requerido".into()
        }
    }
}
