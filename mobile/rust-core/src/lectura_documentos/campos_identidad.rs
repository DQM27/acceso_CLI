//! Extractores de campos por tipo de documento de identidad o carnet.
//!
//! Los esquemas de cada documento (con valores ficticios) están junto a su
//! extractor. Regla común: se lee lo MÍNIMO necesario (número, nombre,
//! vencimiento); el resto de las etiquetas sólo sirven de límite para
//! cortar un valor. Cuanto menos se lee, menos riesgo de que un campo
//! contamine a otro del mismo renglón.

use std::sync::LazyLock;

use regex::{Match, Regex};

use super::cedula::extraer_cedula;
use super::identidad::{
    DIGITOS_BAC, DocumentoLeido, GAFETE_CONTRATISTA, IN_HOUSE_CEDULA, LICENCIA_DM_DIRECTO,
    TipoDocumento, es_reverso_cedula_anterior, numero_reverso_cedula_anterior,
};
use super::texto::{
    FechaOcr, en_blanco, es_espacio_java, fecha_de_grupos, largo, patron, primero_maximo, renglones,
};

// ---------------------------------------------------------------------------
// Utilidades de etiquetas
// ---------------------------------------------------------------------------

/// Letras (con tildes) que, pegadas a una etiqueta, indican que en realidad
/// es parte de otra palabra ("GENEROSO", "VENCESLAO").
fn es_letra_etiqueta(c: char) -> bool {
    c.is_ascii_alphabetic() || "ÁÉÍÓÚÑáéíóúñ".contains(c)
}

fn siguiente_no_es_letra(texto: &str, fin: usize) -> bool {
    texto[fin..]
        .chars()
        .next()
        .is_none_or(|c| !es_letra_etiqueta(c))
}

/// Primera coincidencia de `etiqueta` cuyo siguiente carácter no sea una
/// letra (equivale a `ETIQUETA(?![letras])`, que `regex` no soporta).
fn primera_sin_letra_despues<'t>(etiqueta: &Regex, texto: &'t str) -> Option<Match<'t>> {
    etiqueta
        .find_iter(texto)
        .find(|m| siguiente_no_es_letra(texto, m.end()))
}

/// Corta `renglon` en la primera etiqueta vecina (el valor de la columna
/// izquierda llega pegado al campo de la derecha en el mismo renglón).
fn cortar_en_etiqueta<'t>(renglon: &'t str, etiquetas: &Regex) -> &'t str {
    primera_sin_letra_despues(etiquetas, renglon).map_or(renglon, |m| &renglon[..m.start()])
}

/// Fin de una etiqueta seguida de `\s*:?` (espacios de Java, sin Unicode).
fn fin_tras_espacios_y_dos_puntos(texto: &str, desde: usize) -> usize {
    let mut fin = desde;
    for c in texto[desde..].chars() {
        if !es_espacio_java(c) {
            break;
        }
        fin += c.len_utf8();
    }
    if texto[fin..].starts_with(':') {
        fin += 1;
    }
    fin
}

// Una entrada por etiqueta usada, compilada una sola vez.
static FECHA_VENCE: LazyLock<Regex> = LazyLock::new(|| patron_fecha("Vence"));
static FECHA_FECHA_VENCIMIENTO: LazyLock<Regex> =
    LazyLock::new(|| patron_fecha("Fecha Vencimiento"));
static FECHA_VENCIMIENTO: LazyLock<Regex> = LazyLock::new(|| patron_fecha("Vencimiento"));

fn patron_fecha(etiqueta: &str) -> Regex {
    patron(&format!(
        r"(?i){}[:.]?\s*([0-9]{{1,2}})[-/\s]([0-9]{{1,2}})[-/\s]([0-9]{{4}})",
        regex::escape(etiqueta)
    ))
}

fn fecha_tras(patron_fecha: &Regex, texto: &str) -> Option<FechaOcr> {
    let c = patron_fecha.captures(texto)?;
    fecha_de_grupos(&c[1], &c[2], &c[3])
}

