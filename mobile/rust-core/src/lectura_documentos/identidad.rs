//! Documentos de identidad y carnets: clasificación por palabras clave y
//! punto de entrada de la extracción (los extractores de cada documento
//! viven en `campos_identidad.rs`).
//!
//! Portado de `LectorDocumentosIdentidad.kt` (auditoría OCR 2026-09-28,
//! A-1): mismo comportamiento, mismos casos de prueba.

use std::sync::LazyLock;

use regex::Regex;

use super::campos_identidad::{
    PRAIND_CEDULA, extraer_bac, extraer_cedula_nacional, extraer_dimex, extraer_gafete_contratista,
    extraer_in_house, extraer_licencia, extraer_praind, nombre_in_house_frente,
};
use super::cedula::extraer_cedula;
use super::texto::{FechaOcr, patron};
use crate::mrz::{FechaMrz, FormatoMrz, RegistroMrz};

/// Tipos de documento que el lector sabe clasificar. `Desconocido` es el
/// resultado cuando el texto no calza ninguna señal conocida: la pantalla
/// de escaneo sigue buscando, no es un error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum TipoDocumento {
    CedulaNacional,
    /// Tarjeta de Identidad de Menores: mismo código MRZ que la cédula
    /// (`IDCRI`); se distingue por la edad (ver [`reclasificar_por_edad`]).
    TarjetaIdentidadMenor,
    CedulaResidencia,
    LicenciaNacional,
    LicenciaExtranjero,
    Pasaporte,
    /// Carnet de inducción al sitio (PRAIND): su vencimiento es el
    /// `fecha_vencimiento_praind` del contratista.
    CarnetInduccionPraind,
    CarnetInHouse,
    CarnetBac,
    GafeteContratista,
    Desconocido,
}

/// De dónde salieron los datos de un documento leído.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FuenteDatos {
    /// Texto libre del frente, por etiquetas.
    OcrFrente,
    /// Zona MRZ del reverso, con dígitos verificadores.
    Mrz,
    /// PDF417 del reverso de la cédula anterior (sólo cédula y nombre, ver
    /// `pdf417_cedula.rs`).
    Pdf417,
}

/// Resultado normalizado de leer un documento, sin importar qué extractor
/// lo produjo. Lo que un tipo de documento no trae queda en `None` en vez
/// de forzar un valor.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DocumentoLeido {
    pub tipo: TipoDocumento,
    pub numero_documento: String,
    /// Lo que se busca en el catálogo cuando no hay número (gafete In House
    /// sin cédula: el nombre) o cuando el número no es una cédula (gafete).
    pub texto_busqueda: Option<String>,
    pub nombre: Option<String>,
    pub apellidos: Option<String>,
    pub nacionalidad: Option<String>,
    /// Sólo el carnet PRAIND: el texto tal cual lo imprime; emparejarlo con
    /// una empresa del catálogo lo decide la pantalla.
    pub empresa: Option<String>,
    pub es_extranjero: bool,
    pub vencimiento: Option<FechaOcr>,
    pub fecha_nacimiento: Option<FechaOcr>,
    pub sexo: Option<String>,
    pub fuente_datos: FuenteDatos,
    pub checksum_valido: Option<bool>,
}

impl DocumentoLeido {
    /// Documento del frente con sólo lo obligatorio; el resto en `None`.
    pub fn nuevo(tipo: TipoDocumento, numero_documento: String) -> Self {
        Self {
            tipo,
            numero_documento,
            texto_busqueda: None,
            nombre: None,
            apellidos: None,
            nacionalidad: None,
            empresa: None,
            es_extranjero: false,
            vencimiento: None,
            fecha_nacimiento: None,
            sexo: None,
            fuente_datos: FuenteDatos::OcrFrente,
            checksum_valido: None,
        }
    }
}

