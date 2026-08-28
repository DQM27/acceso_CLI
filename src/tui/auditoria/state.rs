use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    database::queries::auditoria::{CambioAuditado, LIMITE_AUDITORIA_PREDETERMINADO, PaginaAuditoria},
    tui::ui_kit::{StandardCommand, mover_seleccion, standard_command},
};

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionAuditoria {
    Ninguna,
    Volver,
    Cargar { offset: usize },
}

#[derive(Debug, Default)]
pub struct AuditoriaState {
    pub(super) items: Vec<CambioAuditado>,
    pub(super) total: usize,
    pub(super) offset: usize,
    pub(super) seleccion: Option<usize>,
    pub(super) error: Option<String>,
    pub(super) ayuda_expandida: bool,
}

impl AuditoriaState {
    pub fn reiniciar(&mut self) -> AccionAuditoria {
        *self = Self::default();
        AccionAuditoria::Cargar { offset: 0 }
    }

    pub fn completar(&mut self, resultado: Result<PaginaAuditoria, String>) {
        match resultado {
            Ok(pagina) => {
                self.items = pagina.items;
                self.total = pagina.total;
                self.seleccion = (!self.items.is_empty()).then_some(0);
                self.error = None;
            }
            Err(error) => {
                self.items.clear();
                self.total = 0;
                self.seleccion = None;
                self.error = Some(error);
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AccionAuditoria {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionAuditoria::Ninguna;
        }
        match key.code {
            KeyCode::Esc => AccionAuditoria::Volver,
            KeyCode::Up => {
                self.seleccion = mover_seleccion(self.seleccion, -1, self.items.len());
                AccionAuditoria::Ninguna
            }
            KeyCode::Down => {
                self.seleccion = mover_seleccion(self.seleccion, 1, self.items.len());
                AccionAuditoria::Ninguna
            }
            KeyCode::PageDown if self.offset + self.items.len() < self.total => {
                self.offset += LIMITE_AUDITORIA_PREDETERMINADO;
                AccionAuditoria::Cargar {
                    offset: self.offset,
                }
            }
            KeyCode::PageUp if self.offset > 0 => {
                self.offset = self.offset.saturating_sub(LIMITE_AUDITORIA_PREDETERMINADO);
                AccionAuditoria::Cargar {
                    offset: self.offset,
                }
            }
            _ => AccionAuditoria::Ninguna,
        }
    }
}
