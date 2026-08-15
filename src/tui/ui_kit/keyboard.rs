use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::shell::CommandHint;

/// Convenciones de teclado compartidas por todas las pantallas de BRISAS CLI.
///
/// Las acciones de dominio siguen perteneciendo a cada vista; este módulo solo
/// define el significado estable de las teclas comunes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardCommand {
    Primary,
    Activate,
    Cancel,
    FocusNext,
    FocusPrevious,
    Help,
    Theme,
    QuickExit,
    EmergencyExit,
}

pub const HELP_KEY: KeyCode = KeyCode::F(1);
pub const QUICK_EXIT_KEY: KeyCode = KeyCode::F(2);
pub const THEME_KEY: KeyCode = KeyCode::F(7);

pub const HELP_HINT: CommandHint<'static> = CommandHint::new("F1", "Ayuda");
pub const QUICK_EXIT_HINT: CommandHint<'static> = CommandHint::new("F2", "Salida rápida");
pub const THEME_HINT: CommandHint<'static> = CommandHint::new("F7", "Tema");
pub const CANCEL_HINT: CommandHint<'static> = CommandHint::new("ESC", "Cancelar");

/// Traduce únicamente comandos transversales. Flechas, paginación y letras de
/// acceso rápido se resuelven en la vista que conoce su contexto.
pub fn standard_command(key: KeyEvent) -> Option<StandardCommand> {
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && !key.modifiers.contains(KeyModifiers::ALT)
        && matches!(key.code, KeyCode::Char(character) if character.eq_ignore_ascii_case(&'c'))
    {
        return Some(StandardCommand::EmergencyExit);
    }

    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }

    match key.code {
        KeyCode::Enter => Some(StandardCommand::Primary),
        KeyCode::Char(' ') => Some(StandardCommand::Activate),
        KeyCode::Esc => Some(StandardCommand::Cancel),
        KeyCode::BackTab => Some(StandardCommand::FocusPrevious),
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
            Some(StandardCommand::FocusPrevious)
        }
        KeyCode::Tab => Some(StandardCommand::FocusNext),
        HELP_KEY => Some(StandardCommand::Help),
        QUICK_EXIT_KEY => Some(StandardCommand::QuickExit),
        THEME_KEY => Some(StandardCommand::Theme),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{StandardCommand, standard_command};

    #[test]
    fn reconoce_convenciones_comunes() {
        let cases = [
            (KeyCode::Enter, KeyModifiers::NONE, StandardCommand::Primary),
            (
                KeyCode::Char(' '),
                KeyModifiers::NONE,
                StandardCommand::Activate,
            ),
            (KeyCode::Esc, KeyModifiers::NONE, StandardCommand::Cancel),
            (KeyCode::Tab, KeyModifiers::NONE, StandardCommand::FocusNext),
            (
                KeyCode::BackTab,
                KeyModifiers::SHIFT,
                StandardCommand::FocusPrevious,
            ),
            (KeyCode::F(1), KeyModifiers::NONE, StandardCommand::Help),
            (
                KeyCode::F(2),
                KeyModifiers::NONE,
                StandardCommand::QuickExit,
            ),
            (KeyCode::F(7), KeyModifiers::NONE, StandardCommand::Theme),
        ];

        for (code, modifiers, expected) in cases {
            assert_eq!(
                standard_command(KeyEvent::new(code, modifiers)),
                Some(expected)
            );
        }
    }

    #[test]
    fn no_roba_combinaciones_ajenas() {
        assert_eq!(
            standard_command(KeyEvent::new(
                KeyCode::Char('p'),
                KeyModifiers::CONTROL | KeyModifiers::ALT,
            )),
            None
        );
        assert_eq!(
            standard_command(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            None
        );
    }
}
