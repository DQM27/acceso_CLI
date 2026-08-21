use super::*;
use crate::{
    services::{
        autenticacion_service::UsuarioSesion, registro_ingreso_service::IngresoActivoResumen,
    },
    tiempo::{a_costa_rica, hora_actual_texto},
    tui::ui_kit::{
        CommandHint, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, ScreenShell, StatusKind, Theme,
        detail_line, identidad_sesion, master_detail_areas, posicionar_cursor_campo,
        render_form_field, render_separator, render_terminal_too_small,
    },
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table},
};

const COMANDOS_NORMAL: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Salida"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("F4", "Columnas"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_NORMAL_FILTRADO: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Salida"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("F4", "Columnas"),
    CommandHint::new("ESC", "Limpiar filtro"),
];
const COMANDOS_BUSQUEDA: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Aplicar"),
    CommandHint::new("ESC", "Limpiar"),
];
const COMANDOS_CONFIRMAR: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Confirmar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_COLUMNAS: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("SPACE", "Mostrar/Ocultar"),
    CommandHint::new("ESC", "Cerrar"),
];

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &ActivosState,
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
        ModoActivos::Normal if !state.filtro.is_empty() => COMANDOS_NORMAL_FILTRADO,
        ModoActivos::Normal => COMANDOS_NORMAL,
        ModoActivos::Busqueda { .. } => COMANDOS_BUSQUEDA,
        ModoActivos::ConfirmarSalida { .. } => COMANDOS_CONFIRMAR,
        ModoActivos::Columnas { .. } => COMANDOS_COLUMNAS,
    };

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "INGRESOS ACTIVOS",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        authenticated: true,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: Some("Claves: empresa, tipo, gafete, medio"),
    };
    let areas = shell.render(frame, area, theme);

    render_cuerpo(frame, areas.body, state, theme);
}

fn estado_shell(state: &ActivosState) -> (String, StatusKind) {
    match &state.modo {
        ModoActivos::ConfirmarSalida { id } => {
            let nombre = state
                .registro(*id)
                .map(|r| r.contratista_nombre.as_str())
                .unwrap_or_default();
            (
                format!(
                    "CONFIRMAR SALIDA · {nombre} · cerrará el ingreso activo y liberará el gafete"
                ),
                StatusKind::Warning,
            )
        }
        _ => {
            if let Some(mensaje) = &state.mensaje {
                let tipo = if mensaje.starts_with('✓') {
                    StatusKind::Success
                } else {
                    StatusKind::Error
                };
                return (mensaje.clone(), tipo);
            }
            (String::new(), StatusKind::Normal)
        }
    }
}

fn render_cuerpo(frame: &mut Frame, area: Rect, state: &ActivosState, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);

    let enfocado_busqueda = matches!(state.modo, ModoActivos::Busqueda { .. });
    let texto_busqueda = match &state.modo {
        ModoActivos::Busqueda { texto } => texto.value(),
        _ => state.filtro.as_str(),
    };
    let area_busqueda = render_form_field(
        frame,
        filas[0],
        &format!(
            "BUSCAR · {} DE {} DENTRO · {}",
            state.cantidad(),
            state.total_activos(),
            state.resumen_consulta()
        ),
        texto_busqueda,
        enfocado_busqueda,
        theme,
    );

    let areas = master_detail_areas(filas[1], 63, 10);
    render_separator(frame, areas.separator, areas.orientation, theme);
    let (area_tabla, area_panel) = (areas.master, areas.detail);
    render_tabla(frame, area_tabla, state, theme);
    render_panel(frame, area_panel, state, theme);

    if let ModoActivos::Busqueda { texto } = &state.modo {
        posicionar_cursor_campo(frame, area_busqueda, texto);
    }
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &ActivosState, theme: Theme) {
    let columnas: Vec<_> = state
        .columnas
        .iter()
        .filter_map(|(columna, visible)| visible.then_some(*columna))
        .collect();
    let anchos: Vec<_> = columnas.iter().map(|c| c.constraint()).collect();
    let capacidad = area.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.registros.len());
    let filas = state
        .registros
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, registro)| {
            let seleccionada = state.seleccion == Some(inicio + visible);
            let celdas = columnas.iter().enumerate().map(|(indice, columna)| {
                let estilo = if seleccionada {
                    theme.selected()
                } else if *columna == Columna::Tipo
                    && !matches!(
                        registro.resultado_acceso,
                        crate::domain::resultado_acceso::ResultadoAcceso::Permitido
                    )
                {
                    theme.warning()
                } else {
                    theme.base()
                };
                let valor = valor_columna(registro, *columna);
                let valor = if indice == 0 {
                    format!("{} {valor}", if seleccionada { ">" } else { " " })
                } else {
                    valor
                };
                Cell::from(valor).style(estilo)
            });
            Row::new(celdas).style(if seleccionada {
                theme.selected()
            } else {
                theme.base()
            })
        });
    let encabezado = Row::new(columnas.iter().map(|c| c.titulo()))
        .style(theme.muted())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(filas, anchos)
            .header(encabezado)
            .column_spacing(1),
        area,
    );
    if state.registros.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay ingresos activos que coincidan con la búsqueda.")
                .style(theme.warning())
                .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
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
        Columna::Hora => a_costa_rica(registro.fecha_hora_ingreso)
            .format("%H:%M")
            .to_string(),
        Columna::Gafete => registro
            .gafete_numero
            .map_or_else(|| "S/G".to_owned(), |gafete| format!("{gafete:02}")),
        Columna::Medio => texto_medio(registro.medio_ingreso).into(),
        Columna::Usuario => registro.usuario_ingreso_nombre.clone(),
    }
}

