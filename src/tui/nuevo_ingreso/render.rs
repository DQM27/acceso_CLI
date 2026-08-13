use super::*;
use crate::tui::{layout, theme};
use chrono::Local;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph, Row, Table},
};

pub fn render(frame: &mut Frame, area: Rect, state: &NuevoIngresoState) {
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
        Constraint::Min(10),
        Constraint::Length(3),
    ])
    .split(contenido);
    cabecera(frame, zonas[0], state);
    contenido_etapa(frame, zonas[1], state);
    pie(frame, zonas[2], state);
}
fn cabecera(frame: &mut Frame, area: Rect, state: &NuevoIngresoState) {
    let b = Block::bordered().border_style(theme::borde());
    let i = b.inner(area);
    frame.render_widget(b, area);
    let c = Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(40),
        Constraint::Percentage(30),
    ])
    .split(i);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(" B R I S A S   C L I").style(theme::titulo()),
            Line::from(" CONTROL DE ACCESO").style(theme::texto_secundario()),
        ]),
        c[0],
    );
    frame.render_widget(
        Paragraph::new("NUEVO INGRESO")
            .style(theme::foco())
            .alignment(Alignment::Center),
        Rect::new(c[1].x, c[1].y + c[1].height / 2, c[1].width, 1),
    );
    frame.render_widget(
        Paragraph::new(format!("Usuario: {} ", state.usuario_nombre)).alignment(Alignment::Right),
        Rect::new(c[2].x, c[2].y + c[2].height / 2, c[2].width, 1),
    );
}
fn contenido_etapa(frame: &mut Frame, area: Rect, state: &NuevoIngresoState) {
    match state.etapa {
        EtapaNuevoIngreso::Buscar => buscar(frame, area, state),
        EtapaNuevoIngreso::Preparacion => preparacion(frame, area, state),
        EtapaNuevoIngreso::Medio { opcion } => medio(frame, area, opcion),
        EtapaNuevoIngreso::Gafete => gafete(frame, area, state),
        EtapaNuevoIngreso::Confirmar => confirmar(frame, area, state),
    }
}
fn buscar(frame: &mut Frame, area: Rect, state: &NuevoIngresoState) {
    let z = Layout::vertical([Constraint::Length(3), Constraint::Min(5)]).split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Buscar: ", theme::foco()),
            Span::raw(format!("{}_", state.busqueda)),
        ]))
        .alignment(Alignment::Center),
        z[0],
    );
    let b = Block::bordered().border_style(theme::borde());
    let i = b.inner(z[1]);
    frame.render_widget(b, z[1]);
    let capacidad = i.height.saturating_sub(2) as usize;
    let inicio = state.inicio_visible(capacidad);
    let filas = state
        .contratistas
        .iter()
        .enumerate()
        .skip(inicio)
        .take(capacidad)
        .map(|(v, c)| {
            Row::new([
                c.cedula.clone(),
                c.nombre.clone(),
                c.empresa_nombre.clone(),
                texto_tipo(c.tipo_ingreso).into(),
            ])
            .style(if state.seleccion == Some(v) {
                theme::seleccionado()
            } else {
                theme::texto_normal()
            })
        });
    frame.render_widget(
        Table::new(
            filas,
            [
                Constraint::Length(20),
                Constraint::Fill(2),
                Constraint::Fill(2),
                Constraint::Length(14),
            ],
        )
        .header(
            Row::new(["CÉDULA", "NOMBRE", "EMPRESA", "TIPO"])
                .style(theme::foco())
                .bottom_margin(1),
        ),
        i,
    );
    if state.contratistas.is_empty() {
        frame.render_widget(
            Paragraph::new("Sin resultados")
                .style(theme::advertencia())
                .alignment(Alignment::Center),
            i,
        )
    }
}
fn preparacion(frame: &mut Frame, area: Rect, state: &NuevoIngresoState) {
    let Some(c) = state.contratista() else { return };
    let p = state.preparacion.as_ref().unwrap();
    let (acceso, estilo, motivo) = match &p.resultado_acceso {
        ResultadoAcceso::Permitido => ("PERMITIDO", theme::exito(), None),
        ResultadoAcceso::PermitidoConAdvertencia => (
            "PERMITIDO CON ADVERTENCIA",
            theme::advertencia(),
            Some("PRAIND vence hoy o dentro de 30 días".to_owned()),
        ),
        ResultadoAcceso::Denegado(m) => (
            "ACCESO DENEGADO",
            theme::error(),
            Some(texto_denegacion(m.clone())),
        ),
    };
    let activo = p.tiene_ingreso_activo;
    let mut l = vec![
        Line::from("CONTRATISTA").style(theme::titulo()),
        Line::from(""),
        Line::from(format!("Nombre             {}", c.nombre)),
        Line::from(format!("Cédula             {}", c.cedula)),
        Line::from(format!("Empresa            {}", p.empresa_nombre)),
        Line::from(format!("Tipo               {}", texto_tipo(p.tipo_ingreso))),
        Line::from(format!("Acceso             {acceso}")).style(estilo),
        Line::from(format!(
            "Gafete             {}",
            if p.requiere_gafete {
                "REQUERIDO"
            } else {
                "S/G"
            }
        )),
    ];
    if let Some(m) = motivo {
        l.push(Line::from(""));
        l.push(Line::from(m).style(estilo))
    }
    if activo {
        l.push(Line::from(""));
        l.push(
            Line::from("INGRESO NO DISPONIBLE — El contratista ya tiene un ingreso activo")
                .style(theme::error()),
        )
    }
    frame.render_widget(
        Paragraph::new(l).block(
            Block::bordered()
                .border_style(theme::borde())
                .padding(ratatui::widgets::Padding::uniform(2)),
        ),
        area,
    )
}
fn medio(frame: &mut Frame, area: Rect, opcion: usize) {
    let l = MEDIOS
        .iter()
        .enumerate()
        .map(|(i, m)| {
            Line::from(format!(
                "{} {}",
                if i == opcion { ">" } else { " " },
                texto_medio(*m)
            ))
            .style(if i == opcion {
                theme::seleccionado()
            } else {
                theme::texto_normal()
            })
        })
        .collect();
    layout::render_overlay(frame, area, 48, 10, 4, "MEDIO DE INGRESO", l)
}
fn gafete(frame: &mut Frame, area: Rect, state: &NuevoIngresoState) {
    layout::render_overlay(
        frame,
        area,
        58,
        12,
        4,
        "INGRESAR GAFETE",
        vec![
            Line::from(""),
            Line::from(format!("Número de gafete   {}_", state.gafete_texto)).style(theme::foco()),
            Line::from(""),
            Line::from(state.error.clone().unwrap_or_default()).style(theme::error()),
            Line::from("ENTER Continuar                 ESC Volver").style(theme::foco()),
        ],
    )
}
fn confirmar(frame: &mut Frame, area: Rect, state: &NuevoIngresoState) {
    let c = state.contratista().unwrap();
    let p = state.preparacion.as_ref().unwrap();
    let mut l = vec![
        Line::from("CONFIRMAR INGRESO").style(theme::titulo()),
        Line::from(""),
        Line::from(format!("Contratista       {}", c.nombre)),
        Line::from(format!("Cédula            {}", c.cedula)),
        Line::from(format!("Empresa           {}", p.empresa_nombre)),
        Line::from(format!("Tipo              {}", texto_tipo(p.tipo_ingreso))),
        Line::from(format!(
            "Medio             {}",
            texto_medio(state.medio.unwrap())
        )),
        Line::from(format!(
            "Acceso            {}",
            if matches!(p.resultado_acceso, ResultadoAcceso::PermitidoConAdvertencia) {
                "ADVERTENCIA PRAIND"
            } else {
                "PERMITIDO"
            }
        )),
        Line::from(format!(
            "Gafete            {}",
            state.gafete.map_or("S/G".into(), |n| n.to_string())
        )),
        Line::from(format!("Registrado por    {}", state.usuario_nombre)),
    ];
    if matches!(
        state.preparacion.as_ref().map(|p| &p.resultado_acceso),
        Some(ResultadoAcceso::PermitidoConAdvertencia)
    ) {
        l.push(Line::from("PRAIND próximo a vencer").style(theme::advertencia()))
    }
    if let Some(e) = &state.error {
        l.push(Line::from(e.clone()).style(theme::error()))
    }
    frame.render_widget(
        Paragraph::new(l).block(
            Block::bordered()
                .border_style(theme::borde())
                .padding(ratatui::widgets::Padding::uniform(2)),
        ),
        area,
    )
}

