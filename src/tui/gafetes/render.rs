use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table},
};

use super::*;
use crate::{
    database::queries::{gafetes::GafeteResumen, gafetes_incidentes::IncidenteGafete},
    models::gafete::{EstadoGafete, MotivoResolucionGafete, TipoIncidenteGafete},
    services::autenticacion_service::UsuarioSesion,
    tiempo::{a_costa_rica, hora_actual_texto},
    tui::ui_kit::{
        CommandHint, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, ScreenShell, StatusKind, Theme,
        clasificar_mensaje, detail_line, empty_state, identidad_sesion, marcador_seleccion,
        master_detail_areas, panel_vacio, posicionar_cursor_campo, render_form_field,
        render_separator, render_terminal_too_small,
    },
};

const COMANDOS_NORMAL: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("N", "Alta"),
    CommandHint::new("B", "Baja"),
    CommandHint::new("P", "Perdido"),
    CommandHint::new("R", "Resolver"),
    CommandHint::new("H", "Historial"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_HISTORIAL: &[CommandHint<'static>] = &[CommandHint::new("ESC", "Volver")];
const COMANDOS_BUSQUEDA: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Aplicar"),
    CommandHint::new("ESC", "Limpiar"),
];
const COMANDOS_ALTA: &[CommandHint<'static>] = &[
    CommandHint::new("TAB", "Individual/Rango"),
    CommandHint::new("ENTER", "Guardar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_BUSCAR_DEUDOR: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Marcar perdido"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_CONFIRMACION_RESOLVER: &[CommandHint<'static>] = &[
    CommandHint::new("1", "Pagado"),
    CommandHint::new("2", "Apareció"),
    CommandHint::new("ENTER", "Confirmar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_CONFIRMACION: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Confirmar"),
    CommandHint::new("ESC", "Cancelar"),
];

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &GafetesState,
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
    let comandos = match &state.modo {
        ModoGafetes::Normal => COMANDOS_NORMAL,
        ModoGafetes::Busqueda { .. } => COMANDOS_BUSQUEDA,
        ModoGafetes::Alta(_) => COMANDOS_ALTA,
        ModoGafetes::MarcarPerdidoBuscarDeudor(_) => COMANDOS_BUSCAR_DEUDOR,
        ModoGafetes::ConfirmacionResolver { .. } => COMANDOS_CONFIRMACION_RESOLVER,
        ModoGafetes::ConfirmacionBaja { .. } => COMANDOS_CONFIRMACION,
        ModoGafetes::Historial { .. } => COMANDOS_HISTORIAL,
    };

    // Sin pestañas a propósito: `GestionGafetes` es un acceso por letra (G),
    // no numérico — mismo grupo que Cli/CerrarSesion/Salir, que
    // tampoco participan de la barra de pestañas del tema Negro
    // (`OpcionMenu::indice_pestana`).
    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "GESTIÓN DE GAFETES",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        tabs: None,
        authenticated: true,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);

    match &state.modo {
        ModoGafetes::Historial { incidentes, .. } => {
            render_historial(frame, areas.body, incidentes, theme)
        }
        _ => render_cuerpo(frame, areas.body, state, theme),
    }
}

fn estado_shell(state: &GafetesState) -> (String, StatusKind) {
    if let ModoGafetes::Historial { numero, incidentes } = &state.modo {
        return (
            format!(
                "HISTORIAL · Gafete {numero:02} · {} incidente(s)",
                incidentes.len()
            ),
            StatusKind::Normal,
        );
    }
    if let ModoGafetes::Alta(formulario) = &state.modo
        && let Some(error) = &formulario.error
    {
        return (format!("✕ {error}"), StatusKind::Error);
    }
    if let ModoGafetes::ConfirmacionBaja { numero, .. } = &state.modo {
        return (
            format!("CONFIRMAR · dar de baja el gafete {numero:02}"),
            StatusKind::Warning,
        );
    }
    if let ModoGafetes::ConfirmacionResolver { numero, motivo, .. } = &state.modo {
        let texto_motivo = texto_motivo(*motivo);
        return (
            format!("CONFIRMAR · resolver deuda del gafete {numero:02} ({texto_motivo})"),
            StatusKind::Warning,
        );
    }
    if let Some(mensaje) = &state.mensaje {
        return (mensaje.clone(), clasificar_mensaje(mensaje));
    }
    (String::new(), StatusKind::Normal)
}

fn render_cuerpo(frame: &mut Frame, area: Rect, state: &GafetesState, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);

    let enfocado_busqueda = matches!(state.modo, ModoGafetes::Busqueda { .. });
    let area_busqueda = render_form_field(
        frame,
        filas[0],
        &format!(
            "BUSCAR (numero / estado:) · {} GAFETES",
            state.gafetes.len()
        ),
        &state.filtro,
        enfocado_busqueda,
        theme,
    );

    let areas = master_detail_areas(filas[1], 63, 7);
    render_separator(frame, areas.separator, areas.orientation, theme);
    let (area_tabla, area_panel) = (areas.master, areas.detail);
    render_tabla(frame, area_tabla, state, theme);
    let area_cursor = render_panel(frame, area_panel, state, theme);

    let cursor = match &state.modo {
        ModoGafetes::Busqueda { texto } => Some((area_busqueda, texto.clone())),
        _ => area_cursor,
    };
    if let Some((area_campo, texto)) = cursor {
        posicionar_cursor_campo(frame, area_campo, &texto);
    }
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &GafetesState, theme: Theme) {
    let capacidad = area.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.gafetes.len());
    let filas = state
        .gafetes
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, g): (usize, &GafeteResumen)| {
            let seleccionada = state.seleccion == Some(inicio + visible);
            let estilo = if seleccionada {
                theme.selected()
            } else if g.estado == EstadoGafete::DeBaja {
                theme.muted()
            } else if g.estado == EstadoGafete::Perdido {
                theme.danger()
            } else {
                theme.base()
            };
            Row::new([
                Cell::from(format!(
                    "{} {:02}",
                    marcador_seleccion(seleccionada),
                    g.numero
                )),
                Cell::from(texto_estado(g.estado)),
                Cell::from(g.contratista_deudor_nombre.clone().unwrap_or_default()),
            ])
            .style(estilo)
        });
    let encabezado = Row::new(["NÚMERO", "ESTADO", "DEUDOR"])
        .style(theme.muted())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(
            filas,
            [
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Fill(3),
            ],
        )
        .header(encabezado)
        .column_spacing(1),
        area,
    );
    if state.gafetes.is_empty() {
        empty_state(
            frame,
            area,
            if state.filtro.is_empty() {
                "Sin gafetes en el catálogo — presione N para dar de alta."
            } else {
                "Sin gafetes que coincidan con la búsqueda."
            },
            theme,
        );
    }
}