/// Nombre para mostrar en el mensaje de la cámara: quien opera nunca elige
/// el tipo a mano, así que esto le confirma qué se detectó.
#[uniffi::export]
pub fn nombre_legible_tipo_documento(tipo: TipoDocumento) -> String {
    match tipo {
        TipoDocumento::CedulaNacional => "Cédula de identidad",
        TipoDocumento::TarjetaIdentidadMenor => "Tarjeta de Identidad de Menores",
        TipoDocumento::CedulaResidencia => "Cédula de residencia (DIMEX)",
        TipoDocumento::LicenciaNacional => "Licencia de conducir",
        TipoDocumento::LicenciaExtranjero => "Licencia de conducir de extranjero",
        TipoDocumento::Pasaporte => "Pasaporte",
        TipoDocumento::CarnetInduccionPraind => "Carnet de inducción (PRAIND)",
        TipoDocumento::CarnetInHouse => "Carnet in-house",
        TipoDocumento::CarnetBac => "Carnet BAC",
        TipoDocumento::GafeteContratista => "Gafete de contratista",
        TipoDocumento::Desconocido => "Documento",
    }
    .to_owned()
}

static LICENCIA_EXTRANJERO: LazyLock<Regex> = LazyLock::new(|| patron(r"N[º9O]?[:.]?\s*DM[- ]"));
pub(super) static LICENCIA_DM_DIRECTO: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)\bDM[- ]?([0-9]{6,15})\b"));
// "CONTRATISTA" solo en su renglón (tolera un signo suelto del OCR).
static RENGLON_CONTRATISTA: LazyLock<Regex> = LazyLock::new(|| patron(r"(?im)^\W*CONTRATISTA\W*$"));
pub(super) static IN_HOUSE_CEDULA: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)C[ÉE]DULA:?\s*\n?\s*([0-9]{6,15})"));
pub(super) static DIGITOS_BAC: LazyLock<Regex> = LazyLock::new(|| patron(r"\b[0-9]{9,15}\b"));
pub(super) static GAFETE_CONTRATISTA: LazyLock<Regex> =
    LazyLock::new(|| patron(r"(?i)\bCRC\s*[-:]?\s*([0-9]{1,4})\b"));

// Reverso de la cédula azul anterior: no trae "Tribunal Supremo de
// Elecciones" escrito (sólo el logo), sí sus etiquetas propias. Nombre del
// padre y de la madre NUNCA son la persona.
const MARCAS_REVERSO_CEDULA_ANTERIOR: [&str; 4] = [
    "NOMBRE DEL PADRE",
    "NOMBRE DE LA MADRE",
    "DOMICILIO ELECTORAL",
    "LUGAR DE NACIMIENTO",
];
pub(super) static NUMERO_REVERSO_CEDULA: LazyLock<Regex> = LazyLock::new(|| {
    patron(r"(?i)N[ÚU]MERO\s*DE\s*C[ÉE]DULA\s*:?\s*([0-9][- ]?[0-9]{4}[- ]?[0-9]{4})")
});

fn contiene_alguna(texto: &str, frases: &[&str]) -> bool {
    frases.iter().any(|f| texto.contains(f))
}

/// Clasifica el tipo de documento a partir del texto, antes de extraer
/// ningún campo. El ORDEN importa: DIMEX y licencia de extranjero comparten
/// el rango del número y sólo el contexto los distingue; el gafete va antes
/// que la DIMEX y el PRAIND antes que la cédula (cada caso explicado abajo).
#[uniffi::export]
pub fn clasificar_tipo_documento(texto: String) -> TipoDocumento {
    clasificar(&texto)
}

