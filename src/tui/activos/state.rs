use crate::services::registro_ingreso_service::IngresoActivoResumen;
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
        id: i64,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModoActivos {
    Normal,
    Busqueda { texto: String },
    Detalle { id: i64 },
    ConfirmarSalida { id: i64 },
    SalidaPorGafete(SalidaGafete),
    Columnas { seleccion: usize },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionActivos {
    Ninguna,
    Volver,
    Buscar {
        texto: Option<String>,
        seleccionar_id: Option<i64>,
    },
    BuscarPorGafete {
        numero: i64,
    },
    RegistrarSalida {
        registro_id: i64,
        nombre: String,
    },
}
#[derive(Debug)]
pub struct ActivosState {
    registros: Vec<IngresoActivoResumen>,
    seleccion: Option<usize>,
    modo: ModoActivos,
    columnas: Vec<(Columna, bool)>,
    mensaje: Option<String>,
    pub(crate) filtro: String,
    usuario_nombre: String,
}
impl Default for ActivosState {
    fn default() -> Self {
        Self {
            registros: vec![],
            seleccion: None,
            modo: ModoActivos::Normal,
            columnas: Columna::TODAS
                .into_iter()
                .map(|c| (c, !matches!(c, Columna::Medio | Columna::Usuario)))
                .collect(),
            mensaje: None,
            filtro: String::new(),
            usuario_nombre: "Quintana".into(),
        }
    }
}
impl ActivosState {
    pub fn set_usuario_nombre(&mut self, n: impl Into<String>) {
        self.usuario_nombre = n.into()
    }
    pub fn solicitud_carga(&self) -> AccionActivos {
        AccionActivos::Buscar {
            texto: texto_filtro(&self.filtro),
            seleccionar_id: self.id_seleccionado(),
        }
    }
    pub fn cantidad(&self) -> usize {
        self.registros.len()
    }
    pub fn modo(&self) -> &ModoActivos {
        &self.modo
    }
    pub fn completar_busqueda(
        &mut self,
        r: Result<Vec<IngresoActivoResumen>, String>,
        id: Option<i64>,
    ) {
        match r {
            Ok(v) => {
                self.registros = v;
                self.seleccion = id
                    .and_then(|x| self.registros.iter().position(|r| r.registro_id == x))
                    .or((!self.registros.is_empty()).then_some(0))
            }
            Err(e) => {
                self.registros.clear();
                self.seleccion = None;
                self.mensaje = Some(e)
            }
        }
    }
    pub fn completar_gafete(
        &mut self,
        r: Result<i64, String>,
        registros: Option<Vec<IngresoActivoResumen>>,
    ) {
        match r {
            Ok(id) => {
                if let Some(v) = registros {
                    self.registros = v;
                    self.filtro.clear()
                }
                self.seleccion = self.registros.iter().position(|x| x.registro_id == id);
                if self.seleccion.is_some() {
                    self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Encontrado { id })
                } else {
                    self.modo = ModoActivos::Normal;
                    self.mensaje = Some("No existe un ingreso activo con ese gafete".into())
                }
            }
            Err(e) => {
                if let ModoActivos::SalidaPorGafete(SalidaGafete::Capturando { error, .. }) =
                    &mut self.modo
                {
                    *error = Some(e)
                }
            }
        }
    }
    pub fn completar_salida(
        &mut self,
        r: Result<(), String>,
        id: i64,
        nombre: &str,
    ) -> AccionActivos {
        self.modo = ModoActivos::Normal;
        match r {
            Ok(()) => {
                self.mensaje = Some(format!("✓ Salida registrada — {nombre}"));
                AccionActivos::Buscar {
                    texto: texto_filtro(&self.filtro),
                    seleccionar_id: Some(id),
                }
            }
            Err(e) => {
                self.mensaje = Some(e);
                AccionActivos::Buscar {
                    texto: texto_filtro(&self.filtro),
                    seleccionar_id: None,
                }
            }
        }
    }
    pub fn handle_key(&mut self, k: KeyEvent) -> AccionActivos {
        match self.modo.clone() {
            ModoActivos::Normal => self.normal(k),
            ModoActivos::Busqueda { .. } => self.busqueda(k),
            ModoActivos::Detalle { id } => match k.code {
                KeyCode::Char('s' | 'S') => {
                    self.modo = ModoActivos::ConfirmarSalida { id };
                    AccionActivos::Ninguna
                }
                KeyCode::Esc => {
                    self.modo = ModoActivos::Normal;
                    AccionActivos::Ninguna
                }
                _ => AccionActivos::Ninguna,
            },
            ModoActivos::ConfirmarSalida { id } => self.confirmar(k, id),
            ModoActivos::SalidaPorGafete(s) => self.gafete(k, s),
            ModoActivos::Columnas { seleccion } => {
                self.columnas(k, seleccion);
                AccionActivos::Ninguna
            }
        }
    }
    fn normal(&mut self, k: KeyEvent) -> AccionActivos {
        self.mensaje = None;
        match k.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => {
                if let Some(id) = self.id_seleccionado() {
                    self.modo = ModoActivos::Detalle { id }
                }
            }
            KeyCode::Char('s' | 'S') => {
                if let Some(id) = self.id_seleccionado() {
                    self.modo = ModoActivos::ConfirmarSalida { id }
                }
            }
            KeyCode::F(2) => {
                self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                    numero: String::new(),
                    error: None,
                })
            }
            KeyCode::Char('/') => {
                self.modo = ModoActivos::Busqueda {
                    texto: self.filtro.clone(),
                }
            }
            KeyCode::Char('c' | 'C') | KeyCode::F(6) => {
                self.modo = ModoActivos::Columnas { seleccion: 0 }
            }
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                return AccionActivos::Buscar {
                    texto: None,
                    seleccionar_id: None,
                };
            }
            KeyCode::Esc => return AccionActivos::Volver,
            _ => {}
        }
        AccionActivos::Ninguna
    }
    fn busqueda(&mut self, k: KeyEvent) -> AccionActivos {
        match k.code {
            KeyCode::Esc => {
                self.filtro.clear();
                self.modo = ModoActivos::Normal;
                AccionActivos::Buscar {
                    texto: None,
                    seleccionar_id: None,
                }
            }
            KeyCode::Enter => {
                self.modo = ModoActivos::Normal;
                AccionActivos::Ninguna
            }
            KeyCode::Up => {
                self.mover(-1);
                AccionActivos::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionActivos::Ninguna
            }
            KeyCode::Backspace => {
                if let ModoActivos::Busqueda { texto } = &mut self.modo {
                    texto.pop();
                    self.filtro = texto.clone()
                }
                AccionActivos::Buscar {
                    texto: texto_filtro(&self.filtro),
                    seleccionar_id: None,
                }
            }
            KeyCode::Char(c)
                if !k
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let ModoActivos::Busqueda { texto } = &mut self.modo {
                    texto.push(c);
                    self.filtro = texto.clone()
                }
                AccionActivos::Buscar {
                    texto: texto_filtro(&self.filtro),
                    seleccionar_id: None,
                }
            }
            _ => AccionActivos::Ninguna,
        }
    }
    fn confirmar(&mut self, k: KeyEvent, id: i64) -> AccionActivos {
        match k.code {
            KeyCode::Char('y' | 'Y') => {
                let nombre = self
                    .registro(id)
                    .map(|r| r.contratista_nombre.clone())
                    .unwrap_or_default();
                AccionActivos::RegistrarSalida {
                    registro_id: id,
                    nombre,
                }
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.modo = ModoActivos::Normal;
                AccionActivos::Ninguna
            }
            _ => AccionActivos::Ninguna,
        }
    }
    fn gafete(&mut self, k: KeyEvent, s: SalidaGafete) -> AccionActivos {
        match s {
            SalidaGafete::Capturando { mut numero, .. } => match k.code {
                KeyCode::Esc => {
                    self.modo = ModoActivos::Normal;
                    AccionActivos::Ninguna
                }
                KeyCode::Backspace => {
                    numero.pop();
                    self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                        numero,
                        error: None,
                    });
                    AccionActivos::Ninguna
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    numero.push(c);
                    self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                        numero,
                        error: None,
                    });
                    AccionActivos::Ninguna
                }
                KeyCode::Enter => match numero.parse::<i64>() {
                    Ok(n) => AccionActivos::BuscarPorGafete { numero: n },
                    Err(_) => {
                        self.modo = ModoActivos::SalidaPorGafete(SalidaGafete::Capturando {
                            numero,
                            error: Some("Ingrese un número de gafete válido".into()),
                        });
                        AccionActivos::Ninguna
                    }
                },
                _ => AccionActivos::Ninguna,
            },
            SalidaGafete::Encontrado { id } => self.confirmar(k, id),
        }
    }
    fn columnas(&mut self, k: KeyEvent, s: usize) {
        let u = self.columnas.len() - 1;
        match k.code {
            KeyCode::Up => {
                self.modo = ModoActivos::Columnas {
                    seleccion: s.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                self.modo = ModoActivos::Columnas {
                    seleccion: (s + 1).min(u),
                }
            }
            KeyCode::Char(' ') => {
                let n = self.columnas.iter().filter(|x| x.1).count();
                if self.columnas[s].1 && n == 1 {
                    self.mensaje = Some("Debe conservar al menos una columna".into())
                } else {
                    self.columnas[s].1 = !self.columnas[s].1
                }
            }
            KeyCode::Esc => self.modo = ModoActivos::Normal,
            _ => {}
        }
    }
    fn mover(&mut self, d: isize) {
        if self.registros.is_empty() {
            self.seleccion = None
        } else {
            let i = self.seleccion.unwrap_or(0);
            self.seleccion = Some(if d < 0 {
                i.saturating_sub(1)
            } else {
                (i + 1).min(self.registros.len() - 1)
            })
        }
    }
    fn id_seleccionado(&self) -> Option<i64> {
        self.registros.get(self.seleccion?).map(|r| r.registro_id)
    }
    fn registro(&self, id: i64) -> Option<&IngresoActivoResumen> {
        self.registros.iter().find(|r| r.registro_id == id)
    }
    pub fn inicio_visible(&self, c: usize) -> usize {
        self.seleccion
            .unwrap_or(0)
            .saturating_sub(c.saturating_sub(1))
    }
}
fn texto_filtro(s: &str) -> Option<String> {
    (!s.trim().is_empty()).then(|| s.to_owned())
}
