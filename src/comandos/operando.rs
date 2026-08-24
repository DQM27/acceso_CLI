//! Controlador de la fase `Operando`: interpreta comandos y confirmaciones
//! sobre el contexto vigente. Las únicas escrituras a SQLite de este archivo
//! son `registrar_ingreso`/`registrar_salida` — nada de lo que se muestra
//! mientras se teclea persiste nada; ver la sección 7 de
//! `docs/radiografia-dominio-comandos.md`.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::application::AppCore;
use crate::models::medio_ingreso::MedioIngreso;
use crate::services::error::RegistroIngresoServiceError;
use crate::tiempo::hora_actual_texto;

use super::formulario_controller::{abrir_formulario_edicion, abrir_formulario_nuevo};
use super::{
    AppState, Comando, ContextState, Entrada, Fase, GafeteParse, MedioParse, NivelFeedback,
};

pub(super) fn manejar_operando(core: &AppCore, app: &mut AppState, key: KeyEvent) {
    // Con el formulario abierto todas las teclas (salvo Ctrl+C, ya atajada)
    // son del formulario: el input edita campos, no comandos.
    if app.formulario.is_some() {
        super::formulario_controller::manejar_formulario(core, app, key);
        return;
    }
    match key.code {
        // Esc y Ctrl+L: limpiar todo y volver a Inicio.
        KeyCode::Esc => {
            app.input.reset();
            app.feedback = None;
            super::recomputar(core, app);
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input.reset();
            app.feedback = None;
            super::recomputar(core, app);
        }
        KeyCode::Up => mover_seleccion(app, -1),
        KeyCode::Down => mover_seleccion(app, 1),
        KeyCode::Tab => {
            if let Some(nuevo) = super::resolver::autocompletar(core, app.input.value()) {
                app.input = Input::new(nuevo);
                super::recomputar(core, app);
            }
        }
        KeyCode::Enter => confirmar(core, app),
        _ => {
            if app.input.handle_event(&Event::Key(key)).is_some() {
                // Escribir de nuevo despeja el feedback transitorio.
                app.feedback = None;
                super::recomputar(core, app);
            }
        }
    }
}

fn mover_seleccion(app: &mut AppState, delta: isize) {
    let ajustar = |seleccion: &mut usize, total: usize| {
        if total == 0 {
            return;
        }
        let actual = *seleccion as isize;
        *seleccion = (actual + delta).clamp(0, total as isize - 1) as usize;
    };
    match &mut app.contexto {
        ContextState::Coincidencias {
            items, seleccion, ..
        } => ajustar(seleccion, items.len()),
        ContextState::CoincidenciasActivos {
            items, seleccion, ..
        } => ajustar(seleccion, items.len()),
        _ => {}
    }
}

