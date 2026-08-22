//! Pantalla intermedia entre el login y el resto de la app: con la sesión ya
//! autenticada, el operador elige si sigue en la TUI clásica (menús) o pasa
//! al modo CLI (`src/comandos`, input persistente por comandos). Minimalista
//! a propósito — sólo las dos opciones centradas, sin título ni preguntas.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Modifier,
    widgets::{Block, Paragraph},
};

use super::ui_kit::Theme;

const ETIQUETA_TUI: &str = "TUI clásica";
const ETIQUETA_CLI: &str = "Modo CLI";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Opcion {
    #[default]
    Tui,
    Cli,
}

impl Opcion {
    const fn alternar(self) -> Self {
        match self {
            Self::Tui => Self::Cli,
            Self::Cli => Self::Tui,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ElegirInterfazState {
    seleccion: Opcion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionElegirInterfaz {
    Ninguna,
    Tui,
    Cli,
}

impl ElegirInterfazState {
    pub fn reiniciar(&mut self) {
        *self = Self::default();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AccionElegirInterfaz {
        match key.code {
            KeyCode::Up | KeyCode::Down => {
                self.seleccion = self.seleccion.alternar();
                AccionElegirInterfaz::Ninguna
            }
            KeyCode::Char('1') => AccionElegirInterfaz::Tui,
            KeyCode::Char('2') => AccionElegirInterfaz::Cli,
            KeyCode::Enter => match self.seleccion {
                Opcion::Tui => AccionElegirInterfaz::Tui,
                Opcion::Cli => AccionElegirInterfaz::Cli,
            },
            _ => AccionElegirInterfaz::Ninguna,
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, state: &ElegirInterfazState, theme: Theme) {
    frame.render_widget(Block::default().style(theme.base()), area);

    let alto = 2.min(area.height);
    let ancho = 20.min(area.width);
    let bloque = Rect::new(
        area.x + area.width.saturating_sub(ancho) / 2,
        area.y + area.height.saturating_sub(alto) / 2,
        ancho,
        alto,
    );
    let filas = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(bloque);

    render_opcion(frame, filas[0], ETIQUETA_TUI, state.seleccion == Opcion::Tui, theme);
    render_opcion(frame, filas[1], ETIQUETA_CLI, state.seleccion == Opcion::Cli, theme);
}

fn render_opcion(frame: &mut Frame, area: Rect, etiqueta: &str, seleccionada: bool, theme: Theme) {
    let estilo = if seleccionada {
        theme.accent().add_modifier(Modifier::BOLD)
    } else {
        theme.muted()
    };
    let texto = if seleccionada {
        format!("▸ {etiqueta}")
    } else {
        format!("  {etiqueta}")
    };
    frame.render_widget(
        Paragraph::new(texto).style(estilo).alignment(Alignment::Center),
        area,
    );
}
