use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::database::queries::empresas::EmpresaResumen;
use crate::tui::ui_kit::{Debounce, StandardCommand, standard_command};
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
    nombre: String,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoEmpresas {
    Normal,
    Busqueda { texto: String },
    Formulario(FormularioEmpresa),
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
}

#[derive(Debug)]
pub struct EmpresasState {
    empresas: Vec<EmpresaResumen>,
    seleccion: Option<usize>,
    modo: ModoEmpresas,
    filtro: String,
    mensaje: Option<String>,
    error_carga: Option<String>,
    usuario_nombre: String,
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
            error_carga: None,
            usuario_nombre: "Quintana".into(),
            ayuda_expandida: false,
            busqueda_debounce: Debounce::default(),
        }
    }
}

impl EmpresasState {
    pub fn set_usuario_nombre(&mut self, nombre: impl Into<String>) {
        self.usuario_nombre = nombre.into();
    }

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
                self.error_carga = None;
                self.seleccion = seleccionar_id
                    .and_then(|id| self.empresas.iter().position(|e| e.id == id))
                    .or_else(|| (!self.empresas.is_empty()).then_some(0));
            }
            Err(error) => {
                self.empresas.clear();
                self.seleccion = None;
                self.error_carga = Some(error);
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
                self.mensaje = Some(format!("✓ Empresa actualizada — {}", nombre.trim()));
                self.accion_buscar(Some(id))
            }
            Err(error) => {
                self.error_formulario(error);
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
                    nombre: String::new(),
                    error: None,
                })
            }
            KeyCode::Char('/') => {
                self.modo = ModoEmpresas::Busqueda {
                    texto: self.filtro.clone(),
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
            KeyCode::Backspace => {
                if let ModoEmpresas::Busqueda { texto } = &mut self.modo {
                    texto.pop();
                    self.filtro = texto.clone();
                }
                self.seleccion = None;
                self.busqueda_debounce.marcar(Instant::now());
                AccionEmpresas::Ninguna
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let ModoEmpresas::Busqueda { texto } = &mut self.modo {
                    texto.push(c);
                    self.filtro = texto.clone();
                }
                self.seleccion = None;
                self.busqueda_debounce.marcar(Instant::now());
                AccionEmpresas::Ninguna
            }
            _ => AccionEmpresas::Ninguna,
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
            KeyCode::Backspace => {
                formulario.nombre.pop();
                formulario.error = None;
            }
            KeyCode::Enter => {
                return match formulario.modo {
                    ModoFormularioEmpresa::Crear => AccionEmpresas::Crear {
                        nombre: formulario.nombre,
                    },
                    ModoFormularioEmpresa::Editar { id } => AccionEmpresas::Actualizar {
                        id,
                        nombre: formulario.nombre,
                    },
                };
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && formulario.nombre.chars().count() < 80 =>
            {
                formulario.nombre.push(c);
                formulario.error = None;
            }
            _ => {}
        }
        self.modo = ModoEmpresas::Formulario(formulario);
        AccionEmpresas::Ninguna
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
                nombre: e.nombre.clone(),
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
