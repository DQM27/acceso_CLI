//! Aísla las líneas de MRZ dentro del texto de un frame y las lee con
//! [`leer_mrz`] (parseo, dígitos verificadores y corrección de confusables,
//! en `mrz.rs`). Antes esta parte vivía en `MrzParser.kt`.

use std::sync::LazyLock;

use regex::Regex;

use super::texto::{patron, primero_maximo, renglones};
use crate::mrz::{RegistroMrz, leer_mrz};

/// Caracteres de más o de menos que se toleran en una línea antes de
/// descartarla: ML Kit se come o duplica `<` en las corridas de relleno, y
/// con largo exacto UN `<` perdido alcanzaba para no leer el documento.
const TOLERANCIA_LARGO_MRZ: usize = 2;

/// Símbolos que ML Kit devuelve en lugar de `<` (el MRZ nunca los trae).
/// `«` es dos `<` fundidos en un glifo.
const SUSTITUTOS_RELLENO: [(char, &str); 7] = [
    ('«', "<<"),
    ('»', "<<"),
    ('‹', "<"),
    ('›', "<"),
    ('(', "<"),
    ('[', "<"),
    ('{', "<"),
];

/// Corridas de 2+ `K`: lectura típica de ML Kit para `<<`. El checksum NO
/// las distingue (`K` vale 20 y `<` 0, iguales módulo 10), así que la
/// variante con relleno se prueba PRIMERO (ver [`variantes`]).
static K_COMO_RELLENO: LazyLock<Regex> = LazyLock::new(|| patron("K{2,}"));
static CORRIDA_RELLENO: LazyLock<Regex> = LazyLock::new(|| patron("<+"));

/// Un MRZ encontrado en el texto: las líneas normalizadas a su largo exacto
/// (lo que se vota entre frames) y lo que se leyó de ellas.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LecturaMrz {
    pub lineas: Vec<String>,
    pub registro: RegistroMrz,
}

fn es_alfabeto_mrz(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit() || c == '<'
}

fn normalizar_linea(linea: &str) -> String {
    let mut normalizada = String::with_capacity(linea.len());
    for c in linea.to_uppercase().chars().filter(|c| *c != ' ') {
        match SUSTITUTOS_RELLENO.iter().find(|(s, _)| *s == c) {
            Some((_, relleno)) => normalizada.push_str(relleno),
            None => normalizada.push(c),
        }
    }
    normalizada
}

/// Lleva `linea` a `longitud` agregando o quitando `<` en su corrida de
/// relleno más larga. `conservar_ultimo`: el último carácter es un dígito
/// verificador y nunca se toca. `None` si no hay relleno donde ajustar.
/// Seguro: un ajuste mal ubicado no inventa un dato, hace fallar el
/// checksum compuesto.
fn ajustar_largo(linea: &str, longitud: usize, conservar_ultimo: bool) -> Option<String> {
    // Sólo alfabeto MRZ (ASCII): los índices de bytes son de caracteres.
    let actual = linea.len();
    if actual == longitud {
        return Some(linea.to_owned());
    }
    let limite = if conservar_ultimo {
        actual.saturating_sub(1)
    } else {
        actual
    };
    let corrida = primero_maximo(
        CORRIDA_RELLENO.find_iter(&linea[..limite]),
        regex::Match::len,
    );
    match corrida {
        Some(c) if longitud > actual => Some(format!(
            "{}{}{}",
            &linea[..c.end()],
            "<".repeat(longitud - actual),
            &linea[c.end()..]
        )),
        None if longitud > actual && !conservar_ultimo => {
            Some(format!("{linea}{}", "<".repeat(longitud - actual)))
        }
        Some(c) if longitud < actual && c.len() > actual - longitud => Some(format!(
            "{}{}",
            &linea[..c.start()],
            &linea[c.start() + (actual - longitud)..]
        )),
        _ => None,
    }
}

/// Bloques de `cantidad` líneas CONSECUTIVAS con forma de MRZ, ya
/// normalizadas a su largo exacto. Consecutivas: tomar las primeras que
/// pasan el filtro podía juntar líneas de regiones distintas del documento
/// y fabricar un MRZ.
fn bloques(texto: &str, longitud: usize, cantidad: usize) -> Vec<Vec<String>> {
    let normalizadas: Vec<String> = renglones(texto).into_iter().map(normalizar_linea).collect();
    let es_candidata = |l: &String| {
        (longitud - TOLERANCIA_LARGO_MRZ..=longitud + TOLERANCIA_LARGO_MRZ)
            .contains(&l.chars().count())
            && l.chars().all(es_alfabeto_mrz)
    };
    normalizadas
        .windows(cantidad)
        .filter(|bloque| bloque.iter().all(es_candidata))
        .filter_map(|bloque| {
            // La línea con el dígito verificador compuesto al final es la
            // segunda en ambos formatos (TD1: 2 de 3; TD3: 2 de 2).
            bloque
                .iter()
                .enumerate()
                .map(|(i, l)| ajustar_largo(l, longitud, i == 1))
                .collect::<Option<Vec<_>>>()
        })
        .collect()
}