pub fn clasificar(texto: &str) -> TipoDocumento {
    let mayus = texto.to_uppercase();
    let licencia = mayus.contains("LICENCIA DE CONDUCIR");
    if licencia && (LICENCIA_EXTRANJERO.is_match(&mayus) || LICENCIA_DM_DIRECTO.is_match(&mayus)) {
        return TipoDocumento::LicenciaExtranjero;
    }
    if licencia {
        return TipoDocumento::LicenciaNacional;
    }
    // Gafete ("CARNÉ PROVISIONAL / CRC - 12 / CONTRATISTAS") ANTES que la
    // DIMEX: la regla de DIMEX incluye "CARNÉ PROVISIONAL" (carné
    // migratorio) y el gafete se leía como DIMEX sin número. "CRC - n" no
    // aparece en ningún documento migratorio.
    if GAFETE_CONTRATISTA.is_match(&mayus)
        && contiene_alguna(&mayus, &["CONTRATISTAS", "PROVISIONAL"])
    {
        return TipoDocumento::GafeteContratista;
    }
    if contiene_alguna(
        &mayus,
        &[
            "DGME",
            "MIGRACIÓN Y EXTRANJERÍA",
            "MIGRACION Y EXTRANJERIA",
            "CÉDULA DE RESIDENCIA",
            "CEDULA DE RESIDENCIA",
            "RESIDENTE PERMANENTE",
            "RESIDENTE TEMPORAL",
            "CARNE PROVISIONAL",
            "CARNÉ PROVISIONAL",
            "PERMISO LABORAL",
        ],
    ) {
        return TipoDocumento::CedulaResidencia;
    }
    // PRAIND antes que la cédula: trae su propio "No. de cédula: 123456789"
    // que, si no, se leería como la cédula misma. El carnet se sostiene
    // vertical y su título puede quedar fuera del recorte; "No. de cédula:"
    // + "inducción" juntos no aparecen en ningún otro documento.
    if contiene_alguna(&mayus, &["CARNET DE INDUCCIÓN", "CARNET DE INDUCCION"])
        || (PRAIND_CEDULA.is_match(texto) && mayus.contains("INDUCCI"))
    {
        return TipoDocumento::CarnetInduccionPraind;
    }
    if mayus.contains("BAC") && DIGITOS_BAC.is_match(&mayus) {
        return TipoDocumento::CarnetBac;
    }
    clasificar_gafetes_y_cedula(texto, &mayus)
}

fn clasificar_gafetes_y_cedula(texto: &str, mayus: &str) -> TipoDocumento {
    let contratista_en_costa_rica = mayus.contains("CONTRATISTA") && mayus.contains("COSTA RICA");
    if contratista_en_costa_rica
        && mayus.contains("CONTRATISTAS")
        && GAFETE_CONTRATISTA.is_match(mayus)
    {
        return TipoDocumento::GafeteContratista;
    }
    if contratista_en_costa_rica
        && (mayus.contains("EMPRESA")
            || IN_HOUSE_CEDULA.is_match(texto)
            || nombre_in_house_frente(texto).is_some())
    {
        return TipoDocumento::CarnetInHouse;
    }
    // "COSTA RICA" va chiquito en la esquina del gafete y es lo primero que
    // se pierde con el estuche: la franja "CONTRATISTA" como renglón propio
    // + un nombre arriba alcanza.
    if RENGLON_CONTRATISTA.is_match(texto) && nombre_in_house_frente(texto).is_some() {
        return TipoDocumento::CarnetInHouse;
    }
    let encabezado_cedula = contiene_alguna(
        mayus,
        &[
            "TRIBUNAL SUPREMO DE ELECCIONES",
            "CÉDULA DE IDENTIDAD",
            "CEDULA DE IDENTIDAD",
        ],
    );
    if (encabezado_cedula && extraer_cedula(texto).is_some())
        || (es_reverso_cedula_anterior(texto) && numero_reverso_cedula_anterior(texto).is_some())
    {
        return TipoDocumento::CedulaNacional;
    }
    TipoDocumento::Desconocido
}

/// Pista para el lector de códigos: el texto parece el reverso de la
/// cédula anterior, la cara que trae el PDF417.
#[uniffi::export]
pub fn parece_reverso_cedula_anterior(texto: String) -> bool {
    es_reverso_cedula_anterior(&texto)
}

