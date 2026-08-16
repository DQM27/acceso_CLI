use super::*;
use crate::{
    services::registro_ingreso_service::IngresoActivoResumen,
    tiempo::{a_costa_rica, hora_actual_texto},
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, render_terminal_too_small,
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

const COMANDOS_NORMAL: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Detalle"),
    CommandHint::new("S", "Salida"),
    CommandHint::new("F2", "Gafete"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("C", "Columnas"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_BUSQUEDA: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Aplicar"),
    CommandHint::new("ESC", "Limpiar"),
];
const COMANDOS_DETALLE: &[CommandHint<'static>] = &[
    CommandHint::new("S", "Salida"),
    CommandHint::new("ESC", "Cerrar"),
];
const COMANDOS_CONFIRMAR: &[CommandHint<'static>] = &[
    CommandHint::new("Y", "Confirmar"),
    CommandHint::new("N/ESC", "Cancelar"),
];
const COMANDOS_GAFETE: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Buscar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_COLUMNAS: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("SPACE", "Mostrar/Ocultar"),
    CommandHint::new("ESC", "Cerrar"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &ActivosState, theme: Theme) {

    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "ESC salir", theme);
        return;
    }

    let hora = hora_actual_texto();
    let contexto = format!("Usuario: {}", state.usuario_nombre);
    let (estado_texto, estado_tipo) = estado_shell(state);
    let comandos = match &state.modo {
        ModoActivos::Normal => COMANDOS_NORMAL,
        ModoActivos::Busqueda { .. } => COMANDOS_BUSQUEDA,
        ModoActivos::Detalle { .. } => COMANDOS_DETALLE,
        ModoActivos::ConfirmarSalida { .. } => COMANDOS_CONFIRMAR,
        ModoActivos::SalidaPorGafete(SalidaGafete::Capturando { .. }) => COMANDOS_GAFETE,
        ModoActivos::SalidaPorGafete(SalidaGafete::Encontrado { .. }) => COMANDOS_CONFIRMAR,
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
                format!("¿Registrar la salida de {nombre}?"),
                StatusKind::Warning,
            )
        }
        ModoActivos::SalidaPorGafete(SalidaGafete::Capturando { error: Some(e), .. }) => {
            (e.clone(), StatusKind::Error)
        }
        ModoActivos::SalidaPorGafete(SalidaGafete::Capturando { .. }) => (
            "Ingrese el número de gafete.".to_owned(),
            StatusKind::Normal,
        ),
        ModoActivos::SalidaPorGafete(SalidaGafete::Encontrado { registro }) => (
            format!("¿Registrar la salida de {}?", registro.contratista_nombre),
            StatusKind::Warning,
        ),
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
        ModoActivos::Busqueda { texto } => texto.as_str(),
        _ => state.filtro.as_str(),
    };
    let area_busqueda = render_campo(
        frame,
        filas[0],
        &format!(
            "BUSCAR · {} DE {} DENTRO",
            state.cantidad(),
            state.total_activos()
        ),
        texto_busqueda,
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
        let ancho_visible = Line::from(texto_busqueda).width() as u16;
        let x = area_busqueda
            .x
            .saturating_add(ancho_visible.min(area_busqueda.width));
        frame.set_cursor_position((x, area_busqueda.y));
    } else if let ModoActivos::SalidaPorGafete(SalidaGafete::Capturando { numero, .. }) =
        &state.modo
        && let Some(area_campo) = area_gafete
    {
        let ancho_visible = Line::from(numero.as_str()).width() as u16;
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
            let celdas = columnas.iter().map(|columna| {
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
                Cell::from(valor_columna(registro, *columna)).style(estilo)
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
        Table::new(filas, anchos).header(encabezado).column_spacing(1),
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

/// Devuelve, cuando aplica, el área del valor del campo GAFETE para que el
/// llamador posicione el cursor.
fn render_panel(frame: &mut Frame, area: Rect, state: &ActivosState, theme: Theme) -> Option<Rect> {
    match &state.modo {
        ModoActivos::Normal | ModoActivos::Busqueda { .. } => {
            frame.render_widget(
                Paragraph::new("Seleccione ENTER para ver el detalle.").style(theme.muted()),
                area,
            );
            None
        }
        ModoActivos::Detalle { id } | ModoActivos::ConfirmarSalida { id } => {
            if let Some(registro) = state.registro(*id) {
                render_detalle(frame, area, registro, theme);
            }
            None
        }
        ModoActivos::SalidaPorGafete(SalidaGafete::Capturando { numero, .. }) => {
            Some(render_campo(frame, area, "NÚMERO DE GAFETE", numero, true, theme))
        }
        ModoActivos::SalidaPorGafete(SalidaGafete::Encontrado { registro }) => {
            render_detalle(frame, area, registro, theme);
            None
        }
        ModoActivos::Columnas { seleccion } => {
            render_columnas(frame, area, state, *seleccion, theme);
            None
        }
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, registro: &IngresoActivoResumen, theme: Theme) {
    let mut lineas = vec![
        Line::from(registro.contratista_nombre.clone()).style(theme.title()),
        Line::from(format!("{} · {}", registro.cedula, registro.empresa_nombre)).style(theme.base()),
        Line::from(""),
        Line::from(format!("Tipo               {}", texto_tipo(registro.tipo_ingreso))).style(theme.base()),
        Line::from(format!("Medio              {}", texto_medio(registro.medio_ingreso))).style(theme.base()),
        Line::from(format!(
            "Ingreso            {}",
            a_costa_rica(registro.fecha_hora_ingreso).format("%d/%m/%Y %H:%M")
        ))
        .style(theme.base()),
        Line::from(format!(
            "Gafete             {}",
            valor_columna(registro, Columna::Gafete)
        ))
        .style(theme.base()),
        Line::from(format!("Registrado por     {}", registro.usuario_ingreso_nombre)).style(theme.base()),
    ];
    if !matches!(
        registro.resultado_acceso,
        crate::domain::resultado_acceso::ResultadoAcceso::Permitido
    ) {
        lineas.push(Line::from(""));
        lineas.push(Line::from("Condición de acceso actual requiere atención").style(theme.warning()));
    }
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
