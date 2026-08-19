use crate::{
    database::queries::ingresos::{FiltroHistorial, MovimientoIngresoResumen, PaginaHistorial},
    models::empresa::Empresa,
    tiempo::a_costa_rica,
};
use chrono::{Datelike, Duration, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::ui_kit::{Debounce, StandardCommand, TextInput, standard_command};
use std::time::Instant;

const DURACION_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(250);
#[path = "filtros.rs"]
mod filtros;
pub use filtros::*;
#[path = "render.rs"]
pub(super) mod render;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
const LIMIT: usize = 50;

/// Dos formas de leer el mismo conjunto filtrado: el timeline agrupado
/// (curado, con panel de detalle), la tabla clásica (densa, una línea por
/// movimiento, sin panel) y el mapa de calor (para ubicar "cuándo" hubo más
/// actividad). Cambiar de vista conserva la búsqueda y la selección.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Timeline,
    Classic,
    Heatmap,
}
impl ViewMode {
    const fn next(self) -> Self {
        match self {
            Self::Timeline => Self::Classic,
            Self::Classic => Self::Heatmap,
            Self::Heatmap => Self::Timeline,
        }
    }
    const fn label(self) -> &'static str {
        match self {
            Self::Timeline => "LÍNEA DE TIEMPO",
            Self::Classic => "CLÁSICA",
            Self::Heatmap => "MAPA DE CALOR",
        }
    }
}

