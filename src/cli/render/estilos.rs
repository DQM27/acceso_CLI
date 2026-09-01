//! Primitivas visuales compartidas por todas las Surfaces: color/estilo de
//! cada nivel de feedback (`glifo_feedback`), la gramática de glifos
//! (✓ ! × ●) y el fundido de aparición (`estilo_fundido`/`atenuar`), sobre
//! el que se apoyan login, formularios e historial por igual.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::cli::estado::NivelFeedback;

pub(super) fn exito() -> Style {
    Style::default().fg(Color::Green)
}
pub(super) fn advertencia() -> Style {
    Style::default().fg(Color::Yellow)
}
pub(super) fn estilo_error() -> Style {
    Style::default().fg(Color::Red)
}
pub(super) fn muted() -> Style {
    Style::default().fg(Color::DarkGray)
}
pub(super) fn acento() -> Style {
    Style::default().fg(Color::Cyan)
}
pub(super) fn estilo_seleccion() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

/// Gramática visual compartida por toda la app (ver
/// `docs/lenguaje-visual-mutaciones.md`): el glifo nunca depende del color
/// para transmitir significado — el color sólo refuerza.
///
/// ```text
/// ●  procesando / sistema activo
/// ›  esperando entrada / foco
/// ✓  completado
/// !  advertencia
/// ×  falló / error / rechazo
/// ```
pub(super) fn glifo_feedback(nivel: NivelFeedback) -> (&'static str, Style) {
    match nivel {
        NivelFeedback::Exito => ("✓", exito()),
        NivelFeedback::Advertencia => ("!", advertencia()),
        NivelFeedback::Error => ("×", estilo_error()),
    }
}

/// Mismo símbolo que `glifo_feedback`, en RGB para fundir (`estilo_fundido`
/// necesita interpolar componentes) — reutiliza esa función en vez de
/// repetir el `match` de símbolos por nivel.
pub(super) fn glifo_feedback_color(nivel: NivelFeedback) -> (&'static str, (u8, u8, u8)) {
    let (simbolo, estilo) = glifo_feedback(nivel);
    (simbolo, color_a_rgb(estilo.fg))
}

/// Paleta propia del login en RGB explícito (no los `Color` con nombre del
/// resto del archivo): un fundido necesita interpolar componentes, y sólo
/// `Color::Rgb` los tiene. Fondo asumido oscuro — es la base de todo el tema
/// actual (ver `muted()`/`acento()`), no una novedad de esta escena.
pub(super) const FADE_FONDO: (u8, u8, u8) = (10, 10, 12);
pub(super) const FADE_ACENTO: (u8, u8, u8) = (86, 200, 214);
pub(super) const FADE_MUTED: (u8, u8, u8) = (120, 120, 130);
pub(super) const FADE_TEXTO: (u8, u8, u8) = (225, 225, 230);
pub(super) const FADE_EXITO: (u8, u8, u8) = (94, 201, 133);
pub(super) const FADE_ADVERTENCIA: (u8, u8, u8) = (214, 181, 92);
pub(super) const FADE_ERROR: (u8, u8, u8) = (214, 92, 92);

pub(super) fn interpolar_color(desde: (u8, u8, u8), hasta: (u8, u8, u8), t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    // El `.clamp(0.0, 255.0)` ya deja el valor dentro de lo que `u8` puede
    // representar — Clippy no razona sobre el resultado de un `clamp` en
    // tiempo de compilación, así que sigue viendo el cast como arriesgado
    // aunque ya no lo sea.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mezclar = |a: u8, b: u8| {
        (a as f32 + (b as f32 - a as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::Rgb(
        mezclar(desde.0, hasta.0),
        mezclar(desde.1, hasta.1),
        mezclar(desde.2, hasta.2),
    )
}

/// Estilo que funde desde `FADE_FONDO` hacia `color` según `opacidad`
/// (0.0 = invisible, fundido con el fondo; 1.0 = color final). Con
/// `opacidad` en 1.0 (elemento ya resuelto, o `VisualQuality::Off`) el color
/// resultante coincide exactamente con el color final, sin diferencia visual.
pub(super) fn estilo_fundido(color: (u8, u8, u8), opacidad: f32, modificador: Modifier) -> Style {
    Style::default()
        .fg(interpolar_color(FADE_FONDO, color, opacidad))
        .add_modifier(modificador)
}

/// Contraparte de `estilo_fundido` para el color con nombre (`Color::Cyan`,
/// no `Color::Rgb`) que ya usan `acento()`/`muted()`/etc. — sólo hace falta
/// para re-interpolar líneas ya construidas (`atenuar`), donde no hay forma
/// de saber con qué constante `FADE_*` se armaron originalmente salvo
/// leyendo qué `Color` terminaron usando.
pub(super) fn color_a_rgb(color: Option<Color>) -> (u8, u8, u8) {
    match color {
        Some(Color::Cyan) => FADE_ACENTO,
        Some(Color::DarkGray) => FADE_MUTED,
        Some(Color::Red) => FADE_ERROR,
        Some(Color::Green) => FADE_EXITO,
        Some(Color::Yellow) => FADE_ADVERTENCIA,
        _ => FADE_TEXTO,
    }
}

/// Re-interpola el color de cada `Span` ya construido hacia `FADE_FONDO`
/// según `opacidad`, sin tocar el modificador (BOLD/REVERSED se
/// conservan tal cual). Alternativa a enhebrar un parámetro de opacidad
/// por cada función de `lineas_contexto` (como sí hacen login/formulario/
/// historial, DEC-040): el área de contexto tiene más de 15 variantes de
/// pantalla, y reescribir cada `Span::styled` en cada una para una sola
/// aparición no se justificaba — re-interpolar el color que la línea ya
/// trae logra el mismo resultado visual desde un solo punto (DEC-059).
/// Con `opacidad >= 1.0` no toca nada, así que el color en reposo sigue
/// siendo exactamente el original — nunca una aproximación de
/// `color_a_rgb` (que sólo entra en juego mientras la aparición está en
/// curso, un par de cientos de ms, no en reposo).
pub(super) fn atenuar(lineas: Vec<Line<'static>>, opacidad: f32) -> Vec<Line<'static>> {
    if opacidad >= 1.0 {
        return lineas;
    }
    lineas
        .into_iter()
        .map(|linea| {
            let spans: Vec<Span<'static>> = linea
                .spans
                .into_iter()
                .map(|span| {
                    let rgb = color_a_rgb(span.style.fg);
                    let estilo = Style {
                        fg: Some(interpolar_color(FADE_FONDO, rgb, opacidad)),
                        ..span.style
                    };
                    Span::styled(span.content, estilo)
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}
