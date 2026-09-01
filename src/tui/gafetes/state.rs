use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};

use crate::database::queries::Igualdad;
use crate::database::queries::contratistas::ContratistaResumen;
use crate::database::queries::gafetes::{FiltroGafetes, GafeteResumen};
use crate::database::queries::gafetes_incidentes::IncidenteGafete;
use crate::models::gafete::{EstadoGafete, MotivoResolucionGafete};
use crate::tui::ui_kit::{Debounce, StandardCommand, TextInput, standard_command};

const DURACION_DEBOUNCE: Duration = Duration::from_millis(120);

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModoFormularioAlta {
    Individual,
    Rango,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CampoAlta {
    Numero,
    Desde,
    Hasta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FormularioAlta {
    modo: ModoFormularioAlta,
    campo: CampoAlta,
    numero: TextInput,
    desde: TextInput,
    hasta: TextInput,
    error: Option<String>,
}

impl FormularioAlta {
    fn nuevo() -> Self {
        Self {
            modo: ModoFormularioAlta::Individual,
            campo: CampoAlta::Numero,
            numero: TextInput::default().with_max_chars(6),
            desde: TextInput::default().with_max_chars(6),
            hasta: TextInput::default().with_max_chars(6),
            error: None,
        }
    }

    /// Tab alterna Individual/Rango — cambia qué campos se muestran y
    /// resetea el foco al primero de ellos.
    fn alternar_modo(&mut self) {
        self.modo = match self.modo {
            ModoFormularioAlta::Individual => ModoFormularioAlta::Rango,
            ModoFormularioAlta::Rango => ModoFormularioAlta::Individual,
        };
        self.campo = match self.modo {
            ModoFormularioAlta::Individual => CampoAlta::Numero,
            ModoFormularioAlta::Rango => CampoAlta::Desde,
        };
        self.error = None;
    }

    fn siguiente_campo(&mut self) {
        if self.modo == ModoFormularioAlta::Rango {
            self.campo = match self.campo {
                CampoAlta::Desde => CampoAlta::Hasta,
                _ => CampoAlta::Desde,
            };
        }
    }
}

/// Búsqueda de contratista deudor al marcar un gafete perdido — mismo
/// patrón (debounce 120ms, `AppCore::buscar_contratistas`) que el buscador
/// de `nuevo_ingreso`, acotado a este sub-flujo en vez de reutilizar
/// `NuevoIngresoState` completo.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BuscarDeudor {
    gafete_id: i64,
    numero: i64,
    texto: TextInput,
    resultados: Vec<ContratistaResumen>,
    seleccion: Option<usize>,
    debounce: Debounce,
}

impl BuscarDeudor {
    fn nuevo(gafete_id: i64, numero: i64) -> Self {
        Self {
            gafete_id,
            numero,
            texto: TextInput::default(),
            resultados: Vec::new(),
            seleccion: None,
            debounce: Debounce::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoGafetes {
    Normal,
    Busqueda {
        texto: TextInput,
    },
    Alta(FormularioAlta),
    MarcarPerdidoBuscarDeudor(BuscarDeudor),
    ConfirmacionResolver {
        gafete_id: i64,
        numero: i64,
        motivo: MotivoResolucionGafete,
    },
    ConfirmacionBaja {
        gafete_id: i64,
        numero: i64,
    },
    /// Historial de incidentes de un gafete puntual — se abre con `incidentes`
    /// vacío y `AppCore::historial_gafete` lo llena en el mismo tick (la app
    /// es de instancia única y un solo hilo, no hay estado "cargando" real
    /// que mostrar), mismo dato que ya consume `HistorialGafeteModal` en la
    /// GUI (`docs/pendientes.md`).
    Historial {
        numero: i64,
        incidentes: Vec<IncidenteGafete>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionGafetes {
    Ninguna,
    Volver,
    Buscar {
        filtro: FiltroGafetes,
        seleccionar_id: Option<i64>,
    },
    CrearUno {
        numero: i64,
    },
    CrearRango {
        desde: i64,
        hasta: i64,
    },
    DarDeBaja {
        id: i64,
        numero: i64,
    },
    BuscarDeudor {
        texto: Option<String>,
    },
    MarcarPerdido {
        id: i64,
        numero: i64,
        contratista_id: i64,
    },
    Resolver {
        id: i64,
        numero: i64,
        motivo: MotivoResolucionGafete,
    },
    VerHistorial {
        id: i64,
        numero: i64,
    },
}

#[derive(Debug)]
pub struct GafetesState {
    gafetes: Vec<GafeteResumen>,
    seleccion: Option<usize>,
    modo: ModoGafetes,
    filtro: String,
    mensaje: Option<String>,
    ayuda_expandida: bool,
    busqueda_debounce: Debounce,
}

impl Default for GafetesState {
    fn default() -> Self {
        Self {
            gafetes: vec![],
            seleccion: None,
            modo: ModoGafetes::Normal,
            filtro: String::new(),
            mensaje: None,
            ayuda_expandida: false,
            busqueda_debounce: Debounce::default(),
        }
    }
}

impl GafetesState {
    pub fn cantidad(&self) -> usize {
        self.gafetes.len()
    }

    pub fn gafete_seleccionado(&self) -> Option<&GafeteResumen> {
        self.gafetes.get(self.seleccion?)
    }

    pub fn solicitar_carga(&self) -> AccionGafetes {
        self.accion_buscar(None)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AccionGafetes {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionGafetes::Ninguna;
        }
        match self.modo.clone() {
            ModoGafetes::Normal => self.handle_normal(key),
            ModoGafetes::Busqueda { .. } => self.handle_busqueda(key),
            ModoGafetes::Alta(formulario) => self.handle_alta(key, formulario),
            ModoGafetes::MarcarPerdidoBuscarDeudor(buscar) => {
                self.handle_buscar_deudor(key, buscar)
            }
            ModoGafetes::ConfirmacionResolver {
                gafete_id, motivo, ..
            } => self.handle_confirmacion_resolver(key, gafete_id, motivo),
            ModoGafetes::ConfirmacionBaja { gafete_id, numero } => {
                self.handle_confirmacion_baja(key, gafete_id, numero)
            }
            ModoGafetes::Historial { .. } => self.handle_historial(key),
        }
    }

    pub fn tick(&mut self, ahora: Instant) -> AccionGafetes {
        if self.busqueda_debounce.listo(ahora, DURACION_DEBOUNCE) {
            self.accion_buscar(None)
        } else {
            AccionGafetes::Ninguna
        }
    }

    /// Debounce del sub-buscador de deudor — aparte del principal porque
    /// sólo corre mientras `ModoGafetes::MarcarPerdidoBuscarDeudor` está
    /// activo.
    pub fn tick_deudor(&mut self, ahora: Instant) -> AccionGafetes {
        let ModoGafetes::MarcarPerdidoBuscarDeudor(buscar) = &mut self.modo else {
            return AccionGafetes::Ninguna;
        };
        if buscar.debounce.listo(ahora, DURACION_DEBOUNCE) {
            AccionGafetes::BuscarDeudor {
                texto: (!buscar.texto.value().trim().is_empty())
                    .then(|| buscar.texto.value().trim().to_owned()),
            }
        } else {
            AccionGafetes::Ninguna
        }
    }

    pub fn completar_busqueda(
        &mut self,
        resultado: Result<Vec<GafeteResumen>, String>,
        seleccionar_id: Option<i64>,
    ) {
        match resultado {
            Ok(gafetes) => {
                self.gafetes = gafetes;
                if !matches!(self.mensaje.as_deref(), Some(mensaje) if mensaje.starts_with('✓')) {
                    self.mensaje = None;
                }
                self.seleccion = seleccionar_id
                    .and_then(|id| self.gafetes.iter().position(|g| g.id == id))
                    .or_else(|| (!self.gafetes.is_empty()).then_some(0));
            }
            Err(error) => {
                self.gafetes.clear();
                self.seleccion = None;
                self.mensaje = Some(error);
            }
        }
    }

    pub fn completar_busqueda_deudor(
        &mut self,
        resultado: Result<Vec<ContratistaResumen>, String>,
    ) {
        let ModoGafetes::MarcarPerdidoBuscarDeudor(buscar) = &mut self.modo else {
            return;
        };
        match resultado {
            Ok(resultados) => {
                buscar.seleccion = (!resultados.is_empty()).then_some(0);
                buscar.resultados = resultados;
            }
            Err(error) => {
                buscar.resultados.clear();
                buscar.seleccion = None;
                self.mensaje = Some(error);
            }
        }
    }

    pub fn completar_alta(&mut self, resultado: Result<i64, String>, numero: i64) -> AccionGafetes {
        match resultado {
            Ok(id) => {
                self.modo = ModoGafetes::Normal;
                self.mensaje = Some(format!("✓ Gafete {numero:02} dado de alta"));
                self.accion_buscar(Some(id))
            }
            Err(error) => {
                self.error_alta(error);
                AccionGafetes::Ninguna
            }
        }
    }

    pub fn completar_alta_rango(
        &mut self,
        resultado: Result<Vec<i64>, String>,
        desde: i64,
        hasta: i64,
    ) -> AccionGafetes {
        match resultado {
            Ok(ids) => {
                self.modo = ModoGafetes::Normal;
                self.mensaje = Some(format!(
                    "✓ Gafetes {desde:02}-{hasta:02} dados de alta ({} en total)",
                    ids.len()
                ));
                self.accion_buscar(ids.first().copied())
            }
            Err(error) => {
                self.error_alta(error);
                AccionGafetes::Ninguna
            }
        }
    }

    pub fn completar_baja(
        &mut self,
        resultado: Result<(), String>,
        id: i64,
        numero: i64,
    ) -> AccionGafetes {
        self.modo = ModoGafetes::Normal;
        match resultado {
            Ok(()) => {
                self.mensaje = Some(format!("✓ Gafete {numero:02} dado de baja"));
                AccionGafetes::Buscar {
                    filtro: FiltroGafetes::default(),
                    seleccionar_id: Some(id),
                }
            }
            Err(error) => {
                self.mensaje = Some(error);
                AccionGafetes::Ninguna
            }
        }
    }

    pub fn completar_marcar_perdido(
        &mut self,
        resultado: Result<(), String>,
        id: i64,
        numero: i64,
    ) -> AccionGafetes {
        match resultado {
            Ok(()) => {
                self.modo = ModoGafetes::Normal;
                self.mensaje = Some(format!("✓ Gafete {numero:02} marcado como perdido"));
                AccionGafetes::Buscar {
                    filtro: FiltroGafetes::default(),
                    seleccionar_id: Some(id),
                }
            }
            Err(error) => {
                // Se vuelve a Normal en vez de dejar el sub-buscador abierto
                // con un error de backend — mismo criterio que
                // `completar_baja`: el mensaje de estado ya lo explica.
                self.modo = ModoGafetes::Normal;
                self.mensaje = Some(error);
                AccionGafetes::Ninguna
            }
        }
    }

    pub fn completar_resolver(
        &mut self,
        resultado: Result<(), String>,
        id: i64,
        numero: i64,
    ) -> AccionGafetes {
        self.modo = ModoGafetes::Normal;
        match resultado {
            Ok(()) => {
                self.mensaje = Some(format!("✓ Deuda del gafete {numero:02} resuelta"));
                AccionGafetes::Buscar {
                    filtro: FiltroGafetes::default(),
                    seleccionar_id: Some(id),
                }
            }
            Err(error) => {
                self.mensaje = Some(error);
                AccionGafetes::Ninguna
            }
        }
    }

    /// Completa la carga disparada por `AccionGafetes::VerHistorial` — un
    /// error vuelve a `Normal` con el mensaje de estado, igual que
    /// `completar_marcar_perdido`/`completar_resolver`.
    pub fn completar_historial(
        &mut self,
        resultado: Result<Vec<IncidenteGafete>, String>,
        numero: i64,
    ) {
        match resultado {
            Ok(incidentes) => self.modo = ModoGafetes::Historial { numero, incidentes },
            Err(error) => {
                self.modo = ModoGafetes::Normal;
                self.mensaje = Some(error);
            }
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> AccionGafetes {
        if matches!(
            key.code,
            KeyCode::Char('n' | 'N' | 'b' | 'B' | 'p' | 'P' | 'r' | 'R' | 'h' | 'H' | '/')
                | KeyCode::Esc
        ) {
            self.mensaje = None;
        }
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Char('n' | 'N') => self.modo = ModoGafetes::Alta(FormularioAlta::nuevo()),
            KeyCode::Char('h' | 'H') => {
                if let Some(g) = self.gafete_seleccionado() {
                    let (id, numero) = (g.id, g.numero);
                    self.modo = ModoGafetes::Historial {
                        numero,
                        incidentes: Vec::new(),
                    };
                    return AccionGafetes::VerHistorial { id, numero };
                }
            }
            KeyCode::Char('b' | 'B') => {
                if let Some(g) = self.gafete_seleccionado()
                    && g.estado == EstadoGafete::Disponible
                {
                    self.modo = ModoGafetes::ConfirmacionBaja {
                        gafete_id: g.id,
                        numero: g.numero,
                    };
                }
            }
            KeyCode::Char('p' | 'P') => {
                if let Some(g) = self.gafete_seleccionado()
                    && g.estado == EstadoGafete::Disponible
                {
                    self.modo =
                        ModoGafetes::MarcarPerdidoBuscarDeudor(BuscarDeudor::nuevo(g.id, g.numero));
                }
            }
            KeyCode::Char('r' | 'R') => {
                if let Some(g) = self.gafete_seleccionado()
                    && g.estado == EstadoGafete::Perdido
                {
                    // Sin tercer nivel de menú: 1=Pagado/2=Aparecido arman
                    // la confirmación directo (ver `handle_normal` no
                    // captura esto — se resuelve en el propio `match` de
                    // arriba con un guard de estado si hiciera falta más
                    // adelante; por ahora R abre confirmación con Pagado
                    // por defecto y 1/2 alternan el motivo dentro de ella).
                    self.modo = ModoGafetes::ConfirmacionResolver {
                        gafete_id: g.id,
                        numero: g.numero,
                        motivo: MotivoResolucionGafete::Pagado,
                    };
                }
            }
            KeyCode::Char('/') => {
                self.modo = ModoGafetes::Busqueda {
                    texto: TextInput::new(self.filtro.clone()),
                }
            }
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                self.seleccion = None;
                return self.accion_buscar(None);
            }
            KeyCode::Esc => return AccionGafetes::Volver,
            _ => {}
        }
        AccionGafetes::Ninguna
    }

    fn handle_busqueda(&mut self, key: KeyEvent) -> AccionGafetes {
        match key.code {
            KeyCode::Esc => {
                self.filtro.clear();
                self.modo = ModoGafetes::Normal;
                self.accion_buscar(None)
            }
            KeyCode::Enter => {
                self.modo = ModoGafetes::Normal;
                AccionGafetes::Ninguna
            }
            KeyCode::Up => {
                self.mover(-1);
                AccionGafetes::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionGafetes::Ninguna
            }
            _ => {
                let mut cambio = false;
                if let ModoGafetes::Busqueda { texto } = &mut self.modo {
                    cambio = texto.handle_key(key);
                    self.filtro = texto.value().to_owned();
                }
                if cambio {
                    self.busqueda_debounce.marcar(Instant::now());
                }
                AccionGafetes::Ninguna
            }
        }
    }

    fn handle_alta(&mut self, key: KeyEvent, mut formulario: FormularioAlta) -> AccionGafetes {
        match key.code {
            KeyCode::Esc => {
                self.modo = ModoGafetes::Normal;
                return AccionGafetes::Ninguna;
            }
            KeyCode::Tab => {
                formulario.alternar_modo();
            }
            KeyCode::Down | KeyCode::Up if formulario.modo == ModoFormularioAlta::Rango => {
                formulario.siguiente_campo();
            }
            KeyCode::Enter => {
                return match construir_alta(&formulario) {
                    Ok(AltaValida::Individual(numero)) => AccionGafetes::CrearUno { numero },
                    Ok(AltaValida::Rango(desde, hasta)) => {
                        AccionGafetes::CrearRango { desde, hasta }
                    }
                    Err(error) => {
                        formulario.error = Some(error);
                        self.modo = ModoGafetes::Alta(formulario);
                        AccionGafetes::Ninguna
                    }
                };
            }
            _ => {
                let campo = match formulario.campo {
                    CampoAlta::Numero => &mut formulario.numero,
                    CampoAlta::Desde => &mut formulario.desde,
                    CampoAlta::Hasta => &mut formulario.hasta,
                };
                if campo.handle_key(key) {
                    formulario.error = None;
                }
            }
        }
        self.modo = ModoGafetes::Alta(formulario);
        AccionGafetes::Ninguna
    }

    fn handle_buscar_deudor(&mut self, key: KeyEvent, mut buscar: BuscarDeudor) -> AccionGafetes {
        match key.code {
            KeyCode::Esc => {
                self.modo = ModoGafetes::Normal;
                return AccionGafetes::Ninguna;
            }
            KeyCode::Up => {
                if let Some(seleccion) = buscar.seleccion {
                    buscar.seleccion = Some(seleccion.saturating_sub(1));
                }
            }
            KeyCode::Down => {
                if let Some(seleccion) = buscar.seleccion {
                    buscar.seleccion =
                        Some((seleccion + 1).min(buscar.resultados.len().saturating_sub(1)));
                }
            }
            KeyCode::Enter => {
                if let Some(contratista) = buscar
                    .seleccion
                    .and_then(|indice| buscar.resultados.get(indice))
                {
                    return AccionGafetes::MarcarPerdido {
                        id: buscar.gafete_id,
                        numero: buscar.numero,
                        contratista_id: contratista.id,
                    };
                }
            }
            _ => {
                if buscar.texto.handle_key(key) {
                    buscar.debounce.marcar(Instant::now());
                }
            }
        }
        self.modo = ModoGafetes::MarcarPerdidoBuscarDeudor(buscar);
        AccionGafetes::Ninguna
    }

    fn handle_confirmacion_resolver(
        &mut self,
        key: KeyEvent,
        gafete_id: i64,
        motivo_actual: MotivoResolucionGafete,
    ) -> AccionGafetes {
        let Some(g) = self.gafetes.iter().find(|g| g.id == gafete_id) else {
            self.modo = ModoGafetes::Normal;
            return AccionGafetes::Ninguna;
        };
        let numero = g.numero;
        match key.code {
            KeyCode::Char('1') => {
                self.modo = ModoGafetes::ConfirmacionResolver {
                    gafete_id,
                    numero,
                    motivo: MotivoResolucionGafete::Pagado,
                };
            }
            KeyCode::Char('2') => {
                self.modo = ModoGafetes::ConfirmacionResolver {
                    gafete_id,
                    numero,
                    motivo: MotivoResolucionGafete::Aparecido,
                };
            }
            KeyCode::Enter => {
                return AccionGafetes::Resolver {
                    id: gafete_id,
                    numero,
                    motivo: motivo_actual,
                };
            }
            KeyCode::Esc => self.modo = ModoGafetes::Normal,
            _ => {}
        }
        AccionGafetes::Ninguna
    }

    fn handle_confirmacion_baja(
        &mut self,
        key: KeyEvent,
        gafete_id: i64,
        numero: i64,
    ) -> AccionGafetes {
        match key.code {
            KeyCode::Enter => AccionGafetes::DarDeBaja {
                id: gafete_id,
                numero,
            },
            KeyCode::Esc => {
                self.modo = ModoGafetes::Normal;
                AccionGafetes::Ninguna
            }
            _ => AccionGafetes::Ninguna,
        }
    }

    fn handle_historial(&mut self, key: KeyEvent) -> AccionGafetes {
        if key.code == KeyCode::Esc {
            self.modo = ModoGafetes::Normal;
        }
        AccionGafetes::Ninguna
    }

    fn error_alta(&mut self, error: String) {
        if let ModoGafetes::Alta(f) = &mut self.modo {
            f.error = Some(error);
        }
    }

    /// Traduce el filtro de texto libre (`estado:disponible|perdido|de_baja`,
    /// con negación, o un número exacto) al `FiltroGafetes` real. El
    /// catálogo es chico (decenas de filas, `docs/plan-gafetes.md`) — a
    /// diferencia de Contratistas/Historial no justifica el motor
    /// `clave:valor` completo con múltiples claves combinables; acá alcanza
    /// con reconocer las dos formas que puede tomar la consulta.
    fn accion_buscar(&self, seleccionar_id: Option<i64>) -> AccionGafetes {
        AccionGafetes::Buscar {
            filtro: interpretar_filtro(&self.filtro),
            seleccionar_id,
        }
    }

    fn mover(&mut self, delta: isize) {
        let n = self.gafetes.len();
        self.seleccion = if n == 0 {
            None
        } else {
            let a = self.seleccion.unwrap_or(0);
            Some(if delta < 0 {
                a.saturating_sub(1)
            } else {
                (a + 1).min(n - 1)
            })
        };
    }

    fn inicio_visible(&self, capacidad: usize) -> usize {
        self.seleccion
            .unwrap_or(0)
            .saturating_sub(capacidad.saturating_sub(1))
    }
}

fn interpretar_filtro(texto: &str) -> FiltroGafetes {
    let texto = texto.trim();
    if texto.is_empty() {
        return FiltroGafetes::default();
    }
    if let Ok(numero) = texto.parse::<i64>() {
        return FiltroGafetes {
            numero: Some(numero),
            estado: None,
        };
    }
    let (negado, resto) = texto
        .strip_prefix('-')
        .map_or((false, texto), |resto| (true, resto));
    if let Some(valor) = resto.strip_prefix("estado:") {
        let estado = EstadoGafete::from_str_filtro(valor);
        return FiltroGafetes {
            numero: None,
            estado: estado.map(|e| {
                if negado {
                    Igualdad::Excluye(e)
                } else {
                    Igualdad::Incluye(e)
                }
            }),
        };
    }
    // Consulta no reconocida: ningún filtro real de `FiltroGafetes` la
    // aplica — mostrar el catálogo sin filtrar en vez de fallar es
    // consistente con "el catálogo es chico, se ve entero" cuando la
    // búsqueda no aporta nada concreto.
    FiltroGafetes::default()
}

enum AltaValida {
    Individual(i64),
    Rango(i64, i64),
}

/// Validación de UI antes de despachar la acción — la validación real y
/// atómica queda en `GafeteService`, esto sólo evita mandar un típo
/// evidente (texto vacío, no numérico) o un rango descomunal por error de
/// tecleo (tope defensivo de 200, `docs/plan-gafetes.md`).
fn construir_alta(f: &FormularioAlta) -> Result<AltaValida, String> {
    match f.modo {
        ModoFormularioAlta::Individual => {
            let numero: i64 = f
                .numero
                .value()
                .trim()
                .parse()
                .map_err(|_| "Ingrese un número de gafete válido".to_string())?;
            if numero <= 0 {
                return Err("El número debe ser mayor a cero".into());
            }
            Ok(AltaValida::Individual(numero))
        }
        ModoFormularioAlta::Rango => {
            let desde: i64 = f
                .desde
                .value()
                .trim()
                .parse()
                .map_err(|_| "Ingrese un \"desde\" válido".to_string())?;
            let hasta: i64 = f
                .hasta
                .value()
                .trim()
                .parse()
                .map_err(|_| "Ingrese un \"hasta\" válido".to_string())?;
            if desde <= 0 || hasta < desde {
                return Err("El rango no es válido".into());
            }
            if hasta - desde > 200 {
                return Err("El rango es demasiado grande (máximo 200 a la vez)".into());
            }
            Ok(AltaValida::Rango(desde, hasta))
        }
    }
}