/// Columnas de la vista Clásica. El timeline ya muestra todo en su panel,
/// pero Clásica no tiene panel — ahí sí tiene sentido poder ocultar
/// columnas para controlar la densidad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClassicColumn {
    Fecha,
    Nombre,
    Cedula,
    Empresa,
    Tipo,
    Entrada,
    Salida,
    Gafete,
    Medio,
    Ingreso,
    Egreso,
}
impl ClassicColumn {
    const ALL: [Self; 11] = [
        Self::Fecha,
        Self::Nombre,
        Self::Cedula,
        Self::Empresa,
        Self::Tipo,
        Self::Entrada,
        Self::Salida,
        Self::Gafete,
        Self::Medio,
        Self::Ingreso,
        Self::Egreso,
    ];
    pub(super) const fn label(self) -> &'static str {
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
            Self::Ingreso => "DA INGRESO",
            Self::Egreso => "DA SALIDA",
        }
    }
    pub(super) const fn constraint(self) -> ratatui::layout::Constraint {
        use ratatui::layout::Constraint;
        match self {
            Self::Fecha => Constraint::Length(10),
            Self::Cedula => Constraint::Length(14),
            Self::Nombre | Self::Empresa => Constraint::Fill(3),
            Self::Tipo => Constraint::Length(11),
            Self::Entrada | Self::Salida => Constraint::Length(8),
            Self::Gafete => Constraint::Length(7),
            Self::Medio => Constraint::Fill(1),
            Self::Ingreso | Self::Egreso => Constraint::Fill(2),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoHistorial {
    Normal,
    Columnas { seleccion: usize },
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
    vista: ViewMode,
    columnas_clasica: Vec<(ClassicColumn, bool)>,
    heatmap_seleccion: NaiveDate,
    filtro_aplicado: FiltrosHistorial,
    busqueda: TextInput,
    mensaje: Option<String>,
    empresas: Vec<Empresa>,
    offset: usize,
    corte_id: Option<i64>,
    ayuda_expandida: bool,
    busqueda_debounce: Debounce,
}
impl Default for HistorialState {
    fn default() -> Self {
        let hoy = crate::tiempo::ahora_costa_rica().date_naive();
        Self {
            registros: vec![],
            total: 0,
            seleccion: None,
            modo: ModoHistorial::Normal,
            vista: ViewMode::Timeline,
            columnas_clasica: ClassicColumn::ALL.into_iter().map(|c| (c, true)).collect(),
            heatmap_seleccion: hoy,
            filtro_aplicado: FiltrosHistorial::default(),
            busqueda: TextInput::default(),
            mensaje: None,
            empresas: vec![],
            offset: 0,
            corte_id: None,
            ayuda_expandida: false,
            busqueda_debounce: Debounce::default(),
        }
    }
}
impl HistorialState {
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
        let (filtros, texto_libre) =
            parsear_consulta(&self.filtro_aplicado, self.busqueda.value(), &self.empresas);
        construir(&filtros, &texto_libre, LIMIT, self.offset, self.corte_id)
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
        if standard_command(k) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionHistorial::Ninguna;
        }
        // F3 (cambiar de vista) y F4 (columnas, sólo en Clásica) son
        // transversales a cualquier modo, salvo mientras se edita el editor
        // de columnas mismo.
        if let KeyCode::F(3) = k.code {
            self.vista = self.vista.next();
            self.modo = ModoHistorial::Normal;
            return AccionHistorial::Ninguna;
        }
        if let KeyCode::F(4) = k.code
            && self.vista == ViewMode::Classic
            && self.modo == ModoHistorial::Normal
        {
            self.modo = ModoHistorial::Columnas { seleccion: 0 };
            return AccionHistorial::Ninguna;
        }
        match self.modo.clone() {
            ModoHistorial::Normal if self.vista == ViewMode::Heatmap => self.heatmap(k),
            ModoHistorial::Normal => self.normal(k),
            ModoHistorial::Columnas { seleccion } => {
                self.columnas(k, seleccion);
                AccionHistorial::Ninguna
            }
        }
    }
    /// El campo de búsqueda está siempre activo en modo Normal: cualquier
    /// carácter escrito filtra en vivo, sin necesidad de un atajo que lo
    /// active primero.
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
            KeyCode::Esc if !self.busqueda.value().is_empty() => {
                self.busqueda.clear();
                self.reiniciar_paginacion();
                return self.emitir();
            }
            KeyCode::Esc => return AccionHistorial::Volver,
            _ => {
                if self.busqueda.handle_key(k) {
                    self.reiniciar_paginacion();
                    self.busqueda_debounce.marcar(Instant::now());
                }
            }
        }
        AccionHistorial::Ninguna
    }
    /// Se llama en cada vuelta del bucle principal; dispara la búsqueda
    /// diferida sólo una vez que pasa `DURACION_DEBOUNCE` sin una tecla
    /// nueva, para no lanzar una consulta por cada carácter tecleado.
    pub fn tick(&mut self, ahora: Instant) -> AccionHistorial {
        if self.busqueda_debounce.listo(ahora, DURACION_DEBOUNCE) {
            self.emitir()
        } else {
            AccionHistorial::Ninguna
        }
    }
    /// El mapa de calor navega día a día con Tab/Shift+Tab y semana a
    /// semana con ↑↓ — ninguna de las dos colisiona con el campo de
    /// búsqueda porque ese modo no tiene texto libre que escribir.
    fn heatmap(&mut self, k: KeyEvent) -> AccionHistorial {
        match standard_command(k) {
            Some(StandardCommand::FocusNext) => {
                self.heatmap_seleccion += Duration::days(1);
                return AccionHistorial::Ninguna;
            }
            Some(StandardCommand::FocusPrevious) => {
                self.heatmap_seleccion -= Duration::days(1);
                return AccionHistorial::Ninguna;
            }
            Some(StandardCommand::Primary) => {
                let fecha = self.heatmap_seleccion.format("%d/%m/%Y").to_string();
                self.busqueda.set_value(format!("desde:{fecha} hasta:{fecha}"));
                self.vista = ViewMode::Timeline;
                self.reiniciar_paginacion();
                return self.emitir();
            }
            _ => {}
        }
        match k.code {
            KeyCode::Up => self.heatmap_seleccion -= Duration::days(7),
            KeyCode::Down => self.heatmap_seleccion += Duration::days(7),
            KeyCode::Esc => return AccionHistorial::Volver,
            _ => {}
        }
        AccionHistorial::Ninguna
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
                    seleccion: (s + 1).min(self.columnas_clasica.len() - 1),
                }
            }
            KeyCode::Char(' ') => {
                let n = self.columnas_clasica.iter().filter(|x| x.1).count();
                if self.columnas_clasica[s].1 && n == 1 {
                    // Mismo aviso que Activos/Contratistas — antes esta
                    // pantalla no hacía nada, sin explicar la restricción.
                    self.mensaje = Some("Debe conservar al menos una columna".into());
                } else {
                    self.columnas_clasica[s].1 = !self.columnas_clasica[s].1
                }
            }
            KeyCode::Esc | KeyCode::F(4) => self.modo = ModoHistorial::Normal,
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
    fn seleccionado(&self) -> Option<&MovimientoIngresoResumen> {
        self.registros.get(self.seleccion?)
    }
    /// Cuenta de movimientos por día (fecha local), a partir de la página
    /// actualmente cargada — igual que el timeline y la vista clásica, el
    /// mapa de calor lee del mismo conjunto ya filtrado y paginado, no de
    /// una agregación aparte en el backend.
    fn conteo_por_dia(&self) -> std::collections::BTreeMap<NaiveDate, usize> {
        let mut conteo = std::collections::BTreeMap::new();
        for r in &self.registros {
            *conteo
                .entry(a_costa_rica(r.fecha_hora_ingreso).date_naive())
                .or_insert(0usize) += 1;
        }
        conteo
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

fn start_of_week(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(date.weekday().num_days_from_monday()))
}
fn end_of_week(date: NaiveDate) -> NaiveDate {
    date + Duration::days(6 - i64::from(date.weekday().num_days_from_monday()))
}
fn bucket_glyph(count: usize, max: usize) -> char {
    if count == 0 {
        return '·';
    }
    let ratio = count as f64 / max.max(1) as f64;
    if ratio >= 0.99 {
        '█'
    } else if ratio >= 0.66 {
        '▓'
    } else if ratio >= 0.33 {
        '▒'
    } else {
        '░'
    }
}
