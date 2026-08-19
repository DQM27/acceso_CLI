use crossterm::event::{KeyCode, KeyEvent};

use crate::database::queries::empresas::EmpresaResumen;
use crate::tui::ui_kit::{Debounce, StandardCommand, TextInput, standard_command};
use std::time::Instant;

const DURACION_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(250);

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModoFormularioEmpresa {
    Crear,
    Editar { id: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FormularioEmpresa {
    modo: ModoFormularioEmpresa,
    nombre: TextInput,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConfirmacionEstado {
    id: i64,
    activar: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoEmpresas {
    Normal,
    Busqueda { texto: TextInput },
    Formulario(FormularioEmpresa),
    ConfirmacionEstado(ConfirmacionEstado),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionEmpresas {
    Ninguna,
    Volver,
    Buscar {
        texto: Option<String>,
        seleccionar_id: Option<i64>,
    },
    Crear {
        nombre: String,
    },
    Actualizar {
        id: i64,
        nombre: String,
    },
    EstablecerActivo {
        id: i64,
        activar: bool,
        nombre: String,
    },
}

#[derive(Debug)]
pub struct EmpresasState {
    empresas: Vec<EmpresaResumen>,
    seleccion: Option<usize>,
    modo: ModoEmpresas,
    filtro: String,
    mensaje: Option<String>,
    ayuda_expandida: bool,
    busqueda_debounce: Debounce,
}

impl Default for EmpresasState {
    fn default() -> Self {
        Self {
            empresas: vec![],
            seleccion: None,
            modo: ModoEmpresas::Normal,
            filtro: String::new(),
            mensaje: None,
            ayuda_expandida: false,
            busqueda_debounce: Debounce::default(),
        }
    }
}

impl EmpresasState {
    pub fn cantidad(&self) -> usize {
        self.empresas.len()
    }

    pub fn empresa_seleccionada(&self) -> Option<&EmpresaResumen> {
        self.empresas.get(self.seleccion?)
    }

    pub fn esta_en_formulario(&self) -> bool {
        matches!(self.modo, ModoEmpresas::Formulario(_))
    }

    pub fn error_formulario_actual(&self) -> Option<&str> {
        match &self.modo {
            ModoEmpresas::Formulario(formulario) => formulario.error.as_deref(),
            _ => None,
        }
    }

    pub fn solicitar_carga(&self) -> AccionEmpresas {
        self.accion_buscar(None)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AccionEmpresas {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionEmpresas::Ninguna;
        }
        match self.modo.clone() {
            ModoEmpresas::Normal => self.handle_normal(key),
            ModoEmpresas::Busqueda { .. } => self.handle_busqueda(key),
            ModoEmpresas::Formulario(formulario) => self.handle_formulario(key, formulario),
            ModoEmpresas::ConfirmacionEstado(c) => self.handle_confirmacion(key, c),
        }
    }

    pub fn completar_busqueda(
        &mut self,
        resultado: Result<Vec<EmpresaResumen>, String>,
        seleccionar_id: Option<i64>,
    ) {
        match resultado {
            Ok(empresas) => {
                self.empresas = empresas;
                self.mensaje = None;
                self.seleccion = seleccionar_id
                    .and_then(|id| self.empresas.iter().position(|e| e.id == id))
                    .or_else(|| (!self.empresas.is_empty()).then_some(0));
            }
            Err(error) => {
                self.empresas.clear();
                self.seleccion = None;
                self.mensaje = Some(error);
            }
        }
    }

    pub fn completar_creacion(
        &mut self,
        resultado: Result<i64, String>,
        nombre: &str,
    ) -> AccionEmpresas {
        match resultado {
            Ok(id) => {
                self.modo = ModoEmpresas::Normal;
                self.filtro.clear();
                self.mensaje = Some(format!("✓ Empresa creada — {}", nombre.trim()));
                self.accion_buscar(Some(id))
            }
            Err(error) => {
                self.error_formulario(error);
                AccionEmpresas::Ninguna
            }
        }
    }

    pub fn completar_actualizacion(
        &mut self,
        resultado: Result<(), String>,
        id: i64,
        nombre: &str,
    ) -> AccionEmpresas {
        match resultado {
            Ok(()) => {
                self.modo = ModoEmpresas::Normal;
                // Igual que al crear — antes sólo se limpiaba al crear, mismo
                // patrón de inconsistencia que ya tenía Usuarios.
                self.filtro.clear();
                self.mensaje = Some(format!("✓ Empresa actualizada — {}", nombre.trim()));
                self.accion_buscar(Some(id))
            }
            Err(error) => {
                self.error_formulario(error);
                AccionEmpresas::Ninguna
            }
        }
    }

    pub fn completar_estado(
        &mut self,
        resultado: Result<(), String>,
        id: i64,
        activar: bool,
        nombre: &str,
    ) -> AccionEmpresas {
        match resultado {
            Ok(()) => {
                self.modo = ModoEmpresas::Normal;
                self.filtro.clear();
                self.mensaje = Some(format!(
                    "✓ Empresa {} — {nombre}",
                    if activar { "activada" } else { "desactivada" }
                ));
                AccionEmpresas::Buscar {
                    texto: None,
                    seleccionar_id: Some(id),
                }
            }
            Err(error) => {
                self.modo = ModoEmpresas::Normal;
                self.mensaje = Some(error);
                AccionEmpresas::Ninguna
            }
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> AccionEmpresas {
        self.mensaje = None;
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => {
                if let Some(id) = self.id_seleccionado() {
                    self.abrir_edicion(id);
                }
            }
            KeyCode::Char('n' | 'N') => {
                self.modo = ModoEmpresas::Formulario(FormularioEmpresa {
                    modo: ModoFormularioEmpresa::Crear,
                    nombre: TextInput::default().with_max_chars(80),
                    error: None,
                })
            }
            KeyCode::Char('a' | 'A') => {
                if let Some(id) = self.id_seleccionado() {
                    self.solicitar_estado(id);
                }
            }
            KeyCode::Char('/') => {
                self.modo = ModoEmpresas::Busqueda {
                    texto: TextInput::new(self.filtro.clone()),
                }
            }
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                self.seleccion = None;
                return self.accion_buscar(None);
            }
            KeyCode::Esc => return AccionEmpresas::Volver,
            _ => {}
        }
        AccionEmpresas::Ninguna
    }

    fn handle_busqueda(&mut self, key: KeyEvent) -> AccionEmpresas {
        match key.code {
            KeyCode::Esc => {
                self.filtro.clear();
                self.modo = ModoEmpresas::Normal;
                self.accion_buscar(None)
            }
            KeyCode::Enter => {
                self.modo = ModoEmpresas::Normal;
                AccionEmpresas::Ninguna
            }
            KeyCode::Up => {
                self.mover(-1);
                AccionEmpresas::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionEmpresas::Ninguna
            }
            _ => {
                let mut cambio = false;
                if let ModoEmpresas::Busqueda { texto } = &mut self.modo {
                    cambio = texto.handle_key(key);
                    self.filtro = texto.value().to_owned();
                }
                // No se resetea la selección aquí — mismo criterio que
                // Activos/Contratistas/Historial: mientras el debounce está
                // pendiente se mantiene resaltada la fila anterior en vez de
                // verse vacía.
                if cambio {
                    self.busqueda_debounce.marcar(Instant::now());
                }
                AccionEmpresas::Ninguna
            }
        }
    }
    /// Se llama en cada vuelta del bucle principal; dispara la búsqueda
    /// diferida sólo una vez que pasa `DURACION_DEBOUNCE` sin una tecla
    /// nueva.
    pub fn tick(&mut self, ahora: Instant) -> AccionEmpresas {
        if self.busqueda_debounce.listo(ahora, DURACION_DEBOUNCE) {
            self.accion_buscar(None)
        } else {
            AccionEmpresas::Ninguna
        }
    }

    fn handle_formulario(
        &mut self,
        key: KeyEvent,
        mut formulario: FormularioEmpresa,
    ) -> AccionEmpresas {
        match key.code {
            KeyCode::Esc => {
                self.modo = ModoEmpresas::Normal;
                return AccionEmpresas::Ninguna;
            }
            KeyCode::Enter => {
                return match construir(&formulario) {
                    Ok(nombre) => match formulario.modo {
                        ModoFormularioEmpresa::Crear => AccionEmpresas::Crear { nombre },
                        ModoFormularioEmpresa::Editar { id } => {
                            AccionEmpresas::Actualizar { id, nombre }
                        }
                    },
                    Err(error) => {
                        formulario.error = Some(error);
                        self.modo = ModoEmpresas::Formulario(formulario);
                        AccionEmpresas::Ninguna
                    }
                };
            }
            _ => {
                if formulario.nombre.handle_key(key) {
                    formulario.error = None;
                }
            }
        }
        self.modo = ModoEmpresas::Formulario(formulario);
        AccionEmpresas::Ninguna
    }

    fn handle_confirmacion(&mut self, key: KeyEvent, c: ConfirmacionEstado) -> AccionEmpresas {
        match key.code {
            KeyCode::Enter => {
                let nombre = self
                    .empresa(c.id)
                    .map(|e| e.nombre.clone())
                    .unwrap_or_default();
                AccionEmpresas::EstablecerActivo {
                    id: c.id,
                    activar: c.activar,
                    nombre,
                }
            }
            KeyCode::Esc => {
                self.modo = ModoEmpresas::Normal;
                AccionEmpresas::Ninguna
            }
            _ => AccionEmpresas::Ninguna,
        }
    }

    fn solicitar_estado(&mut self, id: i64) {
        if let Some(e) = self.empresa(id) {
            self.modo = ModoEmpresas::ConfirmacionEstado(ConfirmacionEstado {
                id,
                activar: !e.activo,
            })
        }
    }

    fn accion_buscar(&self, seleccionar_id: Option<i64>) -> AccionEmpresas {
        AccionEmpresas::Buscar {
            texto: (!self.filtro.trim().is_empty()).then(|| self.filtro.clone()),
            seleccionar_id,
        }
    }
    fn error_formulario(&mut self, error: String) {
        if let ModoEmpresas::Formulario(f) = &mut self.modo {
            f.error = Some(error);
        }
    }
    fn abrir_edicion(&mut self, id: i64) {
        if let Some(e) = self.empresa(id) {
            self.modo = ModoEmpresas::Formulario(FormularioEmpresa {
                modo: ModoFormularioEmpresa::Editar { id },
                nombre: TextInput::new(e.nombre.clone()).with_max_chars(80),
                error: None,
            });
        }
    }
    fn mover(&mut self, delta: isize) {
        let n = self.empresas.len();
        self.seleccion = if n == 0 {
            None
        } else {
            let a = self.seleccion.unwrap_or(0);
            Some(if delta < 0 {
                a.saturating_sub(1)
            } else {
                (a + 1).min(n - 1)
            })
        };
    }
    fn id_seleccionado(&self) -> Option<i64> {
        Some(self.empresas.get(self.seleccion?)?.id)
    }
    fn empresa(&self, id: i64) -> Option<&EmpresaResumen> {
        self.empresas.iter().find(|e| e.id == id)
    }
    fn inicio_visible(&self, capacidad: usize) -> usize {
        self.seleccion
            .unwrap_or(0)
            .saturating_sub(capacidad.saturating_sub(1))
    }
}

/// Valida antes de despachar la acción, igual que `contratistas::construir` —
/// antes esta pantalla enviaba `nombre` tal cual y dejaba toda la validación
/// al service, la única de las 3 pantallas CRUD que lo hacía así.
fn construir(f: &FormularioEmpresa) -> Result<String, String> {
    if f.nombre.value().trim().is_empty() {
        return Err("El nombre es obligatorio".into());
    }
    Ok(f.nombre.value().trim().to_string())
}