pub(super) fn es_reverso_cedula_anterior(texto: &str) -> bool {
    let mayus = texto.to_uppercase();
    let marcas = MARCAS_REVERSO_CEDULA_ANTERIOR
        .iter()
        .filter(|m| mayus.contains(*m))
        .count();
    marcas >= 2 || (NUMERO_REVERSO_CEDULA.is_match(texto) && marcas >= 1)
}

pub(super) fn numero_reverso_cedula_anterior(texto: &str) -> Option<String> {
    NUMERO_REVERSO_CEDULA
        .captures(texto)
        .map(|c| c[1].chars().filter(char::is_ascii_digit).collect())
        .or_else(|| extraer_cedula(texto))
}

/// Clasifica (si `tipo` no viene ya calculado) y extrae. `None` cuando
/// todavía no hay información suficiente: la pantalla sigue esperando
/// frames, no es un fallo.
#[uniffi::export]
pub fn leer_documento_de_texto(
    texto: String,
    tipo: Option<TipoDocumento>,
) -> Option<DocumentoLeido> {
    let tipo = tipo.unwrap_or_else(|| clasificar(&texto));
    leer_documento(&texto, tipo)
}

pub fn leer_documento(texto: &str, tipo: TipoDocumento) -> Option<DocumentoLeido> {
    match tipo {
        TipoDocumento::CedulaNacional => extraer_cedula_nacional(texto),
        TipoDocumento::CedulaResidencia => extraer_dimex(texto),
        TipoDocumento::LicenciaNacional => extraer_licencia(texto, false),
        TipoDocumento::LicenciaExtranjero => extraer_licencia(texto, true),
        TipoDocumento::CarnetInduccionPraind => extraer_praind(texto),
        TipoDocumento::CarnetInHouse => extraer_in_house(texto),
        TipoDocumento::CarnetBac => extraer_bac(texto),
        TipoDocumento::GafeteContratista => extraer_gafete_contratista(texto),
        // El pasaporte llega sólo por MRZ; la TIM sólo por reclasificación
        // por edad después del MRZ; lo desconocido no tiene extractor.
        TipoDocumento::Pasaporte
        | TipoDocumento::TarjetaIdentidadMenor
        | TipoDocumento::Desconocido => None,
    }
}

fn fecha_de_mrz(fecha: FechaMrz) -> FechaOcr {
    FechaOcr {
        dia: fecha.dia,
        mes: fecha.mes,
        anio: fecha.anio,
    }
}

/// Un MRZ ya leído (`leer_mrz`) al modelo normalizado. Distinguir cédula
/// nacional de DIMEX (ambas TD1) es una regla de Costa Rica, no de ICAO:
/// código `ID` para la cédula (Decreto TSE n.° 22-2025) y `C<` para el
/// DIMEX de la DGME (confirmado con un documento real). Un TD1 de otro país
/// o con otro código queda `Desconocido`: sin regla verificada, mejor eso
/// que adivinar.
#[uniffi::export]
pub fn documento_desde_mrz(registro: RegistroMrz) -> DocumentoLeido {
    let tipo = match (
        registro.formato,
        registro.pais_emisor.as_str(),
        registro.codigo_documento.as_str(),
    ) {
        (Some(FormatoMrz::Td3), _, _) => TipoDocumento::Pasaporte,
        (Some(FormatoMrz::Td1), "CRI", "ID") => TipoDocumento::CedulaNacional,
        (Some(FormatoMrz::Td1), "CRI", "C<") => TipoDocumento::CedulaResidencia,
        _ => TipoDocumento::Desconocido,
    };
    let no_vacio = |s: String| (!s.trim().is_empty()).then_some(s);
    DocumentoLeido {
        tipo,
        numero_documento: registro.numero_documento,
        texto_busqueda: None,
        nombre: no_vacio(registro.nombres),
        apellidos: no_vacio(registro.apellidos),
        nacionalidad: no_vacio(registro.nacionalidad),
        empresa: None,
        es_extranjero: false,
        vencimiento: registro.fecha_vencimiento.map(fecha_de_mrz),
        fecha_nacimiento: registro.fecha_nacimiento.map(fecha_de_mrz),
        sexo: registro.sexo.chars().next().map(String::from),
        fuente_datos: FuenteDatos::Mrz,
        checksum_valido: Some(registro.checksums_validos),
    }
}

