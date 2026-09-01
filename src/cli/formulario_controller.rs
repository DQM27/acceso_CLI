//! Controlador de teclado del formulario de contratista (`--cli`).
//!
//! `formulario.rs` es el modelo puro (campos, validación, navegación); este
//! archivo es el único que traduce teclas a esos métodos y decide cuándo
//! abrir, cerrar o persistir el formulario. Nada persiste hasta
//! `guardar_formulario`, disparado sólo desde la tarjeta de Resumen.

use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::database::queries::contratistas::{ContratistaResumen, FiltroContratistas};
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

fn mover_campo_formulario(core: &AppCore, app: &mut AppState, delta: isize) {
    let campo_anterior = app.formulario.as_ref().map(|form| form.campo);
    let cambio = match &mut app.formulario {
        Some(form) => form.mover_campo(delta),
        None => false,
    };
    if cambio {
        // Al dejar Cédula (no en cada tecla, sólo al alejarse) se verifica
        // proactivamente si ya existe un contratista con esa cédula, en vez
        // de esperar al intento final de guardar — el operador lo ve antes
        // de llenar el resto del formulario.
        if campo_anterior == Some(Campo::Cedula) {
            verificar_cedula_duplicada(core, app);
        }
        sincronizar_input(app);
    }
}

/// Compara por igualdad exacta (la búsqueda de `AppCore` es difusa/parcial)
/// y, en modo edición, excluye al propio contratista que se está editando
/// — de lo contrario cualquier edición se marcaría duplicada contra sí
/// misma.
fn verificar_cedula_duplicada(core: &AppCore, app: &mut AppState) {
    let Some(form) = &app.formulario else {
        return;
    };
    let cedula = form.cedula.trim().to_string();
    if cedula.is_empty() {
        return;
    }
    let id_actual = match form.modo {
        ModoFormulario::Editar { id } => Some(id),
        ModoFormulario::Nuevo => None,
    };
    let filtro = FiltroContratistas {
        texto: Some(cedula.clone()),
        limite: 5,
        ..FiltroContratistas::default()
    };
    let Ok(pagina) = core.buscar_contratistas(&filtro) else {
        return;
    };
    let duplicado = pagina
        .items
        .iter()
        .any(|item| item.cedula == cedula && Some(item.id) != id_actual);
    if let Some(form) = &mut app.formulario {
        form.errores.retain(|(campo, _)| *campo != Campo::Cedula);
        if duplicado {
            form.errores.push((
                Campo::Cedula,
                "Ya existe un contratista con esta cédula".to_string(),
            ));
        }
    }
}

pub(super) fn manejar_formulario(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    let Some(form) = &app.formulario else {
        return;
    };
    match form.subfase {
        Subfase::Editando => manejar_edicion_formulario(core, app, key),
        Subfase::EligiendoEmpresa { .. } => manejar_selector_empresa(core, app, key),
        Subfase::Resumen => manejar_resumen_formulario(core, app, key),
    }
}

fn manejar_edicion_formulario(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    let Some(campo) = app.formulario.as_ref().map(|form| form.campo) else {
        return;
    };
    match key.code {
        KeyCode::Esc => cerrar_formulario(core, app),
        KeyCode::Up => mover_campo_formulario(core, app, -1),
        KeyCode::Down => mover_campo_formulario(core, app, 1),
        // Enter siempre intenta confirmar el formulario completo, sin
        // importar en qué campo esté el operador — mismo significado que en
        // el resto de la interfaz (§2 principio 6) y que la TUI clásica
        // (que intenta guardar desde cualquier campo, no sólo desde un
        // "botón" al final). Con errores se queda editando (los × ya se
        // muestran junto a cada campo); sin errores pasa al resumen.
        //
        // La cédula se reverifica acá también (no sólo al dejarla con
        // ↓): si el operador la escribe y confirma con Enter sin pasar por
        // otro campo, igual tiene que enterarse de un duplicado antes de
        // llegar al resumen.
        KeyCode::Enter => {
            verificar_cedula_duplicada(core, app);
            // `validar()` reemplaza `errores` entero (limpia o repone según
            // el resultado) — si se llamara igual acá borraría el error de
            // duplicado recién puesto. Con duplicado, ni se intenta: el
            // error ya quedó marcado y el operador tiene que corregirlo
            // primero.
            let duplicada = app
                .formulario
                .as_ref()
                .is_some_and(|form| form.error_de(Campo::Cedula).is_some());
            if duplicada {
                return;
            }
            if let Some(form) = &mut app.formulario
                && form.validar().is_ok()
            {
                form.subfase = Subfase::Resumen;
                app.input.reset();
            }
        }
        // Space/←/→ cambian el valor de los campos no textuales: abren el
        // selector de empresa o alternan tipo/booleanos. En campos de texto
        // Space es un carácter más y ←/→ mueven el cursor (tui_input).
        KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') if campo == Campo::Empresa => {
            if let Some(form) = &mut app.formulario {
                form.subfase = Subfase::EligiendoEmpresa { seleccion: 0 };
            }
            app.input.reset();
        }
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

fn manejar_selector_empresa(core: &AppCore, app: &mut AppState, key: KeyEvent) {
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
                let actual_isize = isize::try_from(*seleccion).unwrap_or(isize::MAX);
                let total_isize = isize::try_from(total).unwrap_or(isize::MAX);
                *seleccion =
                    usize::try_from((actual_isize + delta).clamp(0, total_isize - 1)).unwrap_or(0);
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
                mover_campo_formulario(core, app, 1);
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
