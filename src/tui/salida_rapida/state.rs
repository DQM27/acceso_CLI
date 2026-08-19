use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::services::registro_ingreso_service::IngresoActivoResumen;
use crate::tui::ui_kit::mover_seleccion;

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
    busqueda: String,
    registros: Vec<IngresoActivoResumen>,
    seleccion: Option<usize>,
    error: Option<String>,
}

impl Default for SalidaRapidaState {
    fn default() -> Self {
        Self {
            estado: Estado::Cerrado,
            busqueda: String::new(),
            registros: vec![],
            seleccion: None,
            error: None,
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
        match self.estado.clone() {
            Estado::Cerrado => AccionSalidaRapida::Ninguna,
            Estado::Confirmado { .. } => {
                self.estado = Estado::Cerrado;
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
            KeyCode::Backspace => {
                self.busqueda.pop();
                self.error = None;
                AccionSalidaRapida::Buscar {
                    texto: texto_filtro(&self.busqueda),
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.busqueda.push(c);
                self.error = None;
                AccionSalidaRapida::Buscar {
                    texto: texto_filtro(&self.busqueda),
                }
            }
            _ => AccionSalidaRapida::Ninguna,
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
