use chrono::Local;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table},
};

use super::*;
use crate::{
    database::queries::usuarios::UsuarioResumen,
    tui::{layout, theme},
};

pub fn render(frame: &mut Frame, area: Rect, state: &UsuariosState) {
    frame.render_widget(Block::default().style(theme::texto_normal()), area);
    if area.width < 60 || area.height < 22 {
        layout::render_terminal_pequena(frame, area);
        return;
    }
    let contenido = Rect::new(
        area.x + 2,
        area.y,
        area.width.saturating_sub(4),
        area.height,
    );
    let zonas = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(2),
        Constraint::Min(8),
        Constraint::Length(3),
    ])
    .split(contenido);
    render_cabecera(frame, zonas[0], state);
    render_estado(frame, zonas[1], state);
    render_tabla(frame, zonas[2], state);
    render_pie(frame, zonas[3], state);
    render_modo(frame, contenido, state);
}

fn render_cabecera(frame: &mut Frame, area: Rect, state: &UsuariosState) {
    let bloque = Block::bordered().border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let columnas = Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(40),
        Constraint::Percentage(30),
    ])
    .split(interior);
    let centro = |a: Rect| Rect::new(a.x, a.y + a.height / 2, a.width, 1);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(" B R I S A S   C L I").style(theme::titulo()),
            Line::from(" CONTROL DE ACCESO").style(theme::texto_secundario()),
        ]),
        Rect::new(columnas[0].x, columnas[0].y, columnas[0].width, 2),
    );
    frame.render_widget(
        Paragraph::new("BASE DE USUARIOS")
            .style(theme::foco())
            .alignment(Alignment::Center),
        centro(columnas[1]),
    );
    frame.render_widget(
        Paragraph::new(format!("Usuario: {} ", state.usuario_nombre)).alignment(Alignment::Right),
        centro(columnas[2]),
    );
}

fn render_estado(frame: &mut Frame, area: Rect, state: &UsuariosState) {
    let linea = match &state.modo {
        ModoUsuarios::Busqueda { texto } => Line::from(vec![
            Span::styled("BUSCAR USUARIOS: ", theme::foco()),
            Span::raw(format!("{texto}_")),
        ]),
        _ if !state.filtro.is_empty() => Line::from(vec![
            Span::styled("FILTRO ACTIVO: ", theme::foco()),
            Span::raw(&state.filtro),
            Span::styled(
                format!("    {} resultados    ", state.usuarios.len()),
                theme::texto_secundario(),
            ),
            Span::styled("ESC ", theme::ayuda_tecla()),
            Span::raw("Limpiar"),
        ]),
        _ => Line::from(state.mensaje.clone().unwrap_or_default()).style(
            if state.mensaje.as_deref().is_some_and(|m| m.starts_with('✓')) {
                theme::exito()
            } else {
                theme::error()
            },
        ),
    };
    frame.render_widget(Paragraph::new(linea).alignment(Alignment::Center), area);
}

fn render_tabla(frame: &mut Frame, area: Rect, state: &UsuariosState) {
    let marco = Block::bordered().border_style(theme::borde());
    let interior = marco.inner(area);
    frame.render_widget(marco, area);
    let capacidad = interior.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad).min(state.usuarios.len());
    let filas = state
        .usuarios
        .iter()
        .skip(inicio)
        .take(capacidad)
        .enumerate()
        .map(|(visible, usuario)| {
            let seleccionado = state.seleccion == Some(inicio + visible);
            let estilo_fila = if seleccionado {
                theme::seleccionado()
            } else {
                theme::texto_normal()
            };
            Row::new([
                Cell::from(usuario.cedula.clone()),
                Cell::from(usuario.nombre.clone()),
                Cell::from(texto_rol(usuario.rol)).style(if seleccionado {
                    estilo_fila
                } else if usuario.rol == RolUsuario::Root {
                    theme::advertencia()
                } else {
                    theme::texto_normal()
                }),
                Cell::from(if usuario.activo { "ACTIVO" } else { "INACTIVO" }).style(
                    if seleccionado {
                        estilo_fila
                    } else if usuario.activo {
                        theme::exito()
                    } else {
                        theme::error()
                    },
                ),
            ])
            .style(estilo_fila)
        });
    frame.render_widget(
        Table::new(
            filas,
            [
                Constraint::Length(18),
                Constraint::Fill(3),
                Constraint::Length(16),
                Constraint::Length(10),
            ],
        )
        .header(
            Row::new(["CÉDULA", "NOMBRE", "ROL", "ESTADO"])
                .style(theme::foco())
                .bottom_margin(1),
        )
        .column_spacing(1),
        interior,
    );
    if state.usuarios.is_empty() {
        frame.render_widget(
            Paragraph::new("No hay usuarios que coincidan con la búsqueda.")
                .style(theme::advertencia())
                .alignment(Alignment::Center),
            interior,
        );
    }
}

