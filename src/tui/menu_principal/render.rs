use ratatui::{Frame, layout::Rect, text::Line, widgets::Paragraph};

use super::{ConfirmacionMenu, MenuPrincipalState, OpcionMenu};
use crate::{
    models::usuario::RolUsuario,
    services::autenticacion_service::UsuarioSesion,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, ThemePreset, render_terminal_too_small,
    },
};

const ANCHO_MINIMO: u16 = 60;
const ALTO_MINIMO: u16 = 22;

const COMANDOS_NORMALES: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Seleccionar"),
    CommandHint::new("ENTER", "Abrir"),
    CommandHint::new("1-6", "Acceso rápido"),
    CommandHint::new("L", "Cerrar sesión"),
    CommandHint::new("Q", "Salir"),
];

const COMANDOS_CONFIRMACION: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Confirmar"),
    CommandHint::new("N/ESC", "Cancelar"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &MenuPrincipalState, sesion: &UsuarioSesion) {
    let theme = ThemePreset::Brisas.theme();

    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "Q/ESC salir", theme);
        return;
    }

    let hora = hora_actual_texto();
    let contexto = format!("{} · {}", sesion.nombre, rol(sesion.rol));
    let (estado_texto, estado_tipo) = estado_shell(state, sesion);
    let comandos = if state.confirmacion.is_some() {
        COMANDOS_CONFIRMACION
    } else {
        COMANDOS_NORMALES
    };

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "MENÚ PRINCIPAL",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
    };
    let areas = shell.render(frame, area, theme);

    render_lista(frame, areas.body, state, theme);
}

fn estado_shell(state: &MenuPrincipalState, sesion: &UsuarioSesion) -> (String, StatusKind) {
    match state.confirmacion {
        Some(ConfirmacionMenu::CerrarSesion) => (
            format!("¿Cerrar la sesión de {}?", sesion.nombre),
            StatusKind::Warning,
        ),
        Some(ConfirmacionMenu::Salir) => {
            ("¿Desea cerrar BRISAS CLI?".to_owned(), StatusKind::Warning)
        }
        None => (
            state.seleccion.descripcion().to_owned(),
            StatusKind::Normal,
        ),
    }
}

fn render_lista(frame: &mut Frame, area: Rect, state: &MenuPrincipalState, theme: Theme) {
    let ancho = area.width.min(60);
    let alto = area.height.min(14);
    let lista = centrar(area, ancho, alto);

    let mut lineas = Vec::new();
    grupo(
        &mut lineas,
        "OPERACIÓN",
        &OpcionMenu::TODAS[0..3],
        state,
        theme,
    );
    grupo(
        &mut lineas,
        "ADMINISTRACIÓN",
        &OpcionMenu::TODAS[3..6],
        state,
        theme,
    );
    grupo(
        &mut lineas,
        "SESIÓN",
        &OpcionMenu::TODAS[6..8],
        state,
        theme,
    );

    frame.render_widget(Paragraph::new(lineas), lista);
}

fn grupo<'a>(
    lineas: &mut Vec<Line<'a>>,
    titulo: &'a str,
    opciones: &[OpcionMenu],
    state: &MenuPrincipalState,
    theme: Theme,
) {
    lineas.push(Line::from(titulo).style(theme.muted()));
    for opcion in opciones {
        let marcador = if *opcion == state.seleccion { ">" } else { " " };
        let texto = format!("{marcador} {}", opcion.etiqueta());
        lineas.push(Line::from(texto).style(if *opcion == state.seleccion {
            theme.selected()
        } else {
            theme.base()
        }));
    }
    lineas.push(Line::from(""));
}

fn centrar(area: Rect, ancho: u16, alto: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto,
    )
}

fn rol(rol: RolUsuario) -> &'static str {
    match rol {
        RolUsuario::Root => "ROOT",
        RolUsuario::Administrador => "ADMINISTRADOR",
        RolUsuario::Operador => "OPERADOR",
    }
}
