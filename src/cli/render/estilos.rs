//! Primitivas visuales compartidas por todas las Surfaces: color/estilo de
//! cada nivel de feedback (`glifo_feedback`), la gramática de glifos
//! (✓ ! × ●) y el fundido de aparición (`estilo_fundido`/`atenuar`), sobre
//! el que se apoyan login, formularios e historial por igual.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::cli::estado::NivelFeedback;
use crate::tui::ui_kit::Theme;

/// BRISAS_THEME=light activa el tema claro. Oscuro es el valor predeterminado.
pub(super) fn tema() -> Theme {
    static TEMA: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    *TEMA.get_or_init(|| {
        if std::env::var("BRISAS_THEME").is_ok_and(|valor| valor == "light") {
            crate::diseno_generado::LIGHT
        } else {
            crate::diseno_generado::DARK
        }
    })
}

pub(super) fn exito() -> Style {
    tema().base().fg(tema().success)
}
pub(super) fn advertencia() -> Style {
    tema().base().fg(tema().warning)
}
pub(super) fn estilo_error() -> Style {
    tema().base().fg(tema().danger)
}
pub(super) fn muted() -> Style {
    tema().base().fg(tema().muted)
}
pub(super) fn acento() -> Style {
    tema().base().fg(tema().accent)
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

/// Colores para animación derivados del mismo tema que el texto en reposo.
pub(super) fn fade_fondo() -> (u8, u8, u8) {
    color_a_rgb(Some(tema().background))
}
pub(super) fn fade_acento() -> (u8, u8, u8) {
    color_a_rgb(Some(tema().accent))
}
pub(super) fn fade_muted() -> (u8, u8, u8) {
    color_a_rgb(Some(tema().muted))
}
pub(super) fn fade_texto() -> (u8, u8, u8) {
    color_a_rgb(Some(tema().text))
}
pub(super) fn fade_exito() -> (u8, u8, u8) {
    color_a_rgb(Some(tema().success))
}
pub(super) fn fade_advertencia() -> (u8, u8, u8) {
    color_a_rgb(Some(tema().warning))
}
pub(super) fn fade_error() -> (u8, u8, u8) {
    color_a_rgb(Some(tema().danger))
}

pub(super) fn interpolar_color(desde: (u8, u8, u8), hasta: (u8, u8, u8), t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    // El `.clamp(0.0, 255.0)` ya deja el valor dentro de lo que `u8` puede
    // representar — Clippy no razona sobre el resultado de un `clamp` en
    // tiempo de compilación, así que sigue viendo el cast como arriesgado
    // aunque ya no lo sea.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mezclar = |a: u8, b: u8| {
        (f32::from(a) + (f32::from(b) - f32::from(a)) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::Rgb(
        mezclar(desde.0, hasta.0),
        mezclar(desde.1, hasta.1),
        mezclar(desde.2, hasta.2),
    )
}

/// Estilo que funde desde `fade_fondo()` hacia `color` según `opacidad`
/// (0.0 = invisible, fundido con el fondo; 1.0 = color final). Con
/// `opacidad` en 1.0 (elemento ya resuelto, o `VisualQuality::Off`) el color
/// resultante coincide exactamente con el color final, sin diferencia visual.
pub(super) fn estilo_fundido(color: (u8, u8, u8), opacidad: f32, modificador: Modifier) -> Style {
    Style::default()
        .fg(interpolar_color(fade_fondo(), color, opacidad))
        .add_modifier(modificador)
}

/// Conserva los RGB originales durante el fundido y traduce los nombres ANSI heredados.
pub(super) fn color_a_rgb(color: Option<Color>) -> (u8, u8, u8) {
    match color {
        Some(Color::Rgb(r, g, b)) => (r, g, b),
        Some(Color::Cyan) => fade_acento(),
        Some(Color::DarkGray) => fade_muted(),
        Some(Color::Red) => fade_error(),
        Some(Color::Green) => fade_exito(),
        Some(Color::Yellow) => fade_advertencia(),
        _ => match tema().text {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => unreachable!("Los temas Brisas usan RGB"),
        },
    }
}

/// Re-interpola el color de cada `Span` ya construido hacia `fade_fondo()`
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
                        fg: Some(interpolar_color(fade_fondo(), rgb, opacidad)),
                        ..span.style
                    };
                    Span::styled(span.content, estilo)
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}