fn render_pie(frame: &mut Frame, area: Rect, state: &UsuariosState) {
    let bloque = Block::bordered().border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let columnas = Layout::horizontal([
        Constraint::Length(34),
        Constraint::Min(20),
        Constraint::Length(12),
    ])
    .split(interior);
    let posicion = state.seleccion.map_or_else(
        || "—/—".into(),
        |i| format!("{}/{}", i + 1, state.usuarios.len()),
    );
    frame.render_widget(
        Paragraph::new(format!(
            " {} usuarios │ Registro {posicion}",
            state.usuarios.len()
        )),
        columnas[0],
    );
    frame.render_widget(Paragraph::new("↑↓ Seleccionar │ ENTER Detalle │ N Nuevo │ E Editar │ P Clave │ A Estado │ / Buscar │ ESC Volver").style(theme::foco()).alignment(Alignment::Center), columnas[1]);
    frame.render_widget(
        Paragraph::new(Local::now().format("%H:%M:%S").to_string())
            .style(theme::advertencia())
            .alignment(Alignment::Right)
            .block(Block::default().padding(ratatui::widgets::Padding::right(1))),
        columnas[2],
    );
}

fn render_modo(frame: &mut Frame, area: Rect, state: &UsuariosState) {
    match &state.modo {
        ModoUsuarios::Detalle { id } => {
            if let Some(u) = state.usuario(*id) {
                render_detalle(frame, area, u);
            }
        }
        ModoUsuarios::Formulario(f) => render_formulario(frame, area, f),
        ModoUsuarios::CambioPassword(f) => render_password(frame, area, f),
        ModoUsuarios::ConfirmacionEstado(c) => render_confirmacion(frame, area, state, *c),
        _ => {}
    }
}

fn render_detalle(frame: &mut Frame, area: Rect, u: &UsuarioResumen) {
    layout::render_overlay(
        frame,
        area,
        64,
        14,
        4,
        "DETALLE DEL USUARIO",
        vec![
            Line::from(u.nombre.clone()).style(theme::titulo()),
            Line::from(""),
            Line::from(format!("Nombre        {}", u.nombre)),
            Line::from(format!("Cédula        {}", u.cedula)),
            Line::from(format!("Rol           {}", texto_rol(u.rol))),
            Line::from(format!(
                "Estado        {}",
                if u.activo { "ACTIVO" } else { "INACTIVO" }
            )),
            Line::from(""),
            Line::from("E Editar │ P Cambiar contraseña │ A Estado │ ESC Cerrar")
                .style(theme::foco()),
        ],
    );
}

fn render_formulario(frame: &mut Frame, area: Rect, f: &FormularioUsuario) {
    let titulo = if matches!(f.modo, ModoFormularioUsuario::Crear) {
        "NUEVO USUARIO"
    } else {
        "EDITAR USUARIO"
    };
    let mut lineas = vec![Line::from("")];
    for (indice, campo) in f.campos().iter().enumerate() {
        let (nombre, valor) = match campo {
            CampoUsuario::Cedula => ("Cédula", f.cedula.clone()),
            CampoUsuario::Nombre => ("Nombre", f.nombre.clone()),
            CampoUsuario::Rol => ("Rol", texto_rol(f.rol).into()),
            CampoUsuario::Password => ("Contraseña", f.password.mascara()),
            CampoUsuario::ConfirmarPassword => {
                ("Confirmar contraseña", f.confirmar_password.mascara())
            }
            CampoUsuario::Activo => ("Activo", si_no(f.activo).into()),
        };
        lineas.push(
            Line::from(format!(
                "{} {:<22} {}{}",
                if f.campo == indice { ">" } else { " " },
                nombre,
                valor,
                if f.campo == indice
                    && matches!(
                        campo,
                        CampoUsuario::Cedula
                            | CampoUsuario::Nombre
                            | CampoUsuario::Password
                            | CampoUsuario::ConfirmarPassword
                    )
                {
                    "_"
                } else {
                    ""
                }
            ))
            .style(if f.campo == indice {
                theme::foco()
            } else {
                theme::texto_normal()
            }),
        );
        lineas.push(Line::from(""));
    }
    lineas.push(Line::from(f.error.clone().unwrap_or_default()).style(theme::error()));
    lineas.push(Line::from("↑↓/Tab Navegar │ ENTER Seleccionar/Cambiar").style(theme::foco()));
    lineas
        .push(Line::from("Shift+G Guardar                     ESC Cancelar").style(theme::foco()));
    let alto = if matches!(f.modo, ModoFormularioUsuario::Crear) {
        23
    } else {
        19
    };
    layout::render_overlay(frame, area, 76, alto, 4, titulo, lineas);
    if let Some(opcion) = f.selector_rol {
        render_roles(frame, area, opcion);
    } else if matches!(
        f.campo_actual(),
        CampoUsuario::Cedula
            | CampoUsuario::Nombre
            | CampoUsuario::Password
            | CampoUsuario::ConfirmarPassword
    ) {
        let ancho = 76.min(area.width.saturating_sub(4));
        let modal_x = area.x + area.width.saturating_sub(ancho) / 2;
        let modal_y = area.y + area.height.saturating_sub(alto) / 2;
        let valor_largo = match f.campo_actual() {
            CampoUsuario::Cedula => f.cedula.chars().count(),
            CampoUsuario::Nombre => f.nombre.chars().count(),
            CampoUsuario::Password => f.password.0.chars().count(),
            CampoUsuario::ConfirmarPassword => f.confirmar_password.0.chars().count(),
            _ => 0,
        } as u16;
        frame.set_cursor_position((
            modal_x.saturating_add(28).saturating_add(valor_largo),
            modal_y.saturating_add(3 + f.campo as u16 * 2),
        ));
    }
}