fn render_panel(frame: &mut Frame, area: Rect, state: &ActivosState, theme: Theme) {
    match &state.modo {
        ModoActivos::ConfirmarSalida { id } => {
            if let Some(registro) = state.registro(*id) {
                render_detalle(frame, area, registro, theme);
            }
        }
        ModoActivos::Columnas { seleccion } => {
            render_columnas(frame, area, state, *seleccion, theme);
        }
        ModoActivos::Normal | ModoActivos::Busqueda { .. } => match state.seleccionado() {
            Some(registro) => render_detalle(frame, area, registro, theme),
            None => frame.render_widget(
                Paragraph::new("No hay un registro seleccionado.").style(theme.muted()),
                area,
            ),
        },
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, registro: &IngresoActivoResumen, theme: Theme) {
    let lineas = vec![
        Line::from(registro.contratista_nombre.clone()).style(theme.title()),
        detail_line("Cédula", registro.cedula.clone(), theme),
        detail_line("Empresa", registro.empresa_nombre.clone(), theme),
        Line::from(""),
        detail_line("Tipo", texto_tipo(registro.tipo_ingreso), theme),
        detail_line("Medio", texto_medio(registro.medio_ingreso), theme),
        detail_line("Gafete", valor_columna(registro, Columna::Gafete), theme),
        Line::from(""),
        detail_line(
            "Ingreso",
            a_costa_rica(registro.fecha_hora_ingreso)
                .format("%d/%m/%Y %H:%M")
                .to_string(),
            theme,
        ),
        detail_line(
            "Registrado por",
            registro.usuario_ingreso_nombre.clone(),
            theme,
        ),
    ];
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_columnas(
    frame: &mut Frame,
    area: Rect,
    state: &ActivosState,
    seleccion: usize,
    theme: Theme,
) {
    let mut lineas: Vec<Line<'static>> = state
        .columnas
        .iter()
        .enumerate()
        .map(|(indice, (columna, visible))| {
            let marcador = if indice == seleccion { ">" } else { " " };
            let caja = if *visible { "x" } else { " " };
            Line::from(format!("{marcador} [{caja}] {}", columna.titulo())).style(
                if indice == seleccion {
                    theme.selected()
                } else {
                    theme.base()
                },
            )
        })
        .collect();
    if let Some(mensaje) = &state.mensaje {
        lineas.push(Line::from(""));
        lineas.push(Line::from(mensaje.clone()).style(theme.warning()));
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
fn texto_medio(m: crate::models::medio_ingreso::MedioIngreso) -> &'static str {
    match m {
        crate::models::medio_ingreso::MedioIngreso::Caminando => "Caminando",
        crate::models::medio_ingreso::MedioIngreso::Vehiculo => "Vehículo",
    }
}
