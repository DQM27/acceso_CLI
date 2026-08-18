use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table},
};

use super::*;
use crate::{
    database::queries::usuarios::UsuarioResumen,
    tiempo::hora_actual_texto,
    tui::ui_kit::{
        CommandHint, ScreenShell, StatusKind, Theme, render_terminal_too_small,
    },
};

const ANCHO_MINIMO: u16 = 60;
const ALTO_MINIMO: u16 = 22;
const ANCHO_PANEL_LATERAL: u16 = 100;

const COMANDOS_NORMAL: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Editar"),
    CommandHint::new("N", "Nuevo"),
    CommandHint::new("P", "Clave"),
    CommandHint::new("A", "Estado"),
    CommandHint::new("/", "Buscar"),
    CommandHint::new("ESC", "Volver"),
];
const COMANDOS_BUSQUEDA: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Aplicar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_FORMULARIO: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓/TAB", "Navegar"),
    CommandHint::new("SPACE", "Cambiar"),
    CommandHint::new("ENTER", "Guardar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_SELECTOR_ROL: &[CommandHint<'static>] = &[
    CommandHint::new("↑↓", "Mover"),
    CommandHint::new("ENTER", "Aceptar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_PASSWORD: &[CommandHint<'static>] = &[
    CommandHint::new("TAB", "Campo"),
    CommandHint::new("ENTER", "Guardar"),
    CommandHint::new("ESC", "Cancelar"),
];
const COMANDOS_CONFIRMACION: &[CommandHint<'static>] = &[
    CommandHint::new("ENTER", "Confirmar"),
    CommandHint::new("ESC", "Cancelar"),
];

pub fn render(frame: &mut Frame, area: Rect, state: &UsuariosState, theme: Theme) {

    if area.width < ANCHO_MINIMO || area.height < ALTO_MINIMO {
        render_terminal_too_small(frame, area, ANCHO_MINIMO, ALTO_MINIMO, "ESC salir", theme);
        return;
    }

    let hora = hora_actual_texto();
    let contexto = format!("Usuario: {}", state.usuario_nombre);
    let (estado_texto, estado_tipo) = estado_shell(state);
    let comandos = match &state.modo {
        ModoUsuarios::Normal => COMANDOS_NORMAL,
        ModoUsuarios::Busqueda { .. } => COMANDOS_BUSQUEDA,
        ModoUsuarios::Formulario(f) if f.selector_rol.is_some() => COMANDOS_SELECTOR_ROL,
        ModoUsuarios::Formulario(_) => COMANDOS_FORMULARIO,
        ModoUsuarios::CambioPassword(_) => COMANDOS_PASSWORD,
        ModoUsuarios::ConfirmacionEstado(_) => COMANDOS_CONFIRMACION,
    };

    let shell = ScreenShell {
        product: "BRISAS CLI",
        screen: "USUARIOS",
        context: &contexto,
        clock: &hora,
        status: &estado_texto,
        status_kind: estado_tipo,
        commands: comandos,
        help_expanded: state.ayuda_expandida,
        ayuda_extra: None,
    };
    let areas = shell.render(frame, area, theme);

    render_cuerpo(frame, areas.body, state, theme);
}

fn estado_shell(state: &UsuariosState) -> (String, StatusKind) {
    if state.guardando {
        return ("⠋ Guardando…".to_owned(), StatusKind::Warning);
    }
    if let ModoUsuarios::ConfirmacionEstado(c) = &state.modo
        && let Some(u) = state.usuario(c.id)
    {
        let accion = if c.activar { "activar" } else { "desactivar" };
        return (
            format!("¿Confirma {accion} a {}?", u.nombre),
            StatusKind::Warning,
        );
    }
    if let ModoUsuarios::Formulario(f) = &state.modo
        && let Some(error) = &f.error
    {
        return (format!("✕ {error}"), StatusKind::Error);
    }
    if let ModoUsuarios::CambioPassword(f) = &state.modo
        && let Some(error) = &f.error
    {
        return (format!("✕ {error}"), StatusKind::Error);
    }
    if let Some(mensaje) = &state.mensaje {
        let tipo = if mensaje.starts_with('✓') {
            StatusKind::Success
        } else {
            StatusKind::Error
        };
        return (mensaje.clone(), tipo);
    }
    (String::new(), StatusKind::Normal)
}

fn render_cuerpo(frame: &mut Frame, area: Rect, state: &UsuariosState, theme: Theme) {
    let filas = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);

    let enfocado_busqueda = matches!(state.modo, ModoUsuarios::Busqueda { .. });
    let area_busqueda = render_campo(
        frame,
        filas[0],
        &format!("BUSCAR · {} RESULTADOS", state.usuarios.len()),
        &state.filtro,
        enfocado_busqueda,
        theme,
    );
    if enfocado_busqueda {
        posicionar_cursor(frame, area_busqueda, &state.filtro);
    }

    let (area_tabla, area_panel) = if area.width >= ANCHO_PANEL_LATERAL {
        let columnas = Layout::horizontal([
            Constraint::Percentage(60),
            Constraint::Length(1),
            Constraint::Percentage(39),
        ])
        .split(filas[1]);
        render_separador_vertical(frame, columnas[1], theme);
        (columnas[0], columnas[2])
    } else {
        let alto_panel = altura_panel(state);
        let filas_apiladas = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(alto_panel.min(filas[1].height.saturating_sub(5))),
        ])
        .split(filas[1]);
        render_separador_horizontal(frame, filas_apiladas[1], theme);
        (filas_apiladas[0], filas_apiladas[2])
    };
    render_tabla(frame, area_tabla, state, theme);
    render_panel(frame, area_panel, state, theme);
}

