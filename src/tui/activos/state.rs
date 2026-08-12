use crate::tui::{mock, mock::IngresoActivoMock};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Constraint;

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Columna {
    Cedula,
    Nombre,
    Empresa,
    Tipo,
    Hora,
    Gafete,
    Medio,
    Usuario,
}

impl Columna {
    const TODAS: [Self; 8] = [
        Self::Cedula,
        Self::Nombre,
        Self::Empresa,
        Self::Tipo,
        Self::Hora,
        Self::Gafete,
        Self::Medio,
        Self::Usuario,
    ];

    fn titulo(self) -> &'static str {
        match self {
            Self::Cedula => "CÉDULA",
            Self::Nombre => "NOMBRE",
            Self::Empresa => "EMPRESA",
            Self::Tipo => "TIPO",
            Self::Hora => "HORA",
            Self::Gafete => "GAFETE",
            Self::Medio => "MEDIO",
            Self::Usuario => "USUARIO INGRESO",
        }
    }

    fn constraint(self) -> Constraint {
        match self {
            Self::Nombre | Self::Empresa => Constraint::Fill(3),
            Self::Cedula | Self::Tipo | Self::Usuario => Constraint::Fill(2),
            Self::Medio => Constraint::Fill(1),
            Self::Hora => Constraint::Length(7),
            Self::Gafete => Constraint::Length(8),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SalidaGafete {
    Capturando {
        numero: String,
        error: Option<String>,
    },
    Encontrado {
        id: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModoActivos {
    Normal,
    Busqueda { texto: String },
    Detalle { id: u64 },
    ConfirmarSalida { id: u64 },
    SalidaPorGafete(SalidaGafete),
    Columnas { seleccion: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionActivos {
    Ninguna,
    Volver,
    IrHistorial,
    IrContratistas,
}

#[derive(Debug)]
pub struct ActivosState {
    registros: Vec<IngresoActivoMock>,
    seleccion: Option<usize>,
    modo: ModoActivos,
    columnas: Vec<(Columna, bool)>,
    mensaje: Option<String>,
    filtro: String,
}

impl Default for ActivosState {
    fn default() -> Self {
        Self {
            registros: mock::ingresos_activos(),
            seleccion: Some(0),
            modo: ModoActivos::Normal,
            columnas: Columna::TODAS
                .into_iter()
                .map(|columna| {
                    (
                        columna,
                        !matches!(columna, Columna::Medio | Columna::Usuario),
                    )
                })
                .collect(),
            mensaje: None,
            filtro: String::new(),
        }
    }
}

impl ActivosState {
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionActivos {
        match self.modo.clone() {
            ModoActivos::Normal => self.handle_normal(key),
            ModoActivos::Busqueda { .. } => self.handle_busqueda(key),
            ModoActivos::Detalle { id } => self.handle_detalle(key, id),
            ModoActivos::ConfirmarSalida { id } => self.handle_confirmacion(key, id),
            ModoActivos::SalidaPorGafete(estado) => self.handle_gafete(key, estado),
            ModoActivos::Columnas { seleccion } => self.handle_columnas(key, seleccion),
        }
    }

    pub fn cantidad(&self) -> usize {
        self.registros.len()
    }

    pub fn modo(&self) -> &ModoActivos {
        &self.modo
    }

    pub fn indices_filtrados(&self) -> Vec<usize> {
        let texto = self.filtro.trim().to_lowercase();
        self.registros
            .iter()
            .enumerate()
            .filter(|(_, registro)| {
                texto.is_empty()
                    || registro.nombre.to_lowercase().contains(&texto)
                    || registro.cedula.to_lowercase().contains(&texto)
                    || registro.empresa.to_lowercase().contains(&texto)
                    || registro
                        .gafete
                        .is_some_and(|gafete| gafete.to_string().contains(&texto))
            })
            .map(|(indice, _)| indice)
            .collect()
    }

    pub fn inicio_visible(&self, capacidad: usize) -> usize {
        if capacidad == 0 {
            return 0;
        }
        self.seleccion.unwrap_or(0).saturating_sub(capacidad - 1)
    }

    fn handle_normal(&mut self, key: KeyEvent) -> AccionActivos {
        self.mensaje = None;
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => {
                if let Some(id) = self.id_seleccionado() {
                    self.modo = ModoActivos::Detalle { id };
                }
            }
            KeyCode::Char('s' | 'S') => self.abrir_confirmacion(),
            KeyCode::F(2) => {
                self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                    numero: String::new(),
                    error: None,
                });
            }
            KeyCode::Char('/') => {
                self.modo = ModoActivos::Busqueda {
                    texto: self.filtro.clone(),
                };
                self.seleccion = (!self.registros.is_empty()).then_some(0);
            }
            KeyCode::Char('c' | 'C') | KeyCode::F(6) => {
                self.modo = ModoActivos::Columnas { seleccion: 0 }
            }
            // Navegación temporal del prototipo; desaparecerá al existir MenuPrincipal.
            KeyCode::Char('h' | 'H') => return AccionActivos::IrHistorial,
            // Navegación temporal oculta hacia Base de Contratistas.
            KeyCode::Char('b' | 'B') => return AccionActivos::IrContratistas,
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                self.seleccion = (!self.registros.is_empty()).then_some(0);
            }
            KeyCode::Esc => return AccionActivos::Volver,
            _ => {}
        }
        AccionActivos::Ninguna
    }

    fn handle_busqueda(&mut self, key: KeyEvent) -> AccionActivos {
        match key.code {
            KeyCode::Esc => {
                self.filtro.clear();
                self.modo = ModoActivos::Normal;
                self.seleccion = (!self.registros.is_empty()).then_some(0);
            }
            KeyCode::Enter => self.modo = ModoActivos::Normal,
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Backspace => {
                if let ModoActivos::Busqueda { texto } = &mut self.modo {
                    texto.pop();
                    self.filtro = texto.clone();
                }
                self.ajustar_seleccion_filtro();
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let ModoActivos::Busqueda { texto } = &mut self.modo {
                    texto.push(character);
                    self.filtro = texto.clone();
                }
                self.ajustar_seleccion_filtro();
            }
            _ => {}
        }
        AccionActivos::Ninguna
    }

    fn handle_detalle(&mut self, key: KeyEvent, id: u64) -> AccionActivos {
        match key.code {
            KeyCode::Char('s' | 'S') => self.modo = ModoActivos::ConfirmarSalida { id },
            KeyCode::Esc => self.modo = ModoActivos::Normal,
            _ => {}
        }
        AccionActivos::Ninguna
    }

    fn handle_confirmacion(&mut self, key: KeyEvent, id: u64) -> AccionActivos {
        match key.code {
            KeyCode::Char('y' | 'Y') => self.eliminar(id),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => self.modo = ModoActivos::Normal,
            _ => {}
        }
        AccionActivos::Ninguna
    }

    fn handle_gafete(&mut self, key: KeyEvent, estado: SalidaGafete) -> AccionActivos {
        match estado {
            SalidaGafete::Capturando { mut numero, .. } => match key.code {
                KeyCode::Esc => self.modo = ModoActivos::Normal,
                KeyCode::Backspace => {
                    numero.pop();
                    self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                        numero,
                        error: None,
                    });
                }
                KeyCode::Char(character) if character.is_ascii_digit() => {
                    numero.push(character);
                    self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                        numero,
                        error: None,
                    });
                }
                KeyCode::Enter => {
                    let gafete = numero.parse::<u32>().ok();
                    if let Some(registro) = self.registros.iter().find(|r| r.gafete == gafete) {
                        self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Encontrado {
                            id: registro.id,
                        });
                    } else {
                        let error = format!(
                            "El gafete {} no está asignado a ningún ingreso activo",
                            if numero.is_empty() { "—" } else { &numero }
                        );
                        self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                            numero,
                            error: Some(error),
                        });
                    }
                }
                _ => {}
            },
            SalidaGafete::Encontrado { id } => match key.code {
                KeyCode::Char('y' | 'Y') => self.eliminar(id),
                KeyCode::Char('n' | 'N') | KeyCode::Esc => self.modo = ModoActivos::Normal,
                _ => {}
            },
        }
        AccionActivos::Ninguna
    }

    fn handle_columnas(&mut self, key: KeyEvent, seleccion: usize) -> AccionActivos {
        let ultimo = self.columnas.len() - 1;
        match key.code {
            KeyCode::Up => {
                self.modo = ModoActivos::Columnas {
                    seleccion: seleccion.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                self.modo = ModoActivos::Columnas {
                    seleccion: (seleccion + 1).min(ultimo),
                }
            }
            KeyCode::Char(' ') => {
                let visibles = self.columnas.iter().filter(|(_, visible)| *visible).count();
                if self.columnas[seleccion].1 && visibles == 1 {
                    self.mensaje = Some("Debe conservar al menos una columna".to_owned());
                } else {
                    self.columnas[seleccion].1 = !self.columnas[seleccion].1;
                    self.mensaje = None;
                }
            }
            KeyCode::Esc => self.modo = ModoActivos::Normal,
            _ => {}
        }
        AccionActivos::Ninguna
    }

    fn mover(&mut self, delta: isize) {
        let indices = self.indices_filtrados();
        if indices.is_empty() {
            self.seleccion = None;
            return;
        }
        let actual = self.seleccion.unwrap_or(0);
        self.seleccion = Some(if delta < 0 {
            actual.saturating_sub(1)
        } else {
            (actual + 1).min(indices.len() - 1)
        });
    }

    fn ajustar_seleccion_filtro(&mut self) {
        let cantidad = self.indices_filtrados().len();
        self.seleccion = if cantidad == 0 {
            None
        } else {
            Some(self.seleccion.unwrap_or(0).min(cantidad - 1))
        };
    }

    fn id_seleccionado(&self) -> Option<u64> {
        let indice = *self.indices_filtrados().get(self.seleccion?)?;
        Some(self.registros[indice].id)
    }

    fn abrir_confirmacion(&mut self) {
        if let Some(id) = self.id_seleccionado() {
            self.modo = ModoActivos::ConfirmarSalida { id };
        }
    }

    fn eliminar(&mut self, id: u64) {
        let Some(indice) = self.registros.iter().position(|registro| registro.id == id) else {
            self.modo = ModoActivos::Normal;
            return;
        };
        let eliminado = self.registros.remove(indice);
        self.mensaje = Some(match eliminado.gafete {
            Some(gafete) => format!(
                "✓ Salida registrada — {} — Gafete {:02} liberado",
                eliminado.nombre, gafete
            ),
            None => format!("✓ Salida registrada — {}", eliminado.nombre),
        });
        self.modo = ModoActivos::Normal;
        self.seleccion = if self.registros.is_empty() {
            None
        } else {
            Some(self.seleccion.unwrap_or(0).min(self.registros.len() - 1))
        };
    }

    fn registro(&self, id: u64) -> Option<&IngresoActivoMock> {
        self.registros.iter().find(|registro| registro.id == id)
    }
}
