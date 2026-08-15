use crate::{
    database::queries::ingresos::{
        EstadoMovimiento, FiltroHistorial, MovimientoIngresoResumen, PaginaHistorial,
    },
    models::{empresa::Empresa, tipo_ingreso::TipoIngreso},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Constraint;
#[path = "filtros.rs"]
mod filtros;
pub use filtros::*;
#[path = "render.rs"]
pub(super) mod render;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
const LIMIT: usize = 50;
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
        id: i64,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionHistorial {
    Ninguna,
    Volver,
    Consultar(FiltroHistorial),
}
#[derive(Debug)]
pub struct HistorialState {
    registros: Vec<MovimientoIngresoResumen>,
    total: usize,
    seleccion: Option<usize>,
    modo: ModoHistorial,
    filtro_aplicado: FiltrosHistorial,
    filtro_edicion: FiltrosHistorial,
    busqueda: String,
    columnas: Vec<(ColumnaHistorial, bool)>,
    mensaje: Option<String>,
    empresas: Vec<Empresa>,
    offset: usize,
    corte_id: Option<i64>,
    usuario_nombre: String,
}
impl Default for HistorialState {
    fn default() -> Self {
        let f = FiltrosHistorial::default();
        Self {
            registros: vec![],
            total: 0,
            seleccion: None,
            modo: ModoHistorial::Normal,
            filtro_aplicado: f.clone(),
            filtro_edicion: f,
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
            empresas: vec![],
            offset: 0,
            corte_id: None,
            usuario_nombre: "Quintana".into(),
        }
    }
}
impl HistorialState {
    pub fn set_usuario_nombre(&mut self, n: impl Into<String>) {
        self.usuario_nombre = n.into()
    }
    pub fn completar_empresas(&mut self, r: Result<Vec<Empresa>, String>) {
        match r {
            Ok(v) => self.empresas = v,
            Err(e) => self.mensaje = Some(e),
        }
    }
    pub fn solicitud_carga(&mut self) -> AccionHistorial {
        self.reiniciar_paginacion();
        self.consulta()
            .map_or(AccionHistorial::Ninguna, AccionHistorial::Consultar)
    }
    pub fn completar(&mut self, r: Result<PaginaHistorial, String>) {
        match r {
            Ok(p) => {
                self.corte_id = Some(p.corte_id);
                self.registros = p.items;
                self.total = p.total;
                self.seleccion = (!self.registros.is_empty()).then_some(0)
            }
            Err(e) => {
                self.registros.clear();
                self.total = 0;
                self.seleccion = None;
                self.corte_id = None;
                self.mensaje = Some(e)
            }
        }
    }
    fn consulta(&self) -> Result<FiltroHistorial, String> {
        construir(
            &self.filtro_aplicado,
            &self.busqueda,
            LIMIT,
            self.offset,
            self.corte_id,
        )
    }
    fn emitir(&mut self) -> AccionHistorial {
        match self.consulta() {
            Ok(f) => AccionHistorial::Consultar(f),
            Err(e) => {
                self.mensaje = Some(e);
                AccionHistorial::Ninguna
            }
        }
    }
    pub fn handle_key(&mut self, k: KeyEvent) -> AccionHistorial {
        match self.modo.clone() {
            ModoHistorial::Normal => self.normal(k),
            ModoHistorial::Busqueda { .. } => self.buscar(k),
            ModoHistorial::Detalle { .. } => {
                if k.code == KeyCode::Esc {
                    self.modo = ModoHistorial::Normal
                }
                AccionHistorial::Ninguna
            }
            ModoHistorial::Filtros {
                seleccion,
                editando,
            } => self.filtros(k, seleccion, editando),
            ModoHistorial::Desplegable {
                campo,
                seleccion_filtro,
                opcion,
            } => self.desplegable(k, campo, seleccion_filtro, opcion),
            ModoHistorial::Columnas { seleccion } => {
                self.columnas(k, seleccion);
                AccionHistorial::Ninguna
            }
        }
    }
    fn normal(&mut self, k: KeyEvent) -> AccionHistorial {
        match k.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::PageDown => {
                if self.offset + LIMIT < self.total {
                    self.offset += LIMIT;
                    return self.emitir();
                }
            }
            KeyCode::PageUp => {
                if self.offset > 0 {
                    self.offset = self.offset.saturating_sub(LIMIT);
                    return self.emitir();
                }
            }
            KeyCode::Enter => {
                if let Some(r) = self.registros.get(self.seleccion.unwrap_or(usize::MAX)) {
                    self.modo = ModoHistorial::Detalle { id: r.registro_id }
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
                }
            }
            KeyCode::Char('c' | 'C') => self.modo = ModoHistorial::Columnas { seleccion: 0 },
            KeyCode::Esc if !self.busqueda.is_empty() => {
                self.busqueda.clear();
                self.reiniciar_paginacion();
                return self.emitir();
            }
            KeyCode::Esc => return AccionHistorial::Volver,
            _ => {}
        }
        AccionHistorial::Ninguna
    }
    fn buscar(&mut self, k: KeyEvent) -> AccionHistorial {
        match k.code {
            KeyCode::Esc => {
                self.busqueda.clear();
                self.reiniciar_paginacion();
                self.modo = ModoHistorial::Normal;
                self.emitir()
            }
            KeyCode::Enter => {
                self.modo = ModoHistorial::Normal;
                AccionHistorial::Ninguna
            }
            KeyCode::Backspace => {
                if let ModoHistorial::Busqueda { texto } = &mut self.modo {
                    texto.pop();
                    self.busqueda = texto.clone()
                }
                self.reiniciar_paginacion();
                self.emitir()
            }
            KeyCode::Char(c)
                if !k
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let ModoHistorial::Busqueda { texto } = &mut self.modo {
                    texto.push(c);
                    self.busqueda = texto.clone()
                }
                self.reiniciar_paginacion();
                self.emitir()
            }
            _ => AccionHistorial::Ninguna,
        }
    }
    fn filtros(&mut self, k: KeyEvent, s: usize, editando: bool) -> AccionHistorial {
        let campo = CampoFiltro::TODOS[s];
        if editando {
            match k.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.modo = ModoHistorial::Filtros {
                        seleccion: s,
                        editando: false,
                    }
                }
                KeyCode::Backspace => {
                    if let Some(t) = self.texto_mut(campo) {
                        t.pop();
                    }
                }
                KeyCode::Char(c)
                    if !k
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.agregar(campo, c)
                }
                _ => {}
            }
            return AccionHistorial::Ninguna;
        }
        match k.code {
            KeyCode::Up => {
                self.modo = ModoHistorial::Filtros {
                    seleccion: s.saturating_sub(1),
                    editando: false,
                }
            }
            KeyCode::Down => {
                self.modo = ModoHistorial::Filtros {
                    seleccion: (s + 1).min(6),
                    editando: false,
                }
            }
            KeyCode::Enter
                if matches!(
                    campo,
                    CampoFiltro::Empresa | CampoFiltro::Tipo | CampoFiltro::Estado
                ) =>
            {
                self.modo = ModoHistorial::Desplegable {
                    campo,
                    seleccion_filtro: s,
                    opcion: 0,
                }
            }
            KeyCode::Enter => {
                self.modo = ModoHistorial::Filtros {
                    seleccion: s,
                    editando: true,
                }
            }
            KeyCode::Char('a' | 'A') => {
                match construir(&self.filtro_edicion, &self.busqueda, LIMIT, 0, None) {
                    Ok(f) => {
                        self.filtro_aplicado = self.filtro_edicion.clone();
                        self.reiniciar_paginacion();
                        self.modo = ModoHistorial::Normal;
                        return AccionHistorial::Consultar(f);
                    }
                    Err(e) => self.mensaje = Some(e),
                }
            }
            KeyCode::Char('l' | 'L') => self.filtro_edicion = FiltrosHistorial::default(),
            KeyCode::Esc => self.modo = ModoHistorial::Normal,
            _ => {}
        }
        AccionHistorial::Ninguna
    }
    fn desplegable(
        &mut self,
        k: KeyEvent,
        c: CampoFiltro,
        s: usize,
        mut o: usize,
    ) -> AccionHistorial {
        let n = self.opciones(c);
        match k.code {
            KeyCode::Up => o = o.saturating_sub(1),
            KeyCode::Down => o = (o + 1).min(n.saturating_sub(1)),
            KeyCode::Enter => {
                match c {
                    CampoFiltro::Empresa => {
                        self.filtro_edicion.empresa_id = if o == 0 {
                            None
                        } else {
                            Some(self.empresas[o - 1].id)
                        }
                    }
                    CampoFiltro::Tipo => {
                        self.filtro_edicion.tipo = [
                            None,
                            Some(TipoIngreso::Praind),
                            Some(TipoIngreso::InHouse),
                            Some(TipoIngreso::PorCorreo),
                            Some(TipoIngreso::Swat),
                        ][o]
                    }
                    CampoFiltro::Estado => {
                        self.filtro_edicion.estado = [
                            EstadoMovimiento::Todos,
                            EstadoMovimiento::Activos,
                            EstadoMovimiento::Cerrados,
                        ][o]
                    }
                    _ => {}
                }
                self.modo = ModoHistorial::Filtros {
                    seleccion: s,
                    editando: false,
                };
                return AccionHistorial::Ninguna;
            }
            KeyCode::Esc => {
                self.modo = ModoHistorial::Filtros {
                    seleccion: s,
                    editando: false,
                };
                return AccionHistorial::Ninguna;
            }
            _ => {}
        }
        self.modo = ModoHistorial::Desplegable {
            campo: c,
            seleccion_filtro: s,
            opcion: o,
        };
        AccionHistorial::Ninguna
    }
    fn opciones(&self, c: CampoFiltro) -> usize {
        match c {
            CampoFiltro::Empresa => self.empresas.len() + 1,
            CampoFiltro::Tipo => 5,
            CampoFiltro::Estado => 3,
            _ => 0,
        }
    }
    fn texto_mut(&mut self, c: CampoFiltro) -> Option<&mut String> {
        match c {
            CampoFiltro::Desde => Some(&mut self.filtro_edicion.desde),
            CampoFiltro::Hasta => Some(&mut self.filtro_edicion.hasta),
            CampoFiltro::NombreCedula => Some(&mut self.filtro_edicion.nombre_cedula),
            CampoFiltro::Gafete => Some(&mut self.filtro_edicion.gafete),
            _ => None,
        }
    }
    fn agregar(&mut self, c: CampoFiltro, x: char) {
        if let Some(t) = self.texto_mut(c) {
            match c {
                CampoFiltro::Desde | CampoFiltro::Hasta if x.is_ascii_digit() => {
                    let n = t.chars().filter(char::is_ascii_digit).count();
                    if n < 8 {
                        if matches!(n, 2 | 4) {
                            t.push('/')
                        }
                        t.push(x)
                    }
                }
                CampoFiltro::Gafete if x.is_ascii_digit() => t.push(x),
                CampoFiltro::NombreCedula if t.chars().count() < 40 => t.push(x),
                _ => {}
            }
        }
    }
    fn columnas(&mut self, k: KeyEvent, s: usize) {
        match k.code {
            KeyCode::Up => {
                self.modo = ModoHistorial::Columnas {
                    seleccion: s.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                self.modo = ModoHistorial::Columnas {
                    seleccion: (s + 1).min(self.columnas.len() - 1),
                }
            }
            KeyCode::Char(' ') => {
                let n = self.columnas.iter().filter(|x| x.1).count();
                if !(self.columnas[s].1 && n == 1) {
                    self.columnas[s].1 = !self.columnas[s].1
                }
            }
            KeyCode::Esc => self.modo = ModoHistorial::Normal,
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
    fn registro(&self, id: i64) -> Option<&MovimientoIngresoResumen> {
        self.registros.iter().find(|r| r.registro_id == id)
    }
    fn inicio_visible(&self, c: usize) -> usize {
        self.seleccion
            .unwrap_or(0)
            .saturating_sub(c.saturating_sub(1))
    }
    fn pagina(&self) -> (usize, usize) {
        if self.total == 0 {
            (0, 0)
        } else {
            (self.offset / LIMIT + 1, self.total.div_ceil(LIMIT))
        }
    }

    fn reiniciar_paginacion(&mut self) {
        self.offset = 0;
        self.corte_id = None;
    }
}
