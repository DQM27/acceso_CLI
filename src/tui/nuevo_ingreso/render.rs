use super::{
    EtapaNuevoIngreso, ModoBuscarIngreso, NuevoIngresoState, ResultadoAcceso, texto_medio,
};
use crate::tui::menu_principal::OpcionMenu;
use crate::{
    database::queries::contratistas::ContratistaResumen,
    services::autenticacion_service::UsuarioSesion,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        ChoiceFieldOptions, CommandHint, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, ScreenShell,
        StatusKind, Theme, empty_state, identidad_sesion, marcador_seleccion, master_detail_areas,
        posicionar_cursor, posicionar_cursor_campo, render_choice_field, render_form_field,
        render_separator, render_terminal_too_small,
    },
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table},
};

const COMANDOS_NORMAL: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Preparar"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_NORMAL_FILTRADO: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Preparar"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("ESC", "Limpiar filtro"),
];
const COMANDOS_BUSQUEDA: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Preparar"),
    CommandHint::new("ESC", "Cerrar búsqueda"),
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
    let (estado_texto, estado_tipo) = estado_shell(state);
    let comandos = match (state.etapa, &state.modo) {
        (EtapaNuevoIngreso::Buscar, ModoBuscarIngreso::Normal) if !state.filtro.is_empty() => {
            COMANDOS_NORMAL_FILTRADO
        }
        (EtapaNuevoIngreso::Buscar, ModoBuscarIngreso::Normal) => COMANDOS_NORMAL,
        (EtapaNuevoIngreso::Buscar, ModoBuscarIngreso::Busqueda { .. }) => COMANDOS_BUSQUEDA,
        (EtapaNuevoIngreso::Formulario, _) => COMANDOS_FORMULARIO,
    };

    let tabs = OpcionMenu::barra_pestanas(sesion.rol, OpcionMenu::NuevoIngreso);
    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "NUEVO INGRESO",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        tabs: theme.navegacion_pestanas.then_some(&tabs),
        authenticated: true,
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
    if let Some(mensaje) = &state.mensaje {
        return (mensaje.clone(), StatusKind::Success);
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

    let enfocado_busqueda = matches!(state.modo, ModoBuscarIngreso::Busqueda { .. });
    let texto_busqueda = match &state.modo {
        ModoBuscarIngreso::Busqueda { texto } => texto.value(),
        ModoBuscarIngreso::Normal => state.filtro.as_str(),
    };
    let etiqueta_busqueda = state.resultados_ocultos().map_or_else(
        || {
            format!(
                "BUSCAR CONTRATISTA · {} RESULTADOS",
                state.contratistas.len()
            )
        },
        |total| {
            format!(
                "BUSCAR CONTRATISTA · {} DE {total} RESULTADOS · afine la búsqueda para ver el resto",
                state.contratistas.len()
            )
        },
    );
    let area_busqueda = render_form_field(
        frame,
        filas[0],
        &etiqueta_busqueda,
        texto_busqueda,
        enfocado_busqueda,
        theme,
    );

    let areas = master_detail_areas(filas[1], 63, 10);
    render_separator(frame, areas.separator, areas.orientation, theme);
    let (area_tabla, area_panel) = (areas.master, areas.detail);
    render_tabla(frame, area_tabla, state, theme);
    let area_gafete = render_panel(frame, area_panel, state, theme);

    if let ModoBuscarIngreso::Busqueda { texto } = &state.modo {
        posicionar_cursor_campo(frame, area_busqueda, texto);
    } else if state.campo_es_gafete()
        && let Some(area_campo) = area_gafete
    {
        let antes_del_cursor: String = state
            .gafete_texto
            .chars()
            .take(state.gafete_cursor)
            .collect();
        posicionar_cursor(frame, area_campo, &antes_del_cursor);
    }
}

fn render_opcion(
    frame: &mut Frame,
    area: Rect,
    etiqueta: &str,
    valor: &str,
    activo: bool,
    theme: Theme,
) {
    render_choice_field(
        frame,
        area,
        etiqueta,
        valor,
        activo,
        theme,
        ChoiceFieldOptions::arrows(18),
    );
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &NuevoIngresoState, theme: Theme) {
    let capacidad = area.height.saturating_sub(2) as usize;
    let inicio = state
        .inicio_visible(capacidad)
        .min(state.contratistas.len());
    let filas = state
        .contratistas
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, c): (usize, &ContratistaResumen)| {
            let seleccionada = state.seleccion == Some(inicio + visible);
            Row::new([
                Cell::from(format!("{} {}", marcador_seleccion(seleccionada), c.cedula)),
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
        empty_state(
            frame,
            area,
            if state.filtro.is_empty() {
                "Presione / para buscar un contratista."
            } else {
                "Sin resultados para la búsqueda."
            },
            theme,
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
            let contenido = state
                .seleccion
                .and_then(|indice| state.contratistas.get(indice))
                .map_or_else(
                    || vec![Line::from("Seleccione un contratista.").style(theme.muted())],
                    |contratista| {
                        let mut lineas = render_contexto(contratista);
                        lineas.push(Line::from("ESTADO ACTUAL").style(theme.muted()));
                        if contratista.tiene_ingreso_activo {
                            lineas.push(
                                Line::from("● DENTRO · tiene un ingreso activo")
                                    .style(theme.danger()),
                            );
                        } else {
                            lineas.push(
                                Line::from("● FUERA · sin ingreso activo").style(theme.success()),
                            );
                        }
                        lineas.push(Line::from(""));
                        lineas.push(Line::from("PERMISO DE ACCESO").style(theme.muted()));
                        if contratista.tiene_acceso {
                            lineas.push(Line::from("● HABILITADO").style(theme.success()));
                        } else {
                            lineas.push(
                                Line::from("● DENEGADO · no tiene acceso autorizado")
                                    .style(theme.danger()),
                            );
                        }
                        if contratista.tiene_ingreso_activo {
                            lineas.push(
                                Line::from("No puede registrar otro ingreso.").style(theme.muted()),
                            );
                        } else if !contratista.tiene_acceso {
                            lineas.push(
                                Line::from("No puede registrar un ingreso.").style(theme.muted()),
                            );
                        } else {
                            lineas.push(
                                Line::from("ENTER para validar y preparar el ingreso.")
                                    .style(theme.muted()),
                            );
                        }
                        lineas
                    },
                );
            frame.render_widget(Paragraph::new(contenido), area);
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
    lineas.push(
        Line::from(format!("Tipo de ingreso    {}", texto_tipo(p.tipo_ingreso)))
            .style(theme.base()),
    );
    if !p.gafetes_deuda.is_empty() {
        let numeros = p
            .gafetes_deuda
            .iter()
            .map(|numero| format!("#{numero:02}"))
            .collect::<Vec<_>>()
            .join(", ");
        lineas.push(
            Line::from(format!("⚠ Este contratista debe el gafete {numeros}"))
                .style(theme.warning()),
        );
    }
    lineas.push(Line::from(""));

    let filas = Layout::vertical(if requiere_gafete {
        vec![
            Constraint::Length(u16::try_from(lineas.len()).unwrap_or(u16::MAX)),
            Constraint::Length(1),
            Constraint::Length(3),
        ]
    } else {
        vec![
            Constraint::Length(u16::try_from(lineas.len()).unwrap_or(u16::MAX)),
            Constraint::Length(1),
        ]
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
        let area_gafete = render_form_field(
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
