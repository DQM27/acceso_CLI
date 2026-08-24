//! Controlador de teclado de la Surface de Historial (`/historial`, `/h`) —
//! §5.2/DEC-023/024. `historial.rs` es el modelo puro (filtro, resolución de
//! clave:valor); este archivo es el único que traduce teclas, consulta
//! `AppCore::buscar_historial` y decide cuándo abrir/cerrar la Surface.

use std::path::PathBuf;

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::historial::exportacion::ColumnaHistorial as ColumnaExportacion;
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
    let exportando = app
        .historial
        .as_ref()
        .is_some_and(|h| h.exportacion_destino.is_some());
    if exportando {
        manejar_exportacion(core, app, key);
        return;
    }
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
        // F5 abre el destino de exportación — mismo atajo que la TUI
        // clásica, sobre el resultado ya aplicado en pantalla (exporta el
        // filtro completo, no sólo la página visible).
        KeyCode::F(5) => abrir_exportacion(app),
        _ => {}
    }
}

fn abrir_exportacion(app: &mut AppState) {
    let Some(historial) = &mut app.historial else {
        return;
    };
    let sin_resultados = historial
        .resultado
        .as_ref()
        .is_none_or(|resultado| resultado.total == 0);
    if sin_resultados {
        app.mostrar_feedback(
            "No hay movimientos para exportar".to_string(),
            NivelFeedback::Advertencia,
        );
        return;
    }
    historial.exportacion_destino = Some(Input::new(ruta_exportacion_predeterminada()));
}

fn manejar_exportacion(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        // Cancela la exportación y vuelve a mostrar el mismo resultado —
        // el filtro y la consulta ya aplicada no se tocan.
        KeyCode::Esc => {
            if let Some(historial) = &mut app.historial {
                historial.exportacion_destino = None;
            }
        }
        KeyCode::Enter => confirmar_exportacion(core, app),
        _ => {
            if let Some(historial) = &mut app.historial
                && let Some(destino) = &mut historial.exportacion_destino
            {
                destino.handle_event(&Event::Key(key));
            }
        }
    }
}

/// Exporta el filtro completo (no sólo la página en pantalla) con todas las
/// columnas del exportador (`ColumnaExportacion::ALL`) — elegir un
/// subconjunto de columnas para exportar queda deliberadamente fuera de
/// esta primera versión; hoy es todo o nada.
fn confirmar_exportacion(core: &AppCore, app: &mut AppState) {
    let Some(historial) = &app.historial else {
        return;
    };
    let Some(destino_input) = &historial.exportacion_destino else {
        return;
    };
    let destino = match normalizar_destino(destino_input.value()) {
        Ok(destino) => destino,
        Err(mensaje) => {
            app.mostrar_feedback(mensaje, NivelFeedback::Error);
            return;
        }
    };
    let mut filtro = historial.filtro.clone();
    filtro.offset = 0;

    let resultado = core.exportar_historial(&filtro, &ColumnaExportacion::ALL, &destino);
    if let Some(historial) = &mut app.historial {
        historial.exportacion_destino = None;
    }
    match resultado {
        Ok(cantidad) => app.mostrar_feedback(
            format!("Exportado — {cantidad} movimientos → {}", destino.display()),
            NivelFeedback::Exito,
        ),
        Err(error) => app.mostrar_feedback(
            format!("No se pudo exportar: {error}"),
            NivelFeedback::Error,
        ),
    }
}

/// Mismo criterio que `ruta_exportacion_predeterminada` de la TUI clásica
/// (reescrito, no importado — DEC-002/DEC-014): nombre con fecha/hora en
/// `Documents` si existe una carpeta de usuario válida, si no sólo el
/// nombre (relativo al directorio de trabajo).
fn ruta_exportacion_predeterminada() -> String {
    let nombre = super::historial::nombre_exportacion_predeterminado();
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .filter(|ruta| ruta.is_absolute())
        .map(|ruta| ruta.join("Documents"))
        .filter(|ruta| ruta.is_dir())
        .map(|ruta| ruta.join(&nombre))
        .unwrap_or_else(|| PathBuf::from(nombre))
        .display()
        .to_string()
}

fn normalizar_destino(valor: &str) -> Result<PathBuf, String> {
    let valor = valor.trim();
    if valor.is_empty() {
        return Err("Ingrese una ruta para el archivo XLSX".to_string());
    }
    let mut destino = PathBuf::from(valor);
    match destino.extension().and_then(|extension| extension.to_str()) {
        None => destino.set_extension("xlsx"),
        Some(extension) if extension.eq_ignore_ascii_case("xlsx") => true,
        Some(_) => return Err("La exportación debe usar la extensión .xlsx".to_string()),
    };
    Ok(destino)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruta_vacia_es_error() {
        assert!(normalizar_destino("").is_err());
        assert!(normalizar_destino("   ").is_err());
    }

    #[test]
    fn sin_extension_agrega_xlsx() {
        let destino = normalizar_destino("historial").unwrap();
        assert_eq!(destino.extension().and_then(|e| e.to_str()), Some("xlsx"));
    }

    #[test]
    fn extension_xlsx_se_conserva_sin_distinguir_mayusculas() {
        let destino = normalizar_destino("historial.XLSX").unwrap();
        assert_eq!(destino.extension().and_then(|e| e.to_str()), Some("XLSX"));
    }

    #[test]
    fn otra_extension_es_error() {
        assert!(normalizar_destino("historial.csv").is_err());
    }
}
