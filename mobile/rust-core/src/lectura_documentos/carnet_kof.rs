//! Carnet KOF (personal interno de Coca-Cola FEMSA) del paso "Gafete KOF"
//! del checklist de rutas: el encargado se identifica por su código de
//! empleado, no por cédula.

use std::sync::LazyLock;

use regex::Regex;

use super::texto::{patron, renglones};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CarnetKofDetectado {
    pub nombre: Option<String>,
    pub codigo_empleado: Option<String>,
}

// La marca la comparte el comprobante de carga: hay que excluirlo aparte.
static MARCA_FEMSA: LazyLock<Regex> = LazyLock::new(|| patron(r"(?i)COCA[\s-]*COLA\s+FEMSA"));
// El logo "Coca-Cola" en letra script sale mal; "FEMSA" en bloque alcanza.
static FEMSA_SOLO: LazyLock<Regex> = LazyLock::new(|| patron(r"(?i)\bFEMSA\b"));
static MARCADOR_COMPROBANTE: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)Ruta\s*/\s*No\.?\s*de\s*Carga"));
// Antigüedad decorativa ("5 AÑOS") y texto fijo del reverso: señales del
// carnet que no aparecen en el comprobante ni en una cédula.
static ANTIGUEDAD: LazyLock<Regex> = LazyLock::new(|| patron(r"(?i)\b[0-9]{1,2}\s*A[ÑN]OS\b"));
static RESPALDO_KOF: LazyLock<Regex> = LazyLock::new(|| patron(r"(?i)ALERTA\s+Y\s+RESPUESTA"));
// Código de empleado: 5-7 dígitos (base real de 1438 empleados) en su
// PROPIO renglón: el teléfono de emergencia `800-2256327` esconde 7
// dígitos seguidos.
static CODIGO_EMPLEADO_RENGLON: LazyLock<Regex> = LazyLock::new(|| patron(r"^[0-9]{5,7}$"));
static PALABRA_CAPITALIZADA: LazyLock<Regex> = LazyLock::new(|| patron(r"^\p{Lu}\p{Ll}+$"));

const PALABRAS_NO_NOMBRE_KOF: [&str; 20] = [
    "CENTRAL",
    "COSTA",
    "RICA",
    "ALERTA",
    "RESPUESTA",
    "EMERGENCIA",
    "ACCIDENTE",
    "LLAMAR",
    "SANGRE",
    "ALERGIA",
    "TPO",
    "COCA",
    "COLA",
    "FEMSA",
    "SEOS",
    "ADP",
    "HID",
    "XT",
    "AÑOS",
    "ANOS",
];
const PARTICULAS_NOMBRE: [&str; 6] = ["de", "del", "la", "las", "los", "y"];

/// ¿El texto viene de un carnet KOF (frente o reverso)? Excluye el
/// comprobante de carga, que imprime la misma marca.
#[uniffi::export]
pub fn es_carnet_kof(texto: String) -> bool {
    es_kof(&texto)
}

fn es_kof(texto: &str) -> bool {
    !MARCADOR_COMPROBANTE.is_match(texto)
        && (ANTIGUEDAD.is_match(texto)
            || RESPALDO_KOF.is_match(texto)
            || MARCA_FEMSA.is_match(texto)
            || FEMSA_SOLO.is_match(texto))
}

// Esquema de las dos caras reales (2026-09-27):
//
//   Cara con código                      Cara sin código
//   Erick Steven        [foto]           [foto + bandera]
//   Portuguez Chacon                     Bayron Andrey
//   ════════════                         Sanchez Lezcano
//   1819584            <- código         Coca-Cola / FEMSA
//   CENTRAL / DE ALERTA Y RESPUESTA / 800-2256327
//
// Nombre (renglón de nombres + renglón de apellidos) y código (justo
// DEBAJO del nombre). Nombre y código son independientes: frente y reverso
// son escaneos separados; se devuelve lo que haya.
#[uniffi::export]
pub fn extraer_carnet_kof(texto: String) -> Option<CarnetKofDetectado> {
    if !es_kof(&texto) {
        return None;
    }
    let lineas: Vec<&str> = renglones(&texto).into_iter().map(str::trim).collect();
    let (nombre, fin_nombre) = nombre_carnet(&lineas);
    let es_codigo = |l: &&&str| CODIGO_EMPLEADO_RENGLON.is_match(l);
    let codigo = lineas[fin_nombre.min(lineas.len())..]
        .iter()
        .find(es_codigo)
        .or_else(|| lineas.iter().find(es_codigo))
        .map(|l| (*l).to_owned());
    if nombre.is_none() && codigo.is_none() {
        return None;
    }
    Some(CarnetKofDetectado {
        nombre,
        codigo_empleado: codigo,
    })
}

fn es_candidata(linea: &str) -> bool {
    linea.chars().count() >= 4
        && linea.chars().all(|c| c.is_alphabetic() || c == ' ')
        && linea
            .to_uppercase()
            .split(' ')
            .all(|p| !PALABRAS_NO_NOMBRE_KOF.contains(&p))
}

fn es_capitalizada(linea: &str) -> bool {
    es_candidata(linea)
        && linea
            .split(' ')
            .filter(|p| !p.trim().is_empty())
            .all(|p| PALABRA_CAPITALIZADA.is_match(p) || PARTICULAS_NOMBRE.contains(&p))
}

/// En los carnets reales el nombre va en tipo oración: primero se buscan
/// DOS renglones seguidos así (nombres, apellidos). Si no (OCR todo en
/// mayúsculas), los 2 primeros renglones de sólo letras que no sean del
/// diseño. Devuelve el nombre en mayúsculas y el índice del renglón que
/// le sigue (para buscar el código ahí).
fn nombre_carnet(lineas: &[&str]) -> (Option<String>, usize) {
    if let Some(i) = (0..lineas.len().saturating_sub(1))
        .find(|&i| es_capitalizada(lineas[i]) && es_capitalizada(lineas[i + 1]))
    {
        return (
            Some(format!("{} {}", lineas[i], lineas[i + 1]).to_uppercase()),
            i + 2,
        );
    }
    let indices: Vec<usize> = (0..lineas.len())
        .filter(|&i| es_candidata(lineas[i]))
        .take(2)
        .collect();
    let Some(&ultimo) = indices.last() else {
        return (None, 0);
    };
    let nombre = indices
        .iter()
        .map(|&i| lineas[i])
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase();
    ((!nombre.trim().is_empty()).then_some(nombre), ultimo + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombre_y_codigo_debajo_sin_tomar_el_telefono() {
        let texto =
            "Erick Steven\nPortuguez Chacon\n1819584\nCENTRAL\nDE ALERTA Y RESPUESTA\n800-2256327";
        let c = extraer_carnet_kof(texto.to_owned()).unwrap();
        assert_eq!(c.nombre.as_deref(), Some("ERICK STEVEN PORTUGUEZ CHACON"));
        assert_eq!(c.codigo_empleado.as_deref(), Some("1819584"));
    }

    #[test]
    fn el_comprobante_no_es_un_carnet() {
        assert!(!es_carnet_kof(
            "Coca Cola FEMSA\nRuta / No.de Carga: CRR079 / 1".to_owned()
        ));
        assert!(es_carnet_kof(
            "Bayron Andrey\nSanchez Lezcano\nFEMSA".to_owned()
        ));
    }
}