/// Variantes a probar: con las corridas de `K` como relleno, y tal cual.
fn variantes(bloque: Vec<String>) -> Vec<Vec<String>> {
    let con_relleno: Vec<String> = bloque
        .iter()
        .map(|l| {
            K_COMO_RELLENO
                .replace_all(l, |c: &regex::Captures<'_>| "<".repeat(c[0].len()))
                .into_owned()
        })
        .collect();
    if con_relleno == bloque {
        vec![bloque]
    } else {
        vec![con_relleno, bloque]
    }
}

/// Busca un MRZ (TD1 --cédula, DIMEX-- antes que TD3 --pasaporte--) y se
/// queda con la primera lectura que valida los dígitos verificadores; si
/// ninguna valida, con la primera reconocida (la pantalla sabe así que HAY
/// un MRZ en cuadro, todavía mal leído). `None` si no hay nada con forma de
/// MRZ: la pantalla sigue esperando.
#[uniffi::export]
pub fn leer_lectura_mrz(texto: String, anio_actual: i32) -> Option<LecturaMrz> {
    lectura_mrz(&texto, anio_actual)
}

pub fn lectura_mrz(texto: &str, anio_actual: i32) -> Option<LecturaMrz> {
    let mut primera_reconocida = None;
    let candidatos = bloques(texto, 30, 3)
        .into_iter()
        .chain(bloques(texto, 44, 2));
    for variante in candidatos.flat_map(variantes) {
        let registro = leer_mrz(variante.clone(), anio_actual);
        if !registro.formato_reconocido {
            continue;
        }
        let lectura = LecturaMrz {
            lineas: variante,
            registro,
        };
        if lectura.registro.checksums_validos {
            return Some(lectura);
        }
        primera_reconocida.get_or_insert(lectura);
    }
    primera_reconocida
}

/// Igual que [`lectura_mrz`] sobre varias versiones del mismo frame (ver
/// `renglones_visuales.rs`): gana la primera que valida en cualquiera; si
/// ninguna valida, la primera reconocida.
pub fn lectura_mrz_de_textos(textos: &[String], anio_actual: i32) -> Option<LecturaMrz> {
    let mut primera_reconocida = None;
    for texto in textos {
        match lectura_mrz(texto, anio_actual) {
            Some(lectura) if lectura.registro.checksums_validos => return Some(lectura),
            Some(lectura) => {
                primera_reconocida.get_or_insert(lectura);
            }
            None => {}
        }
    }
    primera_reconocida
}

#[cfg(test)]
mod tests {
    use super::*;

    const TD1: &str = "C<CRI9998887774<<<<<<<<<<<<<<<\n9001011F3001019NIC<<<<<<<<<<<8\nPEREZ<<MARIA<JOSE<<<<<<<<<<<<<";

    #[test]
    fn lee_un_td1_entre_otras_lineas() {
        let texto = format!("REPUBLICA DE COSTA RICA\n{TD1}\nfin");
        let lectura = lectura_mrz(&texto, 2026).unwrap();
        assert!(lectura.registro.checksums_validos);
        assert_eq!(lectura.registro.numero_documento, "999888777");
    }

    #[test]
    fn tolera_rellenos_perdidos_y_sustitutos() {
        let texto = "C<CRI9998887774<<<<<<<<<<<<<\n9001011F3001019NIC<<<<<<<<<<8\nPEREZ«MARIA<JOSE<<<<<<<<<<<<<";
        let lectura = lectura_mrz(texto, 2026).unwrap();
        assert!(lectura.registro.checksums_validos);
        assert_eq!(lectura.lineas[0].len(), 30);
        assert_eq!(lectura.registro.nombres, "MARIA JOSE");
    }

    #[test]
    fn kk_se_prueba_primero_como_relleno() {
        let texto = TD1.replace("PEREZ<<MARIA", "PEREZKKMARIA");
        let lectura = lectura_mrz(&texto, 2026).unwrap();
        assert_eq!(lectura.registro.apellidos, "PEREZ");
    }

    #[test]
    fn sin_mrz_no_hay_lectura() {
        assert_eq!(lectura_mrz("Licencia de Conducir", 2026), None);
    }

    #[test]
    fn entre_varios_textos_gana_el_que_valida() {
        let corrupto = TD1.replace("9998887774", "9998887784");
        let textos = vec![corrupto, TD1.to_owned()];
        assert!(
            lectura_mrz_de_textos(&textos, 2026)
                .unwrap()
                .registro
                .checksums_validos
        );
    }
}