const EDAD_MAYORIA_DE_EDAD: i32 = 18;

/// La TIM usa el mismo código MRZ que la cédula de adulto: sólo la fecha
/// de nacimiento las distingue. Sin efecto sobre otros tipos ni sin fecha.
#[uniffi::export]
pub fn reclasificar_por_edad(documento: DocumentoLeido, hoy: FechaOcr) -> DocumentoLeido {
    match documento.fecha_nacimiento {
        Some(nacimiento)
            if documento.tipo == TipoDocumento::CedulaNacional
                && nacimiento.edad_en_anios(hoy) < EDAD_MAYORIA_DE_EDAD =>
        {
            DocumentoLeido {
                tipo: TipoDocumento::TarjetaIdentidadMenor,
                ..documento
            }
        }
        _ => documento,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clasifica_cada_documento_por_sus_palabras_clave() {
        let casos = [
            (
                "Licencia de Conducir\nNº: 112340567",
                TipoDocumento::LicenciaNacional,
            ),
            (
                "Licencia de Conducir\nNº: DM-155800000001",
                TipoDocumento::LicenciaExtranjero,
            ),
            (
                "CARNÉ PROVISIONAL\nCRC - 12\nCONTRATISTAS",
                TipoDocumento::GafeteContratista,
            ),
            (
                "DGME\nDocumento No.: 155800000000",
                TipoDocumento::CedulaResidencia,
            ),
            (
                "No. de cédula: 112340567\nFecha de inducción",
                TipoDocumento::CarnetInduccionPraind,
            ),
            ("BAC\n112340567890", TipoDocumento::CarnetBac),
            (
                "TRIBUNAL SUPREMO DE ELECCIONES\n1 2345 6789",
                TipoDocumento::CedulaNacional,
            ),
            ("Teléfono 888888888", TipoDocumento::Desconocido),
        ];
        for (texto, esperado) in casos {
            assert_eq!(clasificar(texto), esperado, "{texto:?}");
        }
    }

    #[test]
    fn el_mrz_de_cedula_y_dimex_se_distingue_por_codigo() {
        let lineas = |primera: &str| {
            vec![
                primera.to_owned(),
                "9001011F3001019NIC<<<<<<<<<<<8".to_owned(),
                "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_owned(),
            ]
        };
        let dimex = documento_desde_mrz(crate::mrz::leer_mrz(
            lineas("C<CRI9998887774<<<<<<<<<<<<<<<"),
            2026,
        ));
        assert_eq!(dimex.tipo, TipoDocumento::CedulaResidencia);
        assert_eq!(dimex.numero_documento, "999888777");
        assert_eq!(dimex.nombre.as_deref(), Some("MARIA JOSE"));
        assert_eq!(dimex.fuente_datos, FuenteDatos::Mrz);
        assert_eq!(dimex.checksum_valido, Some(true));
    }

    #[test]
    fn menor_de_edad_con_cedula_es_tim() {
        let hoy = FechaOcr {
            dia: 1,
            mes: 1,
            anio: 2026,
        };
        let mut documento = DocumentoLeido::nuevo(TipoDocumento::CedulaNacional, "1".to_owned());
        documento.fecha_nacimiento = Some(FechaOcr {
            dia: 2,
            mes: 1,
            anio: 2008,
        });
        assert_eq!(
            reclasificar_por_edad(documento.clone(), hoy).tipo,
            TipoDocumento::TarjetaIdentidadMenor
        );
        documento.fecha_nacimiento = Some(FechaOcr {
            dia: 1,
            mes: 1,
            anio: 2008,
        });
        assert_eq!(
            reclasificar_por_edad(documento, hoy).tipo,
            TipoDocumento::CedulaNacional
        );
    }
}
