use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Constraint;

use crate::tui::contratistas_mock::{self, ContratistaMock, EMPRESAS, TipoIngresoMock};

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Columna {
    Cedula,
    Nombre,
    Empresa,
    Tipo,
    Praind,
    Ruta,
    Acceso,
}

impl Columna {
    const TODAS: [Self; 7] = [
        Self::Cedula,
        Self::Nombre,
        Self::Empresa,
        Self::Tipo,
        Self::Praind,
        Self::Ruta,
        Self::Acceso,
    ];
    fn titulo(self) -> &'static str {
        match self {
            Self::Cedula => "CÉDULA",
            Self::Nombre => "NOMBRE",
            Self::Empresa => "EMPRESA",
            Self::Tipo => "TIPO",
            Self::Praind => "PRAIND",
            Self::Ruta => "RUTA",
            Self::Acceso => "ACCESO",
        }
    }
    fn constraint(self) -> Constraint {
        match self {
            Self::Nombre | Self::Empresa => Constraint::Fill(3),
            Self::Cedula | Self::Tipo => Constraint::Fill(2),
            Self::Praind => Constraint::Length(11),
            Self::Ruta | Self::Acceso => Constraint::Length(8),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CampoFormulario {
    Cedula,
    Nombre,
    Empresa,
    Tipo,
    FechaPraind,
    Ruta,
    Acceso,
}
impl CampoFormulario {
    const TODOS: [Self; 7] = [
        Self::Cedula,
        Self::Nombre,
        Self::Empresa,
        Self::Tipo,
        Self::FechaPraind,
        Self::Ruta,
        Self::Acceso,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModoFormulario {
    Crear,
    Editar { id: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Desplegable {
    Empresa,
    Tipo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FormularioContratista {
    modo: ModoFormulario,
    cedula: String,
    nombre: String,
    empresa: usize,
    tipo: usize,
    fecha_praind: String,
    personal_ruta: bool,
    tiene_acceso: bool,
    campo: usize,
    desplegable: Option<(Desplegable, usize)>,
    error: Option<String>,
}

impl FormularioContratista {
    fn nuevo() -> Self {
        Self {
            modo: ModoFormulario::Crear,
            cedula: String::new(),
            nombre: String::new(),
            empresa: 0,
            tipo: 0,
            fecha_praind: String::new(),
            personal_ruta: false,
            tiene_acceso: true,
            campo: 0,
            desplegable: None,
            error: None,
        }
    }
    fn editar(c: &ContratistaMock) -> Self {
        Self {
            modo: ModoFormulario::Editar { id: c.id },
            cedula: c.cedula.clone(),
            nombre: c.nombre.clone(),
            empresa: EMPRESAS.iter().position(|e| *e == c.empresa).unwrap_or(0),
            tipo: TipoIngresoMock::TODOS
                .iter()
                .position(|t| *t == c.tipo_ingreso)
                .unwrap_or(0),
            fecha_praind: c.fecha_praind.clone().unwrap_or_default(),
            personal_ruta: c.personal_ruta,
            tiene_acceso: c.tiene_acceso,
            campo: 0,
            desplegable: None,
            error: None,
        }
    }
    fn tipo(&self) -> TipoIngresoMock {
        TipoIngresoMock::TODOS[self.tipo]
    }
    fn requiere_praind(&self) -> bool {
        self.tipo().requiere_praind(self.personal_ruta)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoContratistas {
    Normal,
    Busqueda { texto: String },
    Detalle { id: u64 },
    Formulario(FormularioContratista),
    Columnas { seleccion: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionContratistas {
    Ninguna,
    Volver,
}

#[derive(Debug)]
pub struct ContratistasState {
    registros: Vec<ContratistaMock>,
    seleccion: Option<usize>,
    modo: ModoContratistas,
    columnas: Vec<(Columna, bool)>,
    filtro: String,
    mensaje: Option<String>,
}

impl Default for ContratistasState {
    fn default() -> Self {
        Self {
            registros: contratistas_mock::contratistas(),
            seleccion: Some(0),
            modo: ModoContratistas::Normal,
            columnas: Columna::TODAS.into_iter().map(|c| (c, true)).collect(),
            filtro: String::new(),
            mensaje: None,
        }
    }
}

impl ContratistasState {
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionContratistas {
        match self.modo.clone() {
            ModoContratistas::Normal => self.handle_normal(key),
            ModoContratistas::Busqueda { .. } => {
                self.handle_busqueda(key);
                AccionContratistas::Ninguna
            }
            ModoContratistas::Detalle { .. } => {
                if key.code == KeyCode::Esc {
                    self.modo = ModoContratistas::Normal;
                }
                AccionContratistas::Ninguna
            }
            ModoContratistas::Formulario(formulario) => {
                self.handle_formulario(key, formulario);
                AccionContratistas::Ninguna
            }
            ModoContratistas::Columnas { seleccion } => {
                self.handle_columnas(key, seleccion);
                AccionContratistas::Ninguna
            }
        }
    }
    fn handle_normal(&mut self, key: KeyEvent) -> AccionContratistas {
        self.mensaje = None;
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => {
                if let Some(id) = self.id_seleccionado() {
                    self.modo = ModoContratistas::Detalle { id };
                }
            }
            KeyCode::Char('n' | 'N') => {
                self.modo = ModoContratistas::Formulario(FormularioContratista::nuevo())
            }
            KeyCode::Char('e' | 'E') => {
                if let Some(id) = self.id_seleccionado()
                    && let Some(c) = self.registro(id)
                {
                    self.modo = ModoContratistas::Formulario(FormularioContratista::editar(c));
                }
            }
            KeyCode::Char('/') => {
                self.modo = ModoContratistas::Busqueda {
                    texto: self.filtro.clone(),
                };
                self.seleccion = Some(0);
            }
            KeyCode::Char('c' | 'C') => self.modo = ModoContratistas::Columnas { seleccion: 0 },
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                self.seleccion = Some(0);
            }
            KeyCode::Esc => return AccionContratistas::Volver,
            _ => {}
        }
        AccionContratistas::Ninguna
    }
    fn handle_busqueda(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.filtro.clear();
                self.modo = ModoContratistas::Normal;
                self.seleccion = Some(0);
            }
            KeyCode::Enter => self.modo = ModoContratistas::Normal,
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Backspace => {
                if let ModoContratistas::Busqueda { texto } = &mut self.modo {
                    texto.pop();
                    self.filtro = texto.clone();
                }
                self.ajustar_seleccion();
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let ModoContratistas::Busqueda { texto } = &mut self.modo {
                    texto.push(c);
                    self.filtro = texto.clone();
                }
                self.ajustar_seleccion();
            }
            _ => {}
        }
    }
    fn handle_formulario(&mut self, key: KeyEvent, mut f: FormularioContratista) {
        if let Some((desplegable, opcion)) = f.desplegable {
            let ultimo = match desplegable {
                Desplegable::Empresa => EMPRESAS.len() - 1,
                Desplegable::Tipo => TipoIngresoMock::TODOS.len() - 1,
            };
            match key.code {
                KeyCode::Up => f.desplegable = Some((desplegable, opcion.saturating_sub(1))),
                KeyCode::Down => f.desplegable = Some((desplegable, (opcion + 1).min(ultimo))),
                KeyCode::Enter => {
                    match desplegable {
                        Desplegable::Empresa => f.empresa = opcion,
                        Desplegable::Tipo => f.tipo = opcion,
                    }
                    f.desplegable = None;
                }
                KeyCode::Esc => f.desplegable = None,
                _ => {}
            }
            self.modo = ModoContratistas::Formulario(f);
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.modo = ModoContratistas::Normal;
                return;
            }
            KeyCode::Up => f.campo = f.campo.saturating_sub(1),
            KeyCode::Down | KeyCode::Tab => {
                f.campo = (f.campo + 1).min(CampoFormulario::TODOS.len() - 1)
            }
            KeyCode::BackTab => f.campo = f.campo.saturating_sub(1),
            KeyCode::Enter => match CampoFormulario::TODOS[f.campo] {
                CampoFormulario::Empresa => f.desplegable = Some((Desplegable::Empresa, f.empresa)),
                CampoFormulario::Tipo => f.desplegable = Some((Desplegable::Tipo, f.tipo)),
                CampoFormulario::Ruta => {
                    f.personal_ruta = !f.personal_ruta;
                    if !f.requiere_praind() {
                        f.fecha_praind.clear();
                    }
                }
                CampoFormulario::Acceso => f.tiene_acceso = !f.tiene_acceso,
                _ => {}
            },
            KeyCode::Char('g' | 'G')
                if CampoFormulario::TODOS[f.campo] != CampoFormulario::Nombre =>
            {
                if let Err(error) = self.guardar(&f) {
                    f.error = Some(error);
                    self.modo = ModoContratistas::Formulario(f);
                }
                return;
            }
            KeyCode::Backspace => match CampoFormulario::TODOS[f.campo] {
                CampoFormulario::Cedula => {
                    f.cedula.pop();
                }
                CampoFormulario::Nombre => {
                    f.nombre.pop();
                }
                CampoFormulario::FechaPraind if f.requiere_praind() => {
                    f.fecha_praind.pop();
                }
                _ => {}
            },
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                match CampoFormulario::TODOS[f.campo] {
                    CampoFormulario::Cedula if c.is_ascii_digit() && f.cedula.len() < 20 => {
                        f.cedula.push(c)
                    }
                    CampoFormulario::Nombre if f.nombre.chars().count() < 60 => f.nombre.push(c),
                    CampoFormulario::FechaPraind if f.requiere_praind() => {
                        agregar_digito_fecha(&mut f.fecha_praind, c)
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        f.error = None;
        self.modo = ModoContratistas::Formulario(f);
    }
    fn guardar(&mut self, f: &FormularioContratista) -> Result<(), String> {
        if f.cedula.trim().is_empty() {
            return Err("La cédula es obligatoria".into());
        }
        if f.nombre.trim().is_empty() {
            return Err("El nombre es obligatorio".into());
        }
        let fecha = if f.requiere_praind() {
            if f.fecha_praind.is_empty() {
                return Err("Fecha PRAIND requerida".into());
            }
            if NaiveDate::parse_from_str(&f.fecha_praind, "%d/%m/%Y").is_err() {
                return Err("Fecha inválida. Use DD/MM/YYYY".into());
            }
            Some(f.fecha_praind.clone())
        } else {
            None
        };
        let id = match f.modo {
            ModoFormulario::Crear => self.registros.iter().map(|c| c.id).max().unwrap_or(0) + 1,
            ModoFormulario::Editar { id } => id,
        };
        let nuevo = ContratistaMock {
            id,
            cedula: f.cedula.trim().into(),
            nombre: f.nombre.trim().into(),
            empresa: EMPRESAS[f.empresa].into(),
            tipo_ingreso: f.tipo(),
            fecha_praind: fecha,
            personal_ruta: f.personal_ruta,
            tiene_acceso: f.tiene_acceso,
        };
        match f.modo {
            ModoFormulario::Crear => {
                self.registros.push(nuevo);
                self.filtro.clear();
                self.seleccion = Some(self.registros.len() - 1);
                self.mensaje = Some("✓ Contratista creado".into());
            }
            ModoFormulario::Editar { id } => {
                if let Some(pos) = self.registros.iter().position(|c| c.id == id) {
                    self.registros[pos] = nuevo;
                    self.seleccion = self.indices_filtrados().iter().position(|i| *i == pos);
                    self.mensaje = Some("✓ Contratista actualizado".into());
                }
            }
        }
        self.modo = ModoContratistas::Normal;
        Ok(())
    }
    fn handle_columnas(&mut self, key: KeyEvent, seleccion: usize) {
        let ultimo = self.columnas.len() - 1;
        match key.code {
            KeyCode::Up => {
                self.modo = ModoContratistas::Columnas {
                    seleccion: seleccion.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                self.modo = ModoContratistas::Columnas {
                    seleccion: (seleccion + 1).min(ultimo),
                }
            }
            KeyCode::Char(' ') => {
                let visibles = self.columnas.iter().filter(|(_, v)| *v).count();
                if !self.columnas[seleccion].1 || visibles > 1 {
                    self.columnas[seleccion].1 = !self.columnas[seleccion].1;
                } else {
                    self.mensaje = Some("Debe conservar al menos una columna".into());
                }
            }
            KeyCode::Esc => self.modo = ModoContratistas::Normal,
            _ => {}
        }
    }
    fn mover(&mut self, delta: isize) {
        let n = self.indices_filtrados().len();
        if n == 0 {
            self.seleccion = None;
        } else {
            let actual = self.seleccion.unwrap_or(0);
            self.seleccion = Some(if delta < 0 {
                actual.saturating_sub(1)
            } else {
                (actual + 1).min(n - 1)
            });
        }
    }
    fn ajustar_seleccion(&mut self) {
        let n = self.indices_filtrados().len();
        self.seleccion = if n == 0 {
            None
        } else {
            Some(self.seleccion.unwrap_or(0).min(n - 1))
        };
    }
    fn indices_filtrados(&self) -> Vec<usize> {
        let q = self.filtro.trim().to_lowercase();
        self.registros
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                q.is_empty()
                    || c.cedula.to_lowercase().contains(&q)
                    || c.nombre.to_lowercase().contains(&q)
                    || c.empresa.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }
    fn id_seleccionado(&self) -> Option<u64> {
        let i = *self.indices_filtrados().get(self.seleccion?)?;
        Some(self.registros[i].id)
    }
    fn registro(&self, id: u64) -> Option<&ContratistaMock> {
        self.registros.iter().find(|c| c.id == id)
    }
    fn inicio_visible(&self, capacidad: usize) -> usize {
        self.seleccion
            .unwrap_or(0)
            .saturating_sub(capacidad.saturating_sub(1))
    }
}

fn agregar_digito_fecha(fecha: &mut String, caracter: char) {
    if !caracter.is_ascii_digit() {
        return;
    }
    let digitos = fecha.chars().filter(char::is_ascii_digit).count();
    if digitos < 8 {
        if matches!(digitos, 2 | 4) {
            fecha.push('/');
        }
        fecha.push(caracter);
    }
}
