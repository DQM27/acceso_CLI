use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    database::backup::{RespaldoResumen, ResultadoValidacion, TipoRespaldo},
    tui::ui_kit::{StandardCommand, TextInput, standard_command},
};

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Default)]
pub struct ConfiguracionState {
    respaldos: RespaldosState,
    ayuda_expandida: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionAjustes {
    Ninguna,
    Volver,
    Respaldos(AccionRespaldos),
}

impl ConfiguracionState {
    pub fn reiniciar(&mut self) -> AccionAjustes {
        // `fallo_automatico` no es estado transitorio de la pantalla, es el
        // estado real del respaldo automático — entrar a la pantalla no
        // debe hacerlo desaparecer, sólo un intento nuevo (exitoso o no) lo
        // reemplaza.
        let fallo_automatico = self.respaldos.fallo_automatico.take();
        *self = Self::default();
        self.respaldos.fallo_automatico = fallo_automatico;
        AccionAjustes::Respaldos(AccionRespaldos::Cargar)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AccionAjustes {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionAjustes::Ninguna;
        }
        let accion = self.respaldos.handle_key(key);
        if accion == AccionRespaldos::Volver {
            AccionAjustes::Volver
        } else {
            AccionAjustes::Respaldos(accion)
        }
    }

    pub fn completar_listado(&mut self, resultado: Result<Vec<RespaldoResumen>, String>) {
        self.respaldos.completar_listado(resultado);
    }

    /// Marca que ya se disparó el hilo aparte que crea el respaldo (ver
    /// `tui/app/backup_jobs.rs`) — la pantalla muestra "Creando respaldo…"
    /// hasta que llegue el resultado real por `completar_creacion`.
    pub fn marcar_creando_respaldo(&mut self) {
        self.respaldos.creando = true;
    }

    pub fn creando_respaldo(&self) -> bool {
        self.respaldos.creando
    }

    pub fn completar_creacion(&mut self, resultado: Result<RespaldoResumen, String>) {
        self.respaldos.completar_creacion(resultado);
    }

    pub fn completar_validacion(
        &mut self,
        ruta: &Path,
        resultado: Result<ResultadoValidacion, String>,
    ) {
        self.respaldos.completar_validacion(ruta, resultado);
    }

    pub fn completar_exportacion(&mut self, resultado: Result<(), String>, destino: &Path) {
        self.respaldos.completar_exportacion(resultado, destino);
    }

