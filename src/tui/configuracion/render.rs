use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table},
};

use super::*;
use crate::tui::ui_kit::{
    CommandHint, ScreenShell, StatusKind, TextInput, Theme, render_terminal_too_small,
};

const ANCHO_MINIMO: u16 = 60;
const ALTO_MINIMO: u16 = 20;

const COMANDOS_MENU: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Abrir"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_RESPALDOS: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("C", "Crear"),
    CommandHint::new("V", "Revalidar"),
    CommandHint::new("E", "Exportar"),
    CommandHint::new("R", "Restaurar"),
    CommandHint::new("A", "Actualizar"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_EXPORTAR: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Exportar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_CONFIRMAR_RESTAURACION: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Confirmar"),
    CommandHint::new("ESC", "Cancelar"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &ConfiguracionState, theme: Theme) {
    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "ESC salir", theme);
        return;
    }

    match &state.modo {
        ModoConfiguracion::Menu => render_menu(frame, area, state, theme),
        ModoConfiguracion::Respaldos(estado) => render_respaldos(frame, area, estado, theme),
    }
}

fn render_menu(frame: &mut Frame, area: Rect, state: &ConfiguracionState, theme: Theme) {
    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "CONFIGURACIÓN",
        context: "Ajustes del sistema",
        clock: &crate::tiempo::hora_actual_texto(),
        status: state.seleccion.descripcion(),
        status_kind: StatusKind::Normal,
        commands: COMANDOS_MENU,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);

    render_lista(frame, areas.body, state, theme);
}

/// Misma silueta que el menú principal: bloque centrado, ancho ajustado al
/// contenido real para que no quede pegado a la izquierda.
fn render_lista(frame: &mut Frame, area: Rect, state: &ConfiguracionState, theme: Theme) {
    let ancho_contenido = OpcionConfiguracion::TODAS
        .iter()
        .map(|o| o.etiqueta().chars().count() as u16 + 2)
        .max()
        .unwrap_or(20);
    let ancho = area.width.min(ancho_contenido);
    let alto = area.height.min(OpcionConfiguracion::TODAS.len() as u16);
    let lista = centrar(area, ancho, alto);

    let lineas: Vec<Line<'_>> = OpcionConfiguracion::TODAS
        .iter()
        .map(|opcion| {
            let marcador = if *opcion == state.seleccion { ">" } else { " " };
            let estilo = if *opcion == state.seleccion {
                theme.selected()
            } else {
                theme.base()
            };
            Line::from(format!("{marcador} {}", opcion.etiqueta())).style(estilo)
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), lista);
}

fn centrar(area: Rect, ancho: u16, alto: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto,
    )
}

fn render_respaldos(frame: &mut Frame, area: Rect, estado: &RespaldosState, theme: Theme) {
    let (estado_texto, estado_tipo) = estado_shell(estado);
    let comandos = match &estado.modo {
        ModoRespaldos::Normal => COMANDOS_RESPALDOS,
        ModoRespaldos::Exportando { .. } => COMANDOS_EXPORTAR,
        ModoRespaldos::ConfirmandoRestauracion { .. } => COMANDOS_CONFIRMAR_RESTAURACION,
    };
    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "CONFIGURACIÓN · RESPALDOS",
        context: &format!("{} respaldo(s)", estado.filas.len()),
        clock: &crate::tiempo::hora_actual_texto(),
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        help_expanded: false,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);

    match &estado.modo {
        ModoRespaldos::Exportando { destino } => {
            render_exportar(frame, areas.body, destino, theme);
            return;
        }
        ModoRespaldos::ConfirmandoRestauracion { fecha, tipo, .. } => {
            render_confirmar_restauracion(frame, areas.body, fecha, tipo, theme);
            return;
        }
        ModoRespaldos::Normal => {}
    }

    let filas_area =
        Layout::vertical([Constraint::Min(4), Constraint::Length(1), Constraint::Length(6)])
            .split(areas.body);
    render_tabla(frame, filas_area[0], estado, theme);
    render_detalle(frame, filas_area[2], estado, theme);
}

