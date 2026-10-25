//! Manual de uso de la app (`docs/manual/movil/`), navegable sin internet y
//! sin IA: la "Ayuda" del menú de la cuenta. El manual viaja dentro del `.so`
//! (lo mete `build.rs` al compilar) y el núcleo lo lee, arma el índice y
//! busca; Kotlin sólo dibuja los bloques que recibe, ya aplanados.
//!
//! Mismas reglas que el escritorio y el panel, que usan
//! `supabase/functions/_shared/manual/` (ver `docs/manual/README.md`).

mod buscar;
mod markdown;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::LazyLock;

use buscar::Trozo;
use markdown::{Bloque, EnLinea, leer_en_linea, leer_markdown, texto_plano};

// `ARCHIVOS`: (ruta dentro de `docs/manual/`, contenido), generado por `build.rs`.
include!(concat!(env!("OUT_DIR"), "/manual_movil.rs"));

/// Carpeta de esta app dentro de `docs/manual/`.
const CARPETA: &str = "movil/";

/// Un proceso del manual, ya leído.
#[derive(Debug, Clone)]
struct Proceso {
    ruta: String,
    titulo: String,
    bloques: Vec<Bloque>,
    trozos: Vec<Trozo>,
}

#[derive(Debug)]
struct Manual {
    procesos: Vec<Proceso>,
    por_ruta: HashMap<String, usize>,
    indice: Vec<GrupoManual>,
}

impl Manual {
    fn proceso(&self, ruta: &str) -> Option<&Proceso> {
        self.por_ruta.get(ruta).map(|&i| &self.procesos[i])
    }
}

static MANUAL: LazyLock<Manual> = LazyLock::new(|| armar(ARCHIVOS));

fn armar(archivos: &[(&str, &str)]) -> Manual {
    let mut procesos: Vec<Proceso> = archivos
        .iter()
        .filter(|(ruta, _)| ruta.starts_with(CARPETA))
        .map(|(ruta, contenido)| {
            let bloques = leer_markdown(contenido);
            let titulo = match bloques.first() {
                Some(Bloque::Titulo { texto, .. }) => texto_plano(&leer_en_linea(texto)),
                _ => (*ruta).to_string(),
            };
            Proceso {
                ruta: (*ruta).to_string(),
                titulo,
                trozos: buscar::trozos(&bloques),
                bloques,
            }
        })
        .collect();
    procesos.sort_by(|a, b| a.ruta.cmp(&b.ruta));
    let por_ruta: HashMap<String, usize> = procesos
        .iter()
        .enumerate()
        .map(|(i, p)| (p.ruta.clone(), i))
        .collect();
    let readme = archivos
        .iter()
        .find(|(ruta, _)| *ruta == "README.md")
        .map_or("", |(_, contenido)| contenido);
    let indice = armar_indice(readme, &procesos, &por_ruta);
    Manual {
        procesos,
        por_ruta,
        indice,
    }
}

