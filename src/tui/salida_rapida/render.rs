use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Clear, Paragraph, Row, Table, TableState},
};

use crate::services::registro_ingreso_service::IngresoActivoResumen;
use crate::tui::ui_kit::{SIMBOLO_RESALTADO_TABLA, Theme, auxiliary_panel, centered_rect};

use super::{Estado, SalidaRapidaState};

pub fn render(frame: &mut Frame, area: Rect, state: &SalidaRapidaState, theme: Theme) {
    if !state.abierto() {
        return;
    }
    let franja = Rect::new(area.x, area.y, area.width, area.height.min(16));
    let popup = centered_rect(franja, 70.min(area.width), 12.min(franja.height));
    frame.render_widget(Clear, popup);
    let block = auxiliary_panel("SALIDA RÁPIDA · F2", theme, true);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    if inner.height == 0 {
        return;
    }

    if let Estado::ConfirmarSalida { registro_id } = &state.estado {
        if let Some(registro) = state.registro(*registro_id) {
            render_confirmar_salida(frame, inner, registro, theme);
        }
        return;
    }

    let filas = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    render_busqueda(frame, filas[0], state, theme);
    render_resultados(frame, filas[1], state, theme);
    render_pie(frame, filas[2], state, theme);
}

/// Paso intermedio antes de registrar la salida: muestra a quién concretamente
/// se va a sacar (nombre, cédula, empresa, gafete) — antes un solo Enter sobre
/// la fila resaltada registraba la salida directo, sin este punto de control.
fn render_confirmar_salida(
    frame: &mut Frame,
    area: Rect,
    registro: &IngresoActivoResumen,
    theme: Theme,
) {
    let gafete = registro
        .gafete_numero
        .map_or_else(|| "S/G".to_owned(), |g| g.to_string());
    let lineas = vec![
        Line::from(format!(
            "¿Confirmar salida de {}?",
            registro.contratista_nombre
        ))
        .style(theme.title()),
        Line::from(""),
        Line::from(format!("Cédula: {}", registro.cedula)).style(theme.base()),
        Line::from(format!("Empresa: {}", registro.empresa_nombre)).style(theme.base()),
        Line::from(format!("Gafete: {gafete}")).style(theme.base()),
        Line::from(""),
        Line::from("ENTER confirma · ESC cancela").style(theme.muted()),
    ];
    frame.render_widget(Paragraph::new(lineas), area);
}

fn render_busqueda(frame: &mut Frame, area: Rect, state: &SalidaRapidaState, theme: Theme) {
    const ETIQUETA: &str = "Buscar gafete o nombre/cédula: ";
    let mut texto = format!("{ETIQUETA}{}", state.busqueda.value());
    if let Some(total) = state.resultados_ocultos() {
        texto.push_str(&format!(
            "  ({} de {total} — afine la búsqueda)",
            state.registros.len()
        ));
    }
    frame.render_widget(Paragraph::new(texto).style(theme.accent()), area);
    let antes_del_cursor: String = state
        .busqueda
        .value()
        .chars()
        .take(state.busqueda.cursor())
        .collect();
    let ancho_visible = u16::try_from(Line::from(format!("{ETIQUETA}{antes_del_cursor}")).width())
        .unwrap_or(u16::MAX);
    let x = area.x.saturating_add(ancho_visible.min(area.width));
    frame.set_cursor_position((x, area.y));
}

fn render_resultados(frame: &mut Frame, area: Rect, state: &SalidaRapidaState, theme: Theme) {
    if state.registros.is_empty() {
        let mensaje = state
            .error
            .as_deref()
            .unwrap_or("Sin ingresos activos con esa búsqueda");
        frame.render_widget(Paragraph::new(mensaje).style(theme.warning()), area);
        return;
    }
    let filas = state.registros.iter().map(|r| {
        let gafete = r
            .gafete_numero
            .map_or_else(|| "S/G".to_owned(), |g| g.to_string());
        Row::new([
            Cell::from(r.cedula.clone()),
            Cell::from(r.contratista_nombre.clone()),
            Cell::from(r.empresa_nombre.clone()),
            Cell::from(gafete),
        ])
        .style(theme.base())
    });
    let anchos = [
        Constraint::Length(13),
        Constraint::Fill(2),
        Constraint::Fill(2),
        Constraint::Length(7),
    ];
    let tabla = Table::new(filas, anchos)
        .row_highlight_style(theme.selected())
        .highlight_symbol(SIMBOLO_RESALTADO_TABLA);
    frame.render_stateful_widget(
        tabla,
        area,
        &mut TableState::default().with_selected(state.seleccion),
    );
}

fn render_pie(frame: &mut Frame, area: Rect, state: &SalidaRapidaState, theme: Theme) {
    if let Some(error) = &state.error {
        frame.render_widget(Paragraph::new(error.as_str()).style(theme.danger()), area);
        return;
    }
    if let Some(mensaje) = &state.mensaje {
        frame.render_widget(
            Paragraph::new(mensaje.as_str()).style(theme.success()),
            area,
        );
        return;
    }
    let texto = if state.ayuda_expandida {
        "↑↓ mover · ENTER seleccionar · ESC cerrar · F1 ayuda · F2 abrir/cerrar salida rápida"
    } else {
        "↑↓ mover · ENTER seleccionar · ESC cerrar · F1 ayuda"
    };
    frame.render_widget(Paragraph::new(texto).style(theme.muted()), area);
}
