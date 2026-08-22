use crate::tui::menu_principal::OpcionMenu;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::Paragraph,
};

use super::{CambioPasswordState, Campo};
use crate::{
    services::autenticacion_service::UsuarioSesion,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, identidad_sesion, render_terminal_too_small,
    },
};

const COMANDOS: &[CommandHint<'static>] = &[
    CommandHint::new("TAB/↑↓", "Cambiar campo"),
    CommandHint::new("ENTER", "Guardar"),
    CommandHint::new("ESC", "Volver"),
];

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &CambioPasswordState,
    sesion: &UsuarioSesion,
    theme: Theme,
) {
    if area.width < 60 || area.height < 22 {
        render_terminal_too_small(frame, area, 60, 22, "ESC volver", theme);
        return;
    }
    let hora = hora_actual_texto();
    let contexto = identidad_sesion(sesion);
    let (status, kind) = match &state.mensaje {
        Some(Ok(mensaje)) => (format!("✓ {mensaje}"), StatusKind::Success),
        Some(Err(mensaje)) => (format!("✕ {mensaje}"), StatusKind::Error),
        None if state.guardando => ("Verificando y guardando…".into(), StatusKind::Warning),
        None => (
            "Confirme su identidad antes de cambiar la contraseña".into(),
            StatusKind::Normal,
        ),
    };
    let tabs = OpcionMenu::barra_pestanas(sesion.rol, OpcionMenu::CambiarPassword);
    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "CAMBIAR MI CONTRASEÑA",
        context: &contexto,
        clock: &hora,
        status: &status,
        status_kind: kind,
        commands: COMANDOS,
        tabs: theme.navegacion_pestanas.then_some(&tabs),
        authenticated: true,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);
    let ancho = 48.min(areas.body.width);
    let formulario = Rect::new(
        areas.body.x + areas.body.width.saturating_sub(ancho) / 2,
        areas.body.y + 1,
        ancho,
        areas.body.height.saturating_sub(1),
    );
    let filas = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
    ])
    .split(formulario);
    for (campo, etiqueta, area) in [
        (Campo::Actual, "CONTRASEÑA ACTUAL", filas[0]),
        (Campo::Nueva, "CONTRASEÑA NUEVA", filas[2]),
        (Campo::Confirmacion, "CONFIRMAR CONTRASEÑA NUEVA", filas[4]),
    ] {
        let estilo = if state.campo == campo {
            theme.accent()
        } else {
            theme.muted()
        };
        frame.render_widget(Paragraph::new(etiqueta).style(estilo), area);
        frame.render_widget(
            Paragraph::new(state.mascara(campo)).style(theme.base()),
            Rect::new(area.x, area.y + 1, area.width, 1),
        );
        frame.render_widget(
            Paragraph::new("─".repeat(area.width as usize)).style(estilo),
            Rect::new(area.x, area.y + 2, area.width, 1),
        );
    }
}
