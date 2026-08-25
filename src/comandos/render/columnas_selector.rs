//! Segunda Surface enclavada (§5.2, junto al formulario): selector de
//! columnas visibles (F4) para búsqueda, activos o historial.

use ratatui::text::{Line, Span};

use crate::comandos::columnas::Columna;
use crate::comandos::estado::{AppState, EdicionColumnas, ObjetivoColumnas};

use super::estilos::{estilo_seleccion, muted};

/// `[✓]`/`[ ]` con `›` en la activa — mismo vocabulario de foco que el
/// resto de la app.
pub(super) fn lineas_selector_columnas(app: &AppState, edicion: EdicionColumnas) -> Vec<Line<'static>> {
    let titulo = match edicion.objetivo {
        ObjetivoColumnas::Busqueda => "COLUMNAS — resultados de búsqueda",
        ObjetivoColumnas::Activos => "COLUMNAS — activos",
        ObjetivoColumnas::Historial => "COLUMNAS — historial",
    };
    let filas: Vec<(&'static str, bool)> = match edicion.objetivo {
        ObjetivoColumnas::Busqueda => app
            .columnas_busqueda
            .iter()
            .map(|(c, v)| (c.etiqueta(), v))
            .collect(),
        ObjetivoColumnas::Activos => app
            .columnas_activos
            .iter()
            .map(|(c, v)| (c.etiqueta(), v))
            .collect(),
        ObjetivoColumnas::Historial => app
            .columnas_historial
            .iter()
            .map(|(c, v)| (c.etiqueta(), v))
            .collect(),
    };

    let mut lineas = vec![Line::from(Span::styled(titulo, muted())), Line::from("")];
    for (indice, (etiqueta, visible)) in filas.into_iter().enumerate() {
        let activo = indice == edicion.seleccion;
        let marcador = if activo { "› " } else { "  " };
        let casillero = if visible { "[✓] " } else { "[ ] " };
        let texto = format!("{marcador}{casillero}{etiqueta}");
        lineas.push(if activo {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else if visible {
            Line::from(texto)
        } else {
            Line::from(Span::styled(texto, muted()))
        });
    }
    lineas
}
