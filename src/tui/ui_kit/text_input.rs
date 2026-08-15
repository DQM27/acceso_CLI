use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

use super::Theme;

/// Entrada de una línea con edición Unicode, desplazamiento y cursor real.
#[derive(Debug, Clone)]
pub struct TextInput {
    input: Input,
    max_chars: usize,
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            input: Input::default(),
            max_chars: 4_096,
        }
    }
}

impl TextInput {
    pub fn new(value: impl Into<String>) -> Self {
        let mut input = Self::default();
        input.set_value(value);
        input
    }

    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars.min(4_096);
        let value = limited(self.input.value(), self.max_chars);
        self.input = Input::new(value);
        self
    }

    pub fn value(&self) -> &str {
        self.input.value()
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        let value = value.into();
        self.input = Input::new(limited(&value, self.max_chars));
    }

    pub fn clear(&mut self) {
        self.input.reset();
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        if let Event::Paste(text) = event {
            let mut changed = false;
            let remaining = self
                .max_chars
                .saturating_sub(self.input.value().chars().count());
            for character in text
                .chars()
                .filter(|character| !character.is_control())
                .take(remaining)
            {
                self.input.handle(InputRequest::InsertChar(character));
                changed = true;
            }
            changed
        } else {
            if matches!(
                event,
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
                        && matches!(key.code, KeyCode::Char(_))
                        && !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        && self.input.value().chars().count() >= self.max_chars
            ) {
                return false;
            }
            self.input.handle_event(event).is_some()
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        placeholder: &str,
        focused: bool,
        theme: Theme,
    ) {
        let border_style = if focused {
            theme.accent()
        } else {
            theme.border()
        };
        let block = Block::default()
            .title(format!(" {label} "))
            .title_style(border_style)
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(theme.base());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        // Se reserva una columna vacía para el cursor cuando queda al final del
        // texto; de lo contrario se superpondría al último carácter visible.
        let viewport_width = inner.width.saturating_sub(1) as usize;
        let scroll = self.input.visual_scroll(viewport_width);
        let line = if self.input.value().is_empty() && !focused {
            Line::from(placeholder).style(theme.muted())
        } else {
            Line::from(self.input.value()).style(theme.base())
        };
        let text_area = Rect::new(
            inner.x,
            inner.y,
            inner.width.saturating_sub(1),
            inner.height,
        );
        let scroll = u16::try_from(scroll).unwrap_or(u16::MAX);
        frame.render_widget(Paragraph::new(line).scroll((0, scroll)), text_area);

        if focused {
            let visual_cursor = self.input.visual_cursor();
            let cursor_column = visual_cursor
                .max(scroll as usize)
                .saturating_sub(scroll as usize)
                .min(viewport_width);
            frame.set_cursor_position((inner.x + cursor_column as u16, inner.y));
        }
    }
}

fn limited(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};

    use super::TextInput;
    use crate::tui::ui_kit::ThemePreset;

    #[test]
    fn edita_texto_unicode_sin_tratar_bytes_como_columnas() {
        let mut input = TextInput::new("José");
        input.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::NONE,
        )));
        input.handle_event(&Event::Paste("Peña".into()));

        assert_eq!(input.value(), "José Peña");
    }

    #[test]
    fn reserva_una_celda_para_el_cursor_al_final_del_campo() {
        let input = TextInput::new("abcdef");
        let backend = TestBackend::new(8, 3);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| {
                input.render(
                    frame,
                    Rect::new(0, 0, 8, 3),
                    "CAMPO",
                    "",
                    true,
                    ThemePreset::Classic.theme(),
                );
            })
            .expect("render");

        let cursor = terminal.backend().cursor_position();
        assert_eq!((cursor.x, cursor.y), (6, 1));
    }

    #[test]
    fn limita_escritura_y_pegado_y_descarta_controles() {
        let mut input = TextInput::default().with_max_chars(5);

        input.handle_event(&Event::Paste("ab\ncdefgh".into()));
        input.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::NONE,
        )));

        assert_eq!(input.value(), "abcde");
    }
}
