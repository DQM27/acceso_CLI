//! Lectura MÍNIMA del PDF417 del reverso de la cédula de identidad anterior
//! (la de fondo azul y la serie 2019; la de 2025 lo reemplazó por MRZ).
//!
//! El código trae, ofuscados con un XOR de 17 bytes, los datos personales y
//! las huellas dactilares. Por diseño este módulo sólo puede devolver
//! cédula, nombre y apellidos:
//!
//! - recibe como máximo los primeros [`LARGO_PREFIJO_PDF417_CEDULA`] bytes
//!   (Kotlin corta el resto y lo pone en cero antes de cruzar el FFI);
//! - de esos, sólo descifra hasta el final del nombre -- sexo, fechas y
//!   todo lo que sigue (huellas) nunca se descifran;
//! - pone en cero lo recibido y lo descifrado antes de devolver.
//!
//! Estructura verificada en dos implementaciones independientes
//! (`LectorCedulasCR`, `Python-Datos-Cedula-Costa-Rica`) y contra el patrón de
//! relleno de una lectura real (las zonas vacías cifradas repiten la clave
//! tal cual). Ver `docs/auditorias/investigacion-lectura-documentos-2026-09-28.md`.

use std::ops::Range;

/// Clave de ofuscación del PDF417 de la cédula anterior. Una de las dos
/// implementaciones públicas trae `0x22` en la posición 10: es una errata,
/// el relleno cifrado de una lectura real muestra `0x33`.
const CLAVE_XOR: [u8; 17] = [
    0x27, 0x30, 0x04, 0xA0, 0x00, 0x0F, 0x93, 0x12, 0xA0, 0xD1, 0x33, 0xE0, 0x03, 0xD0, 0x00, 0xDF,
    0x00,
];

const RANGO_CEDULA: Range<usize> = 0..9;
const RANGO_APELLIDO1: Range<usize> = 9..35;
const RANGO_APELLIDO2: Range<usize> = 35..61;
const RANGO_NOMBRE: Range<usize> = 61..91;

/// Bytes que Kotlin debe pasar: hasta el final del nombre, nada más.
pub const LARGO_PREFIJO_PDF417_CEDULA: usize = RANGO_NOMBRE.end;
const LARGO_PREFIJO_PDF417_CEDULA_U32: u32 = 91;
const _: () = assert!(LARGO_PREFIJO_PDF417_CEDULA == LARGO_PREFIJO_PDF417_CEDULA_U32 as usize);

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DatosPdf417Cedula {
    pub cedula: String,
    pub nombre: String,
    /// "1er apellido 2do apellido", separados por un espacio.
    pub apellidos: String,
}

/// Largo del prefijo que se debe entregar a [`leer_pdf417_cedula`]. Función
/// (no constante exportada) para que Kotlin lo tome de acá y no duplique
/// el número.
#[uniffi::export]
pub fn largo_prefijo_pdf417_cedula() -> u32 {
    LARGO_PREFIJO_PDF417_CEDULA_U32
}

/// Descifra cédula, nombre y apellidos de los primeros bytes crudos del
/// PDF417. `None` si el prefijo es corto o lo descifrado no tiene forma de
/// cédula costarricense (9 dígitos con provincia 1-9 y nombres sólo con
/// letras): así un PDF417 de otro documento, o una lectura corrupta, nunca
/// produce datos.
#[uniffi::export]
pub fn leer_pdf417_cedula(mut prefijo: Vec<u8>) -> Option<DatosPdf417Cedula> {
    let resultado = descifrar_y_validar(&prefijo);
    prefijo.fill(0);
    resultado
}

fn descifrar_y_validar(prefijo: &[u8]) -> Option<DatosPdf417Cedula> {
    if prefijo.len() < LARGO_PREFIJO_PDF417_CEDULA {
        return None;
    }
    let mut claro: Vec<u8> = prefijo[..LARGO_PREFIJO_PDF417_CEDULA]
        .iter()
        .zip(CLAVE_XOR.iter().cycle())
        .map(|(byte, clave)| byte ^ clave)
        .collect();
    let resultado = extraer_campos(&claro);
    claro.fill(0);
    resultado
}

