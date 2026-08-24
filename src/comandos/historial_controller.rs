//! Controlador de teclado de la Surface de Historial (`/historial`, `/h`) —
//! §5.2/DEC-023/024. `historial.rs` es el modelo puro (filtro, resolución de
//! clave:valor); este archivo es el único que traduce teclas, consulta
//! `AppCore::buscar_historial` y decide cuándo abrir/cerrar la Surface.

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::services::error::RegistroIngresoServiceError;

use super::estado::{EdicionColumnas, ObjetivoColumnas};
use super::historial::HistorialState;
use super::{AppState, NivelFeedback};

/// Abre la Surface con el catálogo de empresas (para resolver `empresa:`) y
/// el rango de fechas por defecto (mes actual). Todavía sin resultado: hace
/// falta un Enter para aplicar la primera consulta (DEC-024).
pub(super) fn abrir_historial(core: &AppCore, app: &mut AppState) {
    let empresas = core.listar_empresas().unwrap_or_default();
    app.historial = Some(HistorialState::nuevo(empresas));
    app.input.reset();
    app.feedback = None;
    app.sugerencias.clear();
}

fn cerrar_historial(core: &AppCore, app: &mut AppState) {
    app.historial = None;
    app.input.reset();
    super::recomputar(core, app);
}

pub(super) fn manejar_historial(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    let mostrando_resultado = app
        .historial
        .as_ref()
        .is_some_and(|h| h.resultado.is_some());
    if mostrando_resultado {
        manejar_resultado(core, app, key);
    } else {
        manejar_edicion(core, app, key);
    }
}

fn manejar_edicion(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => cerrar_historial(core, app),
        KeyCode::Enter => aplicar_consulta(core, app),
        _ => {
            app.input.handle_event(&Event::Key(key));
        }
    }
}

fn manejar_resultado(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        // Esc vuelve a editar la MISMA consulta, sin perder lo ya escrito
        // (DEC-024) — no borra `app.input` ni el filtro ya resuelto.
        KeyCode::Esc => {
            if let Some(historial) = &mut app.historial {
                historial.resultado = None;
                historial.no_reconocidos.clear();
            }
        }
        KeyCode::Up => mover_seleccion(app, -1),
        KeyCode::Down => mover_seleccion(app, 1),
        KeyCode::PageDown => paginar(core, app, 1),
        KeyCode::PageUp => paginar(core, app, -1),
        // F4 abre el selector de columnas de la tabla de Historial — mismo
        // mecanismo que las otras dos tablas (§5.2), anidado: Esc lo cierra
        // y vuelve a mostrar los mismos resultados, sin volver a consultar.
        KeyCode::F(4) => {
            app.edicion_columnas = Some(EdicionColumnas {
                objetivo: ObjetivoColumnas::Historial,
                seleccion: 0,
            });
        }
        _ => {}
    }
}

fn mover_seleccion(app: &mut AppState, delta: isize) {
    let Some(historial) = &mut app.historial else {
        return;
    };
    let Some(resultado) = &historial.resultado else {
        return;
    };
    if resultado.items.is_empty() {
        return;
    }
    let actual = historial.seleccion as isize;
    let ultimo = resultado.items.len() as isize - 1;
    historial.seleccion = (actual + delta).clamp(0, ultimo) as usize;
}

/// Interpreta el texto tecleado, arma un filtro "de cero" (sin `corte_id`:
/// una consulta nueva, no una página siguiente de la misma) y consulta.
fn aplicar_consulta(core: &AppCore, app: &mut AppState) {
    let Some(historial) = &app.historial else {
        return;
    };
    let (mut filtro, no_reconocidos) = historial.resolver_filtro(app.input.value());
    filtro.offset = 0;
    filtro.corte_id = None;
    match core.buscar_historial(&filtro) {
        Ok(pagina) => {
            if let Some(historial) = &mut app.historial {
                filtro.corte_id = Some(pagina.corte_id);
                historial.filtro = filtro;
                historial.seleccion = 0;
                historial.no_reconocidos = no_reconocidos;
                historial.resultado = Some(pagina);
            }
        }
        Err(error) => app.mostrar_feedback(mensaje_error(&error), NivelFeedback::Error),
    }
}

/// `delta` en páginas (`+1`/`-1`). El `corte_id` ya fijado en `aplicar_consulta`
/// se conserva: así ingresos nuevos registrados mientras se navega no corren
/// las páginas ya vistas.
fn paginar(core: &AppCore, app: &mut AppState, delta: isize) {
    let Some(historial) = &app.historial else {
        return;
    };
    let Some(resultado) = &historial.resultado else {
        return;
    };
    let limite = historial.filtro.limite.max(1);
    let nuevo_offset = if delta < 0 {
        historial.filtro.offset.saturating_sub(limite)
    } else if historial.filtro.offset + limite < resultado.total {
        historial.filtro.offset + limite
    } else {
        historial.filtro.offset
    };
    if nuevo_offset == historial.filtro.offset {
        return;
    }
    let mut filtro = historial.filtro.clone();
    filtro.offset = nuevo_offset;
    match core.buscar_historial(&filtro) {
        Ok(pagina) => {
            if let Some(historial) = &mut app.historial {
                historial.filtro = filtro;
                historial.seleccion = 0;
                historial.resultado = Some(pagina);
            }
        }
        Err(error) => app.mostrar_feedback(mensaje_error(&error), NivelFeedback::Error),
    }
}

fn mensaje_error(error: &RegistroIngresoServiceError) -> String {
    match error {
        RegistroIngresoServiceError::RangoFechasInvalido => {
            "El rango de fechas no es válido (desde debe ser antes que hasta)".to_string()
        }
        _ => "No se pudo consultar el historial".to_string(),
    }
}
