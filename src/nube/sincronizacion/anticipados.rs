//! Descargas anticipadas de la recepción: los `GET` de una sincronización
//! salen en paralelo antes de que las etapas los pidan, y las etapas siguen
//! corriendo y guardando EN EL MISMO ORDEN de siempre.
//!
//! Medido con telemetría en el teléfono: la sincronización completa hacía
//! ~12 consultas una detrás de otra (~150 ms cada una en 4G) y tardaba
//! 1,8-3,3 s. Cada etapa arma su consulta a partir de marcas de agua de la
//! base local, así que no se puede saber de antemano qué va a pedir sin
//! correrla. En cambio, la sincronización ANTERIOR con el mismo alcance
//! pidió casi siempre las mismas URL (las marcas sólo cambian cuando llegó
//! algo nuevo): se anotan, y la próxima vez se piden todas juntas al
//! arrancar la recepción.
//!
//! Por qué no cambia el resultado:
//! - Sólo se aprovecha una respuesta si la etapa pide EXACTAMENTE la misma
//!   URL (y la misma página); si la marca cambió, la URL es otra y la etapa
//!   la pide por la red como siempre.
//! - Cada respuesta se usa una sola vez y sólo en esta recepción.
//! - Sólo se guardan respuestas 2xx que se pueden leer; un error de red,
//!   un 401 o un cuerpo ilegible se descartan y la etapa repite el pedido
//!   por su cuenta, con el mismo manejo de errores de antes.
//! - La recepción arranca DESPUÉS de vaciar la bandeja de salida (ver
//!   `orquestacion::sincronizar`), así que lo anticipado ya ve lo que este
//!   equipo acaba de mandar. La recepción no escribe nada en la nube.
//! - Es la misma foto de la nube, tomada unos cientos de milisegundos antes
//!   que en el recorrido en serie: lo que cambie en el medio lo trae el
//!   aviso en vivo o la próxima sincronización, igual que antes.
//!
//! El estado de la recepción en curso vive en el hilo que la corre
//! (`thread_local!`): las etapas no tienen que pasárselo de mano en mano y
//! dos sincronizaciones en hilos distintos no se mezclan.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};

use super::ContextoSincronizacion;
use super::paginado::TAMANO_PAGINA_REMOTA;

/// Tope de consultas simultáneas: suficiente para cubrir las ~12 de una
/// sincronización completa en dos o tres tandas sin abrir una conexión TLS
/// por cada una en una red móvil.
const MAXIMO_EN_PARALELO: usize = 6;

/// Un `GET` de la recepción: la URL tal cual y si es la primera página de
/// una lista paginada (lleva el header `Range` de la primera página).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct Pedido {
    url: String,
    primera_pagina: bool,
}

impl Pedido {
    pub(super) fn simple(url: &str) -> Self {
        Self {
            url: url.to_owned(),
            primera_pagina: false,
        }
    }

    pub(super) fn primera_pagina(url: &str) -> Self {
        Self {
            url: url.to_owned(),
            primera_pagina: true,
        }
    }
}

#[derive(Default)]
struct RecepcionEnCurso {
    /// Lo que las etapas pidieron, en orden: se guarda para la próxima.
    pedidos: Vec<Pedido>,
    /// Cuerpos ya descargados, cada uno disponible una sola vez.
    listos: HashMap<Pedido, String>,
}

thread_local! {
    static EN_CURSO: RefCell<Option<RecepcionEnCurso>> = const { RefCell::new(None) };
}

/// Pedidos de la última recepción exitosa por huella (servidor, sitio,
/// dispositivo, alcance y perfil).
static ULTIMOS: LazyLock<Mutex<HashMap<String, Vec<Pedido>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Deja el hilo sin recepción en curso aunque `f` entre en pánico.
struct Limpieza;

impl Drop for Limpieza {
    fn drop(&mut self) {
        EN_CURSO.with(|en_curso| en_curso.borrow_mut().take());
    }
}