/// Posiciona el cursor en `area` (la línea de valor de un campo, devuelta
/// por [`render_campo`]) según el ancho visible de `contenido`.
fn posicionar_cursor(frame: &mut Frame, area: Rect, contenido: &str) {
    let ancho_visible = Line::from(contenido).width() as u16;
    let x = area.x.saturating_add(ancho_visible.min(area.width));
    frame.set_cursor_position((x, area.y));
}

fn altura_panel(state: &UsuariosState) -> u16 {
    match &state.modo {
        ModoUsuarios::Formulario(f) => {
            let mut total = 0u16;
            for campo in f.campos() {
                total += match campo {
                    CampoUsuario::Cedula
                    | CampoUsuario::Nombre
                    | CampoUsuario::Password
                    | CampoUsuario::ConfirmarPassword => 3,
                    CampoUsuario::Rol | CampoUsuario::Activo => 1,
                };
            }
            if f.selector_rol.is_some() {
                total += ROLES.len() as u16;
            }
            total + 1
        }
        ModoUsuarios::CambioPassword(_) => 8,
        _ => 6,
    }
}

/// Misma silueta con foco o sin él (etiqueta, valor, línea); sólo cambian
/// color y peso.
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

fn render_opcion(frame: &mut Frame, area: Rect, etiqueta: &str, valor: &str, activo: bool, theme: Theme) {
    let marcador = if activo { ">" } else { " " };
    let estilo = if activo { theme.accent() } else { theme.base() };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{marcador} {etiqueta:<22}"), estilo),
            Span::styled(valor.to_owned(), estilo),
        ])),
        area,
    );
}

fn render_separador_vertical(frame: &mut Frame, area: Rect, theme: Theme) {
    let lineas: Vec<Line<'static>> = (0..area.height).map(|_| Line::from("│")).collect();
    frame.render_widget(Paragraph::new(lineas).style(theme.border()), area);
}

fn render_separador_horizontal(frame: &mut Frame, area: Rect, theme: Theme) {
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(theme.border()),
        area,
    );
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &UsuariosState, theme: Theme) {
    let capacidad = area.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.usuarios.len());
    let filas = state
        .usuarios
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, usuario)| {
            let seleccionado = state.seleccion == Some(inicio + visible);
            let estilo_fila = if seleccionado { theme.selected() } else { theme.base() };
            Row::new([
                Cell::from(usuario.cedula.clone()),
                Cell::from(usuario.nombre.clone()),
                Cell::from(texto_rol(usuario.rol)).style(if seleccionado {
                    estilo_fila
                } else if usuario.rol == RolUsuario::Root {
                    theme.warning()
                } else {
                    theme.base()
                }),
                Cell::from(if usuario.activo { "ACTIVO" } else { "INACTIVO" }).style(
                    if seleccionado {
                        estilo_fila
                    } else if usuario.activo {
                        theme.success()
                    } else {
                        theme.danger()
                    },
                ),
            ])
            .style(estilo_fila)
        });
    frame.render_widget(
        Table::new(
            filas,
            [
                Constraint::Length(14),
                Constraint::Fill(3),
                Constraint::Length(16),
                Constraint::Length(10),
            ],
        )
        .header(
            Row::new(["CÉDULA", "NOMBRE", "ROL", "ESTADO"])
                .style(theme.muted())
                .bottom_margin(1),
        )
        .column_spacing(1),
        area,
    );
    if state.usuarios.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay usuarios que coincidan con la búsqueda.")
                .style(theme.warning())
                .alignment(Alignment::Center),
            Rect::new(area.x, area.y + area.height / 2, area.width, 1),
        );
    }
}