    /// Empuja el resultado de la revisión periódica del respaldo automático
    /// (`AppCore::respaldo_automatico_diario_si_hace_falta`) — `None` cuando
    /// el más reciente tuvo éxito o no hacía falta ninguno todavía.
    pub fn actualizar_fallo_respaldo_automatico(&mut self, fallo: Option<String>) {
        self.respaldos.fallo_automatico = fallo;
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
    Exportando {
        destino: TextInput,
    },
    ConfirmandoRestauracion {
        ruta: PathBuf,
        fecha: String,
        tipo: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionRespaldos {
    Ninguna,
    Volver,
    Cargar,
    Crear,
    Revalidar { ruta: PathBuf },
    Exportar { ruta: PathBuf, destino: PathBuf },
    Restaurar { ruta: PathBuf },
}

#[derive(Debug)]
struct RespaldosState {
    filas: Vec<FilaRespaldo>,
    seleccion: Option<usize>,
    modo: ModoRespaldos,
    mensaje: Option<String>,
    /// A diferencia de `mensaje` (transitorio, se limpia con la siguiente
    /// tecla), refleja el estado real del respaldo automático hasta que se
    /// resuelva — no debe desaparecer solo porque el operador movió la
    /// selección.
    fallo_automatico: Option<String>,
    /// `true` mientras se espera el resultado real de crear un respaldo
    /// (copiar + validar toda la base en un hilo aparte) — bloquea disparar
    /// otro mientras tanto, mismo criterio que `UsuariosState::guardando`.
    creando: bool,
}

impl Default for RespaldosState {
    fn default() -> Self {
        Self {
            filas: Vec::new(),
            seleccion: None,
            modo: ModoRespaldos::Normal,
            mensaje: None,
            fallo_automatico: None,
            creando: false,
        }
    }
}

impl RespaldosState {
    fn handle_key(&mut self, key: KeyEvent) -> AccionRespaldos {
        match &self.modo {
            ModoRespaldos::Normal => self.normal(key),
            ModoRespaldos::Exportando { .. } => self.exportando(key),
            ModoRespaldos::ConfirmandoRestauracion { .. } => self.confirmando_restauracion(key),
        }
    }

    fn normal(&mut self, key: KeyEvent) -> AccionRespaldos {
        if matches!(
            key.code,
            KeyCode::Char('c' | 'C' | 'l' | 'L' | 'v' | 'V' | 'e' | 'E' | 'r' | 'R') | KeyCode::Esc
        ) {
            self.mensaje = None;
        }
        match key.code {
            KeyCode::Up => {
                self.mover(-1);
                AccionRespaldos::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionRespaldos::Ninguna
            }
            KeyCode::Char('c' | 'C') if !self.creando => AccionRespaldos::Crear,
            // `A` está reservado en el resto de la app para "Activar/
            // Desactivar" (Empresas, Usuarios) — usar `L` (Listar) aquí en
            // vez de reutilizar la misma letra con un significado distinto.
            KeyCode::Char('l' | 'L') => AccionRespaldos::Cargar,
            KeyCode::Char('v' | 'V') => self
                .ruta_seleccionada()
                .map_or(AccionRespaldos::Ninguna, |ruta| {
                    AccionRespaldos::Revalidar { ruta }
                }),
            KeyCode::Char('e' | 'E') => {
                if self.ruta_seleccionada().is_some() {
                    self.modo = ModoRespaldos::Exportando {
                        destino: TextInput::default().with_max_chars(240),
                    };
                }
                AccionRespaldos::Ninguna
            }
            KeyCode::Char('r' | 'R') => {
                if let Some(fila) = self.seleccionada() {
                    self.modo = ModoRespaldos::ConfirmandoRestauracion {
                        ruta: fila.resumen.ruta.clone(),
                        fecha: crate::tiempo::a_costa_rica(fila.resumen.creado_en)
                            .format("%d/%m/%Y %H:%M")
                            .to_string(),
                        tipo: etiqueta_tipo(fila.resumen.tipo),
                    };
                }
                AccionRespaldos::Ninguna
            }
            KeyCode::Esc => AccionRespaldos::Volver,
            _ => AccionRespaldos::Ninguna,
        }
    }

    fn confirmando_restauracion(&mut self, key: KeyEvent) -> AccionRespaldos {
        let ModoRespaldos::ConfirmandoRestauracion { ruta, .. } = &self.modo else {
            return AccionRespaldos::Ninguna;
        };
        match key.code {
            KeyCode::Enter => {
                let ruta = ruta.clone();
                self.modo = ModoRespaldos::Normal;
                AccionRespaldos::Restaurar { ruta }
            }
            KeyCode::Esc => {
                self.modo = ModoRespaldos::Normal;
                AccionRespaldos::Ninguna
            }
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
                let destino = destino.value().trim();
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
            _ => {
                if let ModoRespaldos::Exportando { destino } = &mut self.modo {
                    destino.handle_key(key);
                }
                AccionRespaldos::Ninguna
            }
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
        self.creando = false;
        match resultado {
            Ok(resumen) => {
                self.mensaje = Some(format!(
                    "✓ Respaldo creado — {}",
                    nombre_archivo(&resumen.ruta)
                ));
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

    fn completar_validacion(
        &mut self,
        ruta: &Path,
        resultado: Result<ResultadoValidacion, String>,
    ) {
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
        TipoRespaldo::PorFlag => "Por flag (CLI)",
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
        None | Some(ResultadoValidacion::Invalido(_)) => "—".to_owned(),
        Some(ResultadoValidacion::Valido { version_esquema }) => version_esquema.to_string(),
        Some(ResultadoValidacion::EsquemaIncompatible { version_encontrada }) => {
            version_encontrada.to_string()
        }
    }
}

fn tamano_legible(bytes: u64) -> String {
    const UNIDAD: f64 = 1024.0;
    // Sólo para mostrar KB/MB legibles — un tamaño de archivo real (bytes de
    // un respaldo) está lejísimos del punto donde `f64` empieza a perder
    // precisión (2^52), y aunque lo estuviera sólo afectaría el redondeo
    // mostrado, nunca una decisión real del programa.
    #[allow(clippy::cast_precision_loss)]
    let bytes_f = bytes as f64;
    if bytes_f < UNIDAD {
        format!("{bytes} B")
    } else if bytes_f < UNIDAD.powi(2) {
        format!("{:.1} KB", bytes_f / UNIDAD)
    } else {
        format!("{:.1} MB", bytes_f / UNIDAD.powi(2))
    }
}