/// Enter: selecciona la coincidencia marcada o confirma la tarjeta vigente.
/// Las escrituras reales (`registrar_ingreso`/`registrar_salida`) sólo ocurren
/// aquí — nada de lo que se muestra mientras se teclea persiste nada.
fn confirmar(core: &AppCore, app: &mut AppState) {
    let entrada = super::parser::parsear(app.input.value());
    match app.contexto.clone() {
        ContextState::Coincidencias {
            items, seleccion, ..
        } => {
            let Some(item) = items.get(seleccion) else {
                return;
            };
            let comando = match &entrada {
                Entrada::Comando { comando, .. } => Some(*comando),
                _ => None,
            };
            match comando {
                Some(Comando::Ingreso) => {
                    let (gafete, medio) = parametros_ingreso(&entrada);
                    app.contexto =
                        super::resolver::preparar_resumen_ingreso(core, item.id, gafete, medio);
                }
                Some(Comando::Editar) => abrir_formulario_edicion(core, app, item),
                // El texto libre y cualquier otro comando abren la ficha.
                _ => app.contexto = super::resolver::ficha_desde_resumen(item.clone()),
            }
        }
        ContextState::CoincidenciasActivos {
            items, seleccion, ..
        } => {
            if let Some(item) = items.get(seleccion) {
                app.contexto = ContextState::ResumenSalida {
                    activo: item.clone(),
                };
            }
        }
        ContextState::ResumenIngreso {
            preparacion,
            gafete,
            medio,
            ..
        } => {
            if !app.contexto.ingreso_confirmable() {
                return;
            }
            let Fase::Operando { sesion } = &app.fase else {
                return;
            };
            match core.registrar_ingreso(sesion, preparacion.contratista_id, medio, gafete) {
                Ok(_) => {
                    let gafete_texto = gafete
                        .map(|numero| format!(" — Gafete {numero}"))
                        .unwrap_or_default();
                    app.mostrar_feedback(
                        format!(
                            "Ingreso registrado — {}{gafete_texto} — {}",
                            preparacion.nombre,
                            hora_actual_texto()
                        ),
                        NivelFeedback::Exito,
                    );
                    app.input.reset();
                    super::recomputar(core, app);
                }
                Err(error) => {
                    app.mostrar_feedback(mensaje_error_ingreso(&error), NivelFeedback::Error);
                }
            }
        }
        ContextState::NuevoContratista => abrir_formulario_nuevo(core, app),
        ContextState::ConfirmarCerrarSesion => {
            app.input.reset();
            app.feedback = None;
            app.fase = Fase::LoginCedula;
            app.contexto = ContextState::Ayuda;
            app.sugerencias.clear();
            app.mostrar_feedback("Sesión cerrada".to_string(), NivelFeedback::Exito);
        }
        ContextState::ResumenSalida { activo } => {
            let Fase::Operando { sesion } = &app.fase else {
                return;
            };
            match core.registrar_salida(sesion, activo.registro_id) {
                Ok(()) => {
                    let detalle = activo
                        .gafete_numero
                        .map(|numero| format!(" — Gafete {numero} liberado"))
                        .unwrap_or_default();
                    app.mostrar_feedback(
                        format!("Salida registrada — {}{detalle}", activo.contratista_nombre),
                        NivelFeedback::Exito,
                    );
                    app.input.reset();
                    super::recomputar(core, app);
                }
                Err(error) => {
                    app.mostrar_feedback(mensaje_error_salida(&error), NivelFeedback::Error);
                }
            }
        }
        _ => {}
    }
}

/// Extrae gafete y medio del parseo con los valores ya validados (los inválidos
/// nunca llegan acá: el resolver los convierte en `MensajeError` antes).
fn parametros_ingreso(entrada: &Entrada) -> (Option<i64>, MedioIngreso) {
    match entrada {
        Entrada::Comando { gafete, medio, .. } => {
            let gafete = match gafete {
                Some(GafeteParse::Valido(numero)) => Some(*numero),
                _ => None,
            };
            let medio = match medio {
                Some(MedioParse::Valido(medio)) => *medio,
                _ => MedioIngreso::Caminando,
            };
            (gafete, medio)
        }
        _ => (None, MedioIngreso::Caminando),
    }
}

/// Mensajes operativos en español, mismo criterio que
/// `tui::app::error_messages` (que es privado de la TUI clásica): los errores
/// semánticos conservan su texto accionable y los de base de datos no exponen
/// detalles internos.
fn mensaje_error_ingreso(error: &RegistroIngresoServiceError) -> String {
    use RegistroIngresoServiceError::*;
    match error {
        ContratistaNoEncontrado => "El contratista ya no existe".into(),
        IngresoActivo => "El contratista ya tiene un ingreso activo".into(),
        GafeteRequerido => "El gafete es requerido".into(),
        GafeteOcupado => "El gafete ya está en uso".into(),
        AccesoDenegado(_) => format!("Acceso denegado: {}", motivo_texto(error)),
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        _ => "No se pudo registrar el ingreso".into(),
    }
}

fn motivo_texto(error: &RegistroIngresoServiceError) -> String {
    use crate::domain::resultado_acceso::MotivoDenegacion;
    match error {
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::SinAcceso) => {
            "no tiene acceso autorizado".into()
        }
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::PraindVencido) => {
            "PRAIND vencido".into()
        }
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::PraindNoRegistrado) => {
            "PRAIND sin fecha registrada".into()
        }
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::EmpresaInactiva) => {
            "la empresa está inactiva".into()
        }
        _ => String::new(),
    }
}

fn mensaje_error_salida(error: &RegistroIngresoServiceError) -> String {
    use RegistroIngresoServiceError::*;
    match error {
        RegistroNoActivo => "El ingreso ya no está activo".into(),
        SalidaAnteriorAIngreso => "La salida no puede ser anterior al ingreso".into(),
        RelojRetrocedido => "Revise la fecha y hora del equipo antes de continuar".into(),
        _ => "No se pudo registrar la salida".into(),
    }
}
