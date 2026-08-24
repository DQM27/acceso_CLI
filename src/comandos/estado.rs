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

use super::columnas::{ColumnaActivos, ColumnaBusqueda, ColumnaHistorial, SelectorColumnas};
use super::formulario::{Campo, FormularioContratista, Subfase};
use super::formulario_empresa::FormularioEmpresa;
use super::formulario_usuario::FormularioUsuario;
use super::historial::HistorialState;
use super::presentation;

/// Sobre qué tabla actúa el selector de columnas (`F4`) — determina qué
/// `SelectorColumnas` de `AppState` edita y a qué contexto vuelve al cerrar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjetivoColumnas {
    Busqueda,
    Activos,
    Historial,
}

/// Qué Surface (§4/§5.2) tiene el teclado ahora mismo — primer paso real de
/// Fase 3: antes de esto, `operando.rs` decidía lo mismo con tres `if
/// x.is_some() {...} else if y.is_some()...` encadenados, uno por Surface,
/// cada uno reinventando la misma pregunta. `surface_activa()` la hace una
/// sola vez y en un solo lugar; el orden de precedencia (formulario primero)
/// es el mismo que ya tenía el código, no cambia comportamiento.
///
/// No es (todavía) un trait ni una pila de objetos: cada Surface sigue
/// siendo su propio campo de `AppState` con su propio controlador — eso es
/// deliberado, no una limitación a resolver ahora. Reescribir formulario/
/// columnas/historial sobre una abstracción común de verdad (Fase 3
/// completa: Composer/Surface/Selector/Field/Notice/Summary como tipos) es
/// un rediseño mucho más grande, con riesgo real de romper tres funciones
/// que hoy trabajan bien — se hace aparte, no de paso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceActiva {
    Formulario,
    FormularioEmpresa,
    FormularioUsuario,
    Columnas,
    Historial,
    Ninguna,
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

/// Mismo rol que `FirmaLogin`, para la Surface del formulario (Fase 5): qué
/// campo está activo, si se entró al selector de empresa o al resumen, y si
/// hay algún error — nunca el texto tecleado en los campos, así comparar dos
/// `FirmaFormulario` no dispara una aparición por cada tecla.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmaFormulario {
    pub campo: Campo,
    pub en_selector_empresa: bool,
    pub en_resumen: bool,
    pub tiene_error: bool,
}

