use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Constraint;

use crate::{
    database::queries::{
        Igualdad,
        contratistas::{ContratistaResumen, FiltroContratistas, FiltroPraind, PaginaContratistas},
    },
    models::{contratista::Contratista, empresa::Empresa, tipo_ingreso::TipoIngreso},
    services::contratista_service::{DatosActualizacionContratista, DatosContratista},
    tiempo::ahora_costa_rica,
    tui::ui_kit::{
        Debounce, StandardCommand, TextInput, resolver_terminos_detallado, standard_command,
    },
};
use std::time::Instant;

use form::{agregar_fecha, construir, convertir_actualizacion, mover_campo, tipos};
use query::{aplicar_clave, parsear_consulta};

const DURACION_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(250);
/// Mismo tamaño que `LIMITE_PREDETERMINADO` en `database::queries::contratistas` — se repite
/// aquí (no se importa) porque ese es un detalle interno de la consulta, mientras que esta
/// constante es el tamaño de página que la UI usa para avanzar con PageUp/PageDown.
const LIMITE_PAGINA: usize = 100;

#[path = "form.rs"]
mod form;
#[path = "query.rs"]
mod query;
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
    const fn clave(self) -> &'static str {
        match self {
            Self::Cedula => "cedula",
            Self::Nombre => "nombre",
            Self::Empresa => "empresa",
            Self::Tipo => "tipo",
            Self::Praind => "praind",
            Self::Ruta => "ruta",
            Self::Acceso => "acceso",
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
    cedula: TextInput,
    nombre: TextInput,
    empresa: usize,
    tipo: TipoIngreso,
    fecha_praind: TextInput,
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
            cedula: TextInput::default().with_max_chars(30),
            nombre: TextInput::default().with_max_chars(60),
            empresa: 0,
            tipo: TipoIngreso::Praind,
            fecha_praind: TextInput::default().with_max_chars(10),
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
            cedula: TextInput::new(c.cedula.clone()).with_max_chars(30),
            nombre: TextInput::new(c.nombre.clone()).with_max_chars(60),
            empresa: empresas
                .iter()
                .position(|e| e.id == c.empresa_id)
                .unwrap_or(0),
            tipo: c.tipo_ingreso,
            fecha_praind: TextInput::new(
                c.fecha_vencimiento_praind
                    .map(|f| f.format("%d/%m/%Y").to_string())
                    .unwrap_or_default(),
            )
            .with_max_chars(10),
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
            empresa_activa: true,
        }
        .requiere_praind()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoContratistas {
    Normal,
    Busqueda { texto: TextInput },
    Formulario(Box<FormularioContratista>),
    Columnas { seleccion: usize },
}
pub enum AccionContratistas {
    Ninguna,
    Volver,
    Buscar {
        texto: Option<String>,
        seleccionar_id: Option<i64>,
        empresa_id: Option<Igualdad<i64>>,
        tipos: Option<Vec<TipoIngreso>>,
        praind: Option<FiltroPraind>,
        praind_negado: bool,
        personal_ruta: Option<bool>,
        tiene_acceso: Option<bool>,
        offset: usize,
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
    /// Conteo real, sin recortar por `LIMITE_PAGINA` — `registros.len()` sólo
    /// dice cuántos trae la página actual.
    total: usize,
    offset: usize,
    empresas: Vec<Empresa>,
    seleccion: Option<usize>,
    modo: ModoContratistas,
    columnas: Vec<(Columna, bool)>,
    filtro: String,
    mensaje: Option<String>,
    hoy: NaiveDate,
    ayuda_expandida: bool,
    busqueda_debounce: Debounce,
}
impl Default for ContratistasState {
    fn default() -> Self {
        Self {
            registros: vec![],
            total: 0,
            offset: 0,
            empresas: vec![],
            seleccion: None,
            modo: ModoContratistas::Normal,
            columnas: Columna::TODAS.into_iter().map(|c| (c, true)).collect(),
            filtro: String::new(),
            mensaje: None,
            hoy: ahora_costa_rica().date_naive(),
            ayuda_expandida: false,
            busqueda_debounce: Debounce::default(),
        }
    }
}
impl ContratistasState {
    pub(crate) fn columnas_preferencia(&self) -> String {
        self.columnas
            .iter()
            .filter_map(|(columna, visible)| visible.then_some(columna.clave()))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub(crate) fn aplicar_columnas_preferencia(&mut self, valor: &str) {
        let claves: Vec<&str> = valor.split(',').collect();
        if !self
            .columnas
            .iter()
            .any(|(columna, _)| claves.contains(&columna.clave()))
        {
            return;
        }
        for (columna, visible) in &mut self.columnas {
            *visible = claves.contains(&columna.clave());
        }
    }

    pub fn set_hoy(&mut self, hoy: NaiveDate) {
        self.hoy = hoy
    }
    pub fn completar_empresas(&mut self, r: Result<Vec<Empresa>, String>) {
        match r {
            Ok(e) => self.empresas = e,
            Err(e) => self.mensaje = Some(e),
        }
    }
    pub fn completar_busqueda(&mut self, r: Result<PaginaContratistas, String>, id: Option<i64>) {
        match r {
            Ok(pagina) => {
                self.registros = pagina.items;
                self.total = pagina.total;
                if !matches!(self.mensaje.as_deref(), Some(mensaje) if mensaje.starts_with('✓')) {
                    self.mensaje = None;
                }
                self.seleccion = id
                    .and_then(|id| self.registros.iter().position(|c| c.id == id))
                    .or_else(|| (!self.registros.is_empty()).then_some(0))
            }
            Err(e) => {
                self.registros.clear();
                self.total = 0;
                self.seleccion = None;
                self.mensaje = Some(e)
            }
        }
    }
    pub fn solicitud_carga(&self) -> AccionContratistas {
        self.buscar(None)
    }
    pub(super) fn resumen_consulta(&self) -> String {
        if self.filtro.trim().is_empty() {
            return "Ejemplo: empresa:Brisas tipo:praind acceso:si".to_owned();
        }

        let mut filtro = FiltroContratistas::default();
        let resolucion = resolver_terminos_detallado(&self.filtro, &mut filtro, |f, term| {
            aplicar_clave(f, term, &self.empresas, self.hoy)
        });
        let mut partes = Vec::new();
        if let Some(empresa_id) = filtro.empresa_id {
            let nombre = self
                .empresas
                .iter()
                .find(|empresa| empresa.id == *empresa_id.valor())
                .map_or("?", |empresa| empresa.nombre.as_str());
            let signo = if empresa_id.negado() { "≠" } else { "=" };
            partes.push(format!("empresa{signo}{nombre}"));
        }
        if let Some(tipos) = filtro.tipos_incluidos {
            partes.push(format!(
                "tipo={}",
                tipos
                    .into_iter()
                    .map(TipoIngreso::as_str_sql)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if let Some(praind) = filtro.praind {
            let signo = if filtro.praind_negado { "≠" } else { "=" };
            partes.push(format!(
                "praind{signo}{}",
                match praind {
                    FiltroPraind::Vencido { .. } => "vencido",
                    FiltroPraind::ProximoAVencer { .. } => "próximo",
                    FiltroPraind::SinFecha => "sin fecha",
                }
            ));
        }
        if let Some(ruta) = filtro.personal_ruta {
            partes.push(format!("ruta={}", if ruta { "sí" } else { "no" }));
        }
        if let Some(acceso) = filtro.tiene_acceso {
            partes.push(format!("acceso={}", if acceso { "sí" } else { "no" }));
        }
        if !resolucion.texto_libre.is_empty() {
            partes.push(format!("texto=\"{}\"", resolucion.texto_libre));
        }
        if !resolucion.no_reconocidos.is_empty() {
            partes.push(format!(
                "⚠ sin interpretar: {}",
                resolucion.no_reconocidos.join(", ")
            ));
        }
        partes.join(" · ")
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionContratistas {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionContratistas::Ninguna;
        }
        match self.modo.clone() {
            ModoContratistas::Normal => self.normal(key),
            ModoContratistas::Busqueda { .. } => self.busqueda(key),
            ModoContratistas::Formulario(f) => self.formulario(key, f),
            ModoContratistas::Columnas { seleccion } => {
                self.columnas_key(key, seleccion);
                AccionContratistas::Ninguna
            }
        }
    }
    fn normal(&mut self, key: KeyEvent) -> AccionContratistas {
        if matches!(
            key.code,
            KeyCode::Enter | KeyCode::Char('n' | 'N' | '/') | KeyCode::Esc
        ) {
            self.mensaje = None;
        }
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::PageDown => {
                if self.offset + LIMITE_PAGINA < self.total {
                    self.offset += LIMITE_PAGINA;
                    return self.buscar(None);
                }
            }
            KeyCode::PageUp => {
                if self.offset > 0 {
                    self.offset = self.offset.saturating_sub(LIMITE_PAGINA);
                    return self.buscar(None);
                }
            }
            KeyCode::Enter => {
                if let Some(id) = self.id() {
                    self.editar(id)
                }
            }
            KeyCode::Char('n' | 'N') => {
                if self.empresas.is_empty() {
                    self.mensaje = Some(
                        "Debe registrar al menos una empresa antes de crear contratistas".into(),
                    )
                } else {
                    self.modo =
                        ModoContratistas::Formulario(Box::new(FormularioContratista::nuevo()))
                }
            }
            KeyCode::Char('/') => {
                self.modo = ModoContratistas::Busqueda {
                    texto: TextInput::default(),
                }
            }
            KeyCode::F(4) => self.modo = ModoContratistas::Columnas { seleccion: 0 },
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                self.offset = 0;
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
                self.offset = 0;
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
            _ => {
                let mut cambio = false;
                if let ModoContratistas::Busqueda { texto } = &mut self.modo {
                    cambio = texto.handle_key(key);
                    self.filtro = texto.value().to_owned();
                }
                if cambio {
                    self.offset = 0;
                    self.busqueda_debounce.marcar(Instant::now());
                }
                AccionContratistas::Ninguna
            }
        }
    }
    /// Se llama en cada vuelta del bucle principal; dispara la búsqueda
    /// diferida sólo una vez que pasa `DURACION_DEBOUNCE` sin una tecla
    /// nueva. También refresca `hoy` (usado por `praind:` y el coloreado de
    /// vencimiento) para que una sesión que cruza medianoche no quede
    /// congelada en la fecha de arranque.
    pub fn tick(&mut self, ahora: Instant) -> AccionContratistas {
        self.hoy = ahora_costa_rica().date_naive();
        if self.busqueda_debounce.listo(ahora, DURACION_DEBOUNCE) {
            self.buscar(None)
        } else {
            AccionContratistas::Ninguna
        }
    }
    fn formulario(
        &mut self,
        key: KeyEvent,
        mut f: Box<FormularioContratista>,
    ) -> AccionContratistas {
        if let Some((d, o)) = f.desplegable {
            let ultimo = match d {
                Desplegable::Empresa => self.empresas.len().saturating_sub(1),
                Desplegable::Tipo => tipos().len().saturating_sub(1),
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
            KeyCode::Char(' ')
                if matches!(
                    CampoFormulario::TODOS[f.campo],
                    CampoFormulario::Empresa
                        | CampoFormulario::Tipo
                        | CampoFormulario::Ruta
                        | CampoFormulario::Acceso
                ) =>
            {
                match CampoFormulario::TODOS[f.campo] {
                    CampoFormulario::Empresa => {
                        f.desplegable = Some((Desplegable::Empresa, f.empresa))
                    }
                    CampoFormulario::Tipo => {
                        f.desplegable = Some((
                            Desplegable::Tipo,
                            tipos().iter().position(|t| *t == f.tipo).unwrap_or(0),
                        ))
                    }
                    CampoFormulario::Ruta => f.personal_ruta = !f.personal_ruta,
                    CampoFormulario::Acceso => f.tiene_acceso = !f.tiene_acceso,
                    _ => {}
                }
            }
            KeyCode::Enter => {
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
            KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End | KeyCode::Delete => {
                match CampoFormulario::TODOS[f.campo] {
                    CampoFormulario::Cedula if matches!(f.modo, ModoFormulario::Crear) => {
                        f.cedula.handle_key(key);
                    }
                    CampoFormulario::Nombre => {
                        f.nombre.handle_key(key);
                    }
                    CampoFormulario::FechaPraind if f.requiere_praind() => {
                        f.fecha_praind.handle_key(key);
                    }
                    _ => {}
                }
            }
            KeyCode::Backspace => match CampoFormulario::TODOS[f.campo] {
                CampoFormulario::Cedula if matches!(f.modo, ModoFormulario::Crear) => {
                    f.cedula.handle_key(key);
                }
                CampoFormulario::Nombre => {
                    f.nombre.handle_key(key);
                }
                CampoFormulario::FechaPraind if f.requiere_praind() => {
                    f.fecha_praind.handle_key(key);
                }
                _ => {}
            },
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                match CampoFormulario::TODOS[f.campo] {
                    CampoFormulario::Cedula if matches!(f.modo, ModoFormulario::Crear) => {
                        f.cedula.handle_key(key);
                    }
                    CampoFormulario::Nombre => {
                        f.nombre.handle_key(key);
                    }
                    CampoFormulario::FechaPraind if f.requiere_praind() => {
                        agregar_fecha(&mut f.fecha_praind, key, c)
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
                // Igual tras crear o editar — antes sólo se limpiaba al
                // crear, mismo patrón de inconsistencia que ya tenía Usuarios.
                self.filtro.clear();
                self.offset = 0;
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
            self.modo = ModoContratistas::Formulario(Box::new(FormularioContratista::editar(
                c,
                &self.empresas,
            )))
        }
    }
    fn buscar(&self, id: Option<i64>) -> AccionContratistas {
        let (filtro, libre) = parsear_consulta(&self.filtro, &self.empresas, self.hoy);
        AccionContratistas::Buscar {
            texto: (!libre.trim().is_empty()).then_some(libre),
            seleccionar_id: id,
            empresa_id: filtro.empresa_id,
            tipos: filtro.tipos_incluidos,
            praind: filtro.praind,
            praind_negado: filtro.praind_negado,
            personal_ruta: filtro.personal_ruta,
            tiene_acceso: filtro.tiene_acceso,
            offset: self.offset,
        }
    }
    /// Página actual (1-indexada) y total de páginas — `(0, 0)` sin resultados.
    pub fn pagina(&self) -> (usize, usize) {
        if self.total == 0 {
            (0, 0)
        } else {
            (
                self.offset / LIMITE_PAGINA + 1,
                self.total.div_ceil(LIMITE_PAGINA),
            )
        }
    }
    pub fn total(&self) -> usize {
        self.total
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
    fn seleccionado(&self) -> Option<&ContratistaResumen> {
        self.registros.get(self.seleccion?)
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