// ---------------------------------------------------------------------------
// DIMEX (cédula de residencia, DGME)
// ---------------------------------------------------------------------------
//
//   RESIDENTE PERMANENTE / LIBRE CONDICIÓN        <- categoría (2 renglones)
//   Apellidos:
//   PEREZ MORA
//   Nombre:
//   JUAN CARLOS                    Género: M      <- mismo renglón visual
//   Nacionalidad:
//   NICARAGUA                      F.nac.: 01 01 1990
//   Documento No.: 155800000000    Emitido: 01 01 2023
//   Expediente No.: 135 - 000000   Vence:   01 01 2026
//
// Sólo se extraen nombre, apellidos, "Documento No." (ES el número de la
// cédula de residencia) y "Vence". "Expediente No." es el número de
// trámite de la DGME y nunca es el documento.

const ETIQUETAS_IZQ_DIMEX: &str = "APELLIDOS|NOMBRE|NACIONALIDAD|DOCUMENTO|EXPEDIENTE";
// `\S{0,3}` tolera la tilde de "Género" mal leída ("Genero", "Gnero").
const ETIQUETAS_DER_DIMEX: &str = r"G\S{0,3}NERO|SEXO|F\.?\s*NAC|EMITIDO|VENCE";

static CORTE_ETIQUETA_DIMEX: LazyLock<Regex> = LazyLock::new(|| {
    patron(&format!(
        r"(?i)(?:^|\s)(?:{ETIQUETAS_IZQ_DIMEX}|{ETIQUETAS_DER_DIMEX})"
    ))
});
static EMPIEZA_CON_ETIQUETA_IZQ_DIMEX: LazyLock<Regex> =
    LazyLock::new(|| patron(&format!(r"(?i)^\s*(?:{ETIQUETAS_IZQ_DIMEX})")));
static DIMEX_ETIQUETA_APELLIDOS: LazyLock<Regex> = LazyLock::new(|| patron(r"(?i)APELLIDOS\s*:?"));
static DIMEX_ETIQUETA_NOMBRE: LazyLock<Regex> = LazyLock::new(|| patron(r"(?i)NOMBRE\s*:?"));
static PREFIJO_LETRAS: LazyLock<Regex> = LazyLock::new(|| patron(r"^[A-Za-zÁÉÍÓÚÑáéíóúñ ]+"));
static DIMEX_NUMERO: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)DOCUMENTO\s*NO\.?:?\s*([0-9]{6,15})"));
static DIMEX_NUMERO_PROVISIONAL: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)N[°ºO]?\s*DOCUMENTO\s*:?\s*([0-9]{6,15})"));
static CORRIDA_DIGITOS: LazyLock<Regex> = LazyLock::new(|| patron(r"[0-9]+"));

// Si el valor de Sexo/Género quedó solo en el renglón que sigue a "Nombre:"
// (hallazgo con una DIMEX real: el nombre salía "MASCULINO"), se descarta.
const VALORES_SEXO_DIMEX: [&str; 4] = ["M", "F", "MASCULINO", "FEMENINO"];

/// Valor de texto de un campo del DIMEX: el resto del renglón de la
/// etiqueta o, si ahí no queda nada, los siguientes, hasta la próxima
/// etiqueta de la columna izquierda.
fn valor_texto_dimex(texto: &str, inicio_valor: usize) -> Option<String> {
    for (indice, renglon) in texto[inicio_valor..].split('\n').take(3).enumerate() {
        if indice > 0
            && primera_sin_letra_despues(&EMPIEZA_CON_ETIQUETA_IZQ_DIMEX, renglon).is_some()
        {
            return None;
        }
        let sin_vecino = cortar_en_etiqueta(renglon, &CORTE_ETIQUETA_DIMEX).trim();
        if let Some(valor) = PREFIJO_LETRAS.find(sin_vecino).map(|m| m.as_str().trim())
            && !en_blanco(valor)
            && !VALORES_SEXO_DIMEX.contains(&valor.to_uppercase().as_str())
        {
            return Some(valor.to_owned());
        }
    }
    None
}

/// "Nombre:" sin una letra antes (equivale a `(?<![letras])NOMBRE\s*:?`).
fn inicio_valor_nombre_dimex(texto: &str) -> Option<usize> {
    DIMEX_ETIQUETA_NOMBRE
        .find_iter(texto)
        .find(|m| {
            texto[..m.start()]
                .chars()
                .next_back()
                .is_none_or(|c| !es_letra_etiqueta(c))
        })
        .map(|m| m.end())
}

