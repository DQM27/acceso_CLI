//! Estado de la interfaz `--comandos`.
//!
//! No hay un enum de pantallas: el [`ContextState`] se **deriva** del input en
//! cada cambio (input → `parser::parsear` → `resolver::resolver` → contexto).
//! Las flechas sólo mueven la selección dentro del contexto vigente y Enter lo
//! confirma; nunca se "navega" a otro estado por cuenta propia.

use std::time::Instant;

use tui_input::Input;

use crate::lenguaje_comandos::Comando;
use crate::services::autenticacion_service::UsuarioSesion;

use super::columnas::{ColumnaActivos, ColumnaBusqueda, ColumnaHistorial, SelectorColumnas};
use super::formulario::{Campo, FormularioContratista, Subfase};

/// Ritmo del parpadeo del cursor propio del prompt (`render::blink_on`) —
/// 530ms es el valor por defecto de Windows Terminal/la mayoría de
/// emuladores, así que el nuestro se siente igual de "normal" sin depender
/// del cursor real del terminal. Compartida con `mod.rs` (que la usa para
/// saber cuánto puede esperar el próximo `poll` sin perderse un toggle).
pub(crate) const PERIODO_BLINK_MS: u64 = 530;
use super::formulario_empresa::FormularioEmpresa;
use super::formulario_password::FormularioPassword;
use super::formulario_usuario::FormularioUsuario;
use super::historial::HistorialState;
use super::presentation;
use super::salida_gafete::SalidaGafeteState;

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
    FormularioPassword,
    Columnas,
    Historial,
    SalidaGafete,
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
/// mismo input), luego la operación normal dirigida por comandos. Con la
/// base recién creada (sin ningún usuario todavía), el login se reemplaza
/// por la cadena `Root*` — misma mecánica de un solo input que muta de campo
/// en campo, con los dos campos extra (nombre, confirmar contraseña) que
/// hacen falta para dar de alta la primera cuenta.
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
    /// Configuración inicial — primer campo (cédula del ROOT). Sólo se
    /// alcanza cuando `requiere_configuracion_inicial()` da true.
    RootCedula,
    RootNombre {
        cedula: String,
    },
    RootPassword {
        cedula: String,
        nombre: String,
    },
    RootConfirmarPassword {
        cedula: String,
        nombre: String,
        password: String,
    },
    /// Hasheando la contraseña en un hilo aparte antes del insert atómico —
    /// misma idea que `Verificando`, sobre el alta en vez de la entrada.
    RootCreando {
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
    RootCedula,
    RootNombre,
    RootPassword,
    RootConfirmarPassword,
    RootCreando,
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

/// Movido a `lenguaje_comandos::contexto` (sin dependencia de terminal) para
/// que la GUI pueda reusar el lenguaje de comandos sin arrastrar
/// `ratatui`/`crossterm`/`tui-input` — re-exportado acá para no romper a
/// quien lo importaba como `comandos::estado::ContextState` o
/// `comandos::ContextState`.
pub use crate::lenguaje_comandos::ContextState;

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
    /// `/clave` — Surface separada de `formulario_usuario`: cambia la
    /// contraseña de quien está logueado, nunca la de otro (ver
    /// `formulario_password.rs`).
    pub formulario_password: Option<FormularioPassword>,
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
    /// Modo enclavado de salida por gafete abierto, si lo hay (DEC-057) —
    /// a diferencia de las demás Surfaces no se cierra sola tras confirmar,
    /// pensado para uso repetido (gafete tras gafete).
    pub salida_gafete: Option<SalidaGafeteState>,
    /// Fila resaltada de la paleta de comandos (`paleta_comandos`) mientras
    /// se escribe `/algo` — ↑↓ la mueve, Tab/Enter completan con la
    /// seleccionada en vez de la primera alfabética. Se reinicia a 0 cada
    /// vez que el texto cambia (la lista filtrada ya no es la misma).
    pub seleccion_paleta: usize,
    /// Pistas de la línea de sugerencias (autocompletado contextual, teclas).
    pub sugerencias: Vec<String>,
    pub feedback: Option<(Instant, Feedback)>,
    pub salir: bool,
    /// `/clasico` (ver `formulario` correspondiente en `operando.rs`): pide
    /// salir para que `mod.rs::run` avise a `main.rs` que hay que reiniciar
    /// en la TUI clásica — la preferencia (`interfaz_preferida::guardar`) ya
    /// se guarda al confirmar, antes de llegar a este flag.
    pub reiniciar_en_clasica: bool,
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
    /// Ídem para el área de contexto (DEC-059) — ver `firma_contexto`.
    pub firma_contexto_previa: Option<std::mem::Discriminant<ContextState>>,
    /// Si el glifo del prompt (el `> ` del inicio de línea, sólo con la
    /// línea de comandos vacía de Surface) mostraba el símbolo de feedback
    /// en el frame anterior — DEC-060. `true`→`false` (dejó de mostrarse)
    /// no funde, sólo `false`→`true` (acaba de aparecer), mismo criterio
    /// asimétrico que el resto de apariciones de esta fase.
    pub prompt_glifo_previo: bool,
    /// Referencia de tiempo para el parpadeo del cursor del prompt (ver
    /// `render::blink_on`) — un solo `Instant` fijado al arrancar, nunca se
    /// reinicia con la tecla ni con el cambio de Surface, así que el
    /// parpadeo sigue su propio ritmo constante en vez de "reaparecer" fijo
    /// cada vez que se abre una Surface distinta.
    pub instante_inicio: Instant,
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
            formulario_password: None,
            columnas_busqueda: SelectorColumnas::todas_visibles(),
            columnas_activos: SelectorColumnas::todas_visibles(),
            columnas_historial: SelectorColumnas::todas_visibles(),
            edicion_columnas: None,
            historial: None,
            salida_gafete: None,
            seleccion_paleta: 0,
            sugerencias: Vec::new(),
            feedback: None,
            salir: false,
            reiniciar_en_clasica: false,
            presentacion: presentation::Engine::new(),
            calidad: presentation::VisualQuality::default(),
            firma_login_previa: None,
            firma_formulario_previa: None,
            firma_historial_previa: None,
            firma_contexto_previa: None,
            prompt_glifo_previo: false,
            instante_inicio: Instant::now(),
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
            Fase::RootCedula => (super::NOMBRE_APP.to_string(), TipoPromptLogin::RootCedula),
            Fase::RootNombre { .. } => (super::NOMBRE_APP.to_string(), TipoPromptLogin::RootNombre),
            Fase::RootPassword { nombre, .. } => (nombre.clone(), TipoPromptLogin::RootPassword),
            Fase::RootConfirmarPassword { nombre, .. } => {
                (nombre.clone(), TipoPromptLogin::RootConfirmarPassword)
            }
            Fase::RootCreando { nombre } => (nombre.clone(), TipoPromptLogin::RootCreando),
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

    /// Firma del área de contexto (DEC-059): a diferencia de
    /// `firma_formulario`/`firma_historial`, no hay un struct dedicado con
    /// los campos que importan — `ContextState` tiene más de 15 variantes
    /// y comparar el valor completo dispararía una aparición en cada tecla
    /// (cambia `items`/`consulta` todo el tiempo). El discriminante de la
    /// variante alcanza: sólo importa "cambió de tipo de pantalla"
    /// (Inicio → resultados, resultados → tarjeta de confirmación…), no
    /// qué trae adentro mientras se sigue en la misma.
    pub fn firma_contexto(&self) -> std::mem::Discriminant<ContextState> {
        std::mem::discriminant(&self.contexto)
    }

    /// Arranca ya autenticado. El cambio desde la TUI clásica hoy relanza el
    /// proceso y vuelve a pedir login, pero esta entrada sigue útil para
    /// pruebas o integraciones futuras.
    pub fn con_sesion(sesion: UsuarioSesion) -> Self {
        Self {
            fase: Fase::Operando { sesion },
            ..Self::new()
        }
    }

    /// Arranca en la cadena de configuración inicial (`Fase::RootCedula`) en
    /// vez del login — para cuando la base todavía no tiene ningún usuario.
    pub fn nueva_configuracion_inicial() -> Self {
        Self {
            fase: Fase::RootCedula,
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
        } else if self.formulario_password.is_some() {
            SurfaceActiva::FormularioPassword
        } else if self.edicion_columnas.is_some() {
            SurfaceActiva::Columnas
        } else if self.historial.is_some() {
            SurfaceActiva::Historial
        } else if self.salida_gafete.is_some() {
            SurfaceActiva::SalidaGafete
        } else {
            SurfaceActiva::Ninguna
        }
    }

    /// Comandos cuyo nombre empieza con lo tecleado tras la `/` — `Some`
    /// sólo mientras se está escribiendo el nombre del comando en sí
    /// (`/algo`, sin espacio todavía) y ninguna Surface tiene el teclado.
    /// Único punto de verdad de "qué muestra la paleta ahora": lo usan
    /// tanto el render (qué lista pintar) como `operando.rs` (qué mueve
    /// ↑↓, qué completan Tab/Enter) — antes vivía sólo en `render.rs` y
    /// por eso ↑↓/Enter no podían usarlo.
    pub fn paleta_comandos(&self) -> Option<Vec<Comando>> {
        if !matches!(self.fase, Fase::Operando { .. })
            || self.surface_activa() != SurfaceActiva::Ninguna
        {
            return None;
        }
        let texto = self.input.value();
        if !texto.starts_with('/') || texto.contains(' ') {
            return None;
        }
        let prefijo = texto[1..].to_lowercase();
        let coincidentes: Vec<Comando> = Comando::TODOS
            .into_iter()
            .filter(|comando| comando.nombre().starts_with(&prefijo))
            .collect();
        (!coincidentes.is_empty()).then_some(coincidentes)
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
    fn paleta_vacia_sin_barra_inicial() {
        let mut app = AppState::con_sesion(sesion());
        app.input = Input::new("gafete".to_string());
        assert_eq!(app.paleta_comandos(), None);
    }

    #[test]
    fn paleta_filtra_por_prefijo() {
        let mut app = AppState::con_sesion(sesion());
        app.input = Input::new("/g".to_string());
        assert_eq!(app.paleta_comandos(), Some(vec![Comando::Gafete]));
    }

    #[test]
    fn paleta_desaparece_tras_el_primer_espacio() {
        let mut app = AppState::con_sesion(sesion());
        app.input = Input::new("/gafete ".to_string());
        assert_eq!(app.paleta_comandos(), None);
    }

    #[test]
    fn paleta_ausente_con_una_surface_abierta() {
        let mut app = AppState::con_sesion(sesion());
        app.input = Input::new("/g".to_string());
        app.historial = Some(super::super::historial::HistorialState::nuevo(Vec::new()));
        assert_eq!(app.paleta_comandos(), None);
    }

    #[test]
    fn firma_contexto_distingue_variantes_distintas() {
        let mut app = AppState::con_sesion(sesion());
        app.contexto = ContextState::Inicio { total_dentro: 0 };
        let inicio = app.firma_contexto();
        app.contexto = ContextState::Ayuda;
        let ayuda = app.firma_contexto();
        assert_ne!(inicio, ayuda);
    }

    #[test]
    fn firma_contexto_ignora_los_datos_dentro_de_la_misma_variante() {
        let mut app = AppState::con_sesion(sesion());
        app.contexto = ContextState::Inicio { total_dentro: 0 };
        let antes = app.firma_contexto();
        // Mismo tipo de pantalla (Inicio), sólo cambia el conteo — no debe
        // verse como una pantalla distinta (evita fundir en cada tecla).
        app.contexto = ContextState::Inicio { total_dentro: 5 };
        assert_eq!(antes, app.firma_contexto());
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
