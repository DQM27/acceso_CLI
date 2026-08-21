use ratatui::{Frame, layout::Rect, text::Line, widgets::Paragraph};

use super::{ConfirmacionMenu, MenuPrincipalState, OpcionMenu};
use crate::{
    models::usuario::RolUsuario,
    services::autenticacion_service::UsuarioSesion,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        CommandHint, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, ScreenShell, StatusKind, Theme,
        identidad_sesion, render_terminal_too_small,
    },
};

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
    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        render_terminal_too_small(
            frame,
            area,
            MIN_TERMINAL_WIDTH,
            MIN_TERMINAL_HEIGHT,
            "Q salir",
            theme,
        );
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
        authenticated: true,
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
        // Mensaje genérico a propósito, sin el detalle técnico (disco lleno,
        // permisos, etc.) — ese detalle vive en Respaldos, sólo alcanzable
        // por Root. Cualquier rol ve este aviso porque cualquiera puede ser
        // quien note el problema y avise al administrador.
        None if state.fallo_respaldo_automatico.is_some() => (
            "Fallo en el sistema de respaldo de la base de datos. Contacte al administrador."
                .to_owned(),
            StatusKind::Error,
        ),
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
                    | OpcionMenu::Respaldos
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
    let mostrar_grupos = area.height as usize >= visibles.len() + 3;
    if mostrar_grupos {
        let separar_grupos = area.height as usize >= visibles.len() + 6;
        grupo(
            &mut lineas,
            "OPERACIÓN",
            &operacion,
            state,
            theme,
            separar_grupos,
        );
        grupo(
            &mut lineas,
            "ADMINISTRACIÓN",
            &administracion,
            state,
            theme,
            separar_grupos,
        );
        grupo(&mut lineas, "SESIÓN", &sesion, state, theme, separar_grupos);
    } else {
        lineas.extend(
            visibles
                .iter()
                .map(|opcion| linea_opcion(*opcion, state, theme)),
        );
    }

    let ancho = area.width.min(ancho_contenido);
    let alto = area.height.min(lineas.len() as u16);
    let lista = centrar(area, ancho, alto);

    frame.render_widget(Paragraph::new(lineas), lista);
}

fn grupo<'a>(
    lineas: &mut Vec<Line<'a>>,
    titulo: &'a str,
    opciones: &[OpcionMenu],
    state: &MenuPrincipalState,
    theme: Theme,
    separar: bool,
) {
    lineas.push(Line::from(titulo).style(theme.muted()));
    for opcion in opciones {
        lineas.push(linea_opcion(*opcion, state, theme));
    }
    if separar {
        lineas.push(Line::from(""));
    }
}

fn linea_opcion(opcion: OpcionMenu, state: &MenuPrincipalState, theme: Theme) -> Line<'static> {
    let marcador = if opcion == state.seleccion { ">" } else { " " };
    let texto = format!("{marcador} {}", opcion.etiqueta());
    Line::from(texto).style(if opcion == state.seleccion {
        theme.selected()
    } else {
        theme.base()
    })
}

fn centrar(area: Rect, ancho: u16, alto: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto,
    )
}
