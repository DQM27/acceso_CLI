use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    widgets::{Cell, Row, Table, TableState},
};

use super::AuditoriaState;
use crate::{
    services::autenticacion_service::UsuarioSesion,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, identidad_sesion, render_terminal_too_small,
    },
};

const COMANDOS: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Seleccionar"),
    CommandHint::new("PgUp/PgDn", "Página"),
    CommandHint::new("ESC", "Volver"),
];

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &AuditoriaState,
    sesion: &UsuarioSesion,
    theme: Theme,
) {
    if area.width < 80 || area.height < 22 {
        render_terminal_too_small(frame, area, 80, 22, "ESC volver", theme);
        return;
    }
    let hora = hora_actual_texto();
    let contexto = identidad_sesion(sesion);
    let status = state.error.clone().unwrap_or_else(|| {
        format!(
            "{}–{} de {} cambios",
            state.offset.saturating_add(1).min(state.total),
            state.offset.saturating_add(state.items.len()),
            state.total
        )
    });
    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "AUDITORÍA DE CONTRATISTAS",
        context: &contexto,
        clock: &hora,
        status: &status,
        status_kind: if state.error.is_some() {
            StatusKind::Error
        } else {
            StatusKind::Normal
        },
        commands: COMANDOS,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);
    let encabezado = Row::new([
        "FECHA (UTC)",
        "USUARIO",
        "CONTRATISTA",
        "CAMPO",
        "ANTERIOR",
        "NUEVO",
    ])
    .style(theme.accent());
    let filas = state.items.iter().map(|item| {
        Row::new([
            Cell::from(item.fecha_hora.format("%Y-%m-%d %H:%M").to_string()),
            Cell::from(item.usuario_nombre.clone()),
            Cell::from(item.contratista_nombre.clone()),
            Cell::from(item.campo.clone()),
            Cell::from(item.valor_anterior.clone().unwrap_or_else(|| "—".into())),
            Cell::from(item.valor_nuevo.clone().unwrap_or_else(|| "—".into())),
        ])
    });
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(17),
            Constraint::Fill(2),
            Constraint::Fill(2),
            Constraint::Length(24),
            Constraint::Length(15),
            Constraint::Length(15),
        ],
    )
    .header(encabezado)
    .row_highlight_style(theme.selected())
    .highlight_symbol("> ");
    frame.render_stateful_widget(
        tabla,
        areas.body,
        &mut TableState::default().with_selected(state.seleccion),
    );
}
