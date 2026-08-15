use chrono::{Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Constraint;

use crate::{
    database::queries::contratistas::ContratistaResumen,
    models::{contratista::Contratista, empresa::Empresa, tipo_ingreso::TipoIngreso},
    services::contratista_service::{DatosActualizacionContratista, DatosContratista},
};

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
    Editar { id: i64 },
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
    tipo: TipoIngreso,
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
            tipo: TipoIngreso::Praind,
            fecha_praind: String::new(),
            personal_ruta: false,
            tiene_acceso: true,
            campo: 0,
            desplegable: None,
            error: None,
        }
    }
    fn editar(c: &ContratistaResumen, empresas: &[Empresa]) -> Self {
        Self {
            modo: ModoFormulario::Editar { id: c.id },
            cedula: c.cedula.clone(),
            nombre: c.nombre.clone(),
            empresa: empresas
                .iter()
                .position(|e| e.id == c.empresa_id)
                .unwrap_or(0),
            tipo: c.tipo_ingreso,
            fecha_praind: c
                .fecha_vencimiento_praind
                .map(|f| f.format("%d/%m/%Y").to_string())
                .unwrap_or_default(),
            personal_ruta: c.es_personal_ruta,
            tiene_acceso: c.tiene_acceso,
            campo: 1,
            desplegable: None,
            error: None,
        }
    }
    fn requiere_praind(&self) -> bool {
        Contratista {
            id: 0,
            cedula: String::new(),
            nombre: String::new(),
            empresa_id: 0,
            tipo_ingreso: self.tipo,
            fecha_vencimiento_praind: None,
            es_personal_ruta: self.personal_ruta,
            tiene_acceso: self.tiene_acceso,
        }
        .requiere_praind()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoContratistas {
    Normal,
    Busqueda { texto: String },
    Detalle { id: i64 },
    Formulario(FormularioContratista),
    Columnas { seleccion: usize },
}
pub enum AccionContratistas {
    Ninguna,
    Volver,
    Buscar {
        texto: Option<String>,
        seleccionar_id: Option<i64>,
    },
    Crear {
        datos: DatosContratista,
        nombre: String,
    },
    Actualizar {
        id: i64,
        datos: DatosActualizacionContratista,
        nombre: String,
    },
}
impl std::fmt::Debug for AccionContratistas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ninguna => write!(f, "Ninguna"),
            Self::Volver => write!(f, "Volver"),
            Self::Buscar { .. } => write!(f, "Buscar"),
            Self::Crear { .. } => write!(f, "Crear"),
            Self::Actualizar { .. } => write!(f, "Actualizar"),
        }
    }
}