/// Mismo rol que `FirmaLogin`, para la Surface de Historial (Fase 5): si hay
/// resultado aplicado, cuántas filas trae (una consulta nueva o una página
/// distinta cambia el total) y si se está exportando.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmaHistorial {
    pub tiene_resultado: bool,
    pub total: usize,
    pub exportando: bool,
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
    /// `/nuevo` (o `/nuevo contratista`/`/n c`): tarjeta de entrada al alta
    /// — Enter abre el formulario de contratista, Esc cancela.
    NuevoContratista,
    /// `/nuevo empresa` (`/n em`): tarjeta de entrada al alta de empresa.
    NuevoEmpresa,
    /// `/nuevo usuario` (`/n u`): tarjeta de entrada al alta de usuario.
    NuevoUsuario,
    /// `/historial`: tarjeta de entrada — Enter abre la Surface de
    /// Historial (§5.2/DEC-023/024), Esc cancela.
    AbrirHistorial,
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
    /// `/nuevo empresa` (`/n em`) — Surface separada de `formulario` (un
    /// solo campo, sin Resumen; ver `formulario_empresa.rs`).
    pub formulario_empresa: Option<FormularioEmpresa>,
    /// `/nuevo usuario` (`/n u`) — Surface separada de `formulario`, mismo
    /// patrón (campos, Resumen) que contratista; ver `formulario_usuario.rs`.
    pub formulario_usuario: Option<FormularioUsuario>,
    /// Columnas visibles de cada tabla — `F4` abre `edicion_columnas` sobre
    /// una de las dos (según `app.contexto` en ese momento).
    pub columnas_busqueda: SelectorColumnas<ColumnaBusqueda>,
    pub columnas_activos: SelectorColumnas<ColumnaActivos>,
    pub columnas_historial: SelectorColumnas<ColumnaHistorial>,
    /// Selector de columnas abierto, si lo hay (ver `EdicionColumnas`).
    pub edicion_columnas: Option<EdicionColumnas>,
    /// Surface de Historial abierta, si la hay (§5.2/DEC-023/024) — mientras
    /// es `Some`, el input deja de ser línea de comandos y edita el filtro
    /// clave:valor de Historial.
    pub historial: Option<HistorialState>,
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
    /// Ídem para el formulario y para Historial (Fase 5) — ver
    /// `firma_formulario`/`firma_historial`.
    pub firma_formulario_previa: Option<FirmaFormulario>,
    pub firma_historial_previa: Option<FirmaHistorial>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            fase: Fase::LoginCedula,
            contexto: ContextState::Ayuda,
            formulario: None,
            formulario_empresa: None,
            formulario_usuario: None,
            columnas_busqueda: SelectorColumnas::todas_visibles(),
            columnas_activos: SelectorColumnas::todas_visibles(),
            columnas_historial: SelectorColumnas::todas_visibles(),
            edicion_columnas: None,
            historial: None,
            sugerencias: Vec::new(),
            feedback: None,
            salir: false,
            presentacion: presentation::Engine::new(),
            calidad: presentation::VisualQuality::default(),
            firma_login_previa: None,
            firma_formulario_previa: None,
            firma_historial_previa: None,
        }
    }

    /// `None` fuera de las fases de login — `Operando` tiene sus propias
    /// firmas (`firma_formulario`/`firma_historial`), la misma idea sobre
    /// otra base (Fase 5).
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

    /// `None` con el formulario cerrado.
    pub fn firma_formulario(&self) -> Option<FirmaFormulario> {
        let formulario = self.formulario.as_ref()?;
        Some(FirmaFormulario {
            campo: formulario.campo,
            en_selector_empresa: matches!(formulario.subfase, Subfase::EligiendoEmpresa { .. }),
            en_resumen: matches!(formulario.subfase, Subfase::Resumen),
            tiene_error: !formulario.errores.is_empty(),
        })
    }

    /// `None` con Historial cerrado.
    pub fn firma_historial(&self) -> Option<FirmaHistorial> {
        let historial = self.historial.as_ref()?;
        Some(FirmaHistorial {
            tiene_resultado: historial.resultado.is_some(),
            total: historial.resultado.as_ref().map_or(0, |r| r.total),
            exportando: historial.exportacion_destino.is_some(),
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

    /// Ver `SurfaceActiva`. Mismo orden de precedencia que ya tenía
    /// `operando.rs` antes de unificarse acá — nunca hay dos Surfaces
    /// abiertas al mismo tiempo en la práctica, pero si algún día lo
    /// estuvieran, formulario gana.
    pub fn surface_activa(&self) -> SurfaceActiva {
        if self.formulario.is_some() {
            SurfaceActiva::Formulario
        } else if self.formulario_empresa.is_some() {
            SurfaceActiva::FormularioEmpresa
        } else if self.formulario_usuario.is_some() {
            SurfaceActiva::FormularioUsuario
        } else if self.edicion_columnas.is_some() {
            SurfaceActiva::Columnas
        } else if self.historial.is_some() {
            SurfaceActiva::Historial
        } else {
            SurfaceActiva::Ninguna
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::usuario::RolUsuario;
    use crate::services::autenticacion_service::UsuarioSesion;

    fn sesion() -> UsuarioSesion {
        UsuarioSesion {
            id: 1,
            cedula: "119430546".to_string(),
            nombre: "Operador de prueba".to_string(),
            rol: RolUsuario::Operador,
        }
    }

    #[test]
    fn sin_ninguna_surface_abierta() {
        let app = AppState::con_sesion(sesion());
        assert_eq!(app.surface_activa(), SurfaceActiva::Ninguna);
    }

    #[test]
    fn columnas_abierta_se_detecta() {
        let mut app = AppState::con_sesion(sesion());
        app.edicion_columnas = Some(EdicionColumnas {
            objetivo: ObjetivoColumnas::Activos,
            seleccion: 0,
        });
        assert_eq!(app.surface_activa(), SurfaceActiva::Columnas);
    }

    #[test]
    fn historial_abierto_se_detecta() {
        let mut app = AppState::con_sesion(sesion());
        app.historial = Some(super::super::historial::HistorialState::nuevo(Vec::new()));
        assert_eq!(app.surface_activa(), SurfaceActiva::Historial);
    }

    #[test]
    fn formulario_tiene_precedencia_sobre_las_demas() {
        let mut app = AppState::con_sesion(sesion());
        app.edicion_columnas = Some(EdicionColumnas {
            objetivo: ObjetivoColumnas::Activos,
            seleccion: 0,
        });
        app.historial = Some(super::super::historial::HistorialState::nuevo(Vec::new()));
        app.formulario = Some(super::super::formulario::FormularioContratista::nuevo(
            Vec::new(),
            true,
        ));
        assert_eq!(app.surface_activa(), SurfaceActiva::Formulario);
    }

    #[test]
    fn firma_formulario_es_none_cerrado_y_refleja_el_campo_activo_abierto() {
        let mut app = AppState::con_sesion(sesion());
        assert_eq!(app.firma_formulario(), None);

        app.formulario = Some(super::super::formulario::FormularioContratista::nuevo(
            Vec::new(),
            true,
        ));
        let firma = app.firma_formulario().expect("formulario abierto");
        assert_eq!(firma.campo, Campo::Cedula);
        assert!(!firma.en_resumen);
        assert!(!firma.en_selector_empresa);
        assert!(!firma.tiene_error);
    }

    #[test]
    fn firma_formulario_no_cambia_por_escribir_en_el_mismo_campo() {
        // Comparar dos firmas nunca debe dar falso positivo por tecleo — es
        // justo lo que evita animar en cada tecla (DEC-004).
        let mut app = AppState::con_sesion(sesion());
        app.formulario = Some(super::super::formulario::FormularioContratista::nuevo(
            Vec::new(),
            true,
        ));
        let antes = app.firma_formulario();
        if let Some(form) = &mut app.formulario {
            form.asignar_texto("119430546");
        }
        assert_eq!(app.firma_formulario(), antes);
    }

    #[test]
    fn firma_historial_es_none_cerrado_y_refleja_el_resultado_abierto() {
        let mut app = AppState::con_sesion(sesion());
        assert_eq!(app.firma_historial(), None);

        app.historial = Some(super::super::historial::HistorialState::nuevo(Vec::new()));
        let firma = app.firma_historial().expect("historial abierto");
        assert!(!firma.tiene_resultado);
        assert_eq!(firma.total, 0);
        assert!(!firma.exportando);
    }
}