/// Corrida de dígitos de exactamente 11 o 12 (el número DIMEX suelto,
/// cuando "Documento No.:" y su número quedaron separados).
fn numero_dimex_suelto(renglon: &str) -> Option<&str> {
    CORRIDA_DIGITOS
        .find_iter(renglon)
        .map(|m| m.as_str())
        .find(|d| (11..=12).contains(&d.len()))
}

fn numero_dimex(texto: &str) -> Option<String> {
    DIMEX_NUMERO
        .captures(texto)
        .or_else(|| DIMEX_NUMERO_PROVISIONAL.captures(texto))
        .map(|c| c[1].to_owned())
        .or_else(|| {
            renglones(texto)
                .into_iter()
                .filter(|r| !r.to_uppercase().contains("EXPEDIENTE"))
                .find_map(numero_dimex_suelto)
                .map(str::to_owned)
        })
}

pub fn extraer_dimex(texto: &str) -> Option<DocumentoLeido> {
    let numero = numero_dimex(texto)?;
    let mut documento = DocumentoLeido::nuevo(TipoDocumento::CedulaResidencia, numero);
    documento.nombre = inicio_valor_nombre_dimex(texto).and_then(|i| valor_texto_dimex(texto, i));
    documento.apellidos = DIMEX_ETIQUETA_APELLIDOS
        .find(texto)
        .and_then(|m| valor_texto_dimex(texto, m.end()));
    documento.vencimiento =
        fecha_tras(&FECHA_VENCE, texto).or_else(|| fecha_tras(&FECHA_FECHA_VENCIMIENTO, texto));
    Some(documento)
}

// ---------------------------------------------------------------------------
// Cédula nacional (TSE): frente de los dos diseños y reverso del anterior
// ---------------------------------------------------------------------------
//
//   Nuevo (orquídeas)                   Anterior (azul)
//   1 2345 6789                         1 2345 6789
//   Nombre: JUAN CARLOS                          Nombre: JUAN CARLOS
//   1°Apellido: GOMEZ                        1° Apellido: GOMEZ
//   2°Apellido: VARGAS                       2° Apellido: VARGAS
//   F. Nac:01/01/2000 Vence:01/01/2030             C.C:
//
// Se leen número, nombre, los dos apellidos y "Vence" (sólo el diseño
// nuevo). En el anterior ML Kit a veces devuelve TODAS las etiquetas en un
// bloque y TODOS los valores en otro: si falta algún campo por etiqueta se
// toma el bloque de valores (3 renglones seguidos sólo en MAYÚSCULAS, en
// el orden impreso). Las etiquetas van en minúscula y la firma lleva
// puntos, así que nunca pasan por valor.

const ETIQUETAS_CEDULA: &str = r"NOMBRE|[12][^0-9]{0,4}APELLIDO|C\.?\s*C\b|F\.?\s*NAC|VENCE";
const LETRAS_MAYUS: &str = "A-ZÁÉÍÓÚÑÜ";

static CORTE_ETIQUETA_CEDULA: LazyLock<Regex> =
    LazyLock::new(|| patron(&format!(r"(?i)(?:^|\s)(?:{ETIQUETAS_CEDULA})")));
static EMPIEZA_CON_ETIQUETA_CEDULA: LazyLock<Regex> =
    LazyLock::new(|| patron(&format!(r"(?i)^\s*(?:{ETIQUETAS_CEDULA})")));
static CEDULA_ETIQUETA_NOMBRE: LazyLock<Regex> = LazyLock::new(|| patron(r"(?i)NOMBRE"));
static CEDULA_SIGUE_DE: LazyLock<Regex> = LazyLock::new(|| patron(r"(?i)^\s+DE"));
static CEDULA_ETIQUETA_APELLIDO1: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)1[^0-9]{0,4}APELLIDO\s*:?"));
static CEDULA_ETIQUETA_APELLIDO2: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)2[^0-9]{0,4}APELLIDO\s*:?"));
// Valor impreso: sólo mayúsculas (sensible a mayúsculas a propósito, para
// no tomar nunca "Apellido" ni otra etiqueta como valor).
static VALOR_MAYUS: LazyLock<Regex> =
    LazyLock::new(|| patron(&format!(r"^[{LETRAS_MAYUS}]+(?: [{LETRAS_MAYUS}]+)*")));