/// Devuelve, cuando aplica, el área y el `TextInput` del campo con foco en
/// el panel de detalle para que el llamador posicione el cursor.
fn render_panel(
    frame: &mut Frame,
    area: Rect,
    state: &GafetesState,
    theme: Theme,
) -> Option<(Rect, TextInput)> {
    match &state.modo {
        ModoGafetes::Alta(formulario) => {
            Some(render_formulario_alta(frame, area, formulario, theme))
        }
        ModoGafetes::MarcarPerdidoBuscarDeudor(buscar) => {
            Some(render_buscar_deudor(frame, area, buscar, theme))
        }
        ModoGafetes::Normal
        | ModoGafetes::Busqueda { .. }
        | ModoGafetes::ConfirmacionBaja { .. }
        | ModoGafetes::ConfirmacionResolver { .. }
        | ModoGafetes::Historial { .. } => {
            match state.gafete_seleccionado() {
                Some(g) => render_detalle(frame, area, g, theme),
                None => panel_vacio(frame, area, "No hay un gafete seleccionado.", theme),
            }
            None
        }
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, g: &GafeteResumen, theme: Theme) {
    let mut lineas = vec![
        Line::from(format!("Gafete {:02}", g.numero)).style(theme.title()),
        Line::from(""),
        detail_line("Estado", texto_estado(g.estado), theme),
    ];
    if let Some(deudor) = &g.contratista_deudor_nombre {
        lineas.push(detail_line("Deudor", deudor.as_str(), theme));
        if let Some(fecha) = &g.fecha_marcado_perdido {
            lineas.push(detail_line("Marcado perdido", fecha.as_str(), theme));
        }
    }
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_formulario_alta(
    frame: &mut Frame,
    area: Rect,
    formulario: &FormularioAlta,
    theme: Theme,
) -> (Rect, TextInput) {
    match formulario.modo {
        ModoFormularioAlta::Individual => {
            let filas = Layout::vertical([Constraint::Length(3)]).split(area);
            let campo = render_form_field(
                frame,
                filas[0],
                "NÚMERO DE GAFETE",
                formulario.numero.value(),
                true,
                theme,
            );
            (campo, formulario.numero.clone())
        }
        ModoFormularioAlta::Rango => {
            let filas =
                Layout::vertical([Constraint::Length(3), Constraint::Length(3)]).split(area);
            let area_desde = render_form_field(
                frame,
                filas[0],
                "DESDE",
                formulario.desde.value(),
                formulario.campo == CampoAlta::Desde,
                theme,
            );
            let area_hasta = render_form_field(
                frame,
                filas[1],
                "HASTA",
                formulario.hasta.value(),
                formulario.campo == CampoAlta::Hasta,
                theme,
            );
            if formulario.campo == CampoAlta::Hasta {
                (area_hasta, formulario.hasta.clone())
            } else {
                (area_desde, formulario.desde.clone())
            }
        }
    }
}

fn render_buscar_deudor(
    frame: &mut Frame,
    area: Rect,
    buscar: &BuscarDeudor,
    theme: Theme,
) -> (Rect, TextInput) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(2)]).split(area);
    let area_campo = render_form_field(
        frame,
        filas[0],
        &format!("DEUDOR DEL GAFETE {:02} · CÉDULA O NOMBRE", buscar.numero),
        buscar.texto.value(),
        true,
        theme,
    );
    let filas_resultados = buscar
        .resultados
        .iter()
        .enumerate()
        .map(|(indice, c)| {
            let seleccionada = buscar.seleccion == Some(indice);
            Line::from(format!(
                "{} {} · {}",
                marcador_seleccion(seleccionada),
                c.cedula,
                c.nombre
            ))
            .style(if seleccionada {
                theme.selected()
            } else {
                theme.base()
            })
        })
        .collect::<Vec<_>>();
    if filas_resultados.is_empty() {
        empty_state(frame, filas[1], "Sin coincidencias.", theme);
    } else {
        frame.render_widget(Paragraph::new(filas_resultados), filas[1]);
    }
    (area_campo, buscar.texto.clone())
}