fn render_roles(frame: &mut Frame, area: Rect, opcion: usize) {
    let mut lineas: Vec<_> = ROLES
        .iter()
        .enumerate()
        .map(|(i, rol)| {
            Line::from(format!(
                "{} {}",
                if i == opcion { ">" } else { " " },
                texto_rol(*rol)
            ))
            .style(if i == opcion {
                theme::seleccionado()
            } else {
                theme::texto_normal()
            })
        })
        .collect();
    lineas.push(Line::from(""));
    lineas.push(Line::from("↑↓ Seleccionar │ ENTER Aceptar │ ESC Cancelar").style(theme::foco()));
    layout::render_overlay(frame, area, 50, 10, 4, "SELECCIONAR ROL", lineas);
}

fn render_password(frame: &mut Frame, area: Rect, f: &FormularioPassword) {
    layout::render_overlay(
        frame,
        area,
        68,
        14,
        4,
        "CAMBIAR CONTRASEÑA",
        vec![
            Line::from(format!("Usuario: {}", f.usuario_nombre)).style(theme::titulo()),
            Line::from(""),
            Line::from(format!(
                "{} Nueva contraseña       {}{}",
                if f.campo == 0 { ">" } else { " " },
                f.password.mascara(),
                if f.campo == 0 { "_" } else { "" }
            ))
            .style(if f.campo == 0 {
                theme::foco()
            } else {
                theme::texto_normal()
            }),
            Line::from(""),
            Line::from(format!(
                "{} Confirmar contraseña   {}{}",
                if f.campo == 1 { ">" } else { " " },
                f.confirmar.mascara(),
                if f.campo == 1 { "_" } else { "" }
            ))
            .style(if f.campo == 1 {
                theme::foco()
            } else {
                theme::texto_normal()
            }),
            Line::from(""),
            Line::from(f.error.clone().unwrap_or_default()).style(theme::error()),
            Line::from("Shift+G Guardar                 ESC Cancelar").style(theme::foco()),
        ],
    );
    let ancho = 68.min(area.width.saturating_sub(4));
    let modal_x = area.x + area.width.saturating_sub(ancho) / 2;
    let modal_y = area.y + area.height.saturating_sub(14) / 2;
    let largo = if f.campo == 0 {
        f.password.0.chars().count()
    } else {
        f.confirmar.0.chars().count()
    } as u16;
    frame.set_cursor_position((
        modal_x.saturating_add(29).saturating_add(largo),
        modal_y.saturating_add(if f.campo == 0 { 4 } else { 6 }),
    ));
}

fn render_confirmacion(
    frame: &mut Frame,
    area: Rect,
    state: &UsuariosState,
    c: ConfirmacionEstado,
) {
    if let Some(u) = state.usuario(c.id) {
        let accion = if c.activar { "ACTIVAR" } else { "DESACTIVAR" };
        layout::render_overlay(
            frame,
            area,
            58,
            10,
            4,
            &format!("{accion} USUARIO"),
            vec![
                Line::from(""),
                Line::from(format!(
                    "¿{} a {}?",
                    if c.activar { "Activar" } else { "Desactivar" },
                    u.nombre
                )),
                Line::from(""),
                Line::from("Y Confirmar              N / ESC Cancelar").style(theme::foco()),
            ],
        );
    }
}
