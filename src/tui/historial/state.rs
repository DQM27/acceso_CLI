use crate::{
    database::queries::ingresos::{FiltroHistorial, MovimientoIngresoResumen, PaginaHistorial},
    historial::ColumnaHistorial,
    models::empresa::Empresa,
};
use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::ui_kit::{Debounce, StandardCommand, TextInput, standard_command};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

const DURACION_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(120);
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
/// (curado, con panel de detalle) y la tabla clásica (densa, una línea por
/// movimiento, sin panel). Cambiar de vista conserva búsqueda y selección.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Timeline,
    Classic,
}
impl ViewMode {
    const fn next(self) -> Self {
        match self {
            Self::Timeline => Self::Classic,
            Self::Classic => Self::Timeline,
        }
    }
    const fn label(self) -> &'static str {
        match self {
            Self::Timeline => "LÍNEA DE TIEMPO",
            Self::Classic => "CLÁSICA",
        }
    }
}

/// Columnas de la vista Clásica. El timeline ya muestra todo en su panel,
/// pero Clásica no tiene panel — ahí sí tiene sentido poder ocultar
/// columnas para controlar la densidad.
impl ColumnaHistorial {
    pub(super) const fn constraint(self) -> ratatui::layout::Constraint {
        use ratatui::layout::Constraint;
        match self {
            Self::FechaIngreso | Self::FechaSalida => Constraint::Length(10),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropositoColumnas {
    Vista,
    Exportacion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoHistorial {
    Normal,
    Columnas {
        seleccion: usize,
        proposito: PropositoColumnas,
    },
    RutaExportacion {
        destino: TextInput,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionHistorial {
    Ninguna,
    Volver,
    Consultar(FiltroHistorial),
    Exportar {
        filtro: FiltroHistorial,
        columnas: Vec<ColumnaHistorial>,
        destino: PathBuf,
    },
}
#[derive(Debug)]
pub struct HistorialState {
    registros: Vec<MovimientoIngresoResumen>,
    total: usize,
    seleccion: Option<usize>,
    modo: ModoHistorial,
    vista: ViewMode,
    columnas_clasica: Vec<(ColumnaHistorial, bool)>,
    filtro_aplicado: FiltrosHistorial,
    busqueda: TextInput,
    mensaje: Option<String>,
    empresas: Vec<Empresa>,
    offset: usize,
    corte_id: Option<i64>,
    ayuda_expandida: bool,
    busqueda_debounce: Debounce,
    /// `true` mientras se espera el resultado real de exportar a XLSX (hilo
    /// aparte, `tui/app/historial_jobs.rs`) — bloquea disparar otra
    /// exportación mientras tanto, mismo criterio que
    /// `RespaldosState::creando`.
    exportando: bool,
}
impl Default for HistorialState {
    fn default() -> Self {
        Self {
            registros: vec![],
            total: 0,
            seleccion: None,
            modo: ModoHistorial::Normal,
            vista: ViewMode::Timeline,
            columnas_clasica: ColumnaHistorial::ALL
                .into_iter()
                .map(|c| (c, true))
                .collect(),
            filtro_aplicado: FiltrosHistorial::default(),
            busqueda: TextInput::default(),
            mensaje: None,
            empresas: vec![],
            offset: 0,
            corte_id: None,
            ayuda_expandida: false,
            busqueda_debounce: Debounce::default(),
            exportando: false,
        }
    }
}
impl HistorialState {
    pub(crate) fn vista_preferencia(&self) -> &'static str {
        match self.vista {
            ViewMode::Timeline => "timeline",
            ViewMode::Classic => "classic",
        }
    }

    pub(crate) fn aplicar_vista_preferencia(&mut self, valor: &str) {
        self.vista = match valor {
            "timeline" => ViewMode::Timeline,
            "classic" => ViewMode::Classic,
            _ => return,
        };
    }

    pub(crate) fn columnas_preferencia(&self) -> String {
        self.columnas_clasica
            .iter()
            .filter_map(|(columna, visible)| visible.then_some(columna.clave()))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub(crate) fn aplicar_columnas_preferencia(&mut self, valor: &str) {
        let claves: Vec<&str> = valor.split(',').collect();
        if !self
            .columnas_clasica
            .iter()
            .any(|(columna, _)| claves.contains(&columna.clave()))
        {
            return;
        }
        for (columna, visible) in &mut self.columnas_clasica {
            *visible = claves.contains(&columna.clave());
        }
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
    /// Repite la consulta actual (mismo filtro, página y `corte_id`) sin
    /// reiniciar la paginación — para refrescar el contenido ya visible tras
    /// un cambio hecho desde otra pantalla (p. ej. una salida registrada por
    /// F2 mientras el operador está viendo Historial), sin saltarlo a la
    /// página 1 ni desplazar lo que ya tenía cargado.
    pub fn refrescar(&mut self) -> AccionHistorial {
        self.emitir()
    }
    /// Conteo real de coincidencias de la página cargada, sin recortar por
    /// `LIMIT` — expuesto para que `app.rs` pueda verificar que una
    /// recarga en segundo plano sí actualizó el estado (p. ej. tras F2).
    pub fn total(&self) -> usize {
        self.total
    }
    pub fn completar(&mut self, r: Result<PaginaHistorial, String>) {
        match r {
            Ok(p) => {
                self.corte_id = Some(p.corte_id);
                self.registros = p.items;
                self.total = p.total;
                self.seleccion = (!self.registros.is_empty()).then_some(0);
            }
            Err(e) => {
                self.registros.clear();
                self.total = 0;
                self.seleccion = None;
                self.corte_id = None;
                self.mensaje = Some(e);
            }
        }
    }

    /// Marca que ya se disparó el hilo aparte que exporta a XLSX (ver
    /// `tui/app/historial_jobs.rs`) — la pantalla muestra "Exportando
    /// historial…" hasta que llegue el resultado real por
    /// `completar_exportacion`.
    pub fn marcar_exportando(&mut self) {
        self.exportando = true;
    }

    pub fn exportando(&self) -> bool {
        self.exportando
    }

    pub fn completar_exportacion(&mut self, resultado: Result<usize, String>, destino: &Path) {
        self.exportando = false;
        self.mensaje = Some(match resultado {
            Ok(filas) => format!("✓ Exportados {filas} movimientos a {}", destino.display()),
            Err(error) => format!("✕ {error}"),
        });
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
        match self.modo.clone() {
            ModoHistorial::Columnas {
                seleccion,
                proposito,
            } => return self.columnas(k, seleccion, proposito),
            ModoHistorial::RutaExportacion { .. } => return self.ruta_exportacion(k),
            ModoHistorial::Normal => {}
        }

        if k.code == KeyCode::F(3) {
            self.vista = self.vista.next();
            return AccionHistorial::Ninguna;
        }
        if k.code == KeyCode::F(4)
            && self.vista == ViewMode::Classic
            && self.modo == ModoHistorial::Normal
        {
            self.modo = ModoHistorial::Columnas {
                seleccion: 0,
                proposito: PropositoColumnas::Vista,
            };
            return AccionHistorial::Ninguna;
        }
        if k.code == KeyCode::F(5) {
            if self.exportando {
                // No hace nada — ya hay una exportación en vuelo, no se
                // encola una segunda.
            } else if self.total == 0 {
                self.mensaje = Some("No hay movimientos para exportar".into());
            } else {
                self.mensaje = None;
                self.modo = ModoHistorial::Columnas {
                    seleccion: 0,
                    proposito: PropositoColumnas::Exportacion,
                };
            }
            return AccionHistorial::Ninguna;
        }
        self.normal(k)
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
    fn columnas(&mut self, k: KeyEvent, s: usize, proposito: PropositoColumnas) -> AccionHistorial {
        match k.code {
            KeyCode::Up => {
                self.modo = ModoHistorial::Columnas {
                    seleccion: s.saturating_sub(1),
                    proposito,
                }
            }
            KeyCode::Down => {
                self.modo = ModoHistorial::Columnas {
                    seleccion: (s + 1).min(self.columnas_clasica.len() - 1),
                    proposito,
                }
            }
            KeyCode::Char(' ') => {
                let n = self.columnas_clasica.iter().filter(|x| x.1).count();
                if self.columnas_clasica[s].1 && n == 1 {
                    // Mismo aviso que Activos/Contratistas — antes esta
                    // pantalla no hacía nada, sin explicar la restricción.
                    self.mensaje = Some("Debe conservar al menos una columna".into());
                } else {
                    self.columnas_clasica[s].1 = !self.columnas_clasica[s].1;
                }
            }
            KeyCode::Enter if proposito == PropositoColumnas::Exportacion => {
                self.modo = ModoHistorial::RutaExportacion {
                    destino: TextInput::new(ruta_exportacion_predeterminada()).with_max_chars(240),
                };
            }
            KeyCode::Esc => self.modo = ModoHistorial::Normal,
            KeyCode::F(4) if proposito == PropositoColumnas::Vista => {
                self.modo = ModoHistorial::Normal;
            }
            _ => {}
        }
        AccionHistorial::Ninguna
    }

    fn ruta_exportacion(&mut self, k: KeyEvent) -> AccionHistorial {
        match k.code {
            KeyCode::Esc => {
                self.modo = ModoHistorial::Normal;
                AccionHistorial::Ninguna
            }
            KeyCode::Enter => {
                let ModoHistorial::RutaExportacion { destino } = &self.modo else {
                    return AccionHistorial::Ninguna;
                };
                let destino = match normalizar_destino(destino.value()) {
                    Ok(destino) => destino,
                    Err(error) => {
                        self.mensaje = Some(error);
                        return AccionHistorial::Ninguna;
                    }
                };
                let mut filtro = match self.consulta() {
                    Ok(filtro) => filtro,
                    Err(error) => {
                        self.mensaje = Some(error);
                        return AccionHistorial::Ninguna;
                    }
                };
                filtro.offset = 0;
                let columnas = self
                    .columnas_clasica
                    .iter()
                    .filter_map(|(columna, visible)| visible.then_some(*columna))
                    .collect();
                self.modo = ModoHistorial::Normal;
                AccionHistorial::Exportar {
                    filtro,
                    columnas,
                    destino,
                }
            }
            _ => {
                if let ModoHistorial::RutaExportacion { destino } = &mut self.modo {
                    destino.handle_key(k);
                }
                AccionHistorial::Ninguna
            }
        }
    }
    fn mover(&mut self, d: isize) {
        if self.registros.is_empty() {
            self.seleccion = None;
        } else {
            let i = self.seleccion.unwrap_or(0);
            self.seleccion = Some(if d < 0 {
                i.saturating_sub(1)
            } else {
                (i + 1).min(self.registros.len() - 1)
            });
        }
    }
    fn seleccionado(&self) -> Option<&MovimientoIngresoResumen> {
        self.registros.get(self.seleccion?)
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

fn ruta_exportacion_predeterminada() -> String {
    let nombre = format!(
        "historial_{}.xlsx",
        crate::tiempo::ahora_costa_rica().format("%Y-%m-%d_%H%M")
    );
    // `map_or_else` exigiría clonar `nombre` (una rama lo mueve, la otra lo
    // toma prestado) sólo para complacer al lint — sin beneficio real.
    #[allow(clippy::map_unwrap_or)]
    let ruta = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .filter(|ruta| ruta.is_absolute())
        .map(|ruta| ruta.join("Documents"))
        .filter(|ruta| ruta.is_dir())
        .map(|ruta| ruta.join(&nombre))
        .unwrap_or_else(|| PathBuf::from(nombre));
    ruta.display().to_string()
}

fn normalizar_destino(valor: &str) -> Result<PathBuf, String> {
    let valor = valor.trim();
    if valor.is_empty() {
        return Err("Ingrese una ruta para el archivo XLSX".into());
    }
    let mut destino = PathBuf::from(valor);
    match destino.extension().and_then(|extension| extension.to_str()) {
        None => {
            destino.set_extension("xlsx");
        }
        Some(extension) if extension.eq_ignore_ascii_case("xlsx") => {}
        Some(_) => return Err("La exportación debe usar la extensión .xlsx".into()),
    }
    Ok(destino)
}
