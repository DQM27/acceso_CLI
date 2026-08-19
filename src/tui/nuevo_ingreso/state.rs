use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    database::queries::contratistas::{ContratistaResumen, PaginaContratistas},
    domain::resultado_acceso::{MotivoDenegacion, ResultadoAcceso},
    models::medio_ingreso::MedioIngreso,
    services::registro_ingreso_service::PreparacionIngreso,
    tui::ui_kit::{StandardCommand, mover_seleccion, standard_command},
};

#[path = "render.rs"]
pub(super) mod render;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const MEDIOS: [MedioIngreso; 2] = [MedioIngreso::Caminando, MedioIngreso::Vehiculo];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EtapaNuevoIngreso {
    Buscar,
    Formulario,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CampoIngreso {
    Medio,
    Gafete,
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
    /// Conteo real de coincidencias, sin recortar por el tope de la consulta
    /// — permite avisar "primeros N de M, afine la búsqueda" en vez de dejar
    /// resultados fuera de forma silenciosa.
    total: usize,
    busqueda: String,
    seleccion: Option<usize>,
    contratista_id: Option<i64>,
    preparacion: Option<PreparacionIngreso>,
    campo: CampoIngreso,
    medio_opcion: usize,
    gafete_texto: String,
    error: Option<String>,
    ayuda_expandida: bool,
}

impl Default for NuevoIngresoState {
    fn default() -> Self {
        Self::new()
    }
}

impl NuevoIngresoState {
    pub fn new() -> Self {
        Self {
            etapa: EtapaNuevoIngreso::Buscar,
            contratistas: vec![],
            total: 0,
            busqueda: String::new(),
            seleccion: None,
            contratista_id: None,
            preparacion: None,
            campo: CampoIngreso::Medio,
            medio_opcion: 0,
            gafete_texto: String::new(),
            error: None,
            ayuda_expandida: false,
        }
    }
    pub fn solicitud_carga(&self) -> AccionNuevoIngreso {
        AccionNuevoIngreso::Buscar { texto: None }
    }
    pub fn completar_busqueda(&mut self, r: Result<PaginaContratistas, String>) {
        match r {
            Ok(pagina) => {
                self.contratistas = pagina.items;
                self.total = pagina.total;
                self.seleccion = (!self.contratistas.is_empty()).then_some(0);
                self.error = None;
            }
            Err(e) => {
                self.contratistas.clear();
                self.total = 0;
                self.seleccion = None;
                self.error = Some(e)
            }
        }
    }
    /// `Some(total)` sólo cuando quedaron resultados fuera de la lista
    /// mostrada — la pantalla lo usa para avisar en vez de dejarlo callado.
    pub fn resultados_ocultos(&self) -> Option<usize> {
        (self.total > self.contratistas.len()).then_some(self.total)
    }
    pub fn completar_preparacion(&mut self, r: Result<PreparacionIngreso, String>) {
        match r {
            Ok(p) if !puede_continuar(&p) => {
                self.error = Some(mensaje_bloqueo(&p));
            }
            Ok(p) => {
                self.contratista_id = Some(p.contratista_id);
                self.preparacion = Some(p);
                self.campo = CampoIngreso::Medio;
                self.medio_opcion = 0;
                self.gafete_texto.clear();
                self.error = None;
                self.etapa = EtapaNuevoIngreso::Formulario
            }
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
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionNuevoIngreso::Ninguna;
        }
        match self.etapa {
            EtapaNuevoIngreso::Buscar => self.buscar(key),
            EtapaNuevoIngreso::Formulario => self.formulario(key),
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
    fn formulario(&mut self, key: KeyEvent) -> AccionNuevoIngreso {
        let requiere_gafete = self.preparacion.as_ref().is_some_and(|p| p.requiere_gafete);
        match key.code {
            KeyCode::Esc => {
                self.limpiar_seleccion();
                self.etapa = EtapaNuevoIngreso::Buscar;
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::BackTab | KeyCode::Up if requiere_gafete => {
                self.campo = match self.campo {
                    CampoIngreso::Medio => CampoIngreso::Gafete,
                    CampoIngreso::Gafete => CampoIngreso::Medio,
                };
            }
            KeyCode::Left | KeyCode::Right if self.campo == CampoIngreso::Medio => {
                self.medio_opcion = 1 - self.medio_opcion;
            }
            KeyCode::Enter => {
                let Some(contratista_id) = self.contratista_id else {
                    self.limpiar_seleccion();
                    self.etapa = EtapaNuevoIngreso::Buscar;
                    self.error = Some("Vuelva a seleccionar el contratista".into());
                    return AccionNuevoIngreso::Ninguna;
                };
                let gafete = if requiere_gafete {
                    match self.gafete_texto.trim().parse::<i64>() {
                        Ok(numero) => Some(numero),
                        Err(_) => {
                            self.error = Some(
                                if self.gafete_texto.trim().is_empty() {
                                    "El gafete es requerido"
                                } else {
                                    "Ingrese un número de gafete válido"
                                }
                                .into(),
                            );
                            return AccionNuevoIngreso::Ninguna;
                        }
                    }
                } else {
                    None
                };
                return AccionNuevoIngreso::Registrar {
                    contratista_id,
                    medio: MEDIOS[self.medio_opcion],
                    gafete,
                };
            }
            KeyCode::Backspace if self.campo == CampoIngreso::Gafete => {
                self.gafete_texto.pop();
                self.error = None;
            }
            KeyCode::Char(c)
                if self.campo == CampoIngreso::Gafete
                    && c.is_ascii_digit()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.gafete_texto.push(c);
                self.error = None;
            }
            _ => {}
        }
        AccionNuevoIngreso::Ninguna
    }
    fn mover(&mut self, d: isize) {
        self.seleccion = mover_seleccion(self.seleccion, d, self.contratistas.len());
    }
    fn contratista(&self) -> Option<&ContratistaResumen> {
        let id = self.contratista_id?;
        self.contratistas.iter().find(|c| c.id == id)
    }
    fn preparacion(&self) -> Option<&PreparacionIngreso> {
        self.preparacion.as_ref()
    }
    fn campo_es_gafete(&self) -> bool {
        self.campo == CampoIngreso::Gafete
    }
    fn medio_actual(&self) -> MedioIngreso {
        MEDIOS[self.medio_opcion]
    }
    fn limpiar_seleccion(&mut self) {
        self.contratista_id = None;
        self.preparacion = None;
        self.campo = CampoIngreso::Medio;
        self.medio_opcion = 0;
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
fn puede_continuar(p: &PreparacionIngreso) -> bool {
    !p.tiene_ingreso_activo && !matches!(p.resultado_acceso, ResultadoAcceso::Denegado(_))
}
fn mensaje_bloqueo(p: &PreparacionIngreso) -> String {
    if p.tiene_ingreso_activo {
        return "El contratista ya tiene un ingreso activo.".into();
    }
    match &p.resultado_acceso {
        ResultadoAcceso::Denegado(motivo) => mensaje_motivo_denegacion(motivo),
        ResultadoAcceso::Permitido | ResultadoAcceso::PermitidoConAdvertencia => {
            "No se puede continuar con este contratista.".into()
        }
    }
}

/// Match exhaustivo sobre `MotivoDenegacion` a propósito, sin `_ =>` — si se
/// agrega una variante nueva, el compilador obliga a decidir aquí su mensaje
/// en vez de caer en un texto genérico sin que nadie se entere.
fn mensaje_motivo_denegacion(motivo: &MotivoDenegacion) -> String {
    match motivo {
        MotivoDenegacion::SinAcceso => "Acceso denegado · no tiene acceso autorizado".into(),
        MotivoDenegacion::PraindVencido => "Acceso denegado · PRAIND vencido o requerido".into(),
        MotivoDenegacion::EmpresaInactiva => "Acceso denegado · la empresa está inactiva".into(),
    }
}
fn texto_medio(m: MedioIngreso) -> &'static str {
    match m {
        MedioIngreso::Caminando => "Caminando",
        MedioIngreso::Vehiculo => "Vehículo",
    }
}
