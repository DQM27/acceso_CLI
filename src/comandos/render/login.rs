//! Pantalla de login/root, alineada a la izquierda, pegada a la esquina
//! superior — la misma esquina que usa cualquier otra pantalla (Inicio,
//! Coincidencias...), no una escena aparte flotando a mitad de terminal: es
//! una interfaz de comandos, y el login es la acción que un operador repite
//! más veces por turno (entra y sale de sesión constantemente, ver
//! `/cerrarsesion`), así que no se paga ceremonia visual cada vez. Identidad,
//! foco y aviso se apoyan sólo en espaciado, alineación y la gramática de
//! glifos (● › ✓ ! ×). El cursor es un "_" con estilo, nunca el bloque
//! parpadeante del terminal — por eso esta función jamás llama a
//! `frame.set_cursor_position`.

use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::comandos::estado::{AppState, Fase, NivelFeedback};

use super::estilos::{
    estilo_fundido, glifo_feedback, FADE_ACENTO, FADE_ADVERTENCIA, FADE_ERROR, FADE_EXITO,
    FADE_MUTED, FADE_TEXTO,
};

pub(super) fn render_login(frame: &mut Frame, area: Rect, app: &AppState) {
    let opacidad_titulo = app.presentacion.opacidad("titulo");
    let opacidad_prompt = app.presentacion.opacidad("prompt");
    let opacidad_aviso = app.presentacion.opacidad("feedback");

    // Un renglón de aire entre bloques — pegado a la esquina superior
    // izquierda, la misma que usa cualquier otra pantalla, no flotando a
    // mitad de terminal.
    const AIRE: u16 = 1;
    let y_titulo = area.y;
    let y_prompt = y_titulo + 1 + AIRE;
    let y_aviso = y_prompt + 1 + AIRE;

    frame.render_widget(
        Paragraph::new(linea_titulo_login(&app.fase, opacidad_titulo)),
        Rect::new(area.x, y_titulo, area.width, 1.min(area.height)),
    );

    if area.height > 1 + AIRE {
        match etiqueta_prompt(&app.fase) {
            Some(etiqueta) => {
                let vacio = app.input.value().is_empty();
                let valor = valor_prompt(&app.fase, app);
                let linea = linea_prompt(etiqueta, &valor, vacio, opacidad_prompt);
                frame.render_widget(
                    Paragraph::new(linea),
                    Rect::new(area.x, y_prompt, area.width, 1),
                );
            }
            // Verificando/Creando: no crece con tecleo.
            None => {
                frame.render_widget(
                    Paragraph::new(linea_verificando(&app.fase, opacidad_prompt)),
                    Rect::new(area.x, y_prompt, area.width, 1),
                );
            }
        }
    }

    if area.height > 1 + 2 * AIRE + 1 {
        frame.render_widget(
            Paragraph::new(linea_aviso_login(app, opacidad_aviso)),
            Rect::new(area.x, y_aviso, area.width, 1),
        );
    }
}

/// El nombre de la app muta a la identidad del operador en cuanto se
/// resuelve (misma ranura, misma línea — mutación, no aparición de un
/// elemento nuevo, DEC §1) y de vuelta al nombre de la app si la cuenta deja
/// de ser válida a mitad de camino (ver `login::manejar_login_password`).
fn linea_titulo_login(fase: &Fase, opacidad: f32) -> Line<'static> {
    let (texto, color) = match fase {
        Fase::LoginCedula | Fase::RootCedula | Fase::RootNombre { .. } => {
            (crate::comandos::NOMBRE_APP.to_uppercase(), FADE_ACENTO)
        }
        Fase::LoginPassword { nombre, .. }
        | Fase::Verificando { nombre }
        | Fase::RootPassword { nombre, .. }
        | Fase::RootConfirmarPassword { nombre, .. }
        | Fase::RootCreando { nombre } => (nombre.to_uppercase(), FADE_TEXTO),
        Fase::Operando { .. } => (String::new(), FADE_TEXTO),
    };
    Line::from(Span::styled(
        texto,
        estilo_fundido(color, opacidad, Modifier::BOLD),
    ))
}

