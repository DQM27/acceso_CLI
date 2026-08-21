use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    widgets::{Cell, Row, Table, TableState},
};

use super::AuditoriaState;
use crate::{
    services::autenticacion_service::UsuarioSesion,
    tiempo::{a_costa_rica, hora_actual_texto},
    tui::ui_kit::{
        CommandHint, SIMBOLO_RESALTADO_TABLA, ScreenShell, StatusKind, Theme, identidad_sesion,
        render_terminal_too_small,
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
        authenticated: true,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);
    let encabezado = Row::new([
        "FECHA Y HORA (CR)",
        "CONTRATISTA",
        "CAMBIO REALIZADO",
        "MODIFICADO POR",
    ])
    .style(theme.accent());
    let filas = state.items.iter().map(|item| {
        Row::new([
            Cell::from(
                a_costa_rica(item.fecha_hora)
                    .format("%d/%m/%Y %H:%M")
                    .to_string(),
            ),
            Cell::from(item.contratista_nombre.clone()),
            Cell::from(descripcion_cambio(item)),
            Cell::from(item.usuario_nombre.clone()),
        ])
    });
    let tabla = Table::new(
        filas,
        [
            Constraint::Length(18),
            Constraint::Fill(2),
            Constraint::Fill(4),
            Constraint::Fill(2),
        ],
    )
    .header(encabezado)
    .row_highlight_style(theme.selected())
    .highlight_symbol(SIMBOLO_RESALTADO_TABLA);
    frame.render_stateful_widget(
        tabla,
        areas.body,
        &mut TableState::default().with_selected(state.seleccion),
    );
}

pub(super) fn descripcion_cambio(
    item: &crate::database::queries::auditoria_contratistas::CambioContratistaAuditado,
) -> String {
    let etiqueta = match item.campo.as_str() {
        "tipo_ingreso" => "Tipo de ingreso",
        "fecha_vencimiento_praind" => "Vencimiento PRAIND",
        "tiene_acceso" => "Acceso",
        _ => item.campo.as_str(),
    };
    let anterior = valor_presentable(&item.campo, item.valor_anterior.as_deref());
    let nuevo = valor_presentable(&item.campo, item.valor_nuevo.as_deref());
    format!("{etiqueta}: {anterior} → {nuevo}")
}

fn valor_presentable(campo: &str, valor: Option<&str>) -> String {
    let Some(valor) = valor else {
        return if campo == "fecha_vencimiento_praind" {
            "Sin fecha".to_owned()
        } else {
            "—".to_owned()
        };
    };
    match (campo, valor) {
        ("tipo_ingreso", "IN_HOUSE") => "IN HOUSE".to_owned(),
        ("tipo_ingreso", "POR_CORREO") => "POR CORREO".to_owned(),
        ("fecha_vencimiento_praind", valor) => chrono::NaiveDate::parse_from_str(valor, "%Y-%m-%d")
            .map(|fecha| fecha.format("%d/%m/%Y").to_string())
            .unwrap_or_else(|_| valor.to_owned()),
        ("tiene_acceso", "HABILITADO") => "Habilitado".to_owned(),
        ("tiene_acceso", "DESHABILITADO") => "Deshabilitado".to_owned(),
        _ => valor.to_owned(),
    }
}