#[derive(Debug)]
pub struct ContratistasState {
    registros: Vec<ContratistaResumen>,
    empresas: Vec<Empresa>,
    seleccion: Option<usize>,
    modo: ModoContratistas,
    columnas: Vec<(Columna, bool)>,
    filtro: String,
    mensaje: Option<String>,
    error_carga: Option<String>,
    usuario_nombre: String,
    hoy: NaiveDate,
}
impl Default for ContratistasState {
    fn default() -> Self {
        Self {
            registros: vec![],
            empresas: vec![],
            seleccion: None,
            modo: ModoContratistas::Normal,
            columnas: Columna::TODAS.into_iter().map(|c| (c, true)).collect(),
            filtro: String::new(),
            mensaje: None,
            error_carga: None,
            usuario_nombre: "Quintana".into(),
            hoy: Local::now().date_naive(),
        }
    }
}
impl ContratistasState {
    pub fn set_usuario_nombre(&mut self, nombre: impl Into<String>) {
        self.usuario_nombre = nombre.into()
    }
    pub fn set_hoy(&mut self, hoy: NaiveDate) {
        self.hoy = hoy
    }
    pub fn completar_empresas(&mut self, r: Result<Vec<Empresa>, String>) {
        match r {
            Ok(e) => self.empresas = e,
            Err(e) => self.error_carga = Some(e),
        }
    }
    pub fn completar_busqueda(
        &mut self,
        r: Result<Vec<ContratistaResumen>, String>,
        id: Option<i64>,
    ) {
        match r {
            Ok(v) => {
                self.registros = v;
                self.error_carga = None;
                self.seleccion = id
                    .and_then(|id| self.registros.iter().position(|c| c.id == id))
                    .or_else(|| (!self.registros.is_empty()).then_some(0))
            }
            Err(e) => {
                self.registros.clear();
                self.seleccion = None;
                self.error_carga = Some(e)
            }
        }
    }
    pub fn solicitud_carga(&self) -> AccionContratistas {
        self.buscar(None)
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionContratistas {
        match self.modo.clone() {
            ModoContratistas::Normal => self.normal(key),
            ModoContratistas::Busqueda { .. } => self.busqueda(key),
            ModoContratistas::Detalle { id } => {
                match key.code {
                    KeyCode::Esc => self.modo = ModoContratistas::Normal,
                    KeyCode::Char('e' | 'E') => self.editar(id),
                    _ => {}
                };
                AccionContratistas::Ninguna
            }
            ModoContratistas::Formulario(f) => self.formulario(key, f),
            ModoContratistas::Columnas { seleccion } => {
                self.columnas_key(key, seleccion);
                AccionContratistas::Ninguna
            }
        }
    }
    fn normal(&mut self, key: KeyEvent) -> AccionContratistas {
        self.mensaje = None;
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => {
                if let Some(id) = self.id() {
                    self.modo = ModoContratistas::Detalle { id }
                }
            }
            KeyCode::Char('n' | 'N') => {
                if self.empresas.is_empty() {
                    self.mensaje = Some(
                        "Debe registrar al menos una empresa antes de crear contratistas".into(),
                    )
                } else {
                    self.modo = ModoContratistas::Formulario(FormularioContratista::nuevo())
                }
            }
            KeyCode::Char('e' | 'E') => {
                if let Some(id) = self.id() {
                    self.editar(id)
                }
            }
            KeyCode::Char('/') => {
                self.modo = ModoContratistas::Busqueda {
                    texto: self.filtro.clone(),
                }
            }
            KeyCode::Char('c' | 'C') => self.modo = ModoContratistas::Columnas { seleccion: 0 },
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                return self.buscar(None);
            }
            KeyCode::Esc => return AccionContratistas::Volver,
            _ => {}
        };
        AccionContratistas::Ninguna
    }
    fn busqueda(&mut self, key: KeyEvent) -> AccionContratistas {
        match key.code {
            KeyCode::Esc => {
                self.filtro.clear();
                self.modo = ModoContratistas::Normal;
                self.buscar(None)
            }
            KeyCode::Enter => {
                self.modo = ModoContratistas::Normal;
                AccionContratistas::Ninguna
            }
            KeyCode::Up => {
                self.mover(-1);
                AccionContratistas::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionContratistas::Ninguna
            }
            KeyCode::Backspace => {
                if let ModoContratistas::Busqueda { texto } = &mut self.modo {
                    texto.pop();
                    self.filtro = texto.clone()
                }
                self.buscar(None)
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let ModoContratistas::Busqueda { texto } = &mut self.modo {
                    texto.push(c);
                    self.filtro = texto.clone()
                }
                self.buscar(None)
            }
            _ => AccionContratistas::Ninguna,
        }
    }
    fn formulario(&mut self, key: KeyEvent, mut f: FormularioContratista) -> AccionContratistas {
        if let Some((d, o)) = f.desplegable {
            let ultimo = match d {
                Desplegable::Empresa => self.empresas.len().saturating_sub(1),
                Desplegable::Tipo => 3,
            };
            match key.code {
                KeyCode::Up => f.desplegable = Some((d, o.saturating_sub(1))),
                KeyCode::Down => f.desplegable = Some((d, (o + 1).min(ultimo))),
                KeyCode::Enter => {
                    match d {
                        Desplegable::Empresa => f.empresa = o,
                        Desplegable::Tipo => f.tipo = tipos()[o],
                    }
                    f.desplegable = None
                }
                KeyCode::Esc => f.desplegable = None,
                _ => {}
            }
            self.modo = ModoContratistas::Formulario(f);
            return AccionContratistas::Ninguna;
        }
        match key.code {
            KeyCode::Esc => {
                self.modo = ModoContratistas::Normal;
                return AccionContratistas::Ninguna;
            }
            KeyCode::Up | KeyCode::BackTab => mover_campo(&mut f, -1),
            KeyCode::Down | KeyCode::Tab => mover_campo(&mut f, 1),
            KeyCode::Enter => match CampoFormulario::TODOS[f.campo] {
                CampoFormulario::Empresa => f.desplegable = Some((Desplegable::Empresa, f.empresa)),
                CampoFormulario::Tipo => {
                    f.desplegable = Some((
                        Desplegable::Tipo,
                        tipos().iter().position(|t| *t == f.tipo).unwrap_or(0),
                    ))
                }
                CampoFormulario::Ruta => f.personal_ruta = !f.personal_ruta,
                CampoFormulario::Acceso => f.tiene_acceso = !f.tiene_acceso,
                _ => {}
            },
            KeyCode::Char('g' | 'G')
                if CampoFormulario::TODOS[f.campo] != CampoFormulario::Nombre =>
            {
                return match construir(&f, self.empresas.get(f.empresa).map(|e| e.id)) {
                    Ok(datos) => {
                        let nombre = datos.nombre.clone();
                        match f.modo {
                            ModoFormulario::Crear => AccionContratistas::Crear { datos, nombre },
                            ModoFormulario::Editar { id } => AccionContratistas::Actualizar {
                                id,
                                datos: convertir_actualizacion(datos),
                                nombre,
                            },
                        }
                    }
                    Err(e) => {
                        f.error = Some(e);
                        self.modo = ModoContratistas::Formulario(f);
                        AccionContratistas::Ninguna
                    }
                };
            }
            KeyCode::Backspace => match CampoFormulario::TODOS[f.campo] {
                CampoFormulario::Cedula if matches!(f.modo, ModoFormulario::Crear) => {
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
                    CampoFormulario::Cedula
                        if matches!(f.modo, ModoFormulario::Crear)
                            && f.cedula.chars().count() < 30 =>
                    {
                        f.cedula.push(c)
                    }
                    CampoFormulario::Nombre if f.nombre.chars().count() < 60 => f.nombre.push(c),
                    CampoFormulario::FechaPraind if f.requiere_praind() => {
                        agregar_fecha(&mut f.fecha_praind, c)
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        f.error = None;
        self.modo = ModoContratistas::Formulario(f);
        AccionContratistas::Ninguna
    }
    pub fn completar_guardado(
        &mut self,
        r: Result<Option<i64>, String>,
        id: Option<i64>,
        nombre: &str,
    ) -> AccionContratistas {
        match r {
            Ok(creado) => {
                self.modo = ModoContratistas::Normal;
                if creado.is_some() {
                    self.filtro.clear()
                }
                self.mensaje = Some(format!(
                    "✓ Contratista {} — {}",
                    if creado.is_some() {
                        "creado"
                    } else {
                        "actualizado"
                    },
                    nombre
                ));
                self.buscar(creado.or(id))
            }
            Err(e) => {
                if let ModoContratistas::Formulario(f) = &mut self.modo {
                    f.error = Some(e)
                }
                AccionContratistas::Ninguna
            }
        }
    }
    fn editar(&mut self, id: i64) {
        if let Some(c) = self.registros.iter().find(|c| c.id == id) {
            self.modo =
                ModoContratistas::Formulario(FormularioContratista::editar(c, &self.empresas))
        }
    }
    fn buscar(&self, id: Option<i64>) -> AccionContratistas {
        AccionContratistas::Buscar {
            texto: (!self.filtro.trim().is_empty()).then(|| self.filtro.clone()),
            seleccionar_id: id,
        }
    }
    fn mover(&mut self, d: isize) {
        let n = self.registros.len();
        self.seleccion = if n == 0 {
            None
        } else {
            let a = self.seleccion.unwrap_or(0);
            Some(if d < 0 {
                a.saturating_sub(1)
            } else {
                (a + 1).min(n - 1)
            })
        }
    }
    fn id(&self) -> Option<i64> {
        Some(self.registros.get(self.seleccion?)?.id)
    }
    fn registro(&self, id: i64) -> Option<&ContratistaResumen> {
        self.registros.iter().find(|c| c.id == id)
    }
    fn inicio_visible(&self, c: usize) -> usize {
        self.seleccion
            .unwrap_or(0)
            .saturating_sub(c.saturating_sub(1))
    }
    fn columnas_key(&mut self, key: KeyEvent, s: usize) {
        let u = self.columnas.len() - 1;
        match key.code {
            KeyCode::Up => {
                self.modo = ModoContratistas::Columnas {
                    seleccion: s.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                self.modo = ModoContratistas::Columnas {
                    seleccion: (s + 1).min(u),
                }
            }
            KeyCode::Char(' ') => {
                let v = self.columnas.iter().filter(|(_, v)| *v).count();
                if !self.columnas[s].1 || v > 1 {
                    self.columnas[s].1 = !self.columnas[s].1
                } else {
                    self.mensaje = Some("Debe conservar al menos una columna".into())
                }
            }
            KeyCode::Esc => self.modo = ModoContratistas::Normal,
            _ => {}
        }
    }
}
fn tipos() -> [TipoIngreso; 4] {
    [
        TipoIngreso::Praind,
        TipoIngreso::InHouse,
        TipoIngreso::PorCorreo,
        TipoIngreso::Swat,
    ]
}
fn texto_tipo(t: TipoIngreso) -> &'static str {
    match t {
        TipoIngreso::Praind => "PRAIND",
        TipoIngreso::InHouse => "IN HOUSE",
        TipoIngreso::PorCorreo => "POR CORREO",
        TipoIngreso::Swat => "SWAT",
    }
}
fn construir(
    f: &FormularioContratista,
    empresa_id: Option<i64>,
) -> Result<DatosContratista, String> {
    if f.cedula.trim().is_empty() {
        return Err("La cédula es obligatoria".into());
    }
    if f.nombre.trim().is_empty() {
        return Err("El nombre es obligatorio".into());
    }
    let fecha = if f.requiere_praind() {
        Some(
            NaiveDate::parse_from_str(&f.fecha_praind, "%d/%m/%Y").map_err(|_| {
                if f.fecha_praind.is_empty() {
                    "Fecha PRAIND requerida"
                } else {
                    "Fecha inválida. Use DD/MM/YYYY"
                }
            })?,
        )
    } else {
        None
    };
    Ok(DatosContratista {
        cedula: f.cedula.trim().into(),
        nombre: f.nombre.trim().into(),
        empresa_id: empresa_id.ok_or("La empresa seleccionada ya no existe")?,
        tipo_ingreso: f.tipo,
        fecha_vencimiento_praind: fecha,
        es_personal_ruta: f.personal_ruta,
        tiene_acceso: f.tiene_acceso,
    })
}
fn convertir_actualizacion(datos: DatosContratista) -> DatosActualizacionContratista {
    DatosActualizacionContratista {
        nombre: datos.nombre,
        empresa_id: datos.empresa_id,
        tipo_ingreso: datos.tipo_ingreso,
        fecha_vencimiento_praind: datos.fecha_vencimiento_praind,
        es_personal_ruta: datos.es_personal_ruta,
        tiene_acceso: datos.tiene_acceso,
    }
}
fn mover_campo(f: &mut FormularioContratista, d: isize) {
    let habilitados: Vec<usize> = CampoFormulario::TODOS
        .iter()
        .enumerate()
        .filter_map(|(indice, campo)| {
            let habilitado = !(matches!(f.modo, ModoFormulario::Editar { .. })
                && *campo == CampoFormulario::Cedula)
                && (*campo != CampoFormulario::FechaPraind || f.requiere_praind());
            habilitado.then_some(indice)
        })
        .collect();
    let posicion = habilitados
        .iter()
        .position(|indice| *indice == f.campo)
        .unwrap_or(0);
    let siguiente = if d < 0 {
        posicion.saturating_sub(1)
    } else {
        (posicion + 1).min(habilitados.len() - 1)
    };
    f.campo = habilitados[siguiente];
}
fn agregar_fecha(s: &mut String, c: char) {
    if !c.is_ascii_digit() {
        return;
    }
    let n = s.chars().filter(char::is_ascii_digit).count();
    if n < 8 {
        if matches!(n, 2 | 4) {
            s.push('/')
        }
        s.push(c)
    }
}