fn extraer_campos(claro: &[u8]) -> Option<DatosPdf417Cedula> {
    let cedula = cedula_valida(&claro[RANGO_CEDULA])?;
    let apellido1 = campo_de_nombre(&claro[RANGO_APELLIDO1])?;
    let apellido2 = campo_de_nombre(&claro[RANGO_APELLIDO2])?;
    let nombre = campo_de_nombre(&claro[RANGO_NOMBRE])?;
    if nombre.is_empty() || apellido1.is_empty() {
        return None;
    }
    let ambos_apellidos = [apellido1, apellido2]
        .into_iter()
        .filter(|parte| !parte.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Some(DatosPdf417Cedula {
        cedula,
        nombre,
        apellidos: ambos_apellidos,
    })
}

fn cedula_valida(bytes: &[u8]) -> Option<String> {
    let valida = bytes.iter().all(u8::is_ascii_digit) && bytes.first().is_some_and(|b| *b != b'0');
    valida.then(|| String::from_utf8_lossy(bytes).into_owned())
}

/// Campo de texto de largo fijo, relleno con `0x00` o espacios. Se lee
/// como Latin-1 (un byte = un carácter: cubre Ñ y tildes). Cualquier
/// carácter que no sea letra, espacio, apóstrofo o guion -- un dígito, un
/// control -- significa lectura corrupta o documento ajeno: `None`, nunca
/// un nombre "limpiado" a medias.
fn campo_de_nombre(bytes: &[u8]) -> Option<String> {
    let mut texto = String::with_capacity(bytes.len());
    for &byte in bytes {
        let c = char::from(byte);
        if byte == 0 || c == ' ' {
            texto.push(' ');
        } else if c.is_alphabetic() || c == '\'' || c == '-' {
            texto.extend(c.to_uppercase());
        } else {
            return None;
        }
    }
    Some(texto.split_whitespace().collect::<Vec<_>>().join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Arma un PDF417 SINTÉTICO (datos inventados) con la misma estructura
    /// y ofuscación que el real, incluida una cola que simula el resto del
    /// código.
    fn codificar(cedula: &str, apellido1: &str, apellido2: &str, nombre: &str) -> Vec<u8> {
        fn campo(texto: &str, largo: usize) -> Vec<u8> {
            let mut bytes: Vec<u8> = texto.chars().map(|c| c as u8).collect();
            bytes.resize(largo, 0);
            bytes
        }
        let mut claro = Vec::new();
        claro.extend(campo(cedula, 9));
        claro.extend(campo(apellido1, 26));
        claro.extend(campo(apellido2, 26));
        claro.extend(campo(nombre, 30));
        claro.extend(b"M19900101".iter().chain(b"20300101"));
        claro.extend(std::iter::repeat_n(0xAB, 200));
        claro
            .iter()
            .zip(CLAVE_XOR.iter().cycle())
            .map(|(b, k)| b ^ k)
            .collect()
    }

    fn prefijo(codigo: &[u8]) -> Vec<u8> {
        codigo[..LARGO_PREFIJO_PDF417_CEDULA].to_vec()
    }

    #[test]
    fn lee_cedula_nombre_y_apellidos() {
        let codigo = codificar("112340567", "PEREZ", "MORA", "JUAN CARLOS");
        let datos = leer_pdf417_cedula(prefijo(&codigo)).unwrap();
        assert_eq!(datos.cedula, "112340567");
        assert_eq!(datos.nombre, "JUAN CARLOS");
        assert_eq!(datos.apellidos, "PEREZ MORA");
    }

    #[test]
    fn acepta_enie_y_tildes_en_latin1() {
        let codigo = codificar("512340567", "MUÑOZ", "SOLÍS", "ANA MARÍA");
        let datos = leer_pdf417_cedula(prefijo(&codigo)).unwrap();
        assert_eq!(datos.apellidos, "MUÑOZ SOLÍS");
        assert_eq!(datos.nombre, "ANA MARÍA");
    }

    #[test]
    fn segundo_apellido_vacio_no_deja_espacios_de_mas() {
        let codigo = codificar("812340567", "SMITH", "", "JOHN");
        assert_eq!(
            leer_pdf417_cedula(prefijo(&codigo)).unwrap().apellidos,
            "SMITH"
        );
    }

    #[test]
    fn el_codigo_completo_da_lo_mismo_que_el_prefijo_y_nunca_mas_campos() {
        // Aunque llegue el código entero, sólo se descifra el prefijo: el
        // resultado no tiene de dónde sacar fechas ni huellas.
        let codigo = codificar("112340567", "PEREZ", "MORA", "JUAN");
        assert_eq!(
            leer_pdf417_cedula(codigo.clone()),
            leer_pdf417_cedula(prefijo(&codigo))
        );
    }

    #[test]
    fn prefijo_corto_no_produce_datos() {
        let codigo = codificar("112340567", "PEREZ", "MORA", "JUAN");
        assert_eq!(leer_pdf417_cedula(codigo[..90].to_vec()), None);
    }

    #[test]
    fn cedula_que_empieza_en_cero_se_rechaza() {
        let codigo = codificar("012340567", "PEREZ", "MORA", "JUAN");
        assert_eq!(leer_pdf417_cedula(prefijo(&codigo)), None);
    }

    #[test]
    fn digito_dentro_de_un_nombre_indica_lectura_corrupta() {
        let codigo = codificar("112340567", "PER3Z", "MORA", "JUAN");
        assert_eq!(leer_pdf417_cedula(prefijo(&codigo)), None);
    }

    #[test]
    fn un_pdf417_ajeno_sin_ofuscar_no_produce_datos() {
        // Ej. el código de barras de otro documento: sin la clave, lo
        // "descifrado" es basura y no pasa la validación.
        let ajeno: Vec<u8> = b"ANSI 636000090002DL00410278ZV03190008DLDAQT64235789".repeat(2);
        assert_eq!(leer_pdf417_cedula(ajeno), None);
    }

    #[test]
    fn sin_nombre_no_hay_resultado() {
        let codigo = codificar("112340567", "PEREZ", "MORA", "");
        assert_eq!(leer_pdf417_cedula(prefijo(&codigo)), None);
    }
}
