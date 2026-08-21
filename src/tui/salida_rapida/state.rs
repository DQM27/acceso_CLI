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
    /// Enter sobre una fila no registra la salida directo — primero pide
    /// confirmar mostrando a quién, igual que `ModoActivos::ConfirmarSalida`
    /// en la pantalla completa. Sin este paso, un Enter en blanco justo
    /// después de abrir el overlay (que carga a todos los que están dentro,
    /// más reciente primero, con la fila 0 ya seleccionada) sacaba en
    /// silencio a quien entró más reciente, no a quien el operador tenía
    /// en mente.
    ConfirmarSalida {
        registro_id: i64,
    },
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
    /// Mensaje de éxito ("✓ Salida registrada — X"), independiente de
    /// `error`. Sobrevive al refresco de `registros` que sigue a una
    /// confirmación exitosa (mismo criterio que `ActivosState.mensaje`) para
    /// que el operador vea la confirmación Y la lista con los demás
    /// contratistas al mismo tiempo, en vez de una u otra.
    mensaje: Option<String>,
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
            mensaje: None,
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
        self.mensaje = None;
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

    /// `Ok`: registrada — vuelve a `Abierto`, guarda el mensaje de éxito y
    /// limpia la búsqueda para que la acción devuelta recargue la lista
    /// completa de quienes siguen dentro (antes esto entraba a un estado
    /// `Confirmado` aparte que sólo mostraba el mensaje, sin volver a
    /// consultar — el overlay quedaba abierto pero sin mostrar al resto de
    /// los contratistas hasta cerrarlo y reabrirlo).
    /// `Err`: se queda en `ConfirmarSalida` (si ahí estaba) para que un
    /// nuevo Enter reintente sin tener que volver a seleccionar la fila.
    pub fn completar_confirmacion(&mut self, r: Result<String, String>) -> AccionSalidaRapida {
        match r {
            Ok(mensaje) => {
                self.estado = Estado::Abierto;
                self.mensaje = Some(mensaje);
                self.error = None;
                self.busqueda.clear();
                AccionSalidaRapida::Buscar { texto: None }
            }
            Err(e) => {
                self.error = Some(e);
                AccionSalidaRapida::Ninguna
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AccionSalidaRapida {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionSalidaRapida::Ninguna;
        }
        match self.estado.clone() {
            Estado::Cerrado => AccionSalidaRapida::Ninguna,
            Estado::Abierto => self.handle_abierto(key),
            Estado::ConfirmarSalida { registro_id } => self.handle_confirmar(key, registro_id),
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
                self.mensaje = None;
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
            // No registra la salida directo: primero pide confirmar (ver
            // `Estado::ConfirmarSalida`).
            KeyCode::Enter => {
                if let Some(r) = self.registro_seleccionado() {
                    self.estado = Estado::ConfirmarSalida {
                        registro_id: r.registro_id,
                    };
                }
                AccionSalidaRapida::Ninguna
            }
            _ => {
                if self.busqueda.handle_key(key) {
                    self.error = None;
                    self.mensaje = None;
                    self.busqueda_debounce.marcar(Instant::now());
                }
                AccionSalidaRapida::Ninguna
            }
        }
    }

    fn handle_confirmar(&mut self, key: KeyEvent, registro_id: i64) -> AccionSalidaRapida {
        match key.code {
            KeyCode::Enter => {
                let nombre = self
                    .registro(registro_id)
                    .map(|r| r.contratista_nombre.clone())
                    .unwrap_or_default();
                AccionSalidaRapida::Confirmar {
                    registro_id,
                    nombre,
                }
            }
            KeyCode::Esc => {
                self.estado = Estado::Abierto;
                AccionSalidaRapida::Ninguna
            }
            _ => AccionSalidaRapida::Ninguna,
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

    fn registro(&self, id: i64) -> Option<&IngresoActivoResumen> {
        self.registros.iter().find(|r| r.registro_id == id)
    }
}

fn texto_filtro(s: &str) -> Option<String> {
    (!s.trim().is_empty()).then(|| s.to_owned())
}
