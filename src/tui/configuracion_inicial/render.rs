use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};

use super::*;
use crate::tiempo::hora_actual_texto;
use crate::tui::ui_kit::{
    CommandHint, ScreenShell, StatusKind, Theme, ThemePreset, render_terminal_too_small,
};

const ANCHO_MINIMO: u16 = 60;
const ALTO_MINIMO: u16 = 26;

const COMANDOS: &[CommandHint<'static>] = &[
    CommandHint::new("TAB/↑↓", "Campo"),
    CommandHint::new("G/ENTER", "Crear ROOT"),
    CommandHint::new("ESC", "Salir"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &ConfiguracionInicialState) {
    let theme = ThemePreset::Brisas.theme();

    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "ESC salir", theme);
        return;
    }

    let hora = hora_actual_texto();
    let (estado_texto, estado_tipo) = estado_shell(state);

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "CONFIGURACIÓN INICIAL",
        context: "PRIMER USUARIO DEL SISTEMA",
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: COMANDOS,
    };
    let areas = shell.render(frame, area, theme);

    render_formulario(frame, areas.body, state, theme);
}

fn estado_shell(state: &ConfiguracionInicialState) -> (String, StatusKind) {
    match &state.estado {
        EstadoConfiguracion::Editando => (String::new(), StatusKind::Normal),
        EstadoConfiguracion::Creando => (
            "⠋ Creando usuario ROOT…".to_owned(),
            StatusKind::Warning,
        ),
        EstadoConfiguracion::Error(error) => (format!("✕ {error}"), StatusKind::Error),
    }
}

fn render_formulario(
    frame: &mut Frame,
    area: Rect,
    state: &ConfiguracionInicialState,
    theme: Theme,
) {
    let ancho = 50.min(area.width);
    let alto = 17.min(area.height);
    let hero = centrar(area, ancho, alto);
    let filas = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(hero);

    let area_cedula = render_campo(
        frame,
        filas[0],
        "CÉDULA",
        &state.cedula,
        state.campo_activo == CampoConfiguracion::Cedula,
        theme,
    );
    let area_nombre = render_campo(
        frame,
        filas[2],
        "NOMBRE",
        &state.nombre,
        state.campo_activo == CampoConfiguracion::Nombre,
        theme,
    );
    let password_mascara = state.password_enmascarado();
    let area_password = render_campo(
        frame,
        filas[4],
        "CONTRASEÑA",
        &password_mascara,
        state.campo_activo == CampoConfiguracion::Password,
        theme,
    );
    let confirmacion_mascara = state.confirmacion_enmascarada();
    let area_confirmar = render_campo(
        frame,
        filas[6],
        "CONFIRMAR CONTRASEÑA",
        &confirmacion_mascara,
        state.campo_activo == CampoConfiguracion::ConfirmarPassword,
        theme,
    );

    frame.render_widget(
        Paragraph::new("Este usuario será creado activo y con rol ROOT.")
            .style(theme.warning())
            .alignment(Alignment::Center),
        filas[8],
    );

    if state.estado != EstadoConfiguracion::Creando && state.cursor_visible {
        let (area_campo, contenido) = match state.campo_activo {
            CampoConfiguracion::Cedula => (area_cedula, state.cedula.as_str()),
            CampoConfiguracion::Nombre => (area_nombre, state.nombre.as_str()),
            CampoConfiguracion::Password => (area_password, password_mascara.as_str()),
            CampoConfiguracion::ConfirmarPassword => {
                (area_confirmar, confirmacion_mascara.as_str())
            }
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
