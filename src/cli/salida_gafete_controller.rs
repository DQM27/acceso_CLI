//! Controlador de teclado del modo enclavado de salida por gafete
//! (`salida_gafete.rs`) — DEC-057. A diferencia de las demás Surfaces, no
//! se cierra sola tras confirmar: el caso de uso es repetido (gafete tras
//! gafete, o un grupo entero de una vez), así que sólo Esc sale.

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::database::queries::Igualdad;
use crate::database::queries::ingresos::FiltroIngresosActivos;
use crate::services::registro_ingreso_service::IngresoActivoResumen;

use super::operando::mensaje_error_salida;
use super::salida_gafete::SalidaGafeteState;
use super::{AppState, Fase, NivelFeedback};

/// Si `texto_inicial` ya trae algo (se escribió antes del primer Enter,
/// p. ej. `/gafete 2, 25, 85` de un tirón), se procesa de una vez en el
/// mismo paso — no hace falta un segundo Enter sobre la Surface vacía para
/// lo que el operador ya escribió.
pub(super) fn abrir_salida_gafete(core: &AppCore, app: &mut AppState, texto_inicial: &str) {
    app.salida_gafete = Some(SalidaGafeteState::nuevo());
    app.input.reset();
    app.feedback = None;
    app.sugerencias.clear();
    if texto_inicial.trim().is_empty() {
        return;
    }
    if let Some(estado) = &mut app.salida_gafete {
        estado.asignar_texto(texto_inicial);
    }
    recomputar_coincidencias(core, app);
    confirmar_salida_gafete(core, app);
}

fn cerrar_salida_gafete(core: &AppCore, app: &mut AppState) {
    app.salida_gafete = None;
    app.input.reset();
    super::recomputar(core, app);
}

pub(super) fn manejar_salida_gafete(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => cerrar_salida_gafete(core, app),
        KeyCode::Enter => confirmar_salida_gafete(core, app),
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some() {
                if let Some(estado) = &mut app.salida_gafete {
                    estado.asignar_texto(app.input.value());
                }
                let saneado = app
                    .salida_gafete
                    .as_ref()
                    .map(|e| e.texto.clone())
                    .unwrap_or_default();
                if saneado != app.input.value() {
                    app.input = Input::new(saneado);
                }
                recomputar_coincidencias(core, app);
            }
        }
    }
}

/// Una consulta por número (la tabla de ingresos activos no tiene un
/// "IN" en `FiltroIngresosActivos::gafete_numero`, y son pocos números a
/// la vez) — se recalcula tras cada tecla para que la vista previa siempre
/// refleje lo que hay tecleado, igual criterio que el resto de la app.
fn recomputar_coincidencias(core: &AppCore, app: &mut AppState) {
    let Some(estado) = &app.salida_gafete else {
        return;
    };
    let coincidencias: Vec<(i64, Option<IngresoActivoResumen>)> = estado
        .gafetes()
        .into_iter()
        .map(|numero| (numero, buscar_activo_por_gafete(core, numero)))
        .collect();
    if let Some(estado) = &mut app.salida_gafete {
        estado.coincidencias = coincidencias;
    }
}

fn buscar_activo_por_gafete(core: &AppCore, numero: i64) -> Option<IngresoActivoResumen> {
    let filtro = FiltroIngresosActivos {
        gafete_numero: Some(Igualdad::Incluye(numero)),
        limite: 1,
        ..FiltroIngresosActivos::default()
    };
    core.listar_ingresos_activos(&filtro)
        .ok()
        .and_then(|pagina| pagina.items.into_iter().next())
}

/// Registra la salida de cada gafete con coincidencia — uno o varios de
/// una sola vez (grupo que entra o sale junto). No es transaccional entre
/// sí: cada `registrar_salida` es su propia escritura, así que si uno
/// falla a mitad de la lista los anteriores ya quedaron guardados; el
/// resumen final dice cuáles sí y cuáles no.
fn confirmar_salida_gafete(core: &AppCore, app: &mut AppState) {
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    let Some(estado) = &app.salida_gafete else {
        return;
    };
    if estado.coincidencias.is_empty() {
        return;
    }

    let mut registrados: Vec<String> = Vec::new();
    let mut fallidos: Vec<String> = Vec::new();
    for (numero, item) in &estado.coincidencias {
        match item {
            None => fallidos.push(format!("gafete {numero}: sin ingreso activo")),
            Some(item) => match core.registrar_salida(sesion, item.registro_id) {
                Ok(()) => registrados.push(item.contratista_nombre.clone()),
                Err(error) => {
                    fallidos.push(format!("gafete {numero}: {}", mensaje_error_salida(&error)));
                }
            },
        }
    }

    if let Some(estado) = &mut app.salida_gafete {
        estado.limpiar_tras_confirmar();
    }
    app.input.reset();

    let (mensaje, nivel) = resumen_confirmacion(&registrados, &fallidos);
    app.mostrar_feedback(mensaje, nivel);
}

fn resumen_confirmacion(registrados: &[String], fallidos: &[String]) -> (String, NivelFeedback) {
    match (registrados.is_empty(), fallidos.is_empty()) {
        (true, true) => (String::new(), NivelFeedback::Advertencia),
        (false, true) => (
            format!("Salida registrada — {}", registrados.join(", ")),
            NivelFeedback::Exito,
        ),
        (true, false) => (fallidos.join(" · "), NivelFeedback::Error),
        (false, false) => (
            format!(
                "Salida registrada — {} · {}",
                registrados.join(", "),
                fallidos.join(" · ")
            ),
            NivelFeedback::Advertencia,
        ),
    }
}
