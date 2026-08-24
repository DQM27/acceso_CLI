//! Controlador de teclado del formulario de contratista (`--comandos`).
//!
//! `formulario.rs` es el modelo puro (campos, validación, navegación); este
//! archivo es el único que traduce teclas a esos métodos y decide cuándo
//! abrir, cerrar o persistir el formulario. Nada persiste hasta
//! `guardar_formulario`, disparado sólo desde la tarjeta de Resumen.

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::database::queries::contratistas::ContratistaResumen;
use crate::domain::autorizacion::Operacion;

use super::{
    AppState, Campo, Fase, FormularioContratista, MAX_VISIBLES_EMPRESAS, ModoFormulario,
    NivelFeedback, Subfase,
};

/// Abre el alta vacía con el catálogo de empresas y los permisos del actor.
pub(super) fn abrir_formulario_nuevo(core: &AppCore, app: &mut AppState) {
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    let empresas = core.listar_empresas().unwrap_or_default();
    let acceso_editable = sesion.rol.puede(Operacion::ActivarDesactivarContratista);
    app.formulario = Some(FormularioContratista::nuevo(empresas, acceso_editable));
    app.input.reset();
    app.feedback = None;
    app.sugerencias.clear();
}

/// Abre la edición precargada desde la coincidencia elegida — el resumen de
/// la búsqueda ya trae todos los campos del formulario, no hace falta otra
/// consulta.
pub(super) fn abrir_formulario_edicion(
    core: &AppCore,
    app: &mut AppState,
    resumen: &ContratistaResumen,
) {
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    let empresas = core.listar_empresas().unwrap_or_default();
    let cedula_editable = sesion.rol.puede(Operacion::EditarCedulaContratista);
    let acceso_editable = sesion.rol.puede(Operacion::ActivarDesactivarContratista);
    app.formulario = Some(FormularioContratista::editar(
        resumen,
        empresas,
        cedula_editable,
        acceso_editable,
    ));
    app.feedback = None;
    app.sugerencias.clear();
    sincronizar_input(app);
}

/// Cierra el formulario y devuelve el input a su papel de línea de comandos.
fn cerrar_formulario(core: &AppCore, app: &mut AppState) {
    app.formulario = None;
    app.input.reset();
    super::recomputar(core, app);
}

/// Vuelca el texto del campo activo al input (cursor al final) — tras cambiar
/// de campo, abrir la edición o salir del selector de empresa.
fn sincronizar_input(app: &mut AppState) {
    let texto = app
        .formulario
        .as_ref()
        .and_then(|form| form.texto_campo().map(str::to_string))
        .unwrap_or_default();
    app.input = Input::new(texto);
}

fn mover_campo_formulario(app: &mut AppState, delta: isize) {
    let cambio = match &mut app.formulario {
        Some(form) => form.mover_campo(delta),
        None => false,
    };
    if cambio {
        sincronizar_input(app);
    }
}

pub(super) fn manejar_formulario(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    let Some(form) = &app.formulario else {
        return;
    };
    match form.subfase {
        Subfase::Editando => manejar_edicion_formulario(core, app, key),
        Subfase::EligiendoEmpresa { .. } => manejar_selector_empresa(app, key),
        Subfase::Resumen => manejar_resumen_formulario(core, app, key),
    }
}

fn manejar_edicion_formulario(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    let Some(campo) = app.formulario.as_ref().map(|form| form.campo) else {
        return;
    };
    match key.code {
        KeyCode::Esc => cerrar_formulario(core, app),
        KeyCode::Up => mover_campo_formulario(app, -1),
        KeyCode::Down => mover_campo_formulario(app, 1),
        KeyCode::Enter => match campo {
            Campo::Empresa => {
                if let Some(form) = &mut app.formulario {
                    form.subfase = Subfase::EligiendoEmpresa { seleccion: 0 };
                }
                app.input.reset();
            }
            Campo::Confirmar => {
                // Con errores se queda editando (los ✗ ya se muestran junto
                // a cada campo); sin errores pasa a la tarjeta de resumen.
                if let Some(form) = &mut app.formulario
                    && form.validar().is_ok()
                {
                    form.subfase = Subfase::Resumen;
                    app.input.reset();
                }
            }
            // En el resto Enter simplemente avanza al siguiente campo.
            _ => mover_campo_formulario(app, 1),
        },
        // Space/←/→ cambian el valor de los campos no textuales; en campos de
        // texto Space es un carácter más y ←/→ mueven el cursor (tui_input).
        KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') if !campo.es_texto() => {
            if let Some(form) = &mut app.formulario {
                form.alternar();
            }
        }
        _ => {
            if campo.es_texto()
                && app.input.handle_event(&Event::Key(key)).is_some()
                && let Some(form) = &mut app.formulario
            {
                form.asignar_texto(app.input.value());
                // El saneado puede acortar el texto (fecha, largos máximos):
                // se refleja en el input sólo cuando difiere, para no
                // reventar el cursor en cada pulsación.
                let saneado = form.texto_campo().unwrap_or_default();
                if saneado != app.input.value() {
                    app.input = Input::new(saneado.to_string());
                }
            }
        }
    }
}

