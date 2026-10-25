//! Lector del markdown del manual de uso (`docs/manual/`). Es el mismo que
//! usan el escritorio y el panel (`supabase/functions/_shared/manual/markdown.ts`),
//! con las mismas reglas: sólo lo que el manual usa (títulos, párrafos, listas
//! numeradas o no y anidadas, tablas, negrita, código y enlaces).

use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnLinea {
    Texto(String),
    Negrita(Vec<Self>),
    Codigo(String),
    Enlace { destino: String, hijos: Vec<Self> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementoLista {
    /// Primer párrafo del elemento, con su markdown en línea.
    pub texto: String,
    /// Lo que viene debajo con sangría (sublistas, párrafos).
    pub hijos: Vec<Bloque>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bloque {
    Titulo {
        nivel: usize,
        texto: String,
        ancla: String,
    },
    Parrafo {
        texto: String,
    },
    Lista {
        ordenada: bool,
        inicio: u32,
        elementos: Vec<ElementoLista>,
    },
    Tabla {
        encabezado: Vec<String>,
        filas: Vec<Vec<String>>,
    },
}

static TITULO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(#{1,6})\s+(.*?)\s*#*\s*$").expect("regex de título"));
static MARCA_LISTA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([-*]|\d+\.)\s+").expect("regex de lista"));
static SEPARADOR_TABLA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)*\|?\s*$").expect("regex de separador")
});
static ENLACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[([^\]]+)\]\(([^)\s]+)\)").expect("regex de enlace"));

fn sangria(linea: &str) -> usize {
    linea.len() - linea.trim_start().len()
}

fn en_blanco(linea: &str) -> bool {
    linea.trim().is_empty()
}

fn empieza_bloque(linea: &str) -> bool {
    TITULO.is_match(linea) || MARCA_LISTA.is_match(linea) || linea.starts_with('|')
}

fn es_ordenada(marca: &str) -> bool {
    marca != "-" && marca != "*"
}

/// Ancla de un título igual a la que genera GitHub, para que los enlaces
/// `proceso.md#titulo` del manual funcionen igual en todas las apps.
pub fn ancla(texto: &str) -> String {
    texto_plano(&leer_en_linea(texto))
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '_' || *c == '-')
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .collect()
}

/// Celdas de una fila de tabla, respetando las barras escapadas (`\|`).
fn celdas(linea: &str) -> Vec<String> {
    let mut fila = linea.trim();
    fila = fila.strip_prefix('|').unwrap_or(fila);
    if fila.ends_with('|') && !fila.ends_with("\\|") {
        fila = &fila[..fila.len() - 1];
    }
    let mut resultado = Vec::new();
    let mut actual = String::new();
    let mut caracteres = fila.chars().peekable();
    while let Some(c) = caracteres.next() {
        if c == '\\' && caracteres.peek() == Some(&'|') {
            actual.push('|');
            caracteres.next();
        } else if c == '|' {
            resultado.push(actual.trim().to_string());
            actual.clear();
        } else {
            actual.push(c);
        }
    }
    resultado.push(actual.trim().to_string());
    resultado
}

/// Convierte el markdown en bloques. Las líneas cortadas de un párrafo se
/// unen con un espacio.
pub fn leer_markdown(markdown: &str) -> Vec<Bloque> {
    let normalizado = markdown.replace("\r\n", "\n");
    let lineas: Vec<&str> = normalizado.split('\n').collect();
    let mut bloques = leer_bloques(&lineas);
    // Títulos repetidos: GitHub agrega -1, -2… al ancla.
    let mut usadas = std::collections::HashMap::<String, usize>::new();
    numerar_anclas(&mut bloques, &mut usadas);
    bloques
}

fn numerar_anclas(bloques: &mut [Bloque], usadas: &mut std::collections::HashMap<String, usize>) {
    for bloque in bloques {
        match bloque {
            Bloque::Titulo { ancla, .. } => {
                let veces = usadas.entry(ancla.clone()).or_insert(0);
                if *veces > 0 {
                    *ancla = format!("{ancla}-{veces}");
                }
                *veces += 1;
            }
            Bloque::Lista { elementos, .. } => {
                for elemento in elementos {
                    numerar_anclas(&mut elemento.hijos, usadas);
                }
            }
            Bloque::Parrafo { .. } | Bloque::Tabla { .. } => {}
        }
    }
}

fn leer_bloques(lineas: &[&str]) -> Vec<Bloque> {
    let mut bloques = Vec::new();
    let mut i = 0;
    while i < lineas.len() {
        let linea = lineas[i];
        if en_blanco(linea) {
            i += 1;
        } else if let Some(titulo) = TITULO.captures(linea) {
            let texto = titulo[2].to_string();
            bloques.push(Bloque::Titulo {
                nivel: titulo[1].len(),
                ancla: ancla(&texto),
                texto,
            });
            i += 1;
        } else if linea.starts_with('|') {
            let mut filas = Vec::new();
            while i < lineas.len() && lineas[i].trim_start().starts_with('|') {
                if !SEPARADOR_TABLA.is_match(lineas[i]) {
                    filas.push(celdas(lineas[i]));
                }
                i += 1;
            }
            let mut filas = filas.into_iter();
            let encabezado = filas.next().unwrap_or_default();
            bloques.push(Bloque::Tabla {
                encabezado,
                filas: filas.collect(),
            });
        } else if MARCA_LISTA.is_match(linea) {
            let (lista, siguiente) = leer_lista(lineas, i);
            bloques.push(lista);
            i = siguiente;
        } else {
            let mut parrafo = Vec::new();
            while i < lineas.len()
                && !en_blanco(lineas[i])
                && (parrafo.is_empty() || !empieza_bloque(lineas[i]))
            {
                parrafo.push(lineas[i].trim());
                i += 1;
            }
            bloques.push(Bloque::Parrafo {
                texto: parrafo.join(" "),
            });
        }
    }
    bloques
}

