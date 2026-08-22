use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    widgets::Paragraph,
};

use super::*;
use crate::tiempo::hora_actual_texto;
use crate::tui::ui_kit::{
    CommandHint, MIN_TERMINAL_WIDTH, ScreenShell, StatusKind, Theme, posicionar_cursor,
    render_form_field, render_terminal_too_small,
};

const ALTO_MINIMO: u16 = 26;

const COMANDOS: &[CommandHint<'static>] = &[
    CommandHint::new("TAB/↑↓", "Campo"),
    CommandHint::new("G/ENTER", "Crear ROOT"),
    CommandHint::new("ESC", "Salir"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &ConfiguracionInicialState, theme: Theme) {
    if area.width < MIN_TERMINAL_WIDTH || area.height < ALTO_MINIMO {
        render_terminal_too_small(
            frame,
            area,
            MIN_TERMINAL_WIDTH,
            ALTO_MINIMO,
            "ESC salir",
            theme,
        );
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
        tabs: None,
        authenticated: false,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);

    render_formulario(frame, areas.body, state, theme);
}

fn estado_shell(state: &ConfiguracionInicialState) -> (String, StatusKind) {
    match &state.estado {
        EstadoConfiguracion::Editando => (String::new(), StatusKind::Normal),
        EstadoConfiguracion::Creando => ("⠋ Creando usuario ROOT…".to_owned(), StatusKind::Warning),
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

    let area_cedula = render_form_field(
        frame,
        filas[0],
        "CÉDULA",
        state.cedula.value(),
        state.campo_activo == CampoConfiguracion::Cedula,
        theme,
    );
    let area_nombre = render_form_field(
        frame,
        filas[2],
        "NOMBRE",
        state.nombre.value(),
        state.campo_activo == CampoConfiguracion::Nombre,
        theme,
    );
    let password_mascara = state.password_enmascarado();
    let area_password = render_form_field(
        frame,
        filas[4],
        "CONTRASEÑA",
        &password_mascara,
        state.campo_activo == CampoConfiguracion::Password,
        theme,
    );
    let confirmacion_mascara = state.confirmacion_enmascarada();
    let area_confirmar = render_form_field(
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
        let (area_campo, antes_del_cursor) = match state.campo_activo {
            CampoConfiguracion::Cedula => (
                area_cedula,
                state
                    .cedula
                    .value()
                    .chars()
                    .take(state.cedula.cursor())
                    .collect::<String>(),
            ),
            CampoConfiguracion::Nombre => (
                area_nombre,
                state
                    .nombre
                    .value()
                    .chars()
                    .take(state.nombre.cursor())
                    .collect::<String>(),
            ),
            CampoConfiguracion::Password => (area_password, "•".repeat(state.password.cursor())),
            CampoConfiguracion::ConfirmarPassword => (
                area_confirmar,
                "•".repeat(state.confirmar_password.cursor()),
            ),
        };
        posicionar_cursor(frame, area_campo, &antes_del_cursor);
    }
}

fn centrar(area: Rect, ancho: u16, alto: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto,
    )
}