static RENGLON_SOLO_MAYUS: LazyLock<Regex> =
    LazyLock::new(|| patron(&format!(r"^[{LETRAS_MAYUS}]+(?: [{LETRAS_MAYUS}]+)*$")));
const PALABRAS_ENCABEZADO_CEDULA: [&str; 10] = [
    "REPÚBLICA",
    "REPUBLICA",
    "COSTA",
    "RICA",
    "TRIBUNAL",
    "SUPREMO",
    "ELECCIONES",
    "CÉDULA",
    "CEDULA",
    "IDENTIDAD",
];

/// "Nombre:" de la persona, nunca "Nombre del Padre" / "de la Madre"
/// (equivale a `(?<![a-zA-Z])NOMBRE(?!\s+DE)\s*:?`).
fn inicio_valor_nombre_cedula(texto: &str) -> Option<usize> {
    CEDULA_ETIQUETA_NOMBRE
        .find_iter(texto)
        .find(|m| {
            texto[..m.start()]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_ascii_alphabetic())
                && !CEDULA_SIGUE_DE.is_match(&texto[m.end()..])
        })
        .map(|m| fin_tras_espacios_y_dos_puntos(texto, m.end()))
}

fn valor_cedula(texto: &str, inicio_valor: Option<usize>) -> Option<String> {
    let inicio = inicio_valor?;
    for (indice, renglon) in texto[inicio..].split('\n').take(2).enumerate() {
        if indice > 0 && EMPIEZA_CON_ETIQUETA_CEDULA.is_match(renglon) {
            return None;
        }
        let sin_vecino = cortar_en_etiqueta(renglon, &CORTE_ETIQUETA_CEDULA).trim();
        if let Some(valor) = VALOR_MAYUS.find(sin_vecino).map(|m| m.as_str().trim())
            && !en_blanco(valor)
        {
            return Some(valor.to_owned());
        }
    }
    None
}

/// Bloque de valores separado de sus etiquetas: primeros 3 renglones
/// SEGUIDOS sólo en mayúsculas que no sean el encabezado de la tarjeta.
fn bloque_de_valores_cedula(texto: &str) -> Option<[String; 3]> {
    let lista: Vec<&str> = renglones(texto).into_iter().map(str::trim).collect();
    let es_valor = |r: &str| {
        RENGLON_SOLO_MAYUS.is_match(r)
            && r.split(' ')
                .all(|p| !PALABRAS_ENCABEZADO_CEDULA.contains(&p))
    };
    lista
        .windows(3)
        .find(|tres| tres.iter().all(|r| es_valor(r)))
        .map(|tres| [tres[0].to_owned(), tres[1].to_owned(), tres[2].to_owned()])
}

type NombreYApellidos = (Option<String>, Option<String>, Option<String>);

fn nombres_cedula_nacional(texto: &str) -> NombreYApellidos {
    let nombre = valor_cedula(texto, inicio_valor_nombre_cedula(texto));
    let apellido1 = valor_cedula(
        texto,
        CEDULA_ETIQUETA_APELLIDO1.find(texto).map(|m| m.end()),
    );
    let apellido2 = valor_cedula(
        texto,
        CEDULA_ETIQUETA_APELLIDO2.find(texto).map(|m| m.end()),
    );
    if nombre.is_some() && apellido1.is_some() && apellido2.is_some() {
        return (nombre, apellido1, apellido2);
    }
    let hay_etiquetas =
        inicio_valor_nombre_cedula(texto).is_some() || CEDULA_ETIQUETA_APELLIDO1.is_match(texto);
    match hay_etiquetas
        .then(|| bloque_de_valores_cedula(texto))
        .flatten()
    {
        Some([n, a1, a2]) => (Some(n), Some(a1), Some(a2)),
        None => (nombre, apellido1, apellido2),
    }
}

