use ratatui::text::{Line, Span};

use super::Theme;

/// Línea estándar para paneles de detalle: la etiqueta guía la lectura con
/// el acento en negrita y el valor conserva el color principal de texto.
pub fn detail_line(
    label: impl Into<String>,
    value: impl Into<String>,
    theme: Theme,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{}: ", label.into()), theme.accent()),
        Span::styled(value.into(), theme.base()),
    ])
}

#[cfg(test)]
mod tests {
    use ratatui::style::Modifier;

    use super::*;
    use crate::tui::ui_kit::ThemePreset;

    #[test]
    fn etiqueta_usa_acento_negrita_y_valor_texto_base() {
        let theme = ThemePreset::Brisas.theme();
        let line = detail_line("Tipo", "PRAIND", theme);

        assert_eq!(line.spans[0].content, "Tipo: ");
        assert_eq!(line.spans[0].style, theme.accent());
        assert!(line.spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(line.spans[1].content, "PRAIND");
        assert_eq!(line.spans[1].style, theme.base());
    }
}
