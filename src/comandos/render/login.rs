//! Pantalla de login/root para la interfaz de comandos. Sigue siendo una
//! composición de texto, sin recuadros ni superficies pesadas, pero ahora
//! muestra estado, paso actual y salida disponible: suficiente orientación
//! para no parecer un campo suelto. El cursor es un "_" con estilo, nunca el
//! bloque parpadeante del terminal — por eso esta función jamás llama a
//! `frame.set_cursor_position`.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::comandos::estado::{AppState, Fase, NivelFeedback};

use super::estilos::{
    FADE_ACENTO, FADE_ADVERTENCIA, FADE_ERROR, FADE_EXITO, FADE_MUTED, FADE_TEXTO, estilo_fundido,
    glifo_feedback,
};

pub(super) fn render_login(frame: &mut Frame, area: Rect, app: &AppState) {
    let opacidad_titulo = app.presentacion.opacidad("titulo");
    let opacidad_prompt = app.presentacion.opacidad("prompt");
    let opacidad_aviso = app.presentacion.opacidad("feedback");

    const AIRE: u16 = 1;
    let y_titulo = area.y;
    let y_estado = y_titulo + 1;
    let y_prompt = y_estado + 1 + AIRE;
    let y_ayuda = y_prompt + 1;
    let y_aviso = y_ayuda + 1 + AIRE;

    frame.render_widget(
        Paragraph::new(linea_titulo_login(&app.fase, opacidad_titulo)),
        Rect::new(area.x, y_titulo, area.width, 1.min(area.height)),
    );

    if area.height > 1 {
        frame.render_widget(
            Paragraph::new(linea_estado_login(&app.fase, opacidad_prompt)),
            Rect::new(area.x, y_estado, area.width, 1),
        );
    }

    if area.height > 2 + AIRE {
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

    if area.height > 3 + AIRE {
        frame.render_widget(
            Paragraph::new(linea_ayuda_login(&app.fase, opacidad_prompt)),
            Rect::new(area.x, y_ayuda, area.width, 1),
        );
    }

    if area.height > 4 + 2 * AIRE {
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

fn linea_estado_login(fase: &Fase, opacidad: f32) -> Line<'static> {
    let (texto, color, modificador) = match fase {
        Fase::LoginCedula => (
            "● Paso 1 de 2 · Identificación",
            FADE_MUTED,
            Modifier::empty(),
        ),
        Fase::LoginPassword { .. } => (
            "✓ Identidad reconocida · Paso 2 de 2",
            FADE_EXITO,
            Modifier::empty(),
        ),
        Fase::Verificando { .. } => ("● Verificando credenciales", FADE_MUTED, Modifier::empty()),
        Fase::RootCedula => (
            "● Configuración inicial · Paso 1 de 4",
            FADE_MUTED,
            Modifier::empty(),
        ),
        Fase::RootNombre { .. } => (
            "● Configuración inicial · Paso 2 de 4",
            FADE_MUTED,
            Modifier::empty(),
        ),
        Fase::RootPassword { .. } => (
            "● Configuración inicial · Paso 3 de 4",
            FADE_MUTED,
            Modifier::empty(),
        ),
        Fase::RootConfirmarPassword { .. } => (
            "● Configuración inicial · Paso 4 de 4",
            FADE_MUTED,
            Modifier::empty(),
        ),
        Fase::RootCreando { .. } => ("● Creando usuario ROOT", FADE_MUTED, Modifier::empty()),
        Fase::Operando { .. } => ("", FADE_MUTED, Modifier::empty()),
    };
    Line::from(Span::styled(
        texto,
        estilo_fundido(color, opacidad, modificador),
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
        Fase::LoginPassword { .. }
        | Fase::RootPassword { .. }
        | Fase::RootConfirmarPassword { .. } => "•".repeat(app.input.value().chars().count()),
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
                format!("{etiqueta}: "),
                estilo_fundido(FADE_MUTED, opacidad, Modifier::empty()),
            ),
            Span::styled(
                "_",
                estilo_fundido(FADE_ACENTO, opacidad, Modifier::empty()),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "› ",
                estilo_fundido(FADE_ACENTO, opacidad, Modifier::empty()),
            ),
            Span::styled(
                format!("{etiqueta}: "),
                estilo_fundido(FADE_MUTED, opacidad, Modifier::empty()),
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

fn linea_ayuda_login(fase: &Fase, opacidad: f32) -> Line<'static> {
    let texto = match fase {
        Fase::LoginCedula => "Enter continúa · Esc limpia · Ctrl+C sale",
        Fase::LoginPassword { .. } => "Enter inicia sesión · Esc cambia usuario · Ctrl+C sale",
        Fase::Verificando { .. } => "Espere un momento · Ctrl+C sale",
        Fase::RootCedula | Fase::RootNombre { .. } | Fase::RootPassword { .. } => {
            "Enter continúa · Esc vuelve al campo anterior · Ctrl+C sale"
        }
        Fase::RootConfirmarPassword { .. } => {
            "Enter crea el usuario · Esc vuelve al campo anterior · Ctrl+C sale"
        }
        Fase::RootCreando { .. } => "Espere un momento · Ctrl+C sale",
        Fase::Operando { .. } => "",
    };
    Line::from(Span::styled(
        texto,
        estilo_fundido(FADE_MUTED, opacidad, Modifier::empty()),
    ))
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

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};
    use tui_input::Input;

    use super::*;

    fn renderizar(app: &AppState) -> String {
        let mut terminal = Terminal::new(TestBackend::new(80, 16)).expect("backend de prueba");
        terminal
            .draw(|frame| render_login(frame, frame.area(), app))
            .expect("render");
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn login_de_comandos_muestra_paso_prompt_y_ayuda() {
        let app = AppState::new();
        let texto = renderizar(&app);

        assert!(texto.contains("BRISAS CLI"), "{texto}");
        assert!(texto.contains("Paso 1 de 2"), "{texto}");
        assert!(texto.contains("Identificación: _"), "{texto}");
        assert!(texto.contains("Enter continúa"), "{texto}");
    }

    #[test]
    fn login_de_comandos_enmascara_password_y_conserva_identidad() {
        let mut app = AppState::new();
        app.fase = Fase::LoginPassword {
            cedula: "1-1111-1111".to_string(),
            nombre: "Ana Operadora".to_string(),
        };
        app.input = Input::new("secreto".to_string());

        let texto = renderizar(&app);

        assert!(texto.contains("ANA OPERADORA"), "{texto}");
        assert!(texto.contains("Paso 2 de 2"), "{texto}");
        assert!(texto.contains("Contraseña: •••••••_"), "{texto}");
        assert!(!texto.contains("secreto"), "{texto}");
    }
}