// Reverso de la cédula azul anterior (valores ficticios):
//
//        Número de Cédula: 1 2345 6789
//     Fecha de Nacimiento: 01 01 1970
//     Lugar de Nacimiento: SAN JOSE
//         Nombre del Padre: JUAN PEREZ MORA        <- NO es la persona
//       Nombre de la Madre: ANA ROJAS VEGA         <- NO es la persona
//            Vencimiento: 01 01 2030      Sexo:
//   [PDF417]   001234567                            <- control, no es la cédula
//
// Sólo número y "Vencimiento" (que el frente azul NO trae).
fn extraer_reverso_cedula_anterior(texto: &str) -> Option<DocumentoLeido> {
    let mut documento = DocumentoLeido::nuevo(
        TipoDocumento::CedulaNacional,
        numero_reverso_cedula_anterior(texto)?,
    );
    documento.vencimiento = fecha_tras(&FECHA_VENCIMIENTO, texto);
    Some(documento)
}

/// Frente de la cédula (o reverso de la anterior). El número es el único
/// campo obligatorio: nombre y apellidos quedan en `None` si todavía no se
/// leyeron, nunca bloquean la lectura.
pub fn extraer_cedula_nacional(texto: &str) -> Option<DocumentoLeido> {
    if es_reverso_cedula_anterior(texto) {
        return extraer_reverso_cedula_anterior(texto);
    }
    let numero = extraer_cedula(texto)?;
    let (nombre, primer_apellido, segundo_apellido) = nombres_cedula_nacional(texto);
    let apellidos = [primer_apellido, segundo_apellido]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    let mut documento = DocumentoLeido::nuevo(TipoDocumento::CedulaNacional, numero);
    documento.nombre = nombre;
    documento.apellidos = (!en_blanco(&apellidos)).then_some(apellidos);
    // Sólo el diseño nuevo trae "Vence:" en el frente.
    documento.vencimiento = fecha_tras(&FECHA_VENCE, texto);
    Some(documento)
}

// ---------------------------------------------------------------------------
// Licencia de conducir (MOPT)
// ---------------------------------------------------------------------------
//
//   REPUBLICA DE COSTA RICA
//   Licencia de Conducir
//   Nº: DM-155800000001         <- CI-<cédula> (nacional) / DM-<DIMEX> (extranjero)
//   Vencimiento 01-01-2026
//   PEREZ MORA JUAN CARLOS      <- SIN etiqueta: 1er apellido, 2do apellido, nombre(s)
//   01/01/2023 11:54 PR-C151 N0940000 830 BCR GOB DIGITAL   <- pie de impresión
//
// El símbolo de "Nº" es OBLIGATORIO en la etiqueta: sin él, el "N0940950"
// del pie pasaba por número de licencia.

static LICENCIA_NUMERO: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)N\s*[º°9O][:.]?\s*(?:DM|CI)?[- ]?([0-9](?:-?[0-9]){5,14})"));
// Respaldo cuando el OCR pierde el "Nº": "CI-" + la cédula de 9 dígitos.
static LICENCIA_CI_DIRECTO: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)\bCI[- ]?([0-9]{9})\b"));
static LICENCIA_NOMBRE_COMPLETO: LazyLock<Regex> =
    LazyLock::new(|| patron(r"^[A-ZÁÉÍÓÚÑ]+(?:[ \t]+[A-ZÁÉÍÓÚÑ]+){2,}$"));
// Palabras impresas en mayúsculas en el resto del diseño (encabezado,
// sello del MOPT, pie de impresión): una línea con alguna no es el nombre.
const PALABRAS_NO_NOMBRE_LICENCIA: [&str; 22] = [
    "REPUBLICA",
    "REPÚBLICA",
    "COSTA",
    "RICA",
    "LICENCIA",
    "CONDUCIR",
    "EXPEDICION",
    "EXPEDICIÓN",
    "NACIMIENTO",
    "VENCIMIENTO",
    "TIPO",
    "DONADOR",
    "DIRECCION",
    "DIRECCIÓN",
    "GENERAL",
    "EDUCACION",
    "EDUCACIÓN",
    "VIAL",
    "MOPT",
    "BCR",
    "GOB",
    "DIGITAL",
];
// Partículas que forman parte de un apellido compuesto cuando lo
// anteceden ("DE LA O", "DEL VALLE").
const PARTICULAS_APELLIDO: [&str; 8] = ["DE", "DEL", "LA", "LAS", "LOS", "Y", "SAN", "SANTA"];