fn estado_shell(estado: &RespaldosState) -> (String, StatusKind) {
    if let Some(mensaje) = &estado.mensaje {
        let tipo = if mensaje.starts_with('✓') {
            StatusKind::Success
        } else if mensaje.starts_with('✕') {
            StatusKind::Error
        } else {
            StatusKind::Normal
        };
        return (mensaje.clone(), tipo);
    }
    (String::new(), StatusKind::Normal)
}

fn render_tabla(frame: &mut Frame, area: Rect, estado: &RespaldosState, theme: Theme) {
    let filas = estado.filas.iter().enumerate().map(|(indice, fila)| {
        let seleccionada = estado.seleccion == Some(indice);
        let estilo = if seleccionada {
            theme.selected()
        } else {
            theme.base()
        };
        let fecha = crate::tiempo::a_costa_rica(fila.resumen.creado_en)
            .format("%Y-%m-%d %H:%M")
            .to_string();
        Row::new([
            Cell::from(fecha),
            Cell::from(etiqueta_tipo(fila.resumen.tipo)),
            Cell::from(tamano_legible(fila.resumen.tamano_bytes)),
            Cell::from(etiqueta_esquema(fila.validacion.as_ref())),
            Cell::from(etiqueta_validacion(fila.validacion.as_ref())),
        ])
        .style(estilo)
    });
    frame.render_widget(
        Table::new(
            filas,
            [
                Constraint::Length(17),
                Constraint::Length(17),
                Constraint::Length(10),
                Constraint::Length(9),
                Constraint::Length(16),
            ],
        )
        .header(
            Row::new(["FECHA", "TIPO", "TAMAÑO", "ESQUEMA", "ESTADO"])
                .style(theme.muted())
                .bottom_margin(1),
        )
        .column_spacing(1),
        area,
    );
    if estado.filas.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay respaldos todavía. Presione C para crear uno.")
                .style(theme.warning())
                .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, estado: &RespaldosState, theme: Theme) {
    let Some(fila) = estado.seleccionada() else {
        frame.render_widget(
            Paragraph::new("No hay un respaldo seleccionado.").style(theme.muted()),
            area,
        );
        return;
    };
    let lineas = vec![
        Line::from(nombre_archivo(&fila.resumen.ruta)).style(theme.title()),
        Line::from(fila.resumen.ruta.display().to_string()).style(theme.muted()),
        Line::from(vec![
            Span::styled("Estado: ", theme.muted()),
            Span::styled(etiqueta_validacion(fila.validacion.as_ref()), theme.base()),
        ]),
    ];
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_exportar(frame: &mut Frame, area: Rect, destino: &TextInput, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(1), Constraint::Length(3)]).split(area);
    frame.render_widget(
        Paragraph::new("Ruta destino para la copia exportada:").style(theme.base()),
        filas[0],
    );
    frame.render_widget(
        Paragraph::new(destino.value())
            .style(theme.accent())
            .block(crate::tui::ui_kit::panel("DESTINO", theme, true)),
        filas[1],
    );
    frame.set_cursor_position((
        filas[1].x
            + 1
            + destino
                .cursor()
                .min(filas[1].width.saturating_sub(3) as usize) as u16,
        filas[1].y + 1,
    ));
}

fn render_confirmar_restauracion(
    frame: &mut Frame,
    area: Rect,
    fecha: &str,
    tipo: &str,
    theme: Theme,
) {
    let lineas = vec![
        Line::from(format!("¿Restaurar el respaldo del {fecha} ({tipo})?")).style(theme.warning()),
        Line::from(""),
        Line::from("Esto reemplaza TODOS los datos activos por los de ese respaldo.")
            .style(theme.base()),
        Line::from("Antes de continuar se crea un respaldo de la base actual.")
            .style(theme.base()),
        Line::from("La aplicación se reiniciará y pedirá iniciar sesión de nuevo.")
            .style(theme.muted()),
    ];
    frame.render_widget(Paragraph::new(lineas), area);
}
