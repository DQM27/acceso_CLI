//! Buscador local del manual: sin internet y sin IA. Mismas reglas que el del
//! escritorio y el panel (`supabase/functions/_shared/manual/buscar.ts`):
//! palabras sueltas sin importar tildes ni mayúsculas, todas si se puede (si
//! no, las más, con un mínimo de la mitad), y premio al mensaje pegado entero.

use std::collections::HashSet;
use std::sync::LazyLock;

use super::Proceso;
use super::markdown::{Bloque, leer_en_linea, texto_plano};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encontrado {
    pub ruta: String,
    pub titulo: String,
    pub puntaje: usize,
    /// El trozo donde más coincide, para mostrar debajo del título.
    pub fragmento: String,
    /// Título de la sección de ese trozo y su ancla, para ir directo.
    pub seccion: Option<String>,
    pub ancla: Option<String>,
}

/// Un párrafo, elemento de lista o fila de tabla, con la sección donde está.
#[derive(Debug, Clone)]
pub struct Trozo {
    texto: String,
    normalizado: String,
    seccion: Option<String>,
    seccion_normalizada: String,
    ancla: Option<String>,
}

/// Minúsculas, sin tildes ni signos, con un solo espacio entre palabras.
pub fn normalizar(texto: &str) -> String {
    let sin_tildes: String = texto
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            otro => otro,
        })
        .collect();
    sin_tildes
        .split(|c: char| !c.is_alphanumeric())
        .filter(|palabra| !palabra.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

// Palabras que no ayudan a encontrar nada ("cómo registro la salida de…").
static VACIAS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    "a al como con cual cuando de del donde el en es esta este hacer hago la las lo los mi mis \
     no o para pero por puedo que qué se si sin su sus un una uno unos y ya"
        .split(' ')
        .map(normalizar)
        .collect()
});

fn plano(texto: &str) -> String {
    texto_plano(&leer_en_linea(texto))
}

/// El proceso en trozos, para buscar dentro.
pub fn trozos(bloques: &[Bloque]) -> Vec<Trozo> {
    let mut lista = Vec::new();
    let mut seccion = None;
    recorrer(bloques, &mut seccion, &mut lista);
    lista
}

fn recorrer(bloques: &[Bloque], seccion: &mut Option<(String, String)>, lista: &mut Vec<Trozo>) {
    let agregar = |texto: String, seccion: &Option<(String, String)>, lista: &mut Vec<Trozo>| {
        if texto.is_empty() {
            return;
        }
        lista.push(Trozo {
            normalizado: normalizar(&texto),
            texto,
            seccion: seccion.as_ref().map(|(titulo, _)| titulo.clone()),
            seccion_normalizada: seccion
                .as_ref()
                .map(|(titulo, _)| normalizar(titulo))
                .unwrap_or_default(),
            ancla: seccion.as_ref().map(|(_, ancla)| ancla.clone()),
        });
    };
    for bloque in bloques {
        match bloque {
            Bloque::Titulo {
                nivel,
                texto,
                ancla,
                ..
            } => {
                if *nivel > 1 {
                    *seccion = Some((plano(texto), ancla.clone()));
                }
            }
            Bloque::Parrafo { texto } => agregar(plano(texto), seccion, lista),
            Bloque::Lista { elementos, .. } => {
                for elemento in elementos {
                    agregar(plano(&elemento.texto), seccion, lista);
                    recorrer(&elemento.hijos, seccion, lista);
                }
            }
            Bloque::Tabla { filas, .. } => {
                for fila in filas {
                    let texto = fila
                        .iter()
                        .map(|c| plano(c))
                        .collect::<Vec<_>>()
                        .join(" — ");
                    agregar(texto, seccion, lista);
                }
            }
        }
    }
}

const LARGO_FRAGMENTO: usize = 180;