pub fn extraer_licencia(texto: &str, es_extranjero: bool) -> Option<DocumentoLeido> {
    let numero = LICENCIA_NUMERO
        .captures(texto)
        .map(|c| c[1].chars().filter(char::is_ascii_digit).collect())
        .or_else(|| LICENCIA_DM_DIRECTO.captures(texto).map(|c| c[1].to_owned()))
        .or_else(|| LICENCIA_CI_DIRECTO.captures(texto).map(|c| c[1].to_owned()))?;
    let tipo = if es_extranjero {
        TipoDocumento::LicenciaExtranjero
    } else {
        TipoDocumento::LicenciaNacional
    };
    let mut documento = DocumentoLeido::nuevo(tipo, numero);
    documento.es_extranjero = es_extranjero;
    documento.vencimiento = fecha_tras(&FECHA_VENCIMIENTO, texto);
    if let Some((nombre, apellidos)) = nombre_completo_licencia(texto) {
        documento.nombre = Some(nombre);
        documento.apellidos = Some(apellidos);
    }
    Some(documento)
}

fn palabras_no_blancas(linea: &str) -> Vec<&str> {
    linea.split(' ').filter(|p| !en_blanco(p)).collect()
}

/// La línea del nombre no tiene etiqueta: de las candidatas (sólo
/// mayúsculas, 3+ palabras, sin palabras del diseño) se toma la de MÁS
/// palabras. Orden legal: 1er apellido, 2do apellido, nombre(s). Devuelve
/// (nombre, apellidos).
fn nombre_completo_licencia(texto: &str) -> Option<(String, String)> {
    let candidatas = renglones(texto)
        .into_iter()
        .map(|l| l.trim().to_uppercase())
        .filter(|l| {
            LICENCIA_NOMBRE_COMPLETO.is_match(l)
                && l.split(' ')
                    .all(|p| !PALABRAS_NO_NOMBRE_LICENCIA.contains(&p))
        });
    let elegida = primero_maximo(candidatas, |l| palabras_no_blancas(l).len())?;
    let palabras = palabras_no_blancas(&elegida);
    (palabras.len() >= 3).then(|| partir_apellidos_primero(&palabras))
}

/// "DE LA O CASTRO ANA" -> ("ANA", "DE LA O CASTRO"). Si las partículas
/// dejarían sin nombre, cae al corte simple (2 apellidos + resto).
fn partir_apellidos_primero(palabras: &[&str]) -> (String, String) {
    let fin_de_apellido = |desde: usize| {
        let mut i = desde;
        while i + 1 < palabras.len() && PARTICULAS_APELLIDO.contains(&palabras[i]) {
            i += 1;
        }
        i + 1
    };
    let fin_apellidos = fin_de_apellido(fin_de_apellido(0));
    let corte = if fin_apellidos < palabras.len() {
        fin_apellidos
    } else {
        2
    };
    (palabras[corte..].join(" "), palabras[..corte].join(" "))
}

// ---------------------------------------------------------------------------
// Carnet de inducción (PRAIND)
// ---------------------------------------------------------------------------
//
// Identifica a la persona por cédula; lo que de verdad importa es el
// vencimiento, que corresponde 1:1 a `fecha_vencimiento_praind`. Dos
// diseños reales, mismas etiquetas. "Fecha de inducción" nunca calza dentro
// de "Fecha de vencimiento de inducción" ni al revés (frases literales).

pub(super) static PRAIND_CEDULA: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)No\.?\s*de\s*c[ée]dula:?\s*([0-9]{6,15})"));
static PRAIND_NOMBRE: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)Nombre:?[ \t]*\n?[ \t]*([^\n]+)"));
static PRAIND_EMPRESA: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)Empresa:?[ \t]*\n?[ \t]*([^\n]+)"));
static PRAIND_FECHA_VENCIMIENTO: LazyLock<Regex> = LazyLock::new(|| {
    patron(
        r"(?i)Fecha\s+de\s+vencimiento\s+de\s+inducci[oó]n:?\s*([0-9]{1,2})[/-]([0-9]{1,2})[/-]([0-9]{4})",
    )
});

