use ratatui::{Frame, layout::Rect, text::Line, widgets::Paragraph};

use super::{ConfirmacionMenu, MenuPrincipalState, OpcionMenu};
use crate::{
    models::usuario::RolUsuario,
    services::autenticacion_service::UsuarioSesion,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, identidad_sesion, render_terminal_too_small,
    },
};

const ANCHO_MINIMO: u16 = 60;
const ALTO_MINIMO: u16 = 22;

const COMANDOS_NORMALES: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Seleccionar"),
    CommandHint::new("ENTER", "Abrir"),
    CommandHint::new("1-9", "Acceso rápido"),
    CommandHint::new("L", "Cerrar sesión"),
    CommandHint::new("Q", "Salir"),
];

const COMANDOS_CONFIRMACION: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Confirmar"),
    CommandHint::new("ESC", "Cancelar"),
];

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &MenuPrincipalState,
    sesion: &UsuarioSesion,
    theme: Theme,
) {
    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "Q/ESC salir", theme);
        return;
    }

    let hora = hora_actual_texto();
    let contexto = identidad_sesion(sesion);
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
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);

    render_lista(frame, areas.body, state, sesion.rol, theme);
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
        None => (state.seleccion.descripcion().to_owned(), StatusKind::Normal),
    }
}

fn render_lista(
    frame: &mut Frame,
    area: Rect,
    state: &MenuPrincipalState,
    rol: RolUsuario,
    theme: Theme,
) {
    let visibles = OpcionMenu::visibles_para(rol);

    // El ancho del bloque se ajusta al contenido real (marcador + etiqueta
    // más larga) en vez de un valor fijo de 60 columnas: con un bloque más
    // ancho que su contenido, el texto queda alineado a la izquierda dentro
    // de él y el conjunto se ve pegado a la izquierda aunque el bloque en sí
    // esté centrado.
    let ancho_contenido = OpcionMenu::TODAS
        .iter()
        .map(|o| o.etiqueta().chars().count() as u16 + 2)
        .max()
        .unwrap_or(20);
    let ancho = area.width.min(ancho_contenido);
    let alto = area.height.min(14);
    let lista = centrar(area, ancho, alto);

    let operacion: Vec<OpcionMenu> = visibles
        .iter()
        .copied()
        .filter(|o| {
            matches!(
                o,
                OpcionMenu::NuevoIngreso | OpcionMenu::IngresosActivos | OpcionMenu::Historial
            )
        })
        .collect();
    let administracion: Vec<OpcionMenu> = visibles
        .iter()
        .copied()
        .filter(|o| {
            matches!(
                o,
                OpcionMenu::Contratistas
                    | OpcionMenu::Empresas
                    | OpcionMenu::Usuarios
                    | OpcionMenu::Auditoria
                    | OpcionMenu::Configuracion
            )
        })
        .collect();
    let sesion: Vec<OpcionMenu> = visibles
        .iter()
        .copied()
        .filter(|o| {
            matches!(
                o,
                OpcionMenu::CambiarPassword | OpcionMenu::CerrarSesion | OpcionMenu::Salir
            )
        })
        .collect();

    let mut lineas = Vec::new();
    grupo(&mut lineas, "OPERACIÓN", &operacion, state, theme);
    grupo(&mut lineas, "ADMINISTRACIÓN", &administracion, state, theme);
    grupo(&mut lineas, "SESIÓN", &sesion, state, theme);

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