/// Lee la lista que arranca en `lineas[inicio]`; devuelve la lista y la
/// primera línea que ya no es suya.
fn leer_lista(lineas: &[&str], inicio: usize) -> (Bloque, usize) {
    let primera = MARCA_LISTA
        .captures(lineas[inicio])
        .map(|c| c[1].to_string())
        .unwrap_or_default();
    let ordenada = es_ordenada(&primera);
    let numero = if ordenada {
        primera.trim_end_matches('.').parse().unwrap_or(1)
    } else {
        1
    };
    let mut elementos = Vec::new();
    let mut i = inicio;
    while i < lineas.len() {
        let Some(marca) = MARCA_LISTA.captures(lineas[i]) else {
            break;
        };
        if es_ordenada(&marca[1]) != ordenada {
            break;
        }
        let ancho_marca = marca[0].len();
        let mut cuerpo: Vec<String> = vec![lineas[i][ancho_marca..].to_string()];
        i += 1;
        while i < lineas.len() {
            let siguiente = lineas[i];
            if en_blanco(siguiente) {
                // Una línea en blanco sigue dentro del elemento sólo si lo
                // que viene después tiene sangría.
                let mut j = i + 1;
                while j < lineas.len() && en_blanco(lineas[j]) {
                    j += 1;
                }
                if j < lineas.len() && sangria(lineas[j]) > 0 {
                    cuerpo.push(String::new());
                    i += 1;
                    continue;
                }
                break;
            }
            if sangria(siguiente) > 0 {
                cuerpo.push(siguiente[sangria(siguiente).min(ancho_marca)..].to_string());
                i += 1;
                continue;
            }
            // Sin sangría: otra marca cierra el elemento; texto suelto pegado
            // a la línea anterior la continúa.
            if empieza_bloque(siguiente) || cuerpo.last().is_some_and(|l| en_blanco(l)) {
                break;
            }
            cuerpo.push(siguiente.to_string());
            i += 1;
        }
        elementos.push(elemento_lista(&cuerpo));
        // Una línea en blanco entre elementos de la misma lista no la corta.
        let mut j = i;
        while j < lineas.len() && en_blanco(lineas[j]) {
            j += 1;
        }
        if j > i && j < lineas.len() && MARCA_LISTA.is_match(lineas[j]) {
            i = j;
        }
    }
    (
        Bloque::Lista {
            ordenada,
            inicio: numero,
            elementos,
        },
        i,
    )
}

fn elemento_lista(cuerpo: &[String]) -> ElementoLista {
    let mut fin = 1;
    while fin < cuerpo.len()
        && !en_blanco(&cuerpo[fin])
        && !empieza_bloque(cuerpo[fin].trim_start())
    {
        fin += 1;
    }
    let texto = cuerpo[..fin]
        .iter()
        .map(|linea| linea.trim())
        .collect::<Vec<_>>()
        .join(" ");
    let resto: Vec<&str> = cuerpo[fin..].iter().map(String::as_str).collect();
    ElementoLista {
        texto,
        hijos: leer_bloques(&resto),
    }
}

/// Markdown en línea: `**negrita**`, `` `código` `` y `[texto](destino)`.
pub fn leer_en_linea(texto: &str) -> Vec<EnLinea> {
    let mut partes = Vec::new();
    let mut suelto = String::new();
    let cerrar = |suelto: &mut String, partes: &mut Vec<EnLinea>| {
        if !suelto.is_empty() {
            partes.push(EnLinea::Texto(std::mem::take(suelto)));
        }
    };
    let mut i = 0;
    while i < texto.len() {
        let resto = &texto[i..];
        if let Some(adentro) = resto.strip_prefix("**")
            && let Some(fin) = adentro.find("**")
            && fin > 0
        {
            cerrar(&mut suelto, &mut partes);
            partes.push(EnLinea::Negrita(leer_en_linea(&adentro[..fin])));
            i += fin + 4;
            continue;
        }
        if let Some(adentro) = resto.strip_prefix('`')
            && let Some(fin) = adentro.find('`')
            && fin > 0
        {
            cerrar(&mut suelto, &mut partes);
            partes.push(EnLinea::Codigo(adentro[..fin].to_string()));
            i += fin + 2;
            continue;
        }
        if resto.starts_with('[')
            && let Some(enlace) = ENLACE.captures(resto)
        {
            cerrar(&mut suelto, &mut partes);
            partes.push(EnLinea::Enlace {
                destino: enlace[2].to_string(),
                hijos: leer_en_linea(&enlace[1]),
            });
            i += enlace[0].len();
            continue;
        }
        let mut caracteres = resto.chars();
        let c = caracteres.next().unwrap_or_default();
        if c == '\\'
            && let Some(escapado) = caracteres.next()
        {
            suelto.push(escapado);
            i += 1 + escapado.len_utf8();
            continue;
        }
        suelto.push(c);
        i += c.len_utf8();
    }
    cerrar(&mut suelto, &mut partes);
    partes
}

/// El texto tal como se lee, sin marcas.
pub fn texto_plano(partes: &[EnLinea]) -> String {
    partes
        .iter()
        .map(|parte| match parte {
            EnLinea::Texto(texto) | EnLinea::Codigo(texto) => texto.clone(),
            EnLinea::Negrita(hijos) | EnLinea::Enlace { hijos, .. } => texto_plano(hijos),
        })
        .collect()
}
