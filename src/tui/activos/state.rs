use crate::{
    database::queries::{Igualdad, ingresos::FiltroIngresosActivos},
    models::{empresa::Empresa, medio_ingreso::MedioIngreso, tipo_ingreso::TipoIngreso},
    services::registro_ingreso_service::{IngresoActivoResumen, ListaIngresosActivosResumen},
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Constraint;

use crate::tui::ui_kit::{
    Debounce, StandardCommand, Term, TextInput, plegar_diacriticos, resolver_terminos,
    resolver_terminos_detallado, standard_command, valores,
};
use std::time::Instant;

const DURACION_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(120);
#[path = "render.rs"]
pub(super) mod render;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Interpreta `texto` con la sintaxis `clave:valor` de `ui_kit::query_lang`
/// (`empresa:`, `tipo:`, `gafete:`, `medio:` — todas con negación
/// `-clave:valor`). Lo no reconocido, y lo que no calza con lo que una clave
/// admite, se deja como texto libre para nombre/cédula/gafete.
fn parsear_consulta(texto: &str, empresas: &[Empresa]) -> (FiltroIngresosActivos, String) {
    let mut filtro = FiltroIngresosActivos::default();
    let libres = resolver_terminos(texto, &mut filtro, |f, term| {
        aplicar_clave(f, term, empresas)
    });
    (filtro, libres)
}

fn aplicar_clave(f: &mut FiltroIngresosActivos, term: &Term, empresas: &[Empresa]) -> bool {
    let clave = term.key.as_deref().unwrap_or_default().to_lowercase();
    let valores = valores(term);
    match clave.as_str() {
        "empresa" if valores.len() == 1 => {
            let buscado = plegar_diacriticos(&valores[0].to_lowercase());
            match empresas
                .iter()
                .find(|e| plegar_diacriticos(&e.nombre.to_lowercase()).contains(&buscado))
            {
                Some(e) => {
                    f.empresa_id = Some(if term.negated {
                        Igualdad::Excluye(e.id)
                    } else {
                        Igualdad::Incluye(e.id)
                    });
                    true
                }
                None => false,
            }
        }
        "tipo" => {
            let Some(reconocidos) = valores
                .iter()
                .map(|v| TipoIngreso::from_str_filtro(v))
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            if reconocidos.is_empty() {
                return false;
            }
            f.tipos_incluidos = Some(if term.negated {
                TipoIngreso::ALL
                    .into_iter()
                    .filter(|t| !reconocidos.contains(t))
                    .collect()
            } else {
                reconocidos
            });
            true
        }
        "gafete" if valores.len() == 1 => match valores[0].parse::<i64>() {
            Ok(n) => {
                f.gafete_numero = Some(if term.negated {
                    Igualdad::Excluye(n)
                } else {
                    Igualdad::Incluye(n)
                });
                true
            }
            Err(_) => false,
        },
        "medio" if valores.len() == 1 => match medio_desde_texto(&valores[0]) {
            Some(m) => {
                f.medio_ingreso = Some(if term.negated { medio_opuesto(m) } else { m });
                true
            }
            None => false,
        },
        _ => false,
    }
}

fn medio_desde_texto(v: &str) -> Option<MedioIngreso> {
    match v.to_lowercase().as_str() {
        "caminando" | "pie" | "apie" => Some(MedioIngreso::Caminando),
        "vehiculo" | "vehículo" | "carro" => Some(MedioIngreso::Vehiculo),
        _ => None,
    }
}

/// Sólo hay 2 variantes de `MedioIngreso`, así que negar una equivale a
/// pedir la otra (`-medio:caminando` == `medio:vehiculo`). El patrón
/// exhaustivo deja de compilar, en vez de fallar en silencio, si se agrega
/// una tercera variante — momento en el que esta función dejaría de ser
/// válida y `medio_ingreso` tendría que pasar a admitir una lista, como ya
/// hace `tipo`.
fn medio_opuesto(m: MedioIngreso) -> MedioIngreso {
    match m {
        MedioIngreso::Caminando => MedioIngreso::Vehiculo,
        MedioIngreso::Vehiculo => MedioIngreso::Caminando,
    }
}

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
            Self::Usuario => "DA INGRESO",
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
    const fn clave(self) -> &'static str {
        match self {
            Self::Cedula => "cedula",
            Self::Nombre => "nombre",
            Self::Empresa => "empresa",
            Self::Tipo => "tipo",
            Self::Hora => "hora",
            Self::Gafete => "gafete",
            Self::Medio => "medio",
            Self::Usuario => "usuario",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModoActivos {
    Normal,
    Busqueda { texto: TextInput },
    ConfirmarSalida { id: i64 },
    Columnas { seleccion: usize },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionActivos {
    Ninguna,
    Volver,
    Buscar {
        texto: Option<String>,
        seleccionar_id: Option<i64>,
        empresa_id: Option<Igualdad<i64>>,
        tipos: Option<Vec<TipoIngreso>>,
        gafete: Option<Igualdad<i64>>,
        medio: Option<MedioIngreso>,
    },
    RegistrarSalida {
        registro_id: i64,
        nombre: String,
    },
}
#[derive(Debug)]
pub struct ActivosState {
    registros: Vec<IngresoActivoResumen>,
    total_activos: usize,
    seleccion: Option<usize>,
    modo: ModoActivos,
    columnas: Vec<(Columna, bool)>,
    mensaje: Option<String>,
    pub(crate) filtro: String,
    empresas: Vec<Empresa>,
    ayuda_expandida: bool,
    busqueda_debounce: Debounce,
}
impl Default for ActivosState {
    fn default() -> Self {
        Self {
            registros: vec![],
            total_activos: 0,
            seleccion: None,
            modo: ModoActivos::Normal,
            columnas: Columna::TODAS
                .into_iter()
                .map(|c| (c, !matches!(c, Columna::Medio | Columna::Usuario)))
                .collect(),
            mensaje: None,
            filtro: String::new(),
            empresas: vec![],
            ayuda_expandida: false,
            busqueda_debounce: Debounce::default(),
        }
    }
}
impl ActivosState {
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

    pub fn completar_empresas(&mut self, r: Result<Vec<Empresa>, String>) {
        if let Ok(e) = r {
            self.empresas = e;
        }
    }
    pub fn solicitud_carga(&self) -> AccionActivos {
        self.buscar(self.id_seleccionado())
    }
    pub(crate) fn buscar(&self, id: Option<i64>) -> AccionActivos {
        let (filtro, libre) = parsear_consulta(&self.filtro, &self.empresas);
        AccionActivos::Buscar {
            texto: (!libre.trim().is_empty()).then_some(libre),
            seleccionar_id: id,
            empresa_id: filtro.empresa_id,
            tipos: filtro.tipos_incluidos,
            gafete: filtro.gafete_numero,
            medio: filtro.medio_ingreso,
        }
    }
    pub(super) fn resumen_consulta(&self) -> String {
        if self.filtro.trim().is_empty() {
            return "Ejemplo: empresa:Brisas tipo:praind gafete:26".to_owned();
        }

        let mut filtro = FiltroIngresosActivos::default();
        let resolucion = resolver_terminos_detallado(&self.filtro, &mut filtro, |f, term| {
            aplicar_clave(f, term, &self.empresas)
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
        if let Some(gafete) = filtro.gafete_numero {
            let signo = if gafete.negado() { "≠" } else { "=" };
            partes.push(format!("gafete{signo}{}", gafete.valor()));
        }
        if let Some(medio) = filtro.medio_ingreso {
            partes.push(format!(
                "medio={}",
                match medio {
                    MedioIngreso::Caminando => "caminando",
                    MedioIngreso::Vehiculo => "vehículo",
                }
            ));
        }
        if !resolucion.no_reconocidos.is_empty() {
            partes.push(format!(
                "⚠ sin interpretar: {}",
                resolucion.no_reconocidos.join(", ")
            ));
        }
        partes.join(" · ")
    }
    pub fn cantidad(&self) -> usize {
        self.registros.len()
    }
    pub fn total_activos(&self) -> usize {
        self.total_activos
    }
    pub fn modo(&self) -> &ModoActivos {
        &self.modo
    }
    pub fn completar_busqueda(
        &mut self,
        r: Result<ListaIngresosActivosResumen, String>,
        id: Option<i64>,
    ) {
        match r {
            Ok(resultado) => {
                self.registros = resultado.items;
                self.total_activos = resultado.total;
                if !matches!(self.mensaje.as_deref(), Some(mensaje) if mensaje.starts_with('✓')) {
                    self.mensaje = None;
                }
                self.seleccion = id
                    .and_then(|x| self.registros.iter().position(|r| r.registro_id == x))
                    .or_else(|| (!self.registros.is_empty()).then_some(0));
            }
            Err(e) => {
                self.registros.clear();
                self.total_activos = 0;
                self.seleccion = None;
                self.mensaje = Some(e);
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
                // Mismo criterio que las demás pantallas tras una escritura
                // exitosa — antes Activos nunca limpiaba el filtro.
                self.filtro.clear();
                self.mensaje = Some(format!("✓ Salida registrada — {nombre}"));
                self.buscar(Some(id))
            }
            Err(e) => {
                self.mensaje = Some(e);
                self.buscar(None)
            }
        }
    }
    pub fn handle_key(&mut self, k: KeyEvent) -> AccionActivos {
        if standard_command(k) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionActivos::Ninguna;
        }
        match self.modo.clone() {
            ModoActivos::Normal => self.normal(k),
            ModoActivos::Busqueda { .. } => self.busqueda(k),
            ModoActivos::ConfirmarSalida { id } => self.confirmar(k, id),
            ModoActivos::Columnas { seleccion } => {
                self.columnas(k, seleccion);
                AccionActivos::Ninguna
            }
        }
    }
    fn normal(&mut self, k: KeyEvent) -> AccionActivos {
        if matches!(k.code, KeyCode::Enter | KeyCode::Char('/') | KeyCode::Esc) {
            self.mensaje = None;
        }
        match k.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => {
                if let Some(id) = self.id_seleccionado() {
                    self.modo = ModoActivos::ConfirmarSalida { id }
                }
            }
            KeyCode::Char('/') => {
                self.modo = ModoActivos::Busqueda {
                    texto: TextInput::default(),
                }
            }
            KeyCode::F(4) => self.modo = ModoActivos::Columnas { seleccion: 0 },
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                return self.buscar(None);
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
                self.buscar(None)
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
            _ => {
                let mut cambio = false;
                if let ModoActivos::Busqueda { texto } = &mut self.modo {
                    cambio = texto.handle_key(k);
                    self.filtro = texto.value().to_owned();
                }
                if cambio {
                    self.busqueda_debounce.marcar(Instant::now());
                }
                AccionActivos::Ninguna
            }
        }
    }
    /// Se llama en cada vuelta del bucle principal; dispara la búsqueda
    /// diferida sólo una vez que pasa `DURACION_DEBOUNCE` sin una tecla
    /// nueva.
    pub fn tick(&mut self, ahora: Instant) -> AccionActivos {
        if self.busqueda_debounce.listo(ahora, DURACION_DEBOUNCE) {
            self.buscar(None)
        } else {
            AccionActivos::Ninguna
        }
    }
    fn confirmar(&mut self, k: KeyEvent, id: i64) -> AccionActivos {
        let nombre = self
            .registro(id)
            .map(|r| r.contratista_nombre.clone())
            .unwrap_or_default();
        self.confirmar_registro(k, id, nombre)
    }
    fn confirmar_registro(&mut self, k: KeyEvent, id: i64, nombre: String) -> AccionActivos {
        match k.code {
            KeyCode::Enter => AccionActivos::RegistrarSalida {
                registro_id: id,
                nombre,
            },
            KeyCode::Esc => {
                self.modo = ModoActivos::Normal;
                AccionActivos::Ninguna
            }
            _ => AccionActivos::Ninguna,
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
                    self.mensaje = Some("Debe conservar al menos una columna".into());
                } else {
                    self.columnas[s].1 = !self.columnas[s].1;
                }
            }
            KeyCode::Esc => self.modo = ModoActivos::Normal,
            _ => {}
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
    fn id_seleccionado(&self) -> Option<i64> {
        self.registros.get(self.seleccion?).map(|r| r.registro_id)
    }
    fn registro(&self, id: i64) -> Option<&IngresoActivoResumen> {
        self.registros.iter().find(|r| r.registro_id == id)
    }
    pub fn seleccionado(&self) -> Option<&IngresoActivoResumen> {
        self.registros.get(self.seleccion?)
    }
    pub fn inicio_visible(&self, c: usize) -> usize {
        self.seleccion
            .unwrap_or(0)
            .saturating_sub(c.saturating_sub(1))
    }
}