/// El índice de esta app tal como lo ordena el README ("Índice — Móvil…");
/// un proceso que no figure ahí queda al final, en "Otros".
fn armar_indice(
    readme: &str,
    procesos: &[Proceso],
    por_ruta: &HashMap<String, usize>,
) -> Vec<GrupoManual> {
    let mut grupos: Vec<GrupoManual> = Vec::new();
    let mut en_indice = std::collections::HashSet::new();
    let mut dentro = false;
    for bloque in leer_markdown(readme) {
        match bloque {
            Bloque::Titulo { nivel, texto, .. } if nivel <= 2 => {
                let t = texto.to_lowercase();
                dentro = t.starts_with("índice") && t.contains("móvil");
            }
            Bloque::Titulo { texto, .. } if dentro => grupos.push(GrupoManual {
                titulo: texto_plano(&leer_en_linea(&texto)),
                temas: Vec::new(),
            }),
            Bloque::Lista { elementos, .. } if dentro => {
                for elemento in elementos {
                    for destino in enlaces(&leer_en_linea(&elemento.texto)) {
                        let ruta = destino.split('#').next().unwrap_or_default();
                        let Some(&i) = por_ruta.get(ruta) else {
                            continue;
                        };
                        if !en_indice.insert(ruta.to_string()) {
                            continue;
                        }
                        if grupos.is_empty() {
                            grupos.push(GrupoManual {
                                titulo: String::new(),
                                temas: Vec::new(),
                            });
                        }
                        if let Some(grupo) = grupos.last_mut() {
                            grupo.temas.push(tema(&procesos[i]));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    // "Fuera del manual", "Mejoras de pantalla…": grupos sin temas.
    grupos.retain(|g| !g.temas.is_empty());
    let mut sueltos: Vec<TemaManual> = procesos
        .iter()
        .filter(|p| !en_indice.contains(&p.ruta))
        .map(tema)
        .collect();
    if !sueltos.is_empty() {
        sueltos.sort_by(|a, b| a.titulo.cmp(&b.titulo));
        grupos.push(GrupoManual {
            titulo: "Otros".to_string(),
            temas: sueltos,
        });
    }
    grupos
}

fn tema(proceso: &Proceso) -> TemaManual {
    TemaManual {
        ruta: proceso.ruta.clone(),
        titulo: proceso.titulo.clone(),
    }
}

fn enlaces(partes: &[EnLinea]) -> Vec<String> {
    let mut destinos = Vec::new();
    for parte in partes {
        match parte {
            EnLinea::Enlace { destino, hijos } => {
                destinos.push(destino.clone());
                destinos.extend(enlaces(hijos));
            }
            EnLinea::Negrita(hijos) => destinos.extend(enlaces(hijos)),
            EnLinea::Texto(_) | EnLinea::Codigo(_) => {}
        }
    }
    destinos
}

/// A dónde lleva un enlace escrito en el proceso `desde`: (ruta, ancla).
/// `None` para direcciones de afuera o que no son del manual.
fn resolver_enlace(desde: &str, destino: &str) -> Option<(String, Option<String>)> {
    if destino.contains("://") || destino.starts_with("mailto:") {
        return None;
    }
    let (camino, ancla) = match destino.split_once('#') {
        Some((camino, ancla)) => (camino, Some(ancla.to_string()).filter(|a| !a.is_empty())),
        None => (destino, None),
    };
    if camino.is_empty() {
        return Some((desde.to_string(), ancla));
    }
    if !std::path::Path::new(camino)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        return None;
    }
    let mut partes: Vec<&str> = desde.split('/').collect();
    partes.pop();
    for parte in camino.split('/') {
        match parte {
            ".." => {
                partes.pop();
            }
            "." => {}
            otra => partes.push(otra),
        }
    }
    Some((partes.join("/"), ancla))
}

// ---- Lo que ve Kotlin ----

/// Un tema del índice.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TemaManual {
    /// Ruta dentro del manual, p. ej. `movil/gafete-kof.md`: se le pasa a
    /// [`tema_manual`].
    pub ruta: String,
    pub titulo: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GrupoManual {
    /// Vacío si el índice no tiene subtítulos.
    pub titulo: String,
    pub temas: Vec<TemaManual>,
}

/// Un pedazo de texto con un solo estilo.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TramoManual {
    pub texto: String,
    pub negrita: bool,
    pub codigo: bool,
    /// Tema al que lleva, si es un enlace a otro tema de este manual.
    pub enlace: Option<String>,
    /// Sección del tema al que lleva (para saltar ahí con [`BloqueManual::Titulo`]).
    pub ancla_enlace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CeldaManual {
    pub tramos: Vec<TramoManual>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FilaManual {
    pub celdas: Vec<CeldaManual>,
}

/// Un bloque para dibujar. Las listas llegan aplanadas: cada elemento con su
/// marca ("1.", "•") y su sangría (0 arriba, 1 en una sublista…).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum BloqueManual {
    Titulo {
        nivel: u32,
        tramos: Vec<TramoManual>,
        /// Para saltar a esta sección desde un enlace o un resultado.
        ancla: String,
    },
    Parrafo {
        sangria: u32,
        tramos: Vec<TramoManual>,
    },
    Elemento {
        sangria: u32,
        marca: String,
        tramos: Vec<TramoManual>,
    },
    /// "Mensajes que pueden aparecer": encabezado y filas.
    Tabla {
        encabezado: Vec<CeldaManual>,
        filas: Vec<FilaManual>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ContenidoTemaManual {
    pub ruta: String,
    pub titulo: String,
    pub bloques: Vec<BloqueManual>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ResultadoManual {
    pub ruta: String,
    pub titulo: String,
    /// Sección donde está lo que se buscó, y su ancla para ir directo.
    pub seccion: Option<String>,
    pub ancla: Option<String>,
    /// El trozo donde aparece, para mostrar debajo del título.
    pub fragmento: String,
}

/// Cuántos resultados devuelve [`buscar_en_manual`] como máximo.
const LIMITE_RESULTADOS: usize = 20;

/// Los temas de la ayuda, agrupados como el índice del manual.
#[uniffi::export]
pub fn indice_manual() -> Vec<GrupoManual> {
    MANUAL.indice.clone()
}

/// Un tema para leer, o `None` si esa ruta no existe.
#[uniffi::export]
pub fn tema_manual(ruta: String) -> Option<ContenidoTemaManual> {
    contenido(&MANUAL, &ruta)
}

/// Busca en la ayuda: palabras sueltas o un mensaje pegado entero.
#[uniffi::export]
pub fn buscar_en_manual(consulta: String) -> Vec<ResultadoManual> {
    buscar::buscar(&MANUAL.procesos, &consulta, LIMITE_RESULTADOS)
        .into_iter()
        .map(|encontrado| ResultadoManual {
            ruta: encontrado.ruta,
            titulo: encontrado.titulo,
            seccion: encontrado.seccion,
            ancla: encontrado.ancla,
            fragmento: encontrado.fragmento,
        })
        .collect()
}

fn contenido(manual: &Manual, ruta: &str) -> Option<ContenidoTemaManual> {
    let proceso = manual.proceso(ruta)?;
    let mut bloques = Vec::new();
    aplanar(manual, proceso, &proceso.bloques, 0, &mut bloques);
    Some(ContenidoTemaManual {
        ruta: proceso.ruta.clone(),
        titulo: proceso.titulo.clone(),
        bloques,
    })
}

fn aplanar(
    manual: &Manual,
    proceso: &Proceso,
    bloques: &[Bloque],
    sangria: u32,
    salida: &mut Vec<BloqueManual>,
) {
    let a_tramos = |texto: &str| tramos(manual, proceso, texto);
    for bloque in bloques {
        match bloque {
            Bloque::Titulo {
                nivel,
                texto,
                ancla,
                ..
            } => salida.push(BloqueManual::Titulo {
                nivel: u32::try_from(*nivel).unwrap_or(6),
                tramos: a_tramos(texto),
                ancla: ancla.clone(),
            }),
            Bloque::Parrafo { texto } => salida.push(BloqueManual::Parrafo {
                sangria,
                tramos: a_tramos(texto),
            }),
            Bloque::Lista {
                ordenada,
                inicio,
                elementos,
            } => {
                for (numero, elemento) in (*inicio..).zip(elementos) {
                    salida.push(BloqueManual::Elemento {
                        sangria,
                        marca: if *ordenada {
                            format!("{numero}.")
                        } else {
                            "•".to_string()
                        },
                        tramos: a_tramos(&elemento.texto),
                    });
                    aplanar(manual, proceso, &elemento.hijos, sangria + 1, salida);
                }
            }
            Bloque::Tabla { encabezado, filas } => {
                let celda = |texto: &String| CeldaManual {
                    tramos: a_tramos(texto),
                };
                salida.push(BloqueManual::Tabla {
                    encabezado: encabezado.iter().map(celda).collect(),
                    filas: filas
                        .iter()
                        .map(|fila| FilaManual {
                            celdas: fila.iter().map(celda).collect(),
                        })
                        .collect(),
                });
            }
        }
    }
}

/// Estilo heredado mientras se baja por negritas y enlaces.
#[derive(Clone, Default)]
struct Estilo {
    negrita: bool,
    enlace: Option<(String, Option<String>)>,
}

fn tramos(manual: &Manual, proceso: &Proceso, texto: &str) -> Vec<TramoManual> {
    let mut salida = Vec::new();
    juntar_tramos(
        manual,
        proceso,
        &leer_en_linea(texto),
        &Estilo::default(),
        &mut salida,
    );
    salida
}

fn juntar_tramos(
    manual: &Manual,
    proceso: &Proceso,
    partes: &[EnLinea],
    estilo: &Estilo,
    salida: &mut Vec<TramoManual>,
) {
    let tramo = |texto: &str, codigo: bool| TramoManual {
        texto: texto.to_string(),
        negrita: estilo.negrita,
        codigo,
        enlace: estilo.enlace.as_ref().map(|(ruta, _)| ruta.clone()),
        ancla_enlace: estilo.enlace.as_ref().and_then(|(_, ancla)| ancla.clone()),
    };
    for parte in partes {
        match parte {
            EnLinea::Texto(texto) => salida.push(tramo(texto, false)),
            EnLinea::Codigo(texto) => salida.push(tramo(texto, true)),
            EnLinea::Negrita(hijos) => {
                let interno = Estilo {
                    negrita: true,
                    ..estilo.clone()
                };
                juntar_tramos(manual, proceso, hijos, &interno, salida);
            }
            EnLinea::Enlace { destino, hijos } => {
                // Sólo se sigue un enlace a un tema que esté en este manual;
                // el resto queda como texto.
                let enlace = resolver_enlace(&proceso.ruta, destino)
                    .filter(|(ruta, _)| manual.proceso(ruta).is_some());
                let interno = Estilo {
                    enlace: enlace.or_else(|| estilo.enlace.clone()),
                    ..estilo.clone()
                };
                juntar_tramos(manual, proceso, hijos, &interno, salida);
            }
        }
    }
}
