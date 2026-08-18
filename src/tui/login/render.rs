use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};

use super::*;
use crate::tiempo::hora_actual_texto;
use crate::tui::ui_kit::{
    CommandHint, ScreenShell, StatusKind, Theme, render_terminal_too_small,
};

const ANCHO_MINIMO: u16 = 60;
const ALTO_MINIMO: u16 = 22;

const COMANDOS: &[CommandHint<'static>] = &[
    CommandHint::new("TAB/↑↓", "Cambiar campo"),
    CommandHint::new("ENTER", "Continuar"),
    CommandHint::new("ESC/Ctrl+C", "Salir"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &LoginState, theme: Theme) {

    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(
            frame,
            area,
            ANCHO_MINIMO,
            ALTO_MINIMO,
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
        &state.cedula,
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
        let (area_campo, contenido) = match state.campo_activo {
            CampoLogin::Cedula => (area_cedula, state.cedula.as_str()),
            CampoLogin::Password => (area_password, password_enmascarado.as_str()),
        };
        let ancho_visible = Line::from(contenido).width() as u16;
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

fn centrar(area: Rect, ancho: u16, alto: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto,
    )
}
