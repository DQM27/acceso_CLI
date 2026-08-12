use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Constraint;

use crate::tui::{historial_mock, historial_mock::MovimientoHistorialMock};

#[path = "filtros.rs"]
mod filtros;
pub use filtros::*;

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
use render::valor;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnaHistorial {
    Fecha,
    Cedula,
    Nombre,
    Empresa,
    Tipo,
    Entrada,
    Salida,
    Gafete,
    Medio,
    UsuarioIngreso,
    UsuarioSalida,
}

impl ColumnaHistorial {
    const TODAS: [Self; 11] = [
        Self::Fecha,
        Self::Cedula,
        Self::Nombre,
        Self::Empresa,
        Self::Tipo,
        Self::Entrada,
        Self::Salida,
        Self::Gafete,
        Self::Medio,
        Self::UsuarioIngreso,
        Self::UsuarioSalida,
    ];

    fn titulo(self) -> &'static str {
        match self {
            Self::Fecha => "FECHA",
            Self::Cedula => "CÉDULA",
            Self::Nombre => "NOMBRE",
            Self::Empresa => "EMPRESA",
            Self::Tipo => "TIPO",
            Self::Entrada => "ENTRADA",
            Self::Salida => "SALIDA",
            Self::Gafete => "GAFETE",
            Self::Medio => "MEDIO",
            Self::UsuarioIngreso => "USUARIO INGRESO",
            Self::UsuarioSalida => "USUARIO SALIDA",
        }
    }

    fn constraint(self) -> Constraint {
        match self {
            Self::Fecha => Constraint::Length(10),
            Self::Entrada | Self::Salida => Constraint::Length(8),
            Self::Gafete => Constraint::Length(7),
            Self::Nombre | Self::Empresa => Constraint::Fill(3),
            Self::Cedula | Self::Tipo | Self::UsuarioIngreso | Self::UsuarioSalida => {
                Constraint::Fill(2)
            }
            Self::Medio => Constraint::Fill(1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoHistorial {
    Normal,
    Busqueda {
        texto: String,
    },
    Detalle {
        id: u64,
    },
    Filtros {
        seleccion: usize,
        editando: bool,
    },
    Desplegable {
        campo: CampoFiltro,
        seleccion_filtro: usize,
        opcion: usize,
    },
    Columnas {
        seleccion: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionHistorial {
    Ninguna,
    Volver,
}

#[derive(Debug)]
pub struct HistorialState {
    registros: Vec<MovimientoHistorialMock>,
    seleccion: Option<usize>,
    modo: ModoHistorial,
    filtro_aplicado: FiltrosHistorial,
    filtro_edicion: FiltrosHistorial,
    busqueda: String,
    columnas: Vec<(ColumnaHistorial, bool)>,
    mensaje: Option<String>,
}

impl Default for HistorialState {
    fn default() -> Self {
        let filtro = FiltrosHistorial::default();
        let mut state = Self {
            registros: historial_mock::movimientos_historial(),
            seleccion: Some(0),
            modo: ModoHistorial::Normal,
            filtro_aplicado: filtro.clone(),
            filtro_edicion: filtro,
            busqueda: String::new(),
            columnas: ColumnaHistorial::TODAS
                .into_iter()
                .map(|c| {
                    (
                        c,
                        !matches!(
                            c,
                            ColumnaHistorial::Medio
                                | ColumnaHistorial::UsuarioIngreso
                                | ColumnaHistorial::UsuarioSalida
                        ),
                    )
                })
                .collect(),
            mensaje: None,
        };
        state.ajustar_seleccion();
        state
    }
}

impl HistorialState {
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionHistorial {
        match self.modo.clone() {
            ModoHistorial::Normal => self.normal(key),
            ModoHistorial::Busqueda { .. } => self.busqueda_key(key),
            ModoHistorial::Detalle { .. } => {
                if key.code == KeyCode::Esc {
                    self.modo = ModoHistorial::Normal;
                }
                AccionHistorial::Ninguna
            }
            ModoHistorial::Filtros {
                seleccion,
                editando,
            } => self.filtros_key(key, seleccion, editando),
            ModoHistorial::Desplegable {
                campo,
                seleccion_filtro,
                opcion,
            } => self.desplegable_key(key, campo, seleccion_filtro, opcion),
            ModoHistorial::Columnas { seleccion } => self.columnas_key(key, seleccion),
        }
    }

    fn normal(&mut self, key: KeyEvent) -> AccionHistorial {
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => {
                if let Some(id) = self.id_seleccionado() {
                    self.modo = ModoHistorial::Detalle { id };
                }
            }
            KeyCode::Char('/') => {
                self.modo = ModoHistorial::Busqueda {
                    texto: self.busqueda.clone(),
                }
            }
            KeyCode::Char('f' | 'F') => {
                self.filtro_edicion = self.filtro_aplicado.clone();
                self.modo = ModoHistorial::Filtros {
                    seleccion: 0,
                    editando: false,
                };
            }
            KeyCode::Char('c' | 'C') => self.modo = ModoHistorial::Columnas { seleccion: 0 },
            KeyCode::Esc if !self.busqueda.is_empty() => {
                self.busqueda.clear();
                self.ajustar_seleccion();
            }
            KeyCode::Esc => return AccionHistorial::Volver,
            _ => {}
        }
        AccionHistorial::Ninguna
    }

    fn busqueda_key(&mut self, key: KeyEvent) -> AccionHistorial {
        match key.code {
            KeyCode::Esc => {
                self.busqueda.clear();
                self.modo = ModoHistorial::Normal;
                self.ajustar_seleccion();
            }
            KeyCode::Enter => self.modo = ModoHistorial::Normal,
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Backspace => {
                if let ModoHistorial::Busqueda { texto } = &mut self.modo {
                    texto.pop();
                    self.busqueda = texto.clone();
                }
                self.ajustar_seleccion();
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let ModoHistorial::Busqueda { texto } = &mut self.modo {
                    texto.push(c);
                    self.busqueda = texto.clone();
                }
                self.ajustar_seleccion();
            }
            _ => {}
        }
        AccionHistorial::Ninguna
    }

    fn filtros_key(&mut self, key: KeyEvent, seleccion: usize, editando: bool) -> AccionHistorial {
        let campo = CampoFiltro::TODOS[seleccion];
        if editando {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.modo = ModoHistorial::Filtros {
                        seleccion,
                        editando: false,
                    }
                }
                KeyCode::Backspace => {
                    self.texto_filtro_mut(campo).map(String::pop);
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.agregar_caracter_filtro(campo, c);
                }
                _ => {}
            }
            return AccionHistorial::Ninguna;
        }
        match key.code {
            KeyCode::Up => {
                self.modo = ModoHistorial::Filtros {
                    seleccion: seleccion.saturating_sub(1),
                    editando: false,
                }
            }
            KeyCode::Down => {
                self.modo = ModoHistorial::Filtros {
                    seleccion: (seleccion + 1).min(6),
                    editando: false,
                }
            }
            KeyCode::Enter => {
                if matches!(
                    campo,
                    CampoFiltro::Empresa | CampoFiltro::Tipo | CampoFiltro::Estado
                ) {
                    self.abrir_desplegable(campo, seleccion);
                } else {
                    self.modo = ModoHistorial::Filtros {
                        seleccion,
                        editando: true,
                    };
                }
            }
            KeyCode::Char('a' | 'A') => {
                if !fechas_validas(&self.filtro_edicion) {
                    self.mensaje =
                        Some("Fechas inválidas. Use DD/MM/YYYY y verifique el rango".into());
                    return AccionHistorial::Ninguna;
                }
                self.filtro_aplicado = self.filtro_edicion.clone();
                self.modo = ModoHistorial::Normal;
                self.ajustar_seleccion();
            }
            KeyCode::Char('l' | 'L') => self.filtro_edicion = FiltrosHistorial::default(),
            KeyCode::Esc => self.modo = ModoHistorial::Normal,
            _ => {}
        }
        AccionHistorial::Ninguna
    }

    fn columnas_key(&mut self, key: KeyEvent, seleccion: usize) -> AccionHistorial {
        match key.code {
            KeyCode::Up => {
                self.modo = ModoHistorial::Columnas {
                    seleccion: seleccion.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                self.modo = ModoHistorial::Columnas {
                    seleccion: (seleccion + 1).min(self.columnas.len() - 1),
                }
            }
            KeyCode::Char(' ') => {
                let visibles = self.columnas.iter().filter(|(_, v)| *v).count();
                if self.columnas[seleccion].1 && visibles == 1 {
                    self.mensaje = Some("Debe conservar al menos una columna".into());
                } else {
                    self.columnas[seleccion].1 = !self.columnas[seleccion].1;
                    self.mensaje = None;
                }
            }
            KeyCode::Esc => self.modo = ModoHistorial::Normal,
            _ => {}
        }
        AccionHistorial::Ninguna
    }

    fn desplegable_key(
        &mut self,
        key: KeyEvent,
        campo: CampoFiltro,
        seleccion_filtro: usize,
        opcion: usize,
    ) -> AccionHistorial {
        let cantidad = opciones_campo(campo).len();
        match key.code {
            KeyCode::Up => {
                self.modo = ModoHistorial::Desplegable {
                    campo,
                    seleccion_filtro,
                    opcion: opcion.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                self.modo = ModoHistorial::Desplegable {
                    campo,
                    seleccion_filtro,
                    opcion: (opcion + 1).min(cantidad - 1),
                }
            }
            KeyCode::Enter => {
                self.asignar_opcion(campo, opcion);
                self.modo = ModoHistorial::Filtros {
                    seleccion: seleccion_filtro,
                    editando: false,
                };
            }
            KeyCode::Esc => {
                self.modo = ModoHistorial::Filtros {
                    seleccion: seleccion_filtro,
                    editando: false,
                }
            }
            _ => {}
        }
        AccionHistorial::Ninguna
    }

    fn texto_filtro_mut(&mut self, campo: CampoFiltro) -> Option<&mut String> {
        match campo {
            CampoFiltro::Desde => Some(&mut self.filtro_edicion.desde),
            CampoFiltro::Hasta => Some(&mut self.filtro_edicion.hasta),
            CampoFiltro::NombreCedula => Some(&mut self.filtro_edicion.nombre_cedula),
            CampoFiltro::Gafete => Some(&mut self.filtro_edicion.gafete),
            _ => None,
        }
    }

    fn agregar_caracter_filtro(&mut self, campo: CampoFiltro, caracter: char) {
        let Some(texto) = self.texto_filtro_mut(campo) else {
            return;
        };
        match campo {
            CampoFiltro::Desde | CampoFiltro::Hasta if caracter.is_ascii_digit() => {
                let digitos = texto.chars().filter(char::is_ascii_digit).count();
                if digitos < 8 {
                    if matches!(digitos, 2 | 4) {
                        texto.push('/');
                    }
                    texto.push(caracter);
                }
            }
            CampoFiltro::Gafete if caracter.is_ascii_digit() && texto.len() < 3 => {
                texto.push(caracter)
            }
            CampoFiltro::NombreCedula if texto.chars().count() < 40 && !caracter.is_control() => {
                texto.push(caracter)
            }
            _ => {}
        }
    }

    fn abrir_desplegable(&mut self, campo: CampoFiltro, seleccion_filtro: usize) {
        let actual = match campo {
            CampoFiltro::Empresa => self.filtro_edicion.empresa.as_str(),
            CampoFiltro::Tipo => self.filtro_edicion.tipo.as_str(),
            CampoFiltro::Estado => estado_texto(self.filtro_edicion.estado),
            _ => return,
        };
        let opcion = opciones_campo(campo)
            .iter()
            .position(|valor| *valor == actual)
            .unwrap_or(0);
        self.modo = ModoHistorial::Desplegable {
            campo,
            seleccion_filtro,
            opcion,
        };
    }

    fn asignar_opcion(&mut self, campo: CampoFiltro, opcion: usize) {
        let valor = opciones_campo(campo)[opcion];
        match campo {
            CampoFiltro::Empresa => self.filtro_edicion.empresa = valor.into(),
            CampoFiltro::Tipo => self.filtro_edicion.tipo = valor.into(),
            CampoFiltro::Estado => {
                self.filtro_edicion.estado = match valor {
                    "Cerrados" => EstadoFiltro::Cerrados,
                    "Activos" => EstadoFiltro::Activos,
                    _ => EstadoFiltro::Todos,
                }
            }
            _ => {}
        }
    }

    fn indices_filtrados(&self) -> Vec<usize> {
        let desde = fecha(&self.filtro_aplicado.desde);
        let hasta = fecha(&self.filtro_aplicado.hasta);
        let nombre = self.filtro_aplicado.nombre_cedula.to_lowercase();
        let rapido = self.busqueda.to_lowercase();
        self.registros
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                desde.is_none_or(|d| r.fecha >= d)
                    && hasta.is_none_or(|h| r.fecha <= h)
                    && (nombre.is_empty()
                        || r.nombre.to_lowercase().contains(&nombre)
                        || r.cedula.contains(&nombre))
                    && (self.filtro_aplicado.empresa == "Todas"
                        || r.empresa == self.filtro_aplicado.empresa)
                    && (self.filtro_aplicado.tipo == "Todos" || r.tipo == self.filtro_aplicado.tipo)
                    && (self.filtro_aplicado.gafete.is_empty()
                        || r.gafete
                            .is_some_and(|g| g.to_string().contains(&self.filtro_aplicado.gafete)))
                    && match self.filtro_aplicado.estado {
                        EstadoFiltro::Todos => true,
                        EstadoFiltro::Cerrados => r.salida.is_some(),
                        EstadoFiltro::Activos => r.salida.is_none(),
                    }
                    && (rapido.is_empty()
                        || r.nombre.to_lowercase().contains(&rapido)
                        || r.cedula.contains(&rapido)
                        || r.empresa.to_lowercase().contains(&rapido)
                        || r.gafete.is_some_and(|g| g.to_string().contains(&rapido)))
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn mover(&mut self, delta: isize) {
        let n = self.indices_filtrados().len();
        self.seleccion = if n == 0 {
            None
        } else {
            Some(if delta < 0 {
                self.seleccion.unwrap_or(0).saturating_sub(1)
            } else {
                (self.seleccion.unwrap_or(0) + 1).min(n - 1)
            })
        };
    }
    fn ajustar_seleccion(&mut self) {
        let n = self.indices_filtrados().len();
        self.seleccion = if n == 0 {
            None
        } else {
            Some(self.seleccion.unwrap_or(0).min(n - 1))
        };
    }
    fn id_seleccionado(&self) -> Option<u64> {
        let i = *self.indices_filtrados().get(self.seleccion?)?;
        Some(self.registros[i].id)
    }
    fn registro(&self, id: u64) -> Option<&MovimientoHistorialMock> {
        self.registros.iter().find(|r| r.id == id)
    }
    fn inicio_visible(&self, capacidad: usize) -> usize {
        self.seleccion
            .unwrap_or(0)
            .saturating_sub(capacidad.saturating_sub(1))
    }
}
