use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    database::queries::contratistas::ContratistaResumen, domain::resultado_acceso::ResultadoAcceso,
    models::medio_ingreso::MedioIngreso, services::registro_ingreso_service::PreparacionIngreso,
};

#[path = "render.rs"]
pub(super) mod render;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const MEDIOS: [MedioIngreso; 2] = [MedioIngreso::Caminando, MedioIngreso::Vehiculo];

#[derive(Debug, Clone, PartialEq, Eq)]
enum EtapaNuevoIngreso {
    Buscar,
    Preparacion,
    Medio { opcion: usize },
    Gafete,
    Confirmar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionNuevoIngreso {
    Ninguna,
    Volver,
    Buscar {
        texto: Option<String>,
    },
    Preparar {
        contratista_id: i64,
    },
    ConsultarGafete {
        numero: i64,
    },
    Registrar {
        contratista_id: i64,
        medio: MedioIngreso,
        gafete: Option<i64>,
    },
}

#[derive(Debug)]
pub struct NuevoIngresoState {
    etapa: EtapaNuevoIngreso,
    contratistas: Vec<ContratistaResumen>,
    busqueda: String,
    seleccion: Option<usize>,
    contratista_id: Option<i64>,
    preparacion: Option<PreparacionIngreso>,
    medio: Option<MedioIngreso>,
    gafete_texto: String,
    gafete: Option<i64>,
    error: Option<String>,
    usuario_nombre: String,
}

impl Default for NuevoIngresoState {
    fn default() -> Self {
        Self::new("Quintana")
    }
}

impl NuevoIngresoState {
    pub fn new(usuario: impl Into<String>) -> Self {
        Self {
            etapa: EtapaNuevoIngreso::Buscar,
            contratistas: vec![],
            busqueda: String::new(),
            seleccion: None,
            contratista_id: None,
            preparacion: None,
            medio: None,
            gafete_texto: String::new(),
            gafete: None,
            error: None,
            usuario_nombre: usuario.into(),
        }
    }
    pub fn solicitud_carga(&self) -> AccionNuevoIngreso {
        AccionNuevoIngreso::Buscar { texto: None }
    }
    pub fn completar_busqueda(&mut self, r: Result<Vec<ContratistaResumen>, String>) {
        match r {
            Ok(v) => {
                self.contratistas = v;
                self.seleccion = (!self.contratistas.is_empty()).then_some(0)
            }
            Err(e) => {
                self.contratistas.clear();
                self.seleccion = None;
                self.error = Some(e)
            }
        }
    }
    pub fn completar_preparacion(&mut self, r: Result<PreparacionIngreso, String>) {
        match r {
            Ok(p) => {
                self.contratista_id = Some(p.contratista_id);
                self.preparacion = Some(p);
                self.medio = None;
                self.gafete = None;
                self.gafete_texto.clear();
                self.error = None;
                self.etapa = EtapaNuevoIngreso::Preparacion
            }
            Err(e) => self.error = Some(e),
        }
    }
    pub fn completar_gafete(&mut self, r: Result<bool, String>) {
        match r {
            Ok(false) => {
                self.gafete = self.gafete_texto.trim().parse().ok();
                self.error = None;
                self.etapa = EtapaNuevoIngreso::Confirmar
            }
            Ok(true) => self.error = Some("El gafete ya está en uso".into()),
            Err(e) => self.error = Some(e),
        }
    }
    pub fn completar_registro(&mut self, r: Result<i64, String>) -> bool {
        match r {
            Ok(_) => {
                self.limpiar();
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionNuevoIngreso {
        match self.etapa {
            EtapaNuevoIngreso::Buscar => self.buscar(key),
            EtapaNuevoIngreso::Preparacion => self.preparacion(key),
            EtapaNuevoIngreso::Medio { opcion } => self.medio(key, opcion),
            EtapaNuevoIngreso::Gafete => self.gafete(key),
            EtapaNuevoIngreso::Confirmar => self.confirmar(key),
        }
    }
    fn buscar(&mut self, key: KeyEvent) -> AccionNuevoIngreso {
        match key.code {
            KeyCode::Esc if !self.busqueda.is_empty() => {
                self.busqueda.clear();
                AccionNuevoIngreso::Buscar { texto: None }
            }
            KeyCode::Esc => AccionNuevoIngreso::Volver,
            KeyCode::Up => {
                self.mover(-1);
                AccionNuevoIngreso::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionNuevoIngreso::Ninguna
            }
            KeyCode::Enter => self
                .contratistas
                .get(self.seleccion.unwrap_or(usize::MAX))
                .map_or(AccionNuevoIngreso::Ninguna, |c| {
                    AccionNuevoIngreso::Preparar {
                        contratista_id: c.id,
                    }
                }),
            KeyCode::Backspace => {
                self.busqueda.pop();
                AccionNuevoIngreso::Buscar {
                    texto: texto_filtro(&self.busqueda),
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.busqueda.push(c);
                AccionNuevoIngreso::Buscar {
                    texto: texto_filtro(&self.busqueda),
                }
            }
            _ => AccionNuevoIngreso::Ninguna,
        }
    }
    fn preparacion(&mut self, key: KeyEvent) -> AccionNuevoIngreso {
        match key.code {
            KeyCode::Esc => {
                self.limpiar_seleccion();
                self.etapa = EtapaNuevoIngreso::Buscar
            }
            KeyCode::Enter if self.puede_continuar() => {
                self.etapa = EtapaNuevoIngreso::Medio { opcion: 0 }
            }
            _ => {}
        }
        AccionNuevoIngreso::Ninguna
    }
    fn medio(&mut self, key: KeyEvent, mut opcion: usize) -> AccionNuevoIngreso {
        match key.code {
            KeyCode::Esc => self.etapa = EtapaNuevoIngreso::Preparacion,
            KeyCode::Up => opcion = opcion.saturating_sub(1),
            KeyCode::Down => opcion = (opcion + 1).min(1),
            KeyCode::Enter => {
                self.medio = Some(MEDIOS[opcion]);
                self.etapa = if self.preparacion.as_ref().is_some_and(|p| p.requiere_gafete) {
                    EtapaNuevoIngreso::Gafete
                } else {
                    self.gafete = None;
                    EtapaNuevoIngreso::Confirmar
                }
            }
            _ => {}
        }
        if matches!(self.etapa, EtapaNuevoIngreso::Medio { .. }) {
            self.etapa = EtapaNuevoIngreso::Medio { opcion }
        }
        AccionNuevoIngreso::Ninguna
    }
    fn gafete(&mut self, key: KeyEvent) -> AccionNuevoIngreso {
        match key.code {
            KeyCode::Esc => {
                self.error = None;
                self.etapa = EtapaNuevoIngreso::Medio {
                    opcion: self.medio.map(indice_medio).unwrap_or(0),
                };
                AccionNuevoIngreso::Ninguna
            }
            KeyCode::Backspace => {
                self.gafete_texto.pop();
                self.error = None;
                AccionNuevoIngreso::Ninguna
            }
            KeyCode::Enter => match self.gafete_texto.trim().parse::<i64>() {
                Ok(numero) => AccionNuevoIngreso::ConsultarGafete { numero },
                Err(_) => {
                    self.error = Some(
                        if self.gafete_texto.trim().is_empty() {
                            "El gafete es requerido"
                        } else {
                            "Ingrese un número de gafete válido"
                        }
                        .into(),
                    );
                    AccionNuevoIngreso::Ninguna
                }
            },
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.gafete_texto.push(c);
                self.error = None;
                AccionNuevoIngreso::Ninguna
            }
            _ => AccionNuevoIngreso::Ninguna,
        }
    }
    fn confirmar(&mut self, key: KeyEvent) -> AccionNuevoIngreso {
        match key.code {
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.error = None;
                self.etapa = EtapaNuevoIngreso::Medio {
                    opcion: self.medio.map(indice_medio).unwrap_or(0),
                };
                AccionNuevoIngreso::Ninguna
            }
            KeyCode::Char('y' | 'Y') => AccionNuevoIngreso::Registrar {
                contratista_id: self.contratista_id.unwrap(),
                medio: self.medio.unwrap(),
                gafete: self.gafete,
            },
            _ => AccionNuevoIngreso::Ninguna,
        }
    }
    fn puede_continuar(&self) -> bool {
        self.preparacion.as_ref().is_some_and(|p| {
            !p.tiene_ingreso_activo && !matches!(p.resultado_acceso, ResultadoAcceso::Denegado(_))
        })
    }
    fn mover(&mut self, d: isize) {
        if self.contratistas.is_empty() {
            self.seleccion = None
        } else {
            let i = self.seleccion.unwrap_or(0);
            self.seleccion = Some(if d < 0 {
                i.saturating_sub(1)
            } else {
                (i + 1).min(self.contratistas.len() - 1)
            })
        }
    }
    fn contratista(&self) -> Option<&ContratistaResumen> {
        let id = self.contratista_id?;
        self.contratistas.iter().find(|c| c.id == id)
    }
    fn limpiar_seleccion(&mut self) {
        self.contratista_id = None;
        self.preparacion = None;
        self.medio = None;
        self.gafete = None;
        self.gafete_texto.clear();
        self.error = None
    }
    fn limpiar(&mut self) {
        self.limpiar_seleccion();
        self.busqueda.clear();
        self.contratistas.clear();
        self.seleccion = None;
        self.etapa = EtapaNuevoIngreso::Buscar
    }
    pub(super) fn inicio_visible(&self, cap: usize) -> usize {
        self.seleccion
            .unwrap_or(0)
            .saturating_sub(cap.saturating_sub(1))
    }
}
fn texto_filtro(s: &str) -> Option<String> {
    (!s.trim().is_empty()).then(|| s.to_owned())
}
fn indice_medio(m: MedioIngreso) -> usize {
    MEDIOS.iter().position(|x| *x == m).unwrap_or(0)
}
fn texto_medio(m: MedioIngreso) -> &'static str {
    match m {
        MedioIngreso::Caminando => "Caminando",
        MedioIngreso::Vehiculo => "Vehículo",
    }
}
