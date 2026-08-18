use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    database::backup::{RespaldoResumen, ResultadoValidacion, TipoRespaldo},
    tui::ui_kit::{StandardCommand, standard_command},
};

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpcionConfiguracion {
    Respaldos,
}

impl OpcionConfiguracion {
    pub const TODAS: [Self; 1] = [Self::Respaldos];

    pub fn etiqueta(self) -> &'static str {
        match self {
            Self::Respaldos => "1   Respaldos",
        }
    }

    pub fn descripcion(self) -> &'static str {
        match self {
            Self::Respaldos => "Crear, listar, validar y exportar respaldos de la base de datos.",
        }
    }
}

#[derive(Debug)]
enum ModoConfiguracion {
    Menu,
    Respaldos(RespaldosState),
}

#[derive(Debug)]
pub struct ConfiguracionState {
    modo: ModoConfiguracion,
    seleccion: OpcionConfiguracion,
    ayuda_expandida: bool,
}

impl Default for ConfiguracionState {
    fn default() -> Self {
        Self {
            modo: ModoConfiguracion::Menu,
            seleccion: OpcionConfiguracion::Respaldos,
            ayuda_expandida: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionAjustes {
    Ninguna,
    Volver,
    Respaldos(AccionRespaldos),
}

impl ConfiguracionState {
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionAjustes {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionAjustes::Ninguna;
        }
        match &mut self.modo {
            ModoConfiguracion::Menu => match key.code {
                KeyCode::Enter | KeyCode::Char('1') => {
                    self.modo = ModoConfiguracion::Respaldos(RespaldosState::default());
                    AccionAjustes::Respaldos(AccionRespaldos::Cargar)
                }
                KeyCode::Esc => AccionAjustes::Volver,
                _ => AccionAjustes::Ninguna,
            },
            ModoConfiguracion::Respaldos(estado) => {
                let accion = estado.handle_key(key);
                if accion == AccionRespaldos::Volver {
                    self.modo = ModoConfiguracion::Menu;
                    AccionAjustes::Ninguna
                } else {
                    AccionAjustes::Respaldos(accion)
                }
            }
        }
    }

    pub fn completar_listado(&mut self, resultado: Result<Vec<RespaldoResumen>, String>) {
        if let ModoConfiguracion::Respaldos(estado) = &mut self.modo {
            estado.completar_listado(resultado);
        }
    }

    pub fn completar_creacion(&mut self, resultado: Result<RespaldoResumen, String>) {
        if let ModoConfiguracion::Respaldos(estado) = &mut self.modo {
            estado.completar_creacion(resultado);
        }
    }

    pub fn completar_validacion(
        &mut self,
        ruta: &Path,
        resultado: Result<ResultadoValidacion, String>,
    ) {
        if let ModoConfiguracion::Respaldos(estado) = &mut self.modo {
            estado.completar_validacion(ruta, resultado);
        }
    }

    pub fn completar_exportacion(&mut self, resultado: Result<(), String>, destino: &Path) {
        if let ModoConfiguracion::Respaldos(estado) = &mut self.modo {
            estado.completar_exportacion(resultado, destino);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilaRespaldo {
    resumen: RespaldoResumen,
    validacion: Option<ResultadoValidacion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoRespaldos {
    Normal,
    Exportando { destino: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionRespaldos {
    Ninguna,
    Volver,
    Cargar,
    Crear,
    Revalidar { ruta: PathBuf },
    Exportar { ruta: PathBuf, destino: PathBuf },
}

#[derive(Debug)]
struct RespaldosState {
    filas: Vec<FilaRespaldo>,
    seleccion: Option<usize>,
    modo: ModoRespaldos,
    mensaje: Option<String>,
}

impl Default for RespaldosState {
    fn default() -> Self {
        Self {
            filas: Vec::new(),
            seleccion: None,
            modo: ModoRespaldos::Normal,
            mensaje: None,
        }
    }
}

impl RespaldosState {
    fn handle_key(&mut self, key: KeyEvent) -> AccionRespaldos {
        match &self.modo {
            ModoRespaldos::Normal => self.normal(key),
            ModoRespaldos::Exportando { .. } => self.exportando(key),
        }
    }

    fn normal(&mut self, key: KeyEvent) -> AccionRespaldos {
        self.mensaje = None;
        match key.code {
            KeyCode::Up => {
                self.mover(-1);
                AccionRespaldos::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionRespaldos::Ninguna
            }
            KeyCode::Char('c' | 'C') => AccionRespaldos::Crear,
            KeyCode::Char('a' | 'A') => AccionRespaldos::Cargar,
            KeyCode::Char('r' | 'R') => self
                .ruta_seleccionada()
                .map_or(AccionRespaldos::Ninguna, |ruta| {
                    AccionRespaldos::Revalidar { ruta }
                }),
            KeyCode::Char('e' | 'E') => {
                if self.ruta_seleccionada().is_some() {
                    self.modo = ModoRespaldos::Exportando {
                        destino: String::new(),
                    };
                }
                AccionRespaldos::Ninguna
            }
            KeyCode::Esc => AccionRespaldos::Volver,
            _ => AccionRespaldos::Ninguna,
        }
    }

    fn exportando(&mut self, key: KeyEvent) -> AccionRespaldos {
        match key.code {
            KeyCode::Esc => {
                self.modo = ModoRespaldos::Normal;
                AccionRespaldos::Ninguna
            }
            KeyCode::Enter => {
                let ModoRespaldos::Exportando { destino } = &self.modo else {
                    return AccionRespaldos::Ninguna;
                };
                let destino = destino.trim();
                if destino.is_empty() {
                    return AccionRespaldos::Ninguna;
                }
                let Some(ruta) = self.ruta_seleccionada() else {
                    self.modo = ModoRespaldos::Normal;
                    return AccionRespaldos::Ninguna;
                };
                let destino = PathBuf::from(destino);
                self.modo = ModoRespaldos::Normal;
                AccionRespaldos::Exportar { ruta, destino }
            }
            KeyCode::Backspace => {
                if let ModoRespaldos::Exportando { destino } = &mut self.modo {
                    destino.pop();
                }
                AccionRespaldos::Ninguna
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let ModoRespaldos::Exportando { destino } = &mut self.modo
                    && destino.chars().count() < 240
                {
                    destino.push(c);
                }
                AccionRespaldos::Ninguna
            }
            _ => AccionRespaldos::Ninguna,
        }
    }

    fn mover(&mut self, delta: isize) {
        if self.filas.is_empty() {
            self.seleccion = None;
            return;
        }
        let actual = self.seleccion.unwrap_or(0);
        self.seleccion = Some(if delta < 0 {
            actual.saturating_sub(1)
        } else {
            (actual + 1).min(self.filas.len() - 1)
        });
    }

    fn ruta_seleccionada(&self) -> Option<PathBuf> {
        self.seleccion
            .and_then(|indice| self.filas.get(indice))
            .map(|fila| fila.resumen.ruta.clone())
    }

    fn seleccionada(&self) -> Option<&FilaRespaldo> {
        self.seleccion.and_then(|indice| self.filas.get(indice))
    }

    fn completar_listado(&mut self, resultado: Result<Vec<RespaldoResumen>, String>) {
        match resultado {
            Ok(items) => {
                self.filas = items
                    .into_iter()
                    .map(|resumen| FilaRespaldo {
                        resumen,
                        validacion: None,
                    })
                    .collect();
                self.seleccion = (!self.filas.is_empty()).then_some(0);
            }
            Err(error) => {
                self.filas.clear();
                self.seleccion = None;
                self.mensaje = Some(error);
            }
        }
    }

    fn completar_creacion(&mut self, resultado: Result<RespaldoResumen, String>) {
        match resultado {
            Ok(resumen) => {
                self.mensaje = Some(format!("✓ Respaldo creado — {}", nombre_archivo(&resumen.ruta)));
                self.filas.insert(
                    0,
                    FilaRespaldo {
                        resumen,
                        validacion: None,
                    },
                );
                self.seleccion = Some(0);
            }
            Err(error) => self.mensaje = Some(format!("✕ {error}")),
        }
    }

    fn completar_validacion(&mut self, ruta: &Path, resultado: Result<ResultadoValidacion, String>) {
        match resultado {
            Ok(validacion) => {
                if let Some(fila) = self
                    .filas
                    .iter_mut()
                    .find(|fila| fila.resumen.ruta.as_path() == ruta)
                {
                    fila.validacion = Some(validacion);
                }
                self.mensaje = Some(format!("Revalidado — {}", nombre_archivo(ruta)));
            }
            Err(error) => self.mensaje = Some(format!("✕ {error}")),
        }
    }

    fn completar_exportacion(&mut self, resultado: Result<(), String>, destino: &Path) {
        match resultado {
            Ok(()) => self.mensaje = Some(format!("✓ Exportado a {}", destino.display())),
            Err(error) => self.mensaje = Some(format!("✕ {error}")),
        }
    }
}

fn nombre_archivo(ruta: &Path) -> String {
    ruta.file_name()
        .map(|nombre| nombre.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn etiqueta_tipo(tipo: TipoRespaldo) -> &'static str {
    match tipo {
        TipoRespaldo::Manual => "Manual",
        TipoRespaldo::Automatico => "Automático",
        TipoRespaldo::PreMigracion => "Pre-migración",
        TipoRespaldo::PreRestauracion => "Pre-restauración",
    }
}

fn etiqueta_validacion(validacion: Option<&ResultadoValidacion>) -> &'static str {
    match validacion {
        None => "—",
        Some(ResultadoValidacion::Valido { .. }) => "VÁLIDO",
        Some(ResultadoValidacion::Invalido(_)) => "INVÁLIDO",
        Some(ResultadoValidacion::EsquemaIncompatible { .. }) => "ESQUEMA FUTURO",
    }
}

fn etiqueta_esquema(validacion: Option<&ResultadoValidacion>) -> String {
    match validacion {
        None => "—".to_owned(),
        Some(ResultadoValidacion::Valido { version_esquema }) => version_esquema.to_string(),
        Some(ResultadoValidacion::EsquemaIncompatible { version_encontrada }) => {
            version_encontrada.to_string()
        }
        Some(ResultadoValidacion::Invalido(_)) => "—".to_owned(),
    }
}

fn tamano_legible(bytes: u64) -> String {
    const UNIDAD: f64 = 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f < UNIDAD {
        format!("{bytes} B")
    } else if bytes_f < UNIDAD.powi(2) {
        format!("{:.1} KB", bytes_f / UNIDAD)
    } else {
        format!("{:.1} MB", bytes_f / UNIDAD.powi(2))
    }
}
