use crate::tiempo::hora_actual_texto;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use super::{ConfirmacionMenu, MenuPrincipalState, OpcionMenu};
use crate::{
    models::usuario::RolUsuario,
    services::autenticacion_service::UsuarioSesion,
    tui::{layout, theme},
};

pub fn render(frame: &mut Frame, area: Rect, state: &MenuPrincipalState, sesion: &UsuarioSesion) {
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
        Constraint::Min(14),
        Constraint::Length(3),
    ])
    .split(contenido);
    cabecera(frame, zonas[0], sesion);
    cuerpo(frame, zonas[1], state);
    pie(frame, zonas[2]);
    if let Some(c) = state.confirmacion {
        confirmacion(frame, contenido, c, sesion);
    }
}

fn cabecera(frame: &mut Frame, area: Rect, sesion: &UsuarioSesion) {
    let bloque = Block::bordered().border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let cols = Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(40),
        Constraint::Percentage(30),
    ])
    .split(interior);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(" B R I S A S   C L I").style(theme::titulo()),
            Line::from(" CONTROL DE ACCESO").style(theme::texto_secundario()),
        ]),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new("MENÚ PRINCIPAL")
            .style(theme::foco())
            .alignment(Alignment::Center),
        Rect::new(cols[1].x, cols[1].y + cols[1].height / 2, cols[1].width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!("{} · {} ", sesion.nombre, rol(sesion.rol)))
            .alignment(Alignment::Right),
        Rect::new(cols[2].x, cols[2].y + cols[2].height / 2, cols[2].width, 1),
    );
}

fn cuerpo(frame: &mut Frame, area: Rect, state: &MenuPrincipalState) {
    let ancho = area.width.min(68);
    let alto = area.height.min(22);
    let panel = Rect::new(
        area.x + (area.width - ancho) / 2,
        area.y + (area.height - alto) / 2,
        ancho,
        alto,
    );
    let bloque = Block::bordered().border_style(theme::borde());
    let interior = bloque.inner(panel);
    frame.render_widget(bloque, panel);
    let mut lineas = Vec::new();
    grupo(&mut lineas, "OPERACIÓN", &OpcionMenu::TODAS[0..3], state);
    grupo(
        &mut lineas,
        "ADMINISTRACIÓN",
        &OpcionMenu::TODAS[3..6],
        state,
    );
    grupo(&mut lineas, "SESIÓN", &OpcionMenu::TODAS[6..8], state);
    lineas.push(Line::from(""));
    lineas.push(
        Line::from(state.seleccion.descripcion())
            .style(theme::texto_secundario())
            .alignment(Alignment::Center),
    );
    frame.render_widget(Paragraph::new(lineas), interior);
}

fn grupo<'a>(
    lineas: &mut Vec<Line<'a>>,
    titulo: &'a str,
    opciones: &[OpcionMenu],
    state: &MenuPrincipalState,
) {
    lineas.push(
        Line::from(titulo)
            .style(theme::foco())
            .alignment(Alignment::Center),
    );
    for opcion in opciones {
        let texto = format!("  {}  ", opcion.etiqueta());
        lineas.push(Line::from(if *opcion == state.seleccion {
            Span::styled(format!("> {texto}"), theme::seleccionado())
        } else {
            Span::raw(format!("  {texto}"))
        }));
    }
    lineas.push(Line::from(""));
}

fn pie(frame: &mut Frame, area: Rect) {
    let bloque = Block::bordered().border_style(theme::borde());
    let interior = bloque.inner(area);
    frame.render_widget(bloque, area);
    let cols = Layout::horizontal([Constraint::Min(1), Constraint::Length(11)]).split(interior);
    let ayuda = if area.width >= 100 {
        "↑↓ Seleccionar │ ENTER Abrir │ 1–6 Acceso rápido │ L Cerrar sesión │ Q Salir"
    } else {
        "↑↓ │ ENTER │ 1–6 │ L Sesión │ Q Salir"
    };
    frame.render_widget(
        Paragraph::new(ayuda)
            .style(theme::foco())
            .alignment(Alignment::Center),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(hora_actual_texto())
            .style(theme::advertencia())
            .alignment(Alignment::Right),
        cols[1],
    );
}

fn confirmacion(frame: &mut Frame, area: Rect, c: ConfirmacionMenu, sesion: &UsuarioSesion) {
    let (titulo, pregunta) = match c {
        ConfirmacionMenu::CerrarSesion => (
            "CERRAR SESIÓN",
            format!("¿Cerrar la sesión de {}?", sesion.nombre),
        ),
        ConfirmacionMenu::Salir => ("SALIR DE BRISAS CLI", "¿Desea cerrar la aplicación?".into()),
    };
    layout::render_overlay(
        frame,
        area,
        54,
        8,
        3,
        titulo,
        vec![
            Line::from(pregunta).alignment(Alignment::Center),
            Line::from(""),
            Line::from("Y Confirmar   N / ESC Cancelar")
                .style(theme::foco())
                .alignment(Alignment::Center),
        ],
    );
}

fn rol(rol: RolUsuario) -> &'static str {
    match rol {
        RolUsuario::Root => "ROOT",
        RolUsuario::Administrador => "ADMINISTRADOR",
        RolUsuario::Operador => "OPERADOR",
    }
}
