//! Utilidades de texto y fechas compartidas por los lectores.
//!
//! Los lectores se portaron desde Kotlin (auditoría OCR 2026-09-28, punto
//! A-1) conservando su comportamiento: estas funciones reproducen la
//! semántica de las de la biblioteca estándar de Kotlin que usaban
//! (`lines()`, `isBlank()`, `maxByOrNull`...), que no siempre coincide con
//! la de Rust.

use chrono::{Datelike, NaiveDate};
use regex::Regex;

/// Compila un patrón fijo del código. Todos se compilan una sola vez
/// (`LazyLock`) y los tests de cada lector los ejercitan: un patrón
/// inválido falla en CI, nunca en el teléfono.
pub fn patron(expresion: &str) -> Regex {
    Regex::new(expresion).unwrap_or_else(|e| panic!("patrón inválido {expresion:?}: {e}"))
}

/// Renglones como `String.lines()` de Kotlin: corta en `\r\n`, `\n` y `\r`,
/// y conserva el renglón vacío final (`"a\n"` da `["a", ""]`), a diferencia
/// de `str::lines`.
pub fn renglones(texto: &str) -> Vec<&str> {
    let mut resultado = Vec::new();
    let mut inicio = 0;
    let bytes = texto.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                resultado.push(&texto[inicio..i]);
                inicio = i + 1;
            }
            b'\r' => {
                resultado.push(&texto[inicio..i]);
                if bytes.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
                inicio = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    resultado.push(&texto[inicio..]);
    resultado
}

/// `isBlank()` de Kotlin: vacío o sólo espacios.
pub fn en_blanco(texto: &str) -> bool {
    texto.chars().all(char::is_whitespace)
}

/// Largo en caracteres (Kotlin cuenta unidades UTF-16; para el texto de un
/// documento, sin emojis, es lo mismo).
pub fn largo(texto: &str) -> usize {
    texto.chars().count()
}

/// El primer elemento de mayor clave (`maxByOrNull` de Kotlin; el
/// `max_by_key` de Rust devuelve el ÚLTIMO en un empate).
pub fn primero_maximo<T, K: Ord>(
    elementos: impl IntoIterator<Item = T>,
    clave: impl Fn(&T) -> K,
) -> Option<T> {
    let mut mejor: Option<(K, T)> = None;
    for elemento in elementos {
        let k = clave(&elemento);
        if mejor.as_ref().is_none_or(|(m, _)| k > *m) {
            mejor = Some((k, elemento));
        }
    }
    mejor.map(|(_, e)| e)
}

/// ¿Es `c` un espacio según `\s` de Java (sin Unicode)? Es lo que
/// consumían las etiquetas `\s*:?` de los patrones originales.
pub fn es_espacio_java(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r')
}

/// Fecha de un documento (vencimiento, nacimiento), ya validada contra el
/// calendario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FechaOcr {
    pub dia: u8,
    pub mes: u8,
    pub anio: i32,
}

impl FechaOcr {
    /// `None` si la fecha no existe (31 de febrero, mes 13...): el OCR puede
    /// leer cualquier número y una fecha imposible no es una fecha.
    pub fn crear_valida(dia: u32, mes: u32, anio: i32) -> Option<Self> {
        let fecha = NaiveDate::from_ymd_opt(anio, mes, dia)?;
        Some(Self {
            dia: u8::try_from(fecha.day()).ok()?,
            mes: u8::try_from(fecha.month()).ok()?,
            anio: fecha.year(),
        })
    }

    fn como_numero(self) -> i64 {
        i64::from(self.anio) * 10_000 + i64::from(self.mes) * 100 + i64::from(self.dia)
    }

    /// Vencida ANTES de `hoy` (el mismo día de vencimiento todavía vale).
    pub fn esta_vencida(self, hoy: Self) -> bool {
        self.como_numero() < hoy.como_numero()
    }

    /// Años cumplidos a `hoy`.
    pub fn edad_en_anios(self, hoy: Self) -> i32 {
        let cumpleanos_ya_paso = hoy.mes > self.mes || (hoy.mes == self.mes && hoy.dia >= self.dia);
        hoy.anio - self.anio - i32::from(!cumpleanos_ya_paso)
    }

    /// `dd-mm-aaaa`, el formato de los formularios de la app.
    pub fn a_texto(self) -> String {
        format!("{:02}-{:02}-{:04}", self.dia, self.mes, self.anio)
    }
}

/// Fecha a partir de los tres grupos (día, mes, año) de un patrón.
pub fn fecha_de_grupos(dia: &str, mes: &str, anio: &str) -> Option<FechaOcr> {
    FechaOcr::crear_valida(dia.parse().ok()?, mes.parse().ok()?, anio.parse().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renglones_como_kotlin() {
        assert_eq!(renglones("a\nb"), vec!["a", "b"]);
        assert_eq!(renglones("a\r\nb\rc"), vec!["a", "b", "c"]);
        assert_eq!(renglones("a\n"), vec!["a", ""]);
        assert_eq!(renglones(""), vec![""]);
    }

    #[test]
    fn primero_maximo_desempata_por_el_primero() {
        let v = ["aa", "bb", "c"];
        assert_eq!(primero_maximo(v, |s| s.len()), Some("aa"));
        assert_eq!(primero_maximo(Vec::<&str>::new(), |s| s.len()), None);
    }

    #[test]
    fn fechas_imposibles_no_se_aceptan() {
        assert_eq!(FechaOcr::crear_valida(31, 2, 2030), None);
        assert_eq!(FechaOcr::crear_valida(0, 1, 2030), None);
        assert_eq!(
            FechaOcr::crear_valida(3, 4, 2030),
            Some(FechaOcr {
                dia: 3,
                mes: 4,
                anio: 2030
            })
        );
    }

    #[test]
    fn vencimiento_y_edad() {
        let hoy = FechaOcr {
            dia: 15,
            mes: 6,
            anio: 2026,
        };
        assert!(
            FechaOcr {
                dia: 14,
                mes: 6,
                anio: 2026
            }
            .esta_vencida(hoy)
        );
        assert!(
            !FechaOcr {
                dia: 15,
                mes: 6,
                anio: 2026
            }
            .esta_vencida(hoy)
        );
        assert_eq!(
            FechaOcr {
                dia: 16,
                mes: 6,
                anio: 2008
            }
            .edad_en_anios(hoy),
            17
        );
        assert_eq!(
            FechaOcr {
                dia: 15,
                mes: 6,
                anio: 2008
            }
            .edad_en_anios(hoy),
            18
        );
        assert_eq!(
            FechaOcr {
                dia: 3,
                mes: 4,
                anio: 2030
            }
            .a_texto(),
            "03-04-2030"
        );
    }
}