pub fn extraer_praind(texto: &str) -> Option<DocumentoLeido> {
    let numero = PRAIND_CEDULA.captures(texto)?[1].to_owned();
    let mut documento = DocumentoLeido::nuevo(TipoDocumento::CarnetInduccionPraind, numero);
    documento.nombre = PRAIND_NOMBRE
        .captures(texto)
        .map(|c| c[1].trim().to_owned());
    documento.empresa = PRAIND_EMPRESA
        .captures(texto)
        .map(|c| c[1].trim().to_owned());
    documento.vencimiento = PRAIND_FECHA_VENCIMIENTO
        .captures(texto)
        .and_then(|c| fecha_de_grupos(&c[1], &c[2], &c[3]));
    Some(documento)
}

// ---------------------------------------------------------------------------
// Gafetes y carnets de empresa
// ---------------------------------------------------------------------------

pub fn extraer_in_house(texto: &str) -> Option<DocumentoLeido> {
    if let Some(c) = IN_HOUSE_CEDULA.captures(texto) {
        return Some(DocumentoLeido::nuevo(
            TipoDocumento::CarnetInHouse,
            c[1].to_owned(),
        ));
    }
    let nombre = nombre_in_house_frente(texto)?;
    let mut documento = DocumentoLeido::nuevo(TipoDocumento::CarnetInHouse, nombre.clone());
    documento.texto_busqueda = Some(nombre.clone());
    documento.nombre = Some(nombre);
    Some(documento)
}

/// Nombre del gafete In House: los (hasta) 2 renglones de sólo letras justo
/// arriba de la franja "CONTRATISTA". Sólo letras y espacios de 3+
/// caracteres: el reflejo del estuche genera renglones basura ("l.", "~ ,").
pub fn nombre_in_house_frente(texto: &str) -> Option<String> {
    let lineas: Vec<&str> = renglones(texto)
        .into_iter()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let indice_contratista = lineas
        .iter()
        .position(|l| l.to_uppercase().contains("CONTRATISTA"))?;
    if indice_contratista == 0 {
        return None;
    }
    let candidatas: Vec<&str> = lineas[..indice_contratista]
        .iter()
        .copied()
        .filter(|l| {
            let mayus = l.to_uppercase();
            largo(l) >= 3
                && l.chars().all(|c| c.is_alphabetic() || c == ' ')
                && !["EMPRESA", "CÉDULA", "CEDULA", "COSTA RICA"]
                    .iter()
                    .any(|p| mayus.contains(p))
        })
        .collect();
    let ultimas = &candidatas[candidatas.len().saturating_sub(2)..];
    let nombre = ultimas
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (largo(&nombre) >= 6).then_some(nombre)
}

pub fn extraer_bac(texto: &str) -> Option<DocumentoLeido> {
    let numero = DIGITOS_BAC.find(texto)?.as_str().to_owned();
    let mut documento = DocumentoLeido::nuevo(TipoDocumento::CarnetBac, numero);
    documento.nombre = renglones(texto)
        .into_iter()
        .map(str::trim)
        .find(|l| {
            largo(l) >= 6 && l.chars().any(char::is_alphabetic) && !l.to_uppercase().contains("BAC")
        })
        .map(str::to_owned);
    Some(documento)
}