/// Corre `f` (la recepción) con los pedidos de la vez anterior ya
/// descargados, y anota los de esta para la próxima si terminó bien.
pub(in crate::nube) fn con_descargas_anticipadas<R, E>(
    contexto: &ContextoSincronizacion<'_>,
    huella: &str,
    f: impl FnOnce() -> Result<R, E>,
) -> Result<R, E> {
    // Una recepción dentro de otra (no pasa hoy) usa la de afuera.
    if EN_CURSO.with(|en_curso| en_curso.borrow().is_some()) {
        return f();
    }
    let anteriores = ULTIMOS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(huella)
        .cloned()
        .unwrap_or_default();
    let listos = descargar_en_paralelo(contexto, &anteriores);
    EN_CURSO.with(|en_curso| {
        *en_curso.borrow_mut() = Some(RecepcionEnCurso {
            pedidos: Vec::new(),
            listos,
        });
    });
    let limpieza = Limpieza;
    let resultado = f();
    let pedidos = EN_CURSO
        .with(|en_curso| en_curso.borrow_mut().take())
        .map(|recepcion| recepcion.pedidos)
        .unwrap_or_default();
    drop(limpieza);
    if resultado.is_ok() {
        ULTIMOS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(huella.to_owned(), pedidos);
    }
    resultado
}

/// Anota `pedido` en la recepción en curso y devuelve su cuerpo si ya se
/// descargó por anticipado. Fuera de una recepción no hace nada.
pub(super) fn tomar(pedido: Pedido) -> Option<String> {
    EN_CURSO.with(|en_curso| {
        let mut en_curso = en_curso.borrow_mut();
        let recepcion = en_curso.as_mut()?;
        let cuerpo = recepcion.listos.remove(&pedido);
        recepcion.pedidos.push(pedido);
        cuerpo
    })
}

fn descargar_en_paralelo(
    contexto: &ContextoSincronizacion<'_>,
    pedidos: &[Pedido],
) -> HashMap<Pedido, String> {
    if pedidos.is_empty() {
        return HashMap::new();
    }
    let cliente = crate::nube::cliente::cliente_http();
    let siguiente = AtomicUsize::new(0);
    let listos = Mutex::new(HashMap::new());
    std::thread::scope(|hilos| {
        for _ in 0..MAXIMO_EN_PARALELO.min(pedidos.len()) {
            hilos.spawn(|| {
                while let Some(pedido) = pedidos.get(siguiente.fetch_add(1, Ordering::Relaxed)) {
                    if let Some(cuerpo) = descargar(&cliente, contexto, pedido) {
                        listos
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .insert(pedido.clone(), cuerpo);
                    }
                }
            });
        }
    });
    listos
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// El mismo pedido que harían `obtener_json` u `obtener_json_paginado_con`
/// (headers incluidos); `None` ante cualquier cosa que no sea un 2xx
/// legible.
fn descargar(
    cliente: &reqwest::blocking::Client,
    contexto: &ContextoSincronizacion<'_>,
    pedido: &Pedido,
) -> Option<String> {
    let mut solicitud = cliente
        .get(&pedido.url)
        .header("apikey", contexto.apikey)
        .header("Authorization", format!("Bearer {}", contexto.token));
    if pedido.primera_pagina {
        solicitud = solicitud
            .header("Range-Unit", "items")
            .header("Range", format!("0-{}", TAMANO_PAGINA_REMOTA - 1));
    }
    let respuesta = solicitud.send().ok()?;
    if !respuesta.status().is_success() {
        return None;
    }
    respuesta.text().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuera_de_una_recepcion_no_anota_ni_devuelve_nada() {
        assert_eq!(tomar(Pedido::simple("http://x/a")), None);
    }

    #[test]
    fn cada_respuesta_se_usa_una_sola_vez_y_los_pedidos_quedan_anotados() {
        EN_CURSO.with(|en_curso| {
            *en_curso.borrow_mut() = Some(RecepcionEnCurso {
                pedidos: Vec::new(),
                listos: HashMap::from([(Pedido::simple("http://x/a"), "[]".to_owned())]),
            });
        });
        let _limpieza = Limpieza;
        assert_eq!(tomar(Pedido::simple("http://x/a")).as_deref(), Some("[]"));
        assert_eq!(tomar(Pedido::simple("http://x/a")), None, "ya se usó");
        assert_eq!(
            tomar(Pedido::primera_pagina("http://x/a")),
            None,
            "la primera página es otro pedido"
        );
        let anotados = EN_CURSO.with(|en_curso| {
            en_curso
                .borrow()
                .as_ref()
                .map(|recepcion| recepcion.pedidos.len())
        });
        assert_eq!(anotados, Some(3));
    }
}
