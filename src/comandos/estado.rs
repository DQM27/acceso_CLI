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

use super::columnas::{ColumnaActivos, ColumnaBusqueda, SelectorColumnas};
use super::formulario::FormularioContratista;
use super::presentation;

/// Sobre qué tabla actúa el selector de columnas (`F4`) — determina qué
/// `SelectorColumnas` de `AppState` edita y a qué contexto vuelve al cerrar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjetivoColumnas {
    Busqueda,
    Activos,
}

/// Selector de columnas abierto (Surface enclavada, §5.2): mientras es
/// `Some`, el teclado deja la gramática de comandos y pasa a la del picker
/// (↑↓ mueve, Space marca/desmarca, Esc cierra) — mismo mecanismo que ya usa
/// `formulario`, generalizado a una segunda Surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdicionColumnas {
    pub objetivo: ObjetivoColumnas,
    pub seleccion: usize,
}

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
///
/// Los errores de login (usuario no válido, credenciales inválidas) ya no
/// viajan como campo de la fase: usan el mismo `feedback` transitorio que el
/// resto de la aplicación — una sola gramática de aviso (`✓ ! ×`) en vez de
/// una variante propia por pantalla.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fase {
    LoginCedula,
    /// `nombre` ya se resolvió contra SQLite al confirmar la cédula (lectura
    /// rápida, sin Argon2) — es lo que muta el título "Brisas CLI" en la
    /// identidad del operador durante el resto del login.
    LoginPassword {
        cedula: String,
        nombre: String,
    },
    /// Argon2 corriendo en un hilo aparte; el input queda bloqueado. Conserva
    /// `nombre` para que la identidad reconocida siga en pantalla mientras
    /// se verifica.
    Verificando {
        nombre: String,
    },
    Operando {
        sesion: UsuarioSesion,
    },
}

/// Qué tipo de prompt ocupa la fila de login — no el texto tecleado, sólo el
/// "tipo de contenido". Cambia una vez por transición de fase, nunca tecla a
/// tecla: por eso es justo lo que hace falta comparar para saber cuándo el
/// motor de presentación tiene que animar algo (ver `FirmaLogin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoPromptLogin {
    Cedula,
    Password,
    Verificando,
}

/// Resumen mínimo de "qué debería verse" en la escena de login — no el
/// estado completo, sólo lo que le importa al motor de presentación para
/// decidir si algo mutó de contenido (título, tipo de prompt, aparición de
/// aviso) y arrancar una aparición. Comparar dos `FirmaLogin` nunca da un
/// falso positivo por typing: el texto tecleado no forma parte de la firma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmaLogin {
    pub titulo: String,
    pub prompt: TipoPromptLogin,
    pub feedback: bool,
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
    /// Columnas visibles de cada tabla — `F4` abre `edicion_columnas` sobre
    /// una de las dos (según `app.contexto` en ese momento).
    pub columnas_busqueda: SelectorColumnas<ColumnaBusqueda>,
    pub columnas_activos: SelectorColumnas<ColumnaActivos>,
    /// Selector de columnas abierto, si lo hay (ver `EdicionColumnas`).
    pub edicion_columnas: Option<EdicionColumnas>,
    /// Pistas de la línea de sugerencias (autocompletado contextual, teclas).
    pub sugerencias: Vec<String>,
    pub feedback: Option<(Instant, Feedback)>,
    pub salir: bool,
    /// Motor de presentación (Fase 4): sabe qué está apareciendo y a qué
    /// opacidad, nada más — no conoce reglas de negocio.
    pub presentacion: presentation::Engine,
    pub calidad: presentation::VisualQuality,
    /// Última `FirmaLogin` vista, para que el loop detecte mutaciones de
    /// contenido comparándola contra la actual (ver `firma_login`).
    pub firma_login_previa: Option<FirmaLogin>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            fase: Fase::LoginCedula,
            contexto: ContextState::Ayuda,
            formulario: None,
            columnas_busqueda: SelectorColumnas::todas_visibles(),
            columnas_activos: SelectorColumnas::todas_visibles(),
            edicion_columnas: None,
            sugerencias: Vec::new(),
            feedback: None,
            salir: false,
            presentacion: presentation::Engine::new(),
            calidad: presentation::VisualQuality::default(),
            firma_login_previa: None,
        }
    }

    /// `None` fuera de las fases de login: `Operando` todavía no tiene
    /// ninguna mutación animada (llega en una fase posterior, sobre esta
    /// misma base).
    pub fn firma_login(&self) -> Option<FirmaLogin> {
        let (titulo, prompt) = match &self.fase {
            Fase::LoginCedula => (super::NOMBRE_APP.to_string(), TipoPromptLogin::Cedula),
            Fase::LoginPassword { nombre, .. } => (nombre.clone(), TipoPromptLogin::Password),
            Fase::Verificando { nombre } => (nombre.clone(), TipoPromptLogin::Verificando),
            Fase::Operando { .. } => return None,
        };
        Some(FirmaLogin {
            titulo,
            prompt,
            feedback: self.feedback.is_some(),
        })
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

    /// El feedback transitorio expira solo. Devuelve `true` si acaba de
    /// expirar (el llamador lo usa para saber si hace falta redibujar).
    pub fn expirar_feedback(&mut self) -> bool {
        let expiro = matches!(&self.feedback, Some((instante, _)) if instante.elapsed() >= DURACION_FEEDBACK);
        if expiro {
            self.feedback = None;
        }
        expiro
    }

    /// Tiempo que falta para que el feedback vigente expire por sí solo, si
    /// hay uno. Lo usa el scheduler del loop para no despertar antes de tiempo.
    pub fn feedback_restante(&self) -> Option<std::time::Duration> {
        let (instante, _) = self.feedback.as_ref()?;
        Some(DURACION_FEEDBACK.saturating_sub(instante.elapsed()))
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
