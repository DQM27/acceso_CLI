//! Estado de la interfaz `--comandos`.
//!
//! No hay un enum de pantallas: el [`ContextState`] se **deriva** del input en
//! cada cambio (input → `parser::parsear` → `resolver::resolver` → contexto).
//! Las flechas sólo mueven la selección dentro del contexto vigente y Enter lo
//! confirma; nunca se "navega" a otro estado por cuenta propia.

use std::time::Instant;

use tui_input::Input;

use crate::database::queries::contratistas::ContratistaResumen;
use crate::models::medio_ingreso::MedioIngreso;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::registro_ingreso_service::{IngresoActivoResumen, PreparacionIngreso};

use super::formulario::FormularioContratista;

/// Cuánto permanece visible un mensaje de feedback transitorio (también
/// desaparece en cuanto el operador vuelve a escribir).
pub const DURACION_FEEDBACK: std::time::Duration = std::time::Duration::from_secs(4);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NivelFeedback {
    Exito,
    Advertencia,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feedback {
    pub texto: String,
    pub nivel: NivelFeedback,
}

/// Fase global de la interfaz: primero el login (cédula y contraseña en el
/// mismo input), luego la operación normal dirigida por comandos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fase {
    LoginCedula {
        error: Option<String>,
    },
    LoginPassword {
        cedula: String,
        error: Option<String>,
    },
    /// Argon2 corriendo en un hilo aparte; el input queda bloqueado.
    Verificando,
    Operando {
        sesion: UsuarioSesion,
    },
}

/// Lo que ocupa el área contextual en este momento. Se reconstruye entero cada
/// vez que cambia el input; la selección (`seleccion`) es el único estado que
/// sobrevive entre reconstrucciones, y lo hace dentro de la propia variante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextState {
    /// Input vacío: título, conteo de personas dentro y comandos disponibles.
    Inicio {
        total_dentro: usize,
    },
    /// Coincidencias de contratistas para `/ingreso` o una búsqueda de texto
    /// libre. Con la consulta demasiado corta o sin resultados, `items` queda
    /// vacío y el render muestra la pista correspondiente a partir de
    /// `consulta`.
    Coincidencias {
        consulta: String,
        items: Vec<ContratistaResumen>,
        seleccion: usize,
    },
    /// Coincidencias de ingresos activos para `/salida`. `descripcion` es la
    /// consulta ya formateada para el mensaje "No hay ingreso activo para …"
    /// (p. ej. `"carlos"` o `gafete 27`).
    CoincidenciasActivos {
        descripcion: String,
        items: Vec<IngresoActivoResumen>,
        seleccion: usize,
    },
    /// Tarjeta de validación previa al ingreso. `gafete_ocupante` es el
    /// ingreso activo que hoy tiene el gafete pedido, si existe.
    ResumenIngreso {
        preparacion: PreparacionIngreso,
        gafete: Option<i64>,
        medio: MedioIngreso,
        gafete_ocupante: Option<IngresoActivoResumen>,
    },
    /// Tarjeta de confirmación de salida sobre un ingreso activo concreto.
    ResumenSalida {
        activo: IngresoActivoResumen,
    },
    /// `/activos`: tabla de personas dentro ahora mismo.
    TablaActivos {
        items: Vec<IngresoActivoResumen>,
        total: usize,
    },
    /// Búsqueda de texto libre resuelta a un contratista concreto.
    FichaContratista {
        resumen: ContratistaResumen,
    },
    /// `/cerrarsesion`: tarjeta de confirmación — Enter cierra la sesión y
    /// vuelve al login, Esc cancela.
    ConfirmarCerrarSesion,
    /// `/nuevo`: tarjeta de entrada al alta — Enter abre el formulario de
    /// contratista, Esc cancela.
    NuevoContratista,
    Ayuda,
    /// Comando desconocido, parámetro inválido o error de consulta: se muestra
    /// el mensaje con `✗` y la sugerencia de `/ayuda` cuando aplica.
    MensajeError {
        mensaje: String,
    },
}

impl ContextState {
    /// El resumen de ingreso sólo puede confirmarse con Enter cuando ningún
    /// chequeo está en ✗: acceso no denegado, sin ingreso activo previo y
    /// gafete presente y libre cuando el contratista lo requiere.
    pub fn ingreso_confirmable(&self) -> bool {
        let ContextState::ResumenIngreso {
            preparacion,
            gafete,
            gafete_ocupante,
            ..
        } = self
        else {
            return false;
        };
        use crate::domain::resultado_acceso::ResultadoAcceso;
        if matches!(preparacion.resultado_acceso, ResultadoAcceso::Denegado(_))
            || preparacion.tiene_ingreso_activo
        {
            return false;
        }
        !preparacion.requiere_gafete || (gafete.is_some() && gafete_ocupante.is_none())
    }
}

/// Estado raíz de la interfaz de comandos.
pub struct AppState {
    pub input: Input,
    pub fase: Fase,
    pub contexto: ContextState,
    /// Formulario de alta/edición de contratista. Mientras es `Some`, el input
    /// deja de ser línea de comandos y edita el campo activo del formulario —
    /// `recomputar` no toca el contexto hasta que el formulario se cierra.
    pub formulario: Option<FormularioContratista>,
    /// Pistas de la línea de sugerencias (autocompletado contextual, teclas).
    pub sugerencias: Vec<String>,
    pub feedback: Option<(Instant, Feedback)>,
    pub salir: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            fase: Fase::LoginCedula { error: None },
            contexto: ContextState::Ayuda,
            formulario: None,
            sugerencias: Vec::new(),
            feedback: None,
            salir: false,
        }
    }

    /// Arranca ya autenticado — para cuando la TUI clásica hizo el login y el
    /// operador eligió el modo CLI desde `ElegirInterfaz`, sin volver a pedir
    /// cédula/contraseña.
    pub fn con_sesion(sesion: UsuarioSesion) -> Self {
        Self {
            fase: Fase::Operando { sesion },
            ..Self::new()
        }
    }

    pub fn mostrar_feedback(&mut self, texto: String, nivel: NivelFeedback) {
        self.feedback = Some((Instant::now(), Feedback { texto, nivel }));
    }

    /// El feedback transitorio expira solo; se revisa en cada vuelta del bucle
    /// (que ya hace poll con timeout corto por el cursor y los hilos).
    pub fn expirar_feedback(&mut self) {
        if let Some((instante, _)) = &self.feedback
            && instante.elapsed() >= DURACION_FEEDBACK
        {
            self.feedback = None;
        }
    }

    pub fn feedback_vigente(&self) -> Option<&Feedback> {
        match &self.feedback {
            Some((instante, feedback)) if instante.elapsed() < DURACION_FEEDBACK => Some(feedback),
            _ => None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
