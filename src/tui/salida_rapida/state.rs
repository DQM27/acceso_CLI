use crossterm::event::{KeyCode, KeyEvent};

use crate::services::registro_ingreso_service::IngresoActivoResumen;
use crate::tui::ui_kit::{StandardCommand, TextInput, mover_seleccion, standard_command};

#[path = "render.rs"]
pub(super) mod render;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Overlay global de "salida rápida" (F2): registra la salida de un ingreso
/// activo por gafete o por nombre/cédula desde cualquier pantalla, sin
/// navegar hasta Ingresos activos.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Estado {
    Cerrado,
    Abierto,
    Confirmado { mensaje: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionSalidaRapida {
    Ninguna,
    Buscar { texto: Option<String> },
    Confirmar { registro_id: i64, nombre: String },
}

#[derive(Debug)]
pub struct SalidaRapidaState {
    estado: Estado,
    busqueda: TextInput,
    registros: Vec<IngresoActivoResumen>,
    seleccion: Option<usize>,
    error: Option<String>,
    ayuda_expandida: bool,
}

impl Default for SalidaRapidaState {
    fn default() -> Self {
        Self {
            estado: Estado::Cerrado,
            busqueda: TextInput::default(),
            registros: vec![],
            seleccion: None,
            error: None,
            ayuda_expandida: false,
        }
    }
}

impl SalidaRapidaState {
    pub fn abierto(&self) -> bool {
        !matches!(self.estado, Estado::Cerrado)
    }

    pub fn abrir(&mut self) -> AccionSalidaRapida {
        self.estado = Estado::Abierto;
        self.busqueda.clear();
        self.registros.clear();
        self.seleccion = None;
        self.error = None;
        AccionSalidaRapida::Buscar { texto: None }
    }

    pub fn completar_busqueda(&mut self, r: Result<Vec<IngresoActivoResumen>, String>) {
        match r {
            Ok(v) => {
                self.registros = v;
                self.seleccion = (!self.registros.is_empty()).then_some(0);
                self.error = None;
            }
            Err(e) => {
                self.registros.clear();
                self.seleccion = None;
                self.error = Some(e);
            }
        }
    }

    pub fn completar_confirmacion(&mut self, r: Result<String, String>) {
        match r {
            Ok(mensaje) => self.estado = Estado::Confirmado { mensaje },
            Err(e) => self.error = Some(e),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AccionSalidaRapida {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionSalidaRapida::Ninguna;
        }
        match self.estado.clone() {
            Estado::Cerrado => AccionSalidaRapida::Ninguna,
            // Mismo criterio que `menu_principal`: sólo Enter/Esc resuelven
            // una confirmación pendiente, no cualquier tecla.
            Estado::Confirmado { .. } => {
                if matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
                    self.estado = Estado::Cerrado;
                }
                AccionSalidaRapida::Ninguna
            }
            Estado::Abierto => self.handle_abierto(key),
        }
    }

    fn handle_abierto(&mut self, key: KeyEvent) -> AccionSalidaRapida {
        match key.code {
            KeyCode::Esc => {
                self.estado = Estado::Cerrado;
                AccionSalidaRapida::Ninguna
            }
            KeyCode::Up => {
                self.mover(-1);
                AccionSalidaRapida::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionSalidaRapida::Ninguna
            }
            KeyCode::Enter => self.registro_seleccionado().map_or(
                AccionSalidaRapida::Ninguna,
                |r| AccionSalidaRapida::Confirmar {
                    registro_id: r.registro_id,
                    nombre: r.contratista_nombre.clone(),
                },
            ),
            _ => {
                if self.busqueda.handle_key(key) {
                    self.error = None;
                    AccionSalidaRapida::Buscar {
                        texto: texto_filtro(self.busqueda.value()),
                    }
                } else {
                    AccionSalidaRapida::Ninguna
                }
            }
        }
    }

    fn mover(&mut self, d: isize) {
        self.seleccion = mover_seleccion(self.seleccion, d, self.registros.len());
    }

    fn registro_seleccionado(&self) -> Option<&IngresoActivoResumen> {
        self.registros.get(self.seleccion?)
    }
}

fn texto_filtro(s: &str) -> Option<String> {
    (!s.trim().is_empty()).then(|| s.to_owned())
}