pub fn extraer_gafete_contratista(texto: &str) -> Option<DocumentoLeido> {
    let numero = GAFETE_CONTRATISTA.captures(texto)?[1].to_owned();
    let mut documento = DocumentoLeido::nuevo(TipoDocumento::GafeteContratista, numero.clone());
    documento.texto_busqueda = Some(numero);
    Some(documento)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimex_toma_documento_no_y_corta_la_columna_derecha() {
        let texto = "RESIDENTE PERMANENTE\nApellidos:\nPEREZ MORA\nNombre:\nJUAN CARLOS Género: M\n\
                     Documento No.: 155800000000 Emitido: 01 01 2023\nExpediente No.: 135 - 000000 Vence: 01 01 2026";
        let d = extraer_dimex(texto).unwrap();
        assert_eq!(d.numero_documento, "155800000000");
        assert_eq!(d.nombre.as_deref(), Some("JUAN CARLOS"));
        assert_eq!(d.apellidos.as_deref(), Some("PEREZ MORA"));
        assert_eq!(
            d.vencimiento,
            Some(FechaOcr {
                dia: 1,
                mes: 1,
                anio: 2026
            })
        );
    }

    #[test]
    fn dimex_descarta_el_sexo_como_nombre_y_no_corta_nombres_parecidos() {
        let texto = "Nombre:\nMASCULINO\nGENEROSO\nDocumento No.: 155800000000";
        assert_eq!(
            extraer_dimex(texto).unwrap().nombre.as_deref(),
            Some("GENEROSO")
        );
    }

    #[test]
    fn cedula_por_etiquetas_y_por_bloque_de_valores() {
        let por_etiquetas = "1 2345 6789\nNombre: JUAN CARLOS\n1°Apellido: GOMEZ\n2°Apellido: VARGAS\nF. Nac:01/01/2000 Vence:01/01/2030";
        let d = extraer_cedula_nacional(por_etiquetas).unwrap();
        assert_eq!(d.numero_documento, "123456789");
        assert_eq!(d.nombre.as_deref(), Some("JUAN CARLOS"));
        assert_eq!(d.apellidos.as_deref(), Some("GOMEZ VARGAS"));
        assert_eq!(
            d.vencimiento,
            Some(FechaOcr {
                dia: 1,
                mes: 1,
                anio: 2030
            })
        );

        let por_bloque =
            "1 2345 6789\nNombre:\n1° Apellido:\n2° Apellido:\nC.C:\nJUAN CARLOS\nGOMEZ\nVARGAS";
        let d = extraer_cedula_nacional(por_bloque).unwrap();
        assert_eq!(d.nombre.as_deref(), Some("JUAN CARLOS"));
        assert_eq!(d.apellidos.as_deref(), Some("GOMEZ VARGAS"));
    }

    #[test]
    fn reverso_anterior_nunca_toma_padre_ni_madre() {
        let texto = "Número de Cédula: 1 2345 6789\nNombre del Padre: JUAN PEREZ MORA\n\
                     Nombre de la Madre: ANA ROJAS VEGA\nVencimiento: 01 01 2030";
        let d = extraer_cedula_nacional(texto).unwrap();
        assert_eq!(d.numero_documento, "123456789");
        assert_eq!(d.nombre, None);
        assert_eq!(
            d.vencimiento,
            Some(FechaOcr {
                dia: 1,
                mes: 1,
                anio: 2030
            })
        );
    }

    #[test]
    fn licencia_con_nombre_sin_etiqueta_y_particulas() {
        let texto = "REPUBLICA DE COSTA RICA\nLicencia de Conducir\nNº: CI-1-1234-0567\nVencimiento 01-01-2026\nDE LA O CASTRO ANA";
        let d = extraer_licencia(texto, false).unwrap();
        assert_eq!(d.numero_documento, "112340567");
        assert_eq!(d.nombre.as_deref(), Some("ANA"));
        assert_eq!(d.apellidos.as_deref(), Some("DE LA O CASTRO"));
    }

    #[test]
    fn praind_lee_cedula_nombre_empresa_y_vencimiento() {
        let texto = "CARNET DE INDUCCIÓN\nNombre: Juan Perez\nNo. de cédula: 112340567\nEmpresa: ACME S.A.\n\
                     Fecha de inducción: 01/01/2025\nFecha de vencimiento de\ninducción: 01/01/2026";
        let d = extraer_praind(texto).unwrap();
        assert_eq!(d.numero_documento, "112340567");
        assert_eq!(d.nombre.as_deref(), Some("Juan Perez"));
        assert_eq!(d.empresa.as_deref(), Some("ACME S.A."));
        assert_eq!(
            d.vencimiento,
            Some(FechaOcr {
                dia: 1,
                mes: 1,
                anio: 2026
            })
        );
    }

    #[test]
    fn in_house_sin_cedula_busca_por_nombre() {
        let texto = "l.\nMARIA\nFERNANDEZ\nCONTRATISTA";
        let d = extraer_in_house(texto).unwrap();
        assert_eq!(d.numero_documento, "MARIA FERNANDEZ");
        assert_eq!(d.texto_busqueda.as_deref(), Some("MARIA FERNANDEZ"));
    }

    #[test]
    fn gafete_de_contratista_por_codigo_crc() {
        let d = extraer_gafete_contratista("CRC - 12").unwrap();
        assert_eq!(d.numero_documento, "12");
        assert_eq!(d.texto_busqueda.as_deref(), Some("12"));
    }
}
