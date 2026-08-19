use std::fmt;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

use super::Theme;

/// Entrada de una línea con edición Unicode, desplazamiento y cursor real.
#[derive(Clone)]
pub struct TextInput {
    input: Input,
    max_chars: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextInputFocus {
    focused: bool,
    cursor_visible: bool,
}

impl TextInputFocus {
    pub const fn new(focused: bool, cursor_visible: bool) -> Self {
        Self {
            focused,
            cursor_visible: focused && cursor_visible,
        }
    }
}

struct RenderSpec<'a> {
    label: &'a str,
    placeholder: &'a str,
    focused: bool,
    cursor_visible: bool,
    displayed_value: &'a str,
    visual_cursor: usize,
    scroll: usize,
    theme: Theme,
}

impl fmt::Debug for TextInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextInput")
            .field("value", &"[REDACTED]")
            .field("chars", &self.input.value().chars().count())
            .field("max_chars", &self.max_chars)
            .finish()
    }
}

/// `tui_input::Input` no deriva `PartialEq` — se compara por contenido visible
/// (valor + tope), no por la posición del cursor, que es puro estado de edición
/// transitorio sin relevancia para comparar dos entradas "iguales".
impl PartialEq for TextInput {
    fn eq(&self, other: &Self) -> bool {
        self.input.value() == other.input.value() && self.max_chars == other.max_chars
    }
}
impl Eq for TextInput {}

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

    /// Posición del cursor en caracteres (no bytes) desde el inicio del valor.
    pub fn cursor(&self) -> usize {
        self.input.visual_cursor()
    }

    /// Atajo para el caso común: manejar directamente un `KeyEvent` sin que
    /// cada pantalla tenga que envolverlo en `Event::Key` por su cuenta.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        self.handle_event(&Event::Key(key))
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
        let viewport_width = field_viewport_width(area);
        self.render_value(
            frame,
            area,
            RenderSpec {
                label,
                placeholder,
                focused,
                cursor_visible: focused,
                displayed_value: self.input.value(),
                visual_cursor: self.input.visual_cursor(),
                scroll: self.input.visual_scroll(viewport_width),
                theme,
            },
        );
    }

    /// Variante para pantallas que controlan el parpadeo del cursor real.
    pub fn render_with_cursor(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        placeholder: &str,
        focus: TextInputFocus,
        theme: Theme,
    ) {
        let viewport_width = field_viewport_width(area);
        self.render_value(
            frame,
            area,
            RenderSpec {
                label,
                placeholder,
                focused: focus.focused,
                cursor_visible: focus.cursor_visible,
                displayed_value: self.input.value(),
                visual_cursor: self.input.visual_cursor(),
                scroll: self.input.visual_scroll(viewport_width),
                theme,
            },
        );
    }

    /// Renderiza el contenido como viñetas sin exponer el valor real.
    pub fn render_masked(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        placeholder: &str,
        focused: bool,
        theme: Theme,
    ) {
        let cursor_column = self.input.value().chars().count();
        let masked = "•".repeat(cursor_column);
        let viewport_width = field_viewport_width(area);
        let scroll = cursor_column.saturating_sub(viewport_width);
        self.render_value(
            frame,
            area,
            RenderSpec {
                label,
                placeholder,
                focused,
                cursor_visible: focused,
                displayed_value: &masked,
                visual_cursor: cursor_column,
                scroll,
                theme,
            },
        );
    }

    /// Variante enmascarada para pantallas que controlan el parpadeo real.
    pub fn render_masked_with_cursor(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        placeholder: &str,
        focus: TextInputFocus,
        theme: Theme,
    ) {
        let cursor_column = self.input.value().chars().count();
        let masked = "•".repeat(cursor_column);
        let viewport_width = field_viewport_width(area);
        let scroll = cursor_column.saturating_sub(viewport_width);
        self.render_value(
            frame,
            area,
            RenderSpec {
                label,
                placeholder,
                focused: focus.focused,
                cursor_visible: focus.cursor_visible,
                displayed_value: &masked,
                visual_cursor: cursor_column,
                scroll,
                theme,
            },
        );
    }

    fn render_value(&self, frame: &mut Frame, area: Rect, spec: RenderSpec<'_>) {
        let border_style = if spec.focused {
            spec.theme.accent()
        } else {
            spec.theme.border()
        };
        let block = Block::default()
            .title(format!(" {} ", spec.label))
            .title_style(border_style)
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(spec.theme.base());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        // Se reserva una columna vacía para el cursor cuando queda al final del
        // texto; de lo contrario se superpondría al último carácter visible.
        let viewport_width = inner.width.saturating_sub(1) as usize;
        let line = if self.input.value().is_empty() && !spec.focused {
            Line::from(spec.placeholder).style(spec.theme.muted())
        } else {
            Line::from(spec.displayed_value).style(spec.theme.base())
        };
        let text_area = Rect::new(
            inner.x,
            inner.y,
            inner.width.saturating_sub(1),
            inner.height,
        );
        let scroll = u16::try_from(spec.scroll).unwrap_or(u16::MAX);
        frame.render_widget(Paragraph::new(line).scroll((0, scroll)), text_area);

        if spec.focused && spec.cursor_visible {
            let cursor_column = spec
                .visual_cursor
                .max(scroll as usize)
                .saturating_sub(scroll as usize)
                .min(viewport_width);
            frame.set_cursor_position((inner.x + cursor_column as u16, inner.y));
        }
    }
}

fn field_viewport_width(area: Rect) -> usize {
    area.width.saturating_sub(3) as usize
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
    fn las_flechas_mueven_el_cursor_sin_borrar_nada() {
        let mut input = TextInput::new("abd");
        // Cursor arranca al final (3). Retrocede uno, inserta 'c' entre
        // "ab" y "d" — exactamente el caso que antes obligaba a borrar todo
        // el campo para corregir algo que no estaba al final.
        input.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        input.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        assert_eq!(input.value(), "abcd");
        assert_eq!(input.cursor(), 3);
    }

    #[test]
    fn home_end_y_delete_funcionan_sobre_el_cursor_real() {
        let mut input = TextInput::new("abcd");
        input.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        assert_eq!(input.cursor(), 0);
        input.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(input.value(), "bcd");
        assert_eq!(input.cursor(), 0);
        input.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        assert_eq!(input.cursor(), 3);
    }

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

    #[test]
    fn mascara_no_renderiza_ni_expone_el_secreto_en_debug() {
        let input = TextInput::new("secreto界");
        let backend = TestBackend::new(14, 3);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| {
                input.render_masked(
                    frame,
                    Rect::new(0, 0, 14, 3),
                    "CLAVE",
                    "",
                    true,
                    ThemePreset::Classic.theme(),
                );
            })
            .expect("render");

        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(!rendered.contains("secreto"));
        assert!(rendered.contains("••••••••"));
        assert!(!format!("{input:?}").contains("secreto"));
    }
}