fn linea_verificando(fase: &Fase, opacidad: f32) -> Line<'static> {
    // Trabajo real (Argon2 en un hilo aparte), no una animación decorativa:
    // el glifo ● es el mismo que en el resto de la app para "sistema activo".
    let texto = match fase {
        Fase::RootCreando { .. } => "● Creando usuario",
        _ => "● Verificando",
    };
    Line::from(Span::styled(
        texto,
        estilo_fundido(FADE_MUTED, opacidad, Modifier::empty()),
    ))
}

/// Etiqueta de marcador de posición del prompt según la fase — largo fijo,
/// usado tanto para el texto mostrado cuando no se ha tecleado nada como
/// para calcular el punto de anclaje horizontal. `None` en `Verificando`:
/// ahí no hay nada que escribir.
fn etiqueta_prompt(fase: &Fase) -> Option<&'static str> {
    match fase {
        Fase::LoginCedula => Some("Identificación"),
        Fase::LoginPassword { .. } => Some("Contraseña"),
        Fase::RootCedula => Some("Cédula"),
        Fase::RootNombre { .. } => Some("Nombre"),
        Fase::RootPassword { .. } => Some("Contraseña"),
        Fase::RootConfirmarPassword { .. } => Some("Confirmar contraseña"),
        Fase::Verificando { .. } | Fase::RootCreando { .. } | Fase::Operando { .. } => None,
    }
}

fn valor_prompt(fase: &Fase, app: &AppState) -> String {
    match fase {
        Fase::LoginCedula | Fase::RootCedula | Fase::RootNombre { .. } => {
            app.input.value().to_string()
        }
        Fase::LoginPassword { .. } | Fase::RootPassword { .. } | Fase::RootConfirmarPassword { .. } => {
            "•".repeat(app.input.value().chars().count())
        }
        Fase::Verificando { .. } | Fase::RootCreando { .. } | Fase::Operando { .. } => {
            String::new()
        }
    }
}

/// `vacio`: sin nada tecleado se muestra la etiqueta como pista (el foco `›`
/// ya está puesto, no hace falta cursor todavía); en cuanto hay texto, la
/// etiqueta se retira y el valor ocupa su lugar con el cursor `_` al final —
/// la etiqueta se simplifica en el propio valor, no coexisten. La aparición
/// (fade-in) es por transición de fase, nunca por tecla: escribir no anima.
fn linea_prompt(etiqueta: &str, valor_mostrado: &str, vacio: bool, opacidad: f32) -> Line<'static> {
    if vacio {
        Line::from(vec![
            Span::styled(
                "› ",
                estilo_fundido(FADE_ACENTO, opacidad, Modifier::empty()),
            ),
            Span::styled(
                etiqueta.to_string(),
                estilo_fundido(FADE_MUTED, opacidad, Modifier::empty()),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "› ",
                estilo_fundido(FADE_ACENTO, opacidad, Modifier::empty()),
            ),
            Span::styled(
                valor_mostrado.to_string(),
                estilo_fundido(FADE_TEXTO, opacidad, Modifier::empty()),
            ),
            Span::styled(
                "_",
                estilo_fundido(FADE_ACENTO, opacidad, Modifier::empty()),
            ),
        ])
    }
}

/// Símbolo de la gramática compartida (`✓ ! ×`) con color propio en RGB —
/// distinto de `glifo_feedback` (que usa `Color` con nombre) porque acá hace
/// falta poder fundirlo con `estilo_fundido`.
fn color_nivel_login(nivel: NivelFeedback) -> (u8, u8, u8) {
    match nivel {
        NivelFeedback::Exito => FADE_EXITO,
        NivelFeedback::Advertencia => FADE_ADVERTENCIA,
        NivelFeedback::Error => FADE_ERROR,
    }
}

fn linea_aviso_login(app: &AppState, opacidad: f32) -> Line<'static> {
    match app.feedback_vigente() {
        Some(feedback) => {
            let (simbolo, _) = glifo_feedback(feedback.nivel);
            let estilo = estilo_fundido(
                color_nivel_login(feedback.nivel),
                opacidad,
                Modifier::empty(),
            );
            Line::from(vec![
                Span::styled(format!("{simbolo} "), estilo),
                Span::styled(feedback.texto.clone(), estilo),
            ])
        }
        None => Line::from(""),
    }
}
