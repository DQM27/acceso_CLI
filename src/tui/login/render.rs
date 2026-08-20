use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
};

use super::*;
use crate::tiempo::hora_actual_texto;
use crate::tui::ui_kit::{
    CommandHint, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, ScreenShell, StatusKind, Theme,
    render_form_field, render_terminal_too_small,
};

const COMANDOS: &[CommandHint<'static>] = &[
    CommandHint::new("TAB/↑↓", "Cambiar campo"),
    CommandHint::new("ENTER", "Continuar"),
    CommandHint::new("ESC/Ctrl+C", "Salir"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &LoginState, theme: Theme) {
    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        render_terminal_too_small(
            frame,
            area,
            MIN_TERMINAL_WIDTH,
            MIN_TERMINAL_HEIGHT,
            "ESC/Ctrl+C salir",
            theme,
        );
        return;
    }

    let hora = hora_actual_texto();
    let (estado_texto, estado_tipo) = estado_shell(state);

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "INICIO DE SESIÓN",
        context: "CONTROL DE ACCESO",
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: COMANDOS,
        authenticated: false,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);

    render_formulario(frame, areas.body, state, theme);
}

fn estado_shell(state: &LoginState) -> (String, StatusKind) {
    match &state.estado {
        EstadoLogin::Normal => (String::new(), StatusKind::Normal),
        EstadoLogin::Validando { .. } => (
            format!("{} Verificando credenciales…", state.spinner()),
            StatusKind::Warning,
        ),
        EstadoLogin::Error(mensaje) => (format!("✕ {mensaje}"), StatusKind::Error),
        EstadoLogin::Exito => ("✓ Acceso autorizado".to_owned(), StatusKind::Success),
    }
}

fn render_formulario(frame: &mut Frame, area: Rect, state: &LoginState, theme: Theme) {
    let ancho = 46.min(area.width);
    let alto = 7.min(area.height);
    let hero = centrar(area, ancho, alto);
    let filas = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
    ])
    .split(hero);

    let area_cedula = render_campo(
        frame,
        filas[0],
        "CÉDULA",
        state.cedula.value(),
        state.campo_activo == CampoLogin::Cedula,
        theme,
    );
    let password_enmascarado = state.password_enmascarado();
    let area_password = render_campo(
        frame,
        filas[2],
        "CONTRASEÑA",
        &password_enmascarado,
        state.campo_activo == CampoLogin::Password,
        theme,
    );

    let cursor_visible =
        !matches!(state.estado, EstadoLogin::Validando { .. }) && state.cursor_visible;
    if cursor_visible {
        let (area_campo, antes_del_cursor) = match state.campo_activo {
            CampoLogin::Cedula => (
                area_cedula,
                state
                    .cedula
                    .value()
                    .chars()
                    .take(state.cedula.cursor())
                    .collect::<String>(),
            ),
            CampoLogin::Password => (area_password, "•".repeat(state.password.cursor())),
        };
        let ancho_visible = Line::from(antes_del_cursor).width() as u16;
        let x = area_campo
            .x
            .saturating_add(ancho_visible.min(area_campo.width));
        let y = area_campo.y;
        frame.set_cursor_position((x, y));
    }
}

/// Misma silueta con foco o sin él (etiqueta, valor, línea); sólo cambian
/// color y peso, para que cambiar de campo nunca reordene ni redimensione
/// nada en pantalla.
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

fn centrar(area: Rect, ancho: u16, alto: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto,
    )
}