fn texto_tipo(t: crate::models::tipo_ingreso::TipoIngreso) -> &'static str {
    match t {
        crate::models::tipo_ingreso::TipoIngreso::Praind => "PRAIND",
        crate::models::tipo_ingreso::TipoIngreso::InHouse => "IN HOUSE",
        crate::models::tipo_ingreso::TipoIngreso::PorCorreo => "POR CORREO",
        crate::models::tipo_ingreso::TipoIngreso::Swat => "SWAT",
    }
}
fn texto_denegacion(m: crate::domain::resultado_acceso::MotivoDenegacion) -> String {
    match m {
        crate::domain::resultado_acceso::MotivoDenegacion::SinAcceso => {
            "No tiene acceso autorizado".into()
        }
        crate::domain::resultado_acceso::MotivoDenegacion::PraindVencido => {
            "PRAIND vencido o requerido".into()
        }
    }
}
fn pie(frame: &mut Frame, area: Rect, state: &NuevoIngresoState) {
    let ayuda = match state.etapa {
        EtapaNuevoIngreso::Buscar => {
            "Escribir Buscar │ ↑↓ Seleccionar │ ENTER Continuar │ ESC Volver"
        }
        EtapaNuevoIngreso::Preparacion => "ENTER Continuar │ ESC Cambiar contratista",
        EtapaNuevoIngreso::Medio { .. } => "↑↓ Seleccionar │ ENTER Continuar │ ESC Volver",
        EtapaNuevoIngreso::Gafete => "Escribir número │ ENTER Continuar │ ESC Volver",
        EtapaNuevoIngreso::Confirmar => "Y Confirmar │ N/ESC Volver",
    };
    let b = Block::bordered().border_style(theme::borde());
    let i = b.inner(area);
    frame.render_widget(b, area);
    let c = Layout::horizontal([Constraint::Min(20), Constraint::Length(12)]).split(i);
    frame.render_widget(
        Paragraph::new(ayuda)
            .style(theme::foco())
            .alignment(Alignment::Center),
        c[0],
    );
    frame.render_widget(
        Paragraph::new(Local::now().format("%H:%M:%S").to_string())
            .style(theme::advertencia())
            .alignment(Alignment::Right),
        c[1],
    );
}
