use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table},
};

use super::*;
use crate::{
    database::queries::empresas::EmpresaResumen,
    services::autenticacion_service::UsuarioSesion,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        CommandHint, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, ScreenShell, StatusKind, Theme,
        identidad_sesion, master_detail_areas, render_form_field, render_separator,
        render_terminal_too_small,
    },
};

const COMANDOS_NORMAL: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Editar"),
    CommandHint::new("N", "Nueva"),
    CommandHint::new("A", "Estado"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_NORMAL_FILTRADO: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Editar"),
    CommandHint::new("N", "Nueva"),
    CommandHint::new("A", "Estado"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("ESC", "Limpiar filtro"),
];
const COMANDOS_BUSQUEDA: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Aplicar"),
    CommandHint::new("ESC", "Limpiar"),
];
const COMANDOS_FORMULARIO: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Guardar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_CONFIRMACION: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Confirmar"),
    CommandHint::new("ESC", "Cancelar"),
];

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &EmpresasState,
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
        ModoEmpresas::Normal if !state.filtro.is_empty() => COMANDOS_NORMAL_FILTRADO,
        ModoEmpresas::Normal => COMANDOS_NORMAL,
        ModoEmpresas::Busqueda { .. } => COMANDOS_BUSQUEDA,
        ModoEmpresas::Formulario(_) => COMANDOS_FORMULARIO,
        ModoEmpresas::ConfirmacionEstado(_) => COMANDOS_CONFIRMACION,
    };

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "EMPRESAS",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        authenticated: true,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
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
    if let ModoEmpresas::ConfirmacionEstado(c) = &state.modo {
        let nombre = state
            .empresa(c.id)
            .map(|e| e.nombre.as_str())
            .unwrap_or_default();
        let accion = if c.activar { "activar" } else { "desactivar" };
        let consecuencia = if c.activar {
            "habilitará nuevos ingresos"
        } else {
            "impedirá nuevos ingresos de sus contratistas"
        };
        return (
            format!("CONFIRMAR · {accion} {nombre} · {consecuencia}"),
            StatusKind::Warning,
        );
    }
    // Mismo criterio que `activos::estado_shell`: un único campo `mensaje`,
    // el contenido decide el estilo en vez de necesitar 2 campos separados.
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

    let areas = master_detail_areas(filas[1], 63, 7);
    render_separator(frame, areas.separator, areas.orientation, theme);
    let (area_tabla, area_panel) = (areas.master, areas.detail);
    render_tabla(frame, area_tabla, state, theme);
    let area_form_nombre = render_panel(frame, area_panel, state, theme);

    let cursor = match &state.modo {
        ModoEmpresas::Busqueda { texto } => Some((area_busqueda, texto)),
        ModoEmpresas::Formulario(formulario) => {
            area_form_nombre.map(|area| (area, &formulario.nombre))
        }
        _ => None,
    };
    if let Some((area_campo, texto)) = cursor {
        let antes_del_cursor: String = texto.value().chars().take(texto.cursor()).collect();
        let ancho_visible = Line::from(antes_del_cursor).width() as u16;
        let x = area_campo
            .x
            .saturating_add(ancho_visible.min(area_campo.width));
        frame.set_cursor_position((x, area_campo.y));
    }
}

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
            let estilo = if seleccionada {
                theme.selected()
            } else if !empresa.activo {
                theme.muted()
            } else {
                theme.base()
            };
            Row::new([
                Cell::from(format!(
                    "{} {}",
                    if seleccionada { ">" } else { " " },
                    empresa.nombre
                )),
                Cell::from(empresa.contratistas.to_string()),
                Cell::from(if empresa.activo { "ACTIVA" } else { "INACTIVA" }),
            ])
            .style(estilo)
        });
    let encabezado = Row::new(["NOMBRE", "CONTRATISTAS", "ESTADO"])
        .style(theme.muted())
        .bottom_margin(1);
    frame.render_widget(
        Table::new(
            filas,
            [
                Constraint::Fill(4),
                Constraint::Length(14),
                Constraint::Length(10),
            ],
        )
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
fn render_panel(
    frame: &mut Frame,
    area: Rect,
    state: &EmpresasState,
    theme: Theme,
) -> Option<Rect> {
    match &state.modo {
        ModoEmpresas::Formulario(formulario) => {
            Some(render_formulario(frame, area, formulario, theme))
        }
        ModoEmpresas::Normal
        | ModoEmpresas::Busqueda { .. }
        | ModoEmpresas::ConfirmacionEstado(_) => {
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
    let estilo_estado = if empresa.activo {
        theme.success()
    } else {
        theme.danger()
    };
    let lineas = vec![
        Line::from(empresa.nombre.as_str()).style(theme.title()),
        Line::from(""),
        Line::from(format!("Contratistas asociados: {}", empresa.contratistas)).style(theme.base()),
        Line::from(if empresa.activo {
            "Estado: ACTIVA"
        } else {
            "Estado: INACTIVA"
        })
        .style(estilo_estado),
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
    render_campo(
        frame,
        filas[0],
        titulo,
        formulario.nombre.value(),
        true,
        theme,
    )
}
