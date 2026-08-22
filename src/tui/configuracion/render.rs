use crate::tui::menu_principal::OpcionMenu;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table, Wrap},
};

use super::*;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::tui::ui_kit::{
    CommandHint, MIN_TERMINAL_WIDTH, ScreenShell, StatusKind, TextInput, Theme, empty_state,
    identidad_sesion, marcador_seleccion, master_detail_areas, panel_vacio, render_separator,
    render_terminal_too_small,
};

const ALTO_MINIMO: u16 = 20;

const COMANDOS_RESPALDOS: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("C", "Crear"),
    CommandHint::new("V", "Revalidar"),
    CommandHint::new("E", "Exportar"),
    CommandHint::new("R", "Restaurar"),
    CommandHint::new("L", "Actualizar"),
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

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &ConfiguracionState,
    sesion: &UsuarioSesion,
    theme: Theme,
) {
    if area.width < MIN_TERMINAL_WIDTH || area.height < ALTO_MINIMO {
        render_terminal_too_small(
            frame,
            area,
            MIN_TERMINAL_WIDTH,
            ALTO_MINIMO,
            "ESC volver",
            theme,
        );
        return;
    }

    render_respaldos(
        frame,
        area,
        &state.respaldos,
        state.ayuda_expandida,
        sesion,
        theme,
    );
}

fn render_respaldos(
    frame: &mut Frame,
    area: Rect,
    estado: &RespaldosState,
    ayuda_expandida: bool,
    sesion: &UsuarioSesion,
    theme: Theme,
) {
    let contexto = identidad_sesion(sesion);
    let (estado_texto, estado_tipo) = estado_shell(estado);
    let comandos = match &estado.modo {
        ModoRespaldos::Normal => COMANDOS_RESPALDOS,
        ModoRespaldos::Exportando { .. } => COMANDOS_EXPORTAR,
        ModoRespaldos::ConfirmandoRestauracion { .. } => COMANDOS_CONFIRMAR_RESTAURACION,
    };
    let tabs = OpcionMenu::barra_pestanas(sesion.rol, OpcionMenu::Respaldos);
    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "RESPALDOS",
        context: &contexto,
        clock: &crate::tiempo::hora_actual_texto(),
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        tabs: theme.navegacion_pestanas.then_some(&tabs),
        authenticated: true,
        help_expanded: ayuda_expandida,
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

    let areas_detalle = master_detail_areas(areas.body, 63, 8);
    render_separator(
        frame,
        areas_detalle.separator,
        areas_detalle.orientation,
        theme,
    );
    render_tabla(frame, areas_detalle.master, estado, theme);
    render_detalle(frame, areas_detalle.detail, estado, theme);
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
    // Persiste hasta que un intento nuevo lo resuelva — a diferencia de
    // `mensaje`, no se limpia solo por navegar dentro de la pantalla.
    if let Some(fallo) = &estado.fallo_automatico {
        return (
            format!("⚠ El respaldo automático de hoy falló: {fallo}"),
            StatusKind::Error,
        );
    }
    (
        format!("{} respaldo(s)", estado.filas.len()),
        StatusKind::Normal,
    )
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
            .format("%d/%m/%Y  %H:%M")
            .to_string();
        Row::new([
            Cell::from(format!("{} {fecha}", marcador_seleccion(seleccionada))),
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
                Constraint::Length(19),
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
        empty_state(
            frame,
            area,
            "No hay respaldos todavía. Presione C para crear uno.",
            theme,
        );
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, estado: &RespaldosState, theme: Theme) {
    let Some(fila) = estado.seleccionada() else {
        panel_vacio(frame, area, "No hay un respaldo seleccionado.", theme);
        return;
    };
    let lineas = vec![
        Line::from(nombre_archivo(&fila.resumen.ruta)).style(theme.title()),
        Line::from(fila.resumen.ruta.display().to_string()).style(theme.muted()),
        crate::tui::ui_kit::detail_line(
            "Estado",
            etiqueta_validacion(fila.validacion.as_ref()),
            theme,
        ),
    ];
    frame.render_widget(Paragraph::new(lineas).wrap(Wrap { trim: false }), area);
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
        Line::from("Antes de continuar se crea un respaldo de la base actual.").style(theme.base()),
        Line::from("La aplicación se reiniciará y pedirá iniciar sesión de nuevo.")
            .style(theme.muted()),
    ];
    frame.render_widget(Paragraph::new(lineas), area);
}