/// Tabla de ancho completo (mismo patrón que `auditoria/render.rs`) en vez de
/// mantener el maestro-detalle de `render_cuerpo` — el historial de un
/// gafete puntual es una lista chica y no necesita el catálogo a la
/// izquierda mientras se consulta.
fn render_historial(frame: &mut Frame, area: Rect, incidentes: &[IncidenteGafete], theme: Theme) {
    let encabezado = Row::new([
        "FECHA Y HORA (CR)",
        "EVENTO",
        "OPERADOR",
        "ASIGNADO A",
        "MOTIVO",
    ])
    .style(theme.muted())
    .bottom_margin(1);
    let filas = incidentes.iter().map(|incidente| {
        Row::new([
            Cell::from(
                a_costa_rica(incidente.fecha_hora)
                    .format("%d/%m/%Y %H:%M")
                    .to_string(),
            ),
            Cell::from(texto_tipo_incidente(incidente.tipo)),
            Cell::from(incidente.usuario_nombre.clone()),
            Cell::from(
                incidente
                    .contratista_nombre
                    .clone()
                    .unwrap_or_else(|| "—".into()),
            ),
            Cell::from(texto_motivo_opcional(incidente.motivo_resolucion)),
        ])
    });
    frame.render_widget(
        Table::new(
            filas,
            [
                Constraint::Length(17),
                Constraint::Length(16),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Length(10),
            ],
        )
        .header(encabezado)
        .column_spacing(1),
        area,
    );
    if incidentes.is_empty() {
        empty_state(
            frame,
            area,
            "Este gafete no tiene incidentes registrados.",
            theme,
        );
    }
}

fn texto_tipo_incidente(tipo: TipoIncidenteGafete) -> &'static str {
    match tipo {
        TipoIncidenteGafete::Perdido => "Marcado perdido",
        TipoIncidenteGafete::Resuelto => "Resuelto",
    }
}

fn texto_motivo_opcional(motivo: Option<MotivoResolucionGafete>) -> &'static str {
    match motivo {
        Some(motivo) => texto_motivo(motivo),
        None => "—",
    }
}

fn texto_estado(estado: EstadoGafete) -> &'static str {
    match estado {
        EstadoGafete::Disponible => "DISPONIBLE",
        EstadoGafete::Perdido => "PERDIDO",
        EstadoGafete::DeBaja => "DE BAJA",
    }
}

fn texto_motivo(motivo: MotivoResolucionGafete) -> &'static str {
    match motivo {
        MotivoResolucionGafete::Pagado => "pagado",
        MotivoResolucionGafete::Aparecido => "apareció",
    }
}
