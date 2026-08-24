//! Controlador de teclado del formulario de empresa (`--comandos`) — mismo
//! patrón que `formulario_controller.rs`, reducido: un solo campo, Enter
//! valida y guarda en el mismo paso (sin Resumen intermedio, ver
//! `formulario_empresa.rs`).

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::database::queries::empresas::EmpresaResumen;

use super::formulario_empresa::{FormularioEmpresa, ModoFormularioEmpresa};
use super::{AppState, Fase, NivelFeedback};

pub(super) fn abrir_formulario_nuevo_empresa(app: &mut AppState) {
    app.formulario_empresa = Some(FormularioEmpresa::nuevo());
    app.input.reset();
    app.feedback = None;
    app.sugerencias.clear();
}

/// La búsqueda que trajo hasta acá (`CoincidenciasEmpresas`) ya trae el
/// nombre vigente — precarga el campo y el input del prompt con él, igual
/// que `abrir_formulario_edicion` hace para contratista.
pub(super) fn abrir_formulario_editar_empresa(app: &mut AppState, resumen: &EmpresaResumen) {
    app.formulario_empresa = Some(FormularioEmpresa::editar(resumen.id, &resumen.nombre));
    app.input = Input::new(resumen.nombre.clone());
    app.feedback = None;
    app.sugerencias.clear();
}

fn cerrar_formulario_empresa(core: &AppCore, app: &mut AppState) {
    app.formulario_empresa = None;
    app.input.reset();
    super::recomputar(core, app);
}

pub(super) fn manejar_formulario_empresa(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => cerrar_formulario_empresa(core, app),
        KeyCode::Enter => guardar_formulario_empresa(core, app),
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some()
                && let Some(form) = &mut app.formulario_empresa
            {
                form.asignar_texto(app.input.value());
                let saneado = form.nombre.clone();
                if saneado != app.input.value() {
                    app.input = Input::new(saneado);
                }
            }
        }
    }
}

/// Única escritura de esta Surface — nada persiste hasta Enter, igual
/// criterio que el resto de la app.
fn guardar_formulario_empresa(core: &AppCore, app: &mut AppState) {
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    let Some(form) = &mut app.formulario_empresa else {
        return;
    };
    let nombre = match form.validar() {
        Ok(nombre) => nombre,
        Err(_) => return,
    };
    let modo = form.modo;
    let resultado = match modo {
        ModoFormularioEmpresa::Nuevo => core.crear_empresa(sesion, &nombre).map(|_| ()),
        ModoFormularioEmpresa::Editar { id } => core.actualizar_empresa(sesion, id, &nombre),
    };
    let mensaje_exito = match modo {
        ModoFormularioEmpresa::Nuevo => format!("Empresa registrada — {nombre}"),
        ModoFormularioEmpresa::Editar { .. } => format!("Empresa actualizada — {nombre}"),
    };
    match resultado {
        Ok(()) => {
            cerrar_formulario_empresa(core, app);
            app.mostrar_feedback(mensaje_exito, NivelFeedback::Exito);
        }
        Err(error) => {
            if let Some(form) = &mut app.formulario_empresa {
                form.error = Some(error.to_string());
            }
            app.mostrar_feedback(error.to_string(), NivelFeedback::Error);
        }
    }
}