/// Recorta un trozo largo alrededor de la primera palabra encontrada.
fn recortar(texto: &str, palabras: &[&str]) -> String {
    let caracteres: Vec<char> = texto.chars().collect();
    if caracteres.len() <= LARGO_FRAGMENTO {
        return texto.to_string();
    }
    // Se busca palabra por palabra en el texto normalizado de cada palabra
    // del original, así la posición es exacta (sin depender de tildes).
    let mut posicion = None;
    let mut desde_caracter = 0;
    for palabra in texto.split(' ') {
        let normal = normalizar(palabra);
        if palabras.iter().any(|p| normal.contains(p)) {
            posicion = Some(desde_caracter);
            break;
        }
        desde_caracter += palabra.chars().count() + 1;
    }
    let mut inicio = posicion.unwrap_or(0).saturating_sub(40);
    if inicio > 0 {
        // Arranca al comienzo de una palabra.
        while inicio < caracteres.len() && caracteres[inicio - 1] != ' ' {
            inicio += 1;
        }
    }
    let fin_maximo = (inicio + LARGO_FRAGMENTO).min(caracteres.len());
    let mut fin = fin_maximo;
    if fin < caracteres.len() {
        while fin > inicio && caracteres[fin] != ' ' {
            fin -= 1;
        }
    }
    let corte: String = caracteres[inicio..fin].iter().collect();
    format!(
        "{}{}{}",
        if inicio > 0 { "…" } else { "" },
        corte.trim_end(),
        if fin < caracteres.len() { "…" } else { "" }
    )
}

/// Procesos que tienen **todas** las palabras buscadas (en el título o en el
/// texto), de más a menos relevante. Si ninguno las tiene todas, los que
/// tienen más, con un mínimo de la mitad. Una palabra coincide también como
/// parte de otra ("gafet" encuentra "gafetes").
pub fn buscar(procesos: &[Proceso], consulta: &str, limite: usize) -> Vec<Encontrado> {
    let frase = normalizar(consulta);
    if frase.chars().count() < 2 {
        return Vec::new();
    }
    let todas: Vec<&str> = frase.split(' ').collect();
    let utiles: Vec<&str> = todas
        .iter()
        .copied()
        .filter(|p| !VACIAS.contains(*p) && p.chars().count() > 1)
        .collect();
    let palabras = if utiles.is_empty() { todas } else { utiles };

    let mut candidatos: Vec<(usize, Encontrado)> = Vec::new();
    for proceso in procesos {
        let titulo = normalizar(&proceso.titulo);
        let todo_el_texto = std::iter::once(titulo.as_str())
            .chain(proceso.trozos.iter().map(|t| t.normalizado.as_str()))
            .collect::<Vec<_>>()
            .join(" ");
        let coincidencias = palabras
            .iter()
            .filter(|p| todo_el_texto.contains(*p))
            .count();
        if coincidencias == 0 {
            continue;
        }

        let mut puntaje = palabras.iter().filter(|p| titulo.contains(*p)).count() * 10;
        if titulo.contains(&frase) {
            puntaje += 15;
        }

        let mut mejor: Option<&Trozo> = None;
        let mut mejor_puntaje = 0;
        for trozo in &proceso.trozos {
            let mut p = 0;
            for palabra in &palabras {
                if trozo.normalizado.contains(palabra) {
                    p += 2;
                }
                if trozo.seccion_normalizada.contains(palabra) {
                    p += 1;
                }
            }
            // El mensaje o la frase entera, tal como se ve en pantalla.
            if palabras.len() > 1 && trozo.normalizado.contains(&frase) {
                p += 6 * palabras.len();
            }
            if p > mejor_puntaje {
                mejor = Some(trozo);
                mejor_puntaje = p;
            }
        }
        puntaje += mejor_puntaje;

        candidatos.push((
            coincidencias,
            Encontrado {
                ruta: proceso.ruta.clone(),
                titulo: proceso.titulo.clone(),
                puntaje,
                fragmento: mejor
                    .map(|t| recortar(&t.texto, &palabras))
                    .unwrap_or_default(),
                seccion: mejor.and_then(|t| t.seccion.clone()),
                ancla: mejor.and_then(|t| t.ancla.clone()),
            },
        ));
    }

    let maximo = candidatos.iter().map(|(c, _)| *c).max().unwrap_or(0);
    if maximo < palabras.len().div_ceil(2) {
        return Vec::new();
    }
    let mut resultados: Vec<Encontrado> = candidatos
        .into_iter()
        .filter(|(c, _)| *c == maximo)
        .map(|(_, encontrado)| encontrado)
        .collect();
    resultados.sort_by(|a, b| {
        b.puntaje
            .cmp(&a.puntaje)
            .then_with(|| a.titulo.cmp(&b.titulo))
    });
    resultados.truncate(limite);
    resultados
}
