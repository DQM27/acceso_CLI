//! Placa o número de unidad de un camión de ruta (paso 3 del checklist de
//! rutas y registro de proveedores). Un solo campo cubre dos datos: la
//! flota roja trae NÚMERO DE UNIDAD (calcomanía, ej. `22906`) y los camiones
//! de apoyo sólo PLACA; se prueban los dos patrones sobre el mismo texto.

use std::sync::LazyLock;

use regex::Regex;

use super::texto::patron;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TipoVehiculoDetectado {
    Placa,
    NumeroUnidad,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VehiculoRutaDetectado {
    pub valor: String,
    pub tipo: TipoVehiculoDetectado,
}

// Placa de carga (`C`/`CL` + dígitos). En la foto real de `CL371931` ML Kit
// devolvió "E37 1931": el prefijo apilado se funde en UN carácter (no
// siempre `C`) y los dígitos salen partidos. Por eso se aceptan 1-2 letras
// cualesquiera (la "E" del "CL" apilado se restituye, ver
// `PREFIJO_CL_APILADO_LEIDO`) y dos tandas de dígitos con separador
// opcional. Las
// posiciones de dígito aceptan también las letras que el OCR confunde con
// un dígito; cuáles se aceptan lo decide [`corregir_grupo`].
static PLACA_CARGA: LazyLock<Regex> = LazyLock::new(|| {
    patron(r"\b([A-Z]{1,2})[\s-]?([0-9ODQILZSGB]{2,3})[\s-]?([0-9ODQILZSGB]{2,4})\b")
});
// Particular (camiones de apoyo): 3 letras + 3 dígitos, ej. `BPH485`, con
// las confusiones en las dos direcciones (`8PH485`, `BPH48S`).
static PLACA_PARTICULAR: LazyLock<Regex> =
    LazyLock::new(|| patron(r"\b([A-Z0-2568]{3})[\s-]?([0-9ODQILZSGB]{3})\b"));
// Moto: dos grupos de 3 en renglones separados y a veces una `M` suelta;
// "COSTA RICA"/"CENTROAMERICA" mal leídos nunca calzan (una sola palabra
// larga).
static GRUPO_TRIPLE_MOTO: LazyLock<Regex> = LazyLock::new(|| patron(r"\b[A-Z0-9]{3}\b"));
static PREFIJO_MOTO: LazyLock<Regex> = LazyLock::new(|| patron(r"\bM\b"));
// Número de unidad: la calcomanía es casi lo único en el recuadro. Única
// muestra real `22906`; se admiten 4-6 dígitos.
static NUMERO_UNIDAD: LazyLock<Regex> = LazyLock::new(|| patron(r"\b([0-9]{4,6})\b"));

/// Una placa no tiene dígito verificador que confirme una corrección: se
/// admite UNA por placa. Suficiente para el carácter suelto mal leído y lo
/// bastante estricto para que un número de unidad (`228051` -> "ZZB051"
/// serían 3) nunca pase por placa.
const MAXIMO_CORRECCIONES_POR_PLACA: usize = 1;

fn digito_por_letra(c: char) -> Option<char> {
    match c {
        'O' | 'D' | 'Q' => Some('0'),
        'I' | 'L' => Some('1'),
        'Z' => Some('2'),
        'S' => Some('5'),
        'G' => Some('6'),
        'B' => Some('8'),
        _ => None,
    }
}

fn letra_por_digito(c: char) -> Option<char> {
    match c {
        '0' => Some('O'),
        '1' => Some('I'),
        '2' => Some('Z'),
        '5' => Some('S'),
        '6' => Some('G'),
        '8' => Some('B'),
        _ => None,
    }
}

/// Lleva cada carácter de `grupo` a la clase que pide su posición. Devuelve
/// el grupo corregido y cuántos cambios hizo, o `None` si algún carácter no
/// es válido ni tiene reemplazo.
fn corregir_grupo(
    grupo: &str,
    es_valido: fn(&char) -> bool,
    reemplazo: fn(char) -> Option<char>,
) -> Option<(String, usize)> {
    let mut correcciones = 0;
    let corregido = grupo
        .chars()
        .map(|c| {
            if es_valido(&c) {
                Some(c)
            } else {
                correcciones += 1;
                reemplazo(c)
            }
        })
        .collect::<Option<String>>()?;
    Some((corregido, correcciones))
}

struct Candidata {
    valor: String,
    correcciones: usize,
}

// En la placa de carga liviana la "C" va ENCIMA de la "L" y ML Kit lee el
// par como una sola "E" (foto real: `CL371931` -> "E37 1931"). No existe
// ninguna clase de placa con prefijo "E" solo (sí "EE", equipo especial),
// así que "E" + 6 dígitos sólo puede ser "CL": se restituye sin contarlo
// como corrección dudosa.
const PREFIJO_CL_APILADO_LEIDO: &str = "E";
const PREFIJO_CARGA_LIVIANA: &str = "CL";
const DIGITOS_PLACA_CARGA: usize = 6;

fn placa_de_carga(texto: &str) -> Option<Candidata> {
    PLACA_CARGA
        .captures_iter(texto)
        .filter_map(|c| {
            let (digitos, correcciones) = corregir_grupo(
                &format!("{}{}", &c[2], &c[3]),
                char::is_ascii_digit,
                digito_por_letra,
            )?;
            let prefijo =
                if &c[1] == PREFIJO_CL_APILADO_LEIDO && digitos.len() == DIGITOS_PLACA_CARGA {
                    PREFIJO_CARGA_LIVIANA
                } else {
                    &c[1]
                };
            ((4..=6).contains(&digitos.len()) && correcciones <= MAXIMO_CORRECCIONES_POR_PLACA)
                .then(|| Candidata {
                    valor: format!("{prefijo}{digitos}"),
                    correcciones,
                })
        })
        .min_by_key(|c| c.correcciones)
}

fn placa_particular(texto: &str) -> Option<Candidata> {
    PLACA_PARTICULAR
        .captures_iter(texto)
        .filter_map(|c| {
            let (letras, en_letras) =
                corregir_grupo(&c[1], char::is_ascii_uppercase, letra_por_digito)?;
            let (digitos, en_digitos) =
                corregir_grupo(&c[2], char::is_ascii_digit, digito_por_letra)?;
            let correcciones = en_letras + en_digitos;
            (correcciones <= MAXIMO_CORRECCIONES_POR_PLACA).then(|| Candidata {
                valor: format!("{letras}{digitos}"),
                correcciones,
            })
        })
        .min_by_key(|c| c.correcciones)
}

/// Dos grupos de 3 seguidos con dígitos en AL MENOS uno (dos palabras de 3
/// letras rotuladas en el camión, "KOF" y "CRC", tapaban la calcomanía), y
/// `M` delante sólo si apareció suelta.
fn moto(texto: &str) -> Option<VehiculoRutaDetectado> {
    let grupos: Vec<&str> = GRUPO_TRIPLE_MOTO
        .find_iter(texto)
        .map(|m| m.as_str())
        .collect();
    let tiene_digito = |g: &str| g.chars().any(|c| c.is_ascii_digit());
    let par = grupos
        .windows(2)
        .find(|par| tiene_digito(par[0]) || tiene_digito(par[1]))?;
    let prefijo = if PREFIJO_MOTO.is_match(texto) {
        "M"
    } else {
        ""
    };
    Some(VehiculoRutaDetectado {
        valor: format!("{prefijo}{}{}", par[0], par[1]),
        tipo: TipoVehiculoDetectado::Placa,
    })
}

/// Placa primero (el patrón más específico) y número de unidad sólo si no
/// hay placa, para no leer la parte numérica de una placa como unidad.
/// Entre carga y particular gana la que necesitó MENOS correcciones (a
/// igualdad, la de carga): `SGB123` no se convierte en `SG8123`.
#[uniffi::export]
pub fn extraer_vehiculo(texto: String) -> Option<VehiculoRutaDetectado> {
    let normalizado = texto.to_uppercase();
    let placa = [placa_de_carga(&normalizado), placa_particular(&normalizado)]
        .into_iter()
        .flatten()
        .min_by_key(|c| c.correcciones);
    if let Some(p) = placa {
        return Some(VehiculoRutaDetectado {
            valor: p.valor,
            tipo: TipoVehiculoDetectado::Placa,
        });
    }
    moto(&normalizado).or_else(|| {
        NUMERO_UNIDAD
            .captures(&normalizado)
            .map(|c| VehiculoRutaDetectado {
                valor: c[1].to_owned(),
                tipo: TipoVehiculoDetectado::NumeroUnidad,
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leer(texto: &str) -> Option<(String, TipoVehiculoDetectado)> {
        extraer_vehiculo(texto.to_owned()).map(|v| (v.valor, v.tipo))
    }

    #[test]
    fn placas_de_carga_particular_y_moto() {
        use TipoVehiculoDetectado::{NumeroUnidad, Placa};
        assert_eq!(
            leer("FIAT\nE37 1931\nCOSTA RICA"),
            Some(("CL371931".into(), Placa))
        );
        assert_eq!(leer("BPH485"), Some(("BPH485".into(), Placa)));
        assert_eq!(leer("8PH485"), Some(("BPH485".into(), Placa)));
        assert_eq!(leer("SGB123"), Some(("SGB123".into(), Placa)));
        assert_eq!(leer("CoSmCA\n947\n369\nM"), Some(("M947369".into(), Placa)));
        assert_eq!(
            leer("KOF\nCRC\n22906"),
            Some(("22906".into(), NumeroUnidad))
        );
    }

    #[test]
    fn la_e_del_cl_apilado_solo_se_restituye_con_seis_digitos() {
        use TipoVehiculoDetectado::Placa;
        assert_eq!(leer("E 12345"), Some(("E12345".into(), Placa)));
        assert_eq!(leer("E-123456"), Some(("CL123456".into(), Placa)));
    }

    #[test]
    fn mas_de_una_correccion_no_es_placa() {
        // Leído como particular serían 3 correcciones ("228" -> "ZZB"): es
        // el número de unidad.
        assert_eq!(
            leer("228051"),
            Some(("228051".into(), TipoVehiculoDetectado::NumeroUnidad))
        );
    }
}
