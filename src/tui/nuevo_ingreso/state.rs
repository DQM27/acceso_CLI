use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

use crate::{
    database::queries::contratistas::{ContratistaResumen, PaginaContratistas},
    domain::resultado_acceso::{MotivoDenegacion, ResultadoAcceso},
    models::medio_ingreso::MedioIngreso,
    services::registro_ingreso_service::PreparacionIngreso,
    tui::ui_kit::{Debounce, StandardCommand, TextInput, mover_seleccion, standard_command},
};

#[path = "render.rs"]
pub(super) mod render;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const MEDIOS: [MedioIngreso; 2] = [MedioIngreso::Caminando, MedioIngreso::Vehiculo];
const DURACION_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EtapaNuevoIngreso {
    Buscar,
    Formulario,
}

/// Mismo criterio Normal/Búsqueda que Activos/Contratistas/Historial: el
/// campo de texto sólo captura teclas mientras está en `Busqueda` (activado
/// con `/`); en `Normal` las flechas siguen navegando la lista, pero
/// escribir no hace nada. Antes esta pantalla no tenía esta distinción — el
/// campo de texto capturaba cualquier tecla todo el tiempo que `etapa` fuera
/// `Buscar`, así que tras registrar un ingreso (que vuelve a esa etapa) el
/// operador quedaba "adentro" del campo de búsqueda sin haberlo pedido, y
/// Esc con el campo ya vacío lo mandaba de una al menú principal.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoBuscarIngreso {
    Normal,
    Busqueda { texto: TextInput },
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
    /// Texto de búsqueda comprometido — se mantiene sincronizado con el
    /// `TextInput` de `ModoBuscarIngreso::Busqueda` mientras se escribe
    /// (mismo patrón que `ActivosState.filtro`).
    filtro: String,
    modo: ModoBuscarIngreso,
    seleccion: Option<usize>,
    contratista_id: Option<i64>,
    preparacion: Option<PreparacionIngreso>,
    campo: CampoIngreso,
    medio_opcion: usize,
    gafete_texto: String,
    gafete_cursor: usize,
    error: Option<String>,
    /// "✓ Ingreso registrado — X". Antes esta pantalla saltaba a Ingresos
    /// Activos tras cada registro (interrumpía el flujo de registrar varios
    /// contratistas seguidos); ahora se queda acá y este mensaje es la única
    /// confirmación de que sí se registró, igual que el resto de pantallas
    /// que escriben (`ActivosState.mensaje`, `SalidaRapidaState.mensaje`).
    mensaje: Option<String>,
    ayuda_expandida: bool,
    busqueda_debounce: Debounce,
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
            filtro: String::new(),
            modo: ModoBuscarIngreso::Normal,
            seleccion: None,
            contratista_id: None,
            preparacion: None,
            campo: CampoIngreso::Medio,
            medio_opcion: 0,
            gafete_texto: String::new(),
            gafete_cursor: 0,
            error: None,
            mensaje: None,
            ayuda_expandida: false,
            busqueda_debounce: Debounce::default(),
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
    pub fn cantidad(&self) -> usize {
        self.contratistas.len()
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
                self.gafete_cursor = 0;
                self.error = None;
                self.etapa = EtapaNuevoIngreso::Formulario
            }
            Err(e) => self.error = Some(e),
        }
    }
    /// `Ok`: vuelve a la etapa de búsqueda sin perder lo que el operador ya
    /// tenía escrito/filtrado — antes se limpiaba todo (filtro incluido),
    /// así que tras cada registro la lista quedaba en blanco y había que
    /// volver a escribir la búsqueda para seguir con el siguiente
    /// contratista. Pide recargar la misma búsqueda (acción devuelta) para
    /// que la lista refleje el estado fresco: el que se acaba de registrar
    /// pasa a mostrarse con `tiene_ingreso_activo`, sin tener que
    /// desaparecer de la vista para eso.
    pub fn completar_registro(&mut self, r: Result<i64, String>) -> AccionNuevoIngreso {
        match r {
            Ok(_) => {
                let nombre = self.preparacion.as_ref().map(|p| p.nombre.clone());
                self.limpiar_seleccion();
                self.modo = ModoBuscarIngreso::Normal;
                self.etapa = EtapaNuevoIngreso::Buscar;
                self.mensaje = nombre.map(|nombre| format!("✓ Ingreso registrado — {nombre}"));
                AccionNuevoIngreso::Buscar {
                    texto: texto_filtro(&self.filtro),
                }
            }
            Err(e) => {
                self.error = Some(e);
                AccionNuevoIngreso::Ninguna
            }
        }
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionNuevoIngreso {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionNuevoIngreso::Ninguna;
        }
        match self.etapa {
            EtapaNuevoIngreso::Buscar => match self.modo.clone() {
                ModoBuscarIngreso::Normal => self.normal(key),
                ModoBuscarIngreso::Busqueda { .. } => self.busqueda_input(key),
            },
            EtapaNuevoIngreso::Formulario => self.formulario(key),
        }
    }
    fn normal(&mut self, key: KeyEvent) -> AccionNuevoIngreso {
        if matches!(key.code, KeyCode::Enter | KeyCode::Char('/') | KeyCode::Esc) {
            self.mensaje = None;
        }
        match key.code {
            KeyCode::Up => {
                self.mover(-1);
                AccionNuevoIngreso::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionNuevoIngreso::Ninguna
            }
            KeyCode::Enter => self.seleccionar_resaltado(),
            KeyCode::Char('/') => {
                self.modo = ModoBuscarIngreso::Busqueda {
                    texto: TextInput::new(self.filtro.clone()),
                };
                AccionNuevoIngreso::Ninguna
            }
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                AccionNuevoIngreso::Buscar { texto: None }
            }
            KeyCode::Esc => AccionNuevoIngreso::Volver,
            _ => AccionNuevoIngreso::Ninguna,
        }
    }
    fn busqueda_input(&mut self, key: KeyEvent) -> AccionNuevoIngreso {
        match key.code {
            KeyCode::Esc => {
                self.filtro.clear();
                self.modo = ModoBuscarIngreso::Normal;
                AccionNuevoIngreso::Buscar { texto: None }
            }
            // A diferencia de Activos (donde Enter en Búsqueda sólo cierra
            // el campo), acá selecciona directo al resaltado — Nuevo
            // Ingreso necesita el camino más rápido posible (buscar y
            // Enter) para no frenar al operador cuando ya sabe a quién
            // busca.
            KeyCode::Enter => self.seleccionar_resaltado(),
            KeyCode::Up => {
                self.mover(-1);
                AccionNuevoIngreso::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionNuevoIngreso::Ninguna
            }
            _ => {
                let mut cambio = false;
                if let ModoBuscarIngreso::Busqueda { texto } = &mut self.modo {
                    cambio = texto.handle_key(key);
                    self.filtro = texto.value().to_owned();
                }
                if cambio {
                    self.mensaje = None;
                    self.busqueda_debounce.marcar(Instant::now());
                }
                AccionNuevoIngreso::Ninguna
            }
        }
    }
    fn seleccionar_resaltado(&self) -> AccionNuevoIngreso {
        self.contratistas
            .get(self.seleccion.unwrap_or(usize::MAX))
            .map_or(AccionNuevoIngreso::Ninguna, |c| {
                AccionNuevoIngreso::Preparar {
                    contratista_id: c.id,
                }
            })
    }
    /// Se llama en cada vuelta del bucle principal; dispara la búsqueda
    /// diferida sólo una vez que pasa `DURACION_DEBOUNCE` sin una tecla
    /// nueva — antes esta pantalla golpeaba la base con cada tecla en vez
    /// de agrupar, a diferencia de las otras 5 pantallas de búsqueda.
    pub fn tick(&mut self, ahora: Instant) -> AccionNuevoIngreso {
        if self.busqueda_debounce.listo(ahora, DURACION_DEBOUNCE) {
            AccionNuevoIngreso::Buscar {
                texto: texto_filtro(&self.filtro),
            }
        } else {
            AccionNuevoIngreso::Ninguna
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
            KeyCode::Left if self.campo == CampoIngreso::Gafete => {
                self.gafete_cursor = self.gafete_cursor.saturating_sub(1);
            }
            KeyCode::Right if self.campo == CampoIngreso::Gafete => {
                self.gafete_cursor =
                    (self.gafete_cursor + 1).min(self.gafete_texto.chars().count());
            }
            KeyCode::Home if self.campo == CampoIngreso::Gafete => {
                self.gafete_cursor = 0;
            }
            KeyCode::End if self.campo == CampoIngreso::Gafete => {
                self.gafete_cursor = self.gafete_texto.chars().count();
            }
            KeyCode::Delete if self.campo == CampoIngreso::Gafete => {
                if self.gafete_cursor < self.gafete_texto.chars().count() {
                    let idx = byte_index(&self.gafete_texto, self.gafete_cursor);
                    self.gafete_texto.remove(idx);
                    self.error = None;
                }
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
                if self.gafete_cursor > 0 {
                    let idx = byte_index(&self.gafete_texto, self.gafete_cursor - 1);
                    self.gafete_texto.remove(idx);
                    self.gafete_cursor -= 1;
                    self.error = None;
                }
            }
            KeyCode::Char(c)
                if self.campo == CampoIngreso::Gafete
                    && c.is_ascii_digit()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                let idx = byte_index(&self.gafete_texto, self.gafete_cursor);
                self.gafete_texto.insert(idx, c);
                self.gafete_cursor += 1;
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
        self.gafete_cursor = 0;
        self.error = None
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
fn byte_index(s: &str, indice_char: usize) -> usize {
    s.char_indices()
        .nth(indice_char)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
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
        MotivoDenegacion::PraindVencido => "Acceso denegado · PRAIND vencido".into(),
        MotivoDenegacion::PraindNoRegistrado => {
            "Acceso denegado · PRAIND sin fecha registrada".into()
        }
        MotivoDenegacion::EmpresaInactiva => "Acceso denegado · la empresa está inactiva".into(),
    }
}
fn texto_medio(m: MedioIngreso) -> &'static str {
    match m {
        MedioIngreso::Caminando => "Caminando",
        MedioIngreso::Vehiculo => "Vehículo",
    }
}
