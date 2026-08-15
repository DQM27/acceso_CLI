//! Salida rápida: F2 abre este panel desde cualquier pantalla, sin navegar
//! hasta Activos. Es la única pieza de este mockup que aparece como capa
//! flotante a propósito — invocada explícitamente, pensada para abrir,
//! resolver y cerrarse en segundos, dejando visible lo que había alrededor.

use control_acceso::tui::ui_kit::{
    StandardCommand, Theme, auxiliary_panel, centered_rect, standard_command,
};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Clear, Paragraph},
};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

#[derive(Debug, Clone, Copy)]
pub struct ActiveEntry {
    pub id: u32,
    pub badge: Option<i64>,
    pub identity: &'static str,
    pub name: &'static str,
    pub company: &'static str,
}

pub enum QuickExitOutcome {
    Ignored,
    Consumed,
    ExitRegistered(String),
}

#[derive(Debug)]
pub struct QuickExitOverlay {
    entries: Vec<ActiveEntry>,
    search: Input,
    selected: usize,
    open: bool,
}

impl QuickExitOverlay {
    pub fn demo() -> Self {
        Self {
            entries: vec![
                ActiveEntry {
                    id: 1,
                    badge: Some(12),
                    identity: "1-1042-0881",
                    name: "José Peña",
                    company: "Brisas del Oeste",
                },
                ActiveEntry {
                    id: 2,
                    badge: Some(25),
                    identity: "3-0520-0917",
                    name: "Juan Rodríguez",
                    company: "Expenic Industrial",
                },
                ActiveEntry {
                    id: 3,
                    badge: None,
                    identity: "2-0731-0440",
                    name: "Ana María Solís",
                    company: "Aldama Servicios",
                },
            ],
            search: Input::default(),
            selected: 0,
            open: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) {
        self.open = true;
        self.search = Input::default();
        self.selected = 0;
    }

    /// Sólo actúa mientras está abierto. `Ignored` significa "no me
    /// correspondía este evento, seguí con tu propio manejo".
    pub fn handle_event(&mut self, event: &Event) -> QuickExitOutcome {
        if !self.open {
            return QuickExitOutcome::Ignored;
        }
        if let Event::Key(key) = event
            && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
        {
            match standard_command(*key) {
                Some(StandardCommand::Cancel) => {
                    self.open = false;
                    return QuickExitOutcome::Consumed;
                }
                Some(StandardCommand::Primary) => return self.confirm_selected(),
                _ => match key.code {
                    KeyCode::Up => {
                        self.move_selection(-1);
                        return QuickExitOutcome::Consumed;
                    }
                    KeyCode::Down => {
                        self.move_selection(1);
                        return QuickExitOutcome::Consumed;
                    }
                    _ => {}
                },
            }
        }
        apply_event(&mut self.search, event);
        self.selected = 0;
        QuickExitOutcome::Consumed
    }

    fn confirm_selected(&mut self) -> QuickExitOutcome {
        let Some(entry) = self.filtered().into_iter().nth(self.selected) else {
            return QuickExitOutcome::Consumed;
        };
        let badge_text = entry
            .badge
            .map_or_else(|| "S/G".into(), |number| format!("gafete {number}"));
        let message = format!("Salida registrada · {} · {badge_text}", entry.name);
        self.entries.retain(|candidate| candidate.id != entry.id);
        self.open = false;
        QuickExitOutcome::ExitRegistered(message)
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.filtered().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self.selected.saturating_add_signed(delta).min(count - 1);
    }

    fn filtered(&self) -> Vec<ActiveEntry> {
        let query = self.search.value().trim().to_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                query.is_empty()
                    || entry
                        .badge
                        .is_some_and(|badge| badge.to_string().contains(&query))
                    || entry.name.to_lowercase().contains(&query)
                    || entry.identity.to_lowercase().contains(&query)
            })
            .copied()
            .collect()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        if !self.open {
            return;
        }
        let top_slice = Rect::new(area.x, area.y, area.width, area.height.min(14));
        let popup = centered_rect(top_slice, 60.min(area.width), 9.min(top_slice.height));
        frame.render_widget(Clear, popup);
        let block = auxiliary_panel("SALIDA RÁPIDA · F2", theme, true);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        if inner.height == 0 {
            return;
        }

        let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);
        let value = self.search.value();
        let cursor_column = self.search.visual_cursor().min(rows[0].width.saturating_sub(1) as usize);
        frame.render_widget(
            Paragraph::new(if value.is_empty() {
                Line::from("Gafete o nombre…").style(theme.muted())
            } else {
                Line::from(value.to_owned()).style(theme.base())
            }),
            rows[0],
        );
        frame.set_cursor_position((rows[0].x + cursor_column as u16, rows[0].y));

        let matches = self.filtered();
        if matches.is_empty() {
            frame.render_widget(
                Paragraph::new("Sin coincidencias").style(theme.muted()),
                rows[1],
            );
            return;
        }
        let lines: Vec<Line<'_>> = matches
            .iter()
            .take(rows[1].height as usize)
            .enumerate()
            .map(|(index, entry)| {
                let selected = index == self.selected;
                let marker = if selected { ">" } else { " " };
                let badge = entry
                    .badge
                    .map_or_else(|| "S/G".to_owned(), |number| number.to_string());
                let text = format!(
                    "{marker} {:<12.12} {:<18.18} {:<14.14} {badge}",
                    entry.identity, entry.name, entry.company
                );
                Line::from(text).style(if selected { theme.selected() } else { theme.base() })
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), rows[1]);
    }
}

fn apply_event(input: &mut Input, event: &Event) -> bool {
    if let Event::Paste(text) = event {
        let mut changed = false;
        for character in text.chars().filter(|character| !character.is_control()) {
            input.handle(InputRequest::InsertChar(character));
            changed = true;
        }
        changed
    } else {
        input.handle_event(event).is_some()
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    use super::{QuickExitOutcome, QuickExitOverlay};

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn cerrado_ignora_todo() {
        let mut overlay = QuickExitOverlay::demo();
        assert!(matches!(
            overlay.handle_event(&key(KeyCode::Enter)),
            QuickExitOutcome::Ignored
        ));
    }

    #[test]
    fn escribir_el_gafete_y_enter_registra_la_salida_sin_navegar() {
        let mut overlay = QuickExitOverlay::demo();
        overlay.open();
        overlay.handle_event(&Event::Paste("12".into()));

        let outcome = overlay.handle_event(&key(KeyCode::Enter));
        let QuickExitOutcome::ExitRegistered(message) = outcome else {
            panic!("debe registrar la salida");
        };
        assert!(message.contains("José Peña"));
        assert!(!overlay.is_open());
    }

    #[test]
    fn escape_cierra_sin_registrar_nada() {
        let mut overlay = QuickExitOverlay::demo();
        overlay.open();
        overlay.handle_event(&key(KeyCode::Esc));

        assert!(!overlay.is_open());
        assert_eq!(overlay.entries.len(), 3);
    }

    #[test]
    fn buscar_por_nombre_tambien_funciona_no_solo_por_gafete() {
        let mut overlay = QuickExitOverlay::demo();
        overlay.open();
        overlay.handle_event(&Event::Paste("Ana María".into()));

        assert_eq!(overlay.filtered().len(), 1);
        let outcome = overlay.handle_event(&key(KeyCode::Enter));
        assert!(matches!(outcome, QuickExitOutcome::ExitRegistered(_)));
    }
}