/// Dibuja el panel lateral según el modo y, cuando corresponde, posiciona
/// el cursor sobre el campo de texto enfocado.
fn render_panel(frame: &mut Frame, area: Rect, state: &UsuariosState, theme: Theme) {
    match &state.modo {
        ModoUsuarios::ConfirmacionEstado(c) => {
            if let Some(u) = state.usuario(c.id) {
                render_detalle(frame, area, u, theme);
            }
        }
        ModoUsuarios::Formulario(f) => render_formulario(frame, area, f, theme),
        ModoUsuarios::CambioPassword(f) => render_password(frame, area, f, theme),
        ModoUsuarios::Normal | ModoUsuarios::Busqueda { .. } => match state.seleccionado() {
            Some(u) => render_detalle(frame, area, u, theme),
            None => frame.render_widget(
                Paragraph::new("No hay un registro seleccionado.").style(theme.muted()),
                area,
            ),
        },
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, u: &UsuarioResumen, theme: Theme) {
    let estilo_rol = if u.rol == RolUsuario::Root { theme.warning() } else { theme.base() };
    let estilo_estado = if u.activo { theme.success() } else { theme.danger() };
    let lineas = vec![
        Line::from(u.nombre.as_str()).style(theme.title()),
        Line::from(u.cedula.as_str()).style(theme.muted()),
        Line::from(""),
        Line::from(texto_rol(u.rol)).style(estilo_rol),
        Line::from(if u.activo { "ACTIVO" } else { "INACTIVO" }).style(estilo_estado),
    ];
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_formulario(frame: &mut Frame, area: Rect, f: &FormularioUsuario, theme: Theme) {
    let campos = f.campos();
    let mut restricciones: Vec<Constraint> = campos
        .iter()
        .map(|campo| match campo {
            CampoUsuario::Cedula
            | CampoUsuario::Nombre
            | CampoUsuario::Password
            | CampoUsuario::ConfirmarPassword => Constraint::Length(3),
            CampoUsuario::Rol | CampoUsuario::Activo => Constraint::Length(1),
        })
        .collect();
    let indice_rol = campos.iter().position(|c| *c == CampoUsuario::Rol);
    if let (Some(indice_rol), Some(_)) = (indice_rol, f.selector_rol) {
        restricciones.insert(indice_rol + 1, Constraint::Length(ROLES.len() as u16));
    }
    restricciones.push(Constraint::Length(1));
    let filas = Layout::vertical(restricciones).split(area);

    let mut fila = 0usize;
    for (indice, campo) in campos.iter().enumerate() {
        let enfocado = f.campo == indice;
        match campo {
            CampoUsuario::Cedula => {
                let r = render_campo(frame, filas[fila], "CÉDULA", &f.cedula, enfocado, theme);
                if enfocado {
                    posicionar_cursor(frame, r, &f.cedula);
                }
            }
            CampoUsuario::Nombre => {
                let r = render_campo(frame, filas[fila], "NOMBRE", &f.nombre, enfocado, theme);
                if enfocado {
                    posicionar_cursor(frame, r, &f.nombre);
                }
            }
            CampoUsuario::Password => {
                let mascara = f.password.mascara();
                let r = render_campo(frame, filas[fila], "CONTRASEÑA", &mascara, enfocado, theme);
                if enfocado {
                    posicionar_cursor(frame, r, &mascara);
                }
            }
            CampoUsuario::ConfirmarPassword => {
                let mascara = f.confirmar_password.mascara();
                let r = render_campo(
                    frame,
                    filas[fila],
                    "CONFIRMAR CONTRASEÑA",
                    &mascara,
                    enfocado,
                    theme,
                );
                if enfocado {
                    posicionar_cursor(frame, r, &mascara);
                }
            }
            CampoUsuario::Rol => {
                render_opcion(frame, filas[fila], "ROL", texto_rol(f.rol), enfocado, theme);
                if let Some(resaltado) = f.selector_rol {
                    fila += 1;
                    render_selector_rol(frame, filas[fila], resaltado, theme);
                }
            }
            CampoUsuario::Activo => {
                render_opcion(frame, filas[fila], "ACTIVO", si_no(f.activo), enfocado, theme);
            }
        }
        fila += 1;
    }
    frame.render_widget(
        Paragraph::new(f.error.as_deref().unwrap_or_default()).style(theme.danger()),
        filas[fila],
    );
}

fn render_selector_rol(frame: &mut Frame, area: Rect, resaltado: usize, theme: Theme) {
    let lineas: Vec<Line<'_>> = ROLES
        .iter()
        .enumerate()
        .map(|(indice, rol)| {
            let seleccionado = indice == resaltado;
            let marcador = if seleccionado { "  >" } else { "   " };
            Line::from(format!("{marcador} {}", texto_rol(*rol)))
                .style(if seleccionado { theme.selected() } else { theme.muted() })
        })
        .collect();
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_password(frame: &mut Frame, area: Rect, f: &FormularioPassword, theme: Theme) {
    let filas = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(format!("Usuario: {}", f.usuario_nombre)).style(theme.title()),
        filas[0],
    );
    let mascara_nueva = f.password.mascara();
    let area_nueva = render_campo(
        frame,
        filas[1],
        "NUEVA CONTRASEÑA",
        &mascara_nueva,
        f.campo == 0,
        theme,
    );
    let mascara_confirmar = f.confirmar.mascara();
    let area_confirmar = render_campo(
        frame,
        filas[2],
        "CONFIRMAR CONTRASEÑA",
        &mascara_confirmar,
        f.campo == 1,
        theme,
    );
    frame.render_widget(
        Paragraph::new(f.error.as_deref().unwrap_or_default()).style(theme.danger()),
        filas[3],
    );
    if f.campo == 0 {
        posicionar_cursor(frame, area_nueva, &mascara_nueva);
    } else {
        posicionar_cursor(frame, area_confirmar, &mascara_confirmar);
    }
}