fn manejar_selector_empresa(app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if let Some(form) = &mut app.formulario {
                form.subfase = Subfase::Editando;
            }
            sincronizar_input(app);
        }
        KeyCode::Up | KeyCode::Down => {
            let delta: isize = if key.code == KeyCode::Up { -1 } else { 1 };
            let total = app
                .formulario
                .as_ref()
                .map(|form| {
                    form.empresas_filtradas(app.input.value())
                        .len()
                        .min(MAX_VISIBLES_EMPRESAS)
                })
                .unwrap_or(0);
            if let Some(form) = &mut app.formulario
                && let Subfase::EligiendoEmpresa { seleccion } = &mut form.subfase
                && total > 0
            {
                *seleccion = (*seleccion as isize + delta).clamp(0, total as isize - 1) as usize;
            }
        }
        KeyCode::Enter => {
            let elegida = app.formulario.as_ref().and_then(|form| match form.subfase {
                Subfase::EligiendoEmpresa { seleccion } => form
                    .empresas_filtradas(app.input.value())
                    .get(seleccion)
                    .map(|empresa| (empresa.id, empresa.nombre.clone())),
                _ => None,
            });
            if let Some((id, nombre)) = elegida {
                if let Some(form) = &mut app.formulario {
                    form.empresa = Some((id, nombre));
                    form.errores.retain(|(campo, _)| *campo != Campo::Empresa);
                    form.subfase = Subfase::Editando;
                }
                mover_campo_formulario(app, 1);
            }
        }
        _ => {
            // Filtrar reinicia la selección al primer resultado.
            if app.input.handle_event(&Event::Key(key)).is_some()
                && let Some(form) = &mut app.formulario
            {
                form.subfase = Subfase::EligiendoEmpresa { seleccion: 0 };
            }
        }
    }
}

fn manejar_resumen_formulario(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if let Some(form) = &mut app.formulario {
                form.subfase = Subfase::Editando;
            }
            sincronizar_input(app);
        }
        KeyCode::Enter => guardar_formulario(core, app),
        _ => {}
    }
}

/// Única escritura del formulario — como el resto de la interfaz, nada
/// persiste hasta la confirmación explícita en la tarjeta de resumen.
fn guardar_formulario(core: &AppCore, app: &mut AppState) {
    let Fase::Operando { sesion } = &app.fase else {
        return;
    };
    let Some(form) = &mut app.formulario else {
        return;
    };
    let modo = form.modo;
    let nombre = form.nombre.trim().to_string();
    let resultado = match modo {
        ModoFormulario::Nuevo => match form.validar() {
            Ok(datos) => core.crear_contratista(sesion, datos).map(|_| ()),
            // No debería pasar (el resumen sólo se abre tras validar), pero si
            // pasa se vuelve a editar con los errores marcados.
            Err(_) => {
                form.subfase = Subfase::Editando;
                return;
            }
        },
        ModoFormulario::Editar { id } => match form.datos_actualizacion() {
            Ok(datos) => core.actualizar_contratista(sesion, id, datos),
            Err(_) => {
                form.subfase = Subfase::Editando;
                return;
            }
        },
    };
    match resultado {
        Ok(()) => {
            let mensaje = match modo {
                ModoFormulario::Nuevo => format!("Contratista registrado — {nombre}"),
                ModoFormulario::Editar { .. } => format!("Cambios guardados — {nombre}"),
            };
            cerrar_formulario(core, app);
            app.mostrar_feedback(mensaje, NivelFeedback::Exito);
        }
        Err(error) => {
            if let Some(form) = &mut app.formulario {
                form.subfase = Subfase::Editando;
            }
            sincronizar_input(app);
            app.mostrar_feedback(error.to_string(), NivelFeedback::Error);
        }
    }
}
