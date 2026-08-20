use crossterm::event::{KeyCode, KeyEvent};
use std::time::Instant;

use crate::services::registro_ingreso_service::{
    IngresoActivoResumen, ListaIngresosActivosResumen,
};
use crate::tui::ui_kit::{Debounce, StandardCommand, TextInput, mover_seleccion, standard_command};

#[path = "render.rs"]
pub(super) mod render;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const DURACION_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(250);

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
    /// Total real de coincidencias, sin recortar por el tope de la consulta
    /// — permite avisar "N de M, afine la búsqueda" igual que Nuevo Ingreso,
    /// en vez de dejar resultados fuera de forma silenciosa.
    total: usize,
    seleccion: Option<usize>,
    error: Option<String>,
    ayuda_expandida: bool,
    busqueda_debounce: Debounce,
}

impl Default for SalidaRapidaState {
    fn default() -> Self {
        Self {
            estado: Estado::Cerrado,
            busqueda: TextInput::default(),
            registros: vec![],
            total: 0,
            seleccion: None,
            error: None,
            ayuda_expandida: false,
            busqueda_debounce: Debounce::default(),
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

    pub fn completar_busqueda(&mut self, r: Result<ListaIngresosActivosResumen, String>) {
        match r {
            Ok(pagina) => {
                self.registros = pagina.items;
                self.total = pagina.total;
                self.seleccion = (!self.registros.is_empty()).then_some(0);
                self.error = None;
            }
            Err(e) => {
                self.registros.clear();
                self.total = 0;
                self.seleccion = None;
                self.error = Some(e);
            }
        }
    }
    /// `Some(total)` sólo cuando quedaron resultados fuera de la lista
    /// mostrada — mismo criterio que `NuevoIngresoState::resultados_ocultos`.
    pub fn resultados_ocultos(&self) -> Option<usize> {
        (self.total > self.registros.len()).then_some(self.total)
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
            // Mismo criterio de dos etapas que el resto de pantallas de
            // búsqueda: con filtro escrito, Esc sólo lo limpia; con filtro
            // vacío, Esc cierra el overlay. Antes un solo Esc cerraba todo
            // de una, descartando lo escrito sin la etapa intermedia que el
            // operador ya aprendió en el resto de la app.
            KeyCode::Esc if !self.busqueda.value().is_empty() => {
                self.busqueda.clear();
                self.error = None;
                AccionSalidaRapida::Buscar { texto: None }
            }
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
            KeyCode::Enter => {
                self.registro_seleccionado()
                    .map_or(AccionSalidaRapida::Ninguna, |r| {
                        AccionSalidaRapida::Confirmar {
                            registro_id: r.registro_id,
                            nombre: r.contratista_nombre.clone(),
                        }
                    })
            }
            _ => {
                if self.busqueda.handle_key(key) {
                    self.error = None;
                    self.busqueda_debounce.marcar(Instant::now());
                }
                AccionSalidaRapida::Ninguna
            }
        }
    }
    /// Se llama en cada vuelta del bucle principal; dispara la búsqueda
    /// diferida sólo una vez que pasa `DURACION_DEBOUNCE` sin una tecla
    /// nueva — antes esta pantalla golpeaba la base con cada tecla en vez
    /// de agrupar, a diferencia de las otras 5 pantallas de búsqueda.
    pub fn tick(&mut self, ahora: Instant) -> AccionSalidaRapida {
        if self.abierto() && self.busqueda_debounce.listo(ahora, DURACION_DEBOUNCE) {
            AccionSalidaRapida::Buscar {
                texto: texto_filtro(self.busqueda.value()),
            }
        } else {
            AccionSalidaRapida::Ninguna
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
