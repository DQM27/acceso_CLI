use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState},
};

use super::{Theme, auxiliary_panel};

/// Estado y navegación de un selector, independiente de la pantalla que lo usa.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectMenuState {
    highlighted: Option<usize>,
}

impl SelectMenuState {
    pub const fn closed() -> Self {
        Self { highlighted: None }
    }

    pub const fn is_open(&self) -> bool {
        self.highlighted.is_some()
    }

    pub const fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    pub fn open(&mut self, current: usize, option_count: usize) -> bool {
        let next = option_count.checked_sub(1).map(|last| current.min(last));
        let changed = self.highlighted != next;
        self.highlighted = next;
        changed
    }

    pub fn move_by(&mut self, delta: isize, option_count: usize) -> bool {
        let Some(current) = self.highlighted else {
            return false;
        };
        let Some(last) = option_count.checked_sub(1) else {
            self.highlighted = None;
            return true;
        };
        let next = current.min(last).saturating_add_signed(delta).min(last);
        let changed = current != next;
        self.highlighted = Some(next);
        changed
    }

    pub fn first(&mut self, option_count: usize) -> bool {
        self.set_edge(0, option_count)
    }

    pub fn last(&mut self, option_count: usize) -> bool {
        self.set_edge(option_count.saturating_sub(1), option_count)
    }

    pub fn accept(&mut self, option_count: usize) -> Option<usize> {
        let highlighted = self.highlighted.take()?;
        (highlighted < option_count).then_some(highlighted)
    }

    pub fn close(&mut self) -> bool {
        self.highlighted.take().is_some()
    }

    fn set_edge(&mut self, edge: usize, option_count: usize) -> bool {
        if self.highlighted.is_none() {
            return false;
        }
        let next = option_count.checked_sub(1).map(|last| edge.min(last));
        let changed = self.highlighted != next;
        self.highlighted = next;
        changed
    }
}

/// Menú desplegable reutilizable para seleccionar una opción con el teclado.
pub struct SelectMenu<'a, T> {
    pub title: &'a str,
    pub options: &'a [T],
    pub current: Option<usize>,
}

impl<T: AsRef<str>> SelectMenu<'_, T> {
    pub fn render(&self, frame: &mut Frame, area: Rect, state: &SelectMenuState, theme: Theme) {
        let Some(highlighted) = state.highlighted() else {
            return;
        };
        frame.render_widget(Clear, area);
        let current = self.current.filter(|index| *index < self.options.len());
        let items = self.options.iter().enumerate().map(|(index, option)| {
            let marker = if current == Some(index) { "●" } else { " " };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{marker} "), theme.muted()),
                Span::styled(option.as_ref(), theme.base()),
            ]))
        });
        let list = List::new(items)
            .block(auxiliary_panel(self.title, theme, true))
            .highlight_style(theme.selected())
            .highlight_symbol("> ");
        let selected = highlighted.min(self.options.len().saturating_sub(1));
        let mut list_state =
            ListState::default().with_selected((!self.options.is_empty()).then_some(selected));
        frame.render_stateful_widget(list, area, &mut list_state);
    }
}

#[cfg(test)]
mod tests {
    use super::SelectMenuState;

    #[test]
    fn navegacion_respeta_los_limites_y_confirma_una_opcion_valida() {
        let mut state = SelectMenuState::closed();

        assert!(state.open(99, 3));
        assert_eq!(state.highlighted(), Some(2));
        assert!(!state.move_by(1, 3));
        assert!(state.first(3));
        assert_eq!(state.highlighted(), Some(0));
        assert!(!state.move_by(-1, 3));
        assert!(state.last(3));
        assert_eq!(state.accept(3), Some(2));
        assert!(!state.is_open());
    }

    #[test]
    fn no_confirma_un_indice_si_la_lista_cambia_mientras_esta_abierta() {
        let mut state = SelectMenuState::closed();

        assert!(state.open(2, 3));
        assert_eq!(state.accept(2), None);
        assert!(!state.is_open());
    }

    #[test]
    fn lista_vacia_no_abre_y_cerrar_descarta_solo_el_resaltado() {
        let mut state = SelectMenuState::closed();

        assert!(!state.open(0, 0));
        assert!(!state.is_open());
        assert!(state.open(0, 2));
        assert!(state.close());
        assert!(!state.close());
    }
}
