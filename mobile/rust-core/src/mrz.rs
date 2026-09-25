//! Parseo, validación por checksum y corrección acotada de MRZ (ICAO 9303),
//! portado de `MrzParser.kt` (móvil) -- ver
//! `docs/auditorias/auditoria-separacion-kotlin-rust-2026-09-25.md`. Vive
//! en `mobile/rust-core`, no en el crate raíz: es lógica exclusiva de la
//! cámara móvil (desktop no escanea documentos), pensada para compartirse
//! con iOS cuando se active -- mismo motivo que el resto de este crate.
//!
//! Sin estado, sin red, sin `AppCore` -- por eso [`leer_mrz`] es una
//! función libre en vez de un método de `Nucleo` (mismo criterio que
//! `CacheTokenDispositivo` viviendo fuera del `Mutex` de `Nucleo`: no hay
//! ninguna razón real para exigir una sesión abierta para parsear texto).
//!
//! División de responsabilidades con Kotlin (`buscarLineasMrz` en
//! `MrzParser.kt`): Kotlin sigue ubicando cuáles líneas del texto crudo de
//! ML Kit tienen forma de MRZ (extracción mecánica) y manda acá sólo esas
//! 2-3 líneas ya aisladas. Acá se decide todo lo demás: formato, parseo,
//! checksum, y corrección de caracteres ambiguos.

const MAX_CORRECCIONES_POR_CAMPO: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FormatoMrz {
    Td1,
    Td3,
}

/// Campo de un MRZ sobre el que se puede intentar una corrección acotada de
/// caracteres ambiguos -- sólo los campos con su propio dígito verificador
/// simple (no el checksum compuesto, que cubre todo el resto de la línea y
/// no es en sí mismo un campo "tipeado" propenso a confundirse letra/dígito
/// carácter por carácter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CampoMrz {
    NumeroDocumento,
    FechaNacimiento,
    FechaVencimiento,
    DatosPersonales,
}

/// Una sustitución de carácter aplicada para que un campo pasara su
/// checksum -- pensado para loguearse a Sentry del lado Kotlin (que ya
/// tiene el SDK inicializado; este crate no agrega una dependencia nueva
/// de Sentry para Rust) y así poder afinar el set de confusables con casos
/// reales.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CorreccionAplicada {
    pub campo: CampoMrz,
    /// Posición del carácter dentro del campo (0-based), no de la línea.
    pub posicion: u8,
    pub caracter_leido: String,
    pub caracter_corregido: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FechaMrz {
    pub dia: u8,
    pub mes: u8,
    pub anio: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RegistroMrz {
    /// `false` cuando las líneas no calzan ni TD1 ni TD3 -- el resto de
    /// los campos quedan vacíos/`false` en ese caso, nunca se inventan.
    pub formato_reconocido: bool,
    pub formato: Option<FormatoMrz>,
    pub codigo_documento: String,
    pub pais_emisor: String,
    pub numero_documento: String,
    pub apellidos: String,
    pub nombres: String,
    pub nacionalidad: String,
    pub fecha_nacimiento: Option<FechaMrz>,
    /// `"M"`, `"F"` o vacío ("<", sin especificar) -- `String` de un
    /// carácter en vez de `char` nativo de Rust para no depender de que
    /// `char` esté entre los tipos escalares que expone esta versión de
    /// `UniFFI` hacia Kotlin (no había precedente de `char` en este crate).
    pub sexo: String,
    pub fecha_vencimiento: Option<FechaMrz>,
    pub checksums_validos: bool,
    /// Mismo significado que en el `MrzParser.kt` original: posición 15
    /// con un dígito (no relleno) y más dígitos en el campo opcional, pero
    /// sin calzar ni el mecanismo estándar de ICAO ni la convención de
    /// Costa Rica verificada -- `numero_documento` sólo trae los primeros
    /// 9 caracteres y no debe usarse como número completo.
    pub numero_documento_extendido_sin_soporte: bool,
    pub correcciones: Vec<CorreccionAplicada>,
}

impl RegistroMrz {
    fn no_reconocido() -> Self {
        Self {
            formato_reconocido: false,
            formato: None,
            codigo_documento: String::new(),
            pais_emisor: String::new(),
            numero_documento: String::new(),
            apellidos: String::new(),
            nombres: String::new(),
            nacionalidad: String::new(),
            fecha_nacimiento: None,
            sexo: String::new(),
            fecha_vencimiento: None,
            checksums_validos: false,
            numero_documento_extendido_sin_soporte: false,
            correcciones: Vec::new(),
        }
    }
}

/// Valor numérico de un carácter de MRZ según ICAO 9303: '<' = 0, dígitos
/// tal cual, letras A-Z = 10-35. `None` para cualquier otro carácter (nunca
/// debería llegar uno, ya que [`leer_mrz`] filtra el alfabeto antes, pero
/// evita un panic en la frontera FFI ante cualquier dato inesperado).
fn valor_caracter_mrz(c: char) -> Option<u32> {
    match c {
        '<' => Some(0),
        '0'..='9' => Some(c as u32 - '0' as u32),
        'A'..='Z' => Some(c as u32 - 'A' as u32 + 10),
        _ => None,
    }
}

/// Dígito verificador ICAO 9303: módulo 10 con pesos 7,3,1 repetidos.
fn digito_verificador_mrz(datos: &str) -> Option<u32> {
    const PESOS: [u32; 3] = [7, 3, 1];
    let mut suma = 0u32;
    for (i, c) in datos.chars().enumerate() {
        suma += valor_caracter_mrz(c)? * PESOS[i % 3];
    }
    Some(suma % 10)
}

fn checksum_valido(datos: &str, esperado: char) -> bool {
    esperado.is_ascii_digit() && digito_verificador_mrz(datos) == Some(esperado as u32 - '0' as u32)
}

/// Confusables ICAO estándar por OCR: `8↔B`, `0↔O`, `1↔I↔L`, `5↔S`, `2↔Z`.
/// Devuelve las alternativas del mismo grupo, sin incluir `c`. Conjunto
/// fijo, no configurable -- ampliarlo es una decisión de producto, no algo
/// que deba variar por llamada FFI.
fn confusables(c: char) -> &'static [char] {
    match c {
        '8' => &['B'],
        'B' => &['8'],
        '0' => &['O'],
        'O' => &['0'],
        '1' => &['I', 'L'],
        'I' => &['1', 'L'],
        'L' => &['1', 'I'],
        '5' => &['S'],
        'S' => &['5'],
        '2' => &['Z'],
        'Z' => &['2'],
        _ => &[],
    }
}

/// Intenta hacer que `datos` pase su propio checksum simple, cambiando como
/// máximo [`MAX_CORRECCIONES_POR_CAMPO`] caracteres por alternativas
/// confusables -- nunca combina con otros campos (cada llamada es
/// independiente, ve sólo `datos`+`check_esperado` de ESTE campo) ni
/// reintenta entre frames (función pura, sin memoria entre llamadas: quien
/// llama de nuevo con el siguiente frame empieza de cero). Si ninguna
/// combinación dentro del límite calza, se rinde y devuelve `datos` tal
/// cual llegó, sin corrección forzada ni inventada.
///
/// Un candidato sólo se acepta si, además de calzar el checksum, el
/// resultado queda compuesto enteramente por dígitos o relleno (`'<'`) --
/// hallazgo real al escribir los tests: el alfabeto MRZ le da un valor
/// numérico válido a CUALQUIER letra A-Z (ICAO 9303), así que sin este
/// filtro un candidato con una letra que quedó SIN corregir (porque no era
/// una de las posiciones elegidas para sustituir) puede calzar el checksum
/// por pura coincidencia aritmética y colarse como "corregido" dejando una
/// letra suelta en un campo que debe ser numérico (número de documento,
/// fechas). Los tres campos donde se llama esto en la práctica
/// (documento/fecha de nacimiento/fecha de vencimiento de cédulas y DIMEX
/// costarricenses, más el campo de datos personales de TD3) son siempre
/// numéricos -- este filtro no descarta ningún caso real, sólo colisiones
/// espurias.
///
/// Acotado por construcción, no por un chequeo de tiempo aparte: el espacio
/// de búsqueda de un campo MRZ (máximo 9 caracteres) con como mucho 2
/// posiciones a la vez y hasta 2 alternativas por posición nunca supera un
/// puñado de combinaciones -- del orden de C(9,2)×2×2 ≈ 140 checksums en el
/// peor caso, microsegundos reales. Sin timeout explícito acá: un timeout
/// sólo tendría sentido si el tamaño de entrada no estuviera acotado por el
/// propio formato MRZ, que sí lo está.
fn es_numerico_o_relleno(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit() || c == '<')
}

fn corregir_campo(
    datos: &str,
    check_esperado: char,
    campo: CampoMrz,
) -> (String, bool, Vec<CorreccionAplicada>) {
    if checksum_valido(datos, check_esperado) {
        return (datos.to_string(), true, Vec::new());
    }

    let caracteres: Vec<char> = datos.chars().collect();
    let posiciones_confusables: Vec<usize> = caracteres
        .iter()
        .enumerate()
        .filter(|(_, c)| !confusables(**c).is_empty())
        .map(|(i, _)| i)
        .collect();

    // 1 corrección -- se juntan TODOS los candidatos que calzan el checksum
    // antes de decidir, no se toma el primero que aparece. Con un checksum
    // módulo 10, más de un candidato calzando al mismo tiempo es una
    // colisión real, no un empate cosmético -- aceptar cualquiera de los
    // dos sería inventar un dato (ej. un número de documento) con la misma
    // confianza que adivinar a ciegas. Ambigüedad = rendirse, igual que
    // "no encontré ninguno".
    let mut candidatos_1: Vec<(String, CorreccionAplicada)> = Vec::new();
    for &i in &posiciones_confusables {
        for &alternativa in confusables(caracteres[i]) {
            let mut intento = caracteres.clone();
            intento[i] = alternativa;
            let intento_str: String = intento.into_iter().collect();
            if checksum_valido(&intento_str, check_esperado) && es_numerico_o_relleno(&intento_str)
            {
                candidatos_1.push((
                    intento_str,
                    correccion(campo, i, caracteres[i], alternativa),
                ));
            }
        }
    }
    if candidatos_1.len() == 1 {
        let (corregido, correccion) = candidatos_1.into_iter().next().unwrap();
        return (corregido, true, vec![correccion]);
    }
    if candidatos_1.len() > 1 || MAX_CORRECCIONES_POR_CAMPO < 2 {
        return (datos.to_string(), false, Vec::new());
    }

    // 2 correcciones, dos posiciones distintas -- mismo criterio de
    // ambigüedad que arriba.
    let mut candidatos_2: Vec<(String, [CorreccionAplicada; 2])> = Vec::new();
    for (idx, &i) in posiciones_confusables.iter().enumerate() {
        for &j in &posiciones_confusables[idx + 1..] {
            for &alt_i in confusables(caracteres[i]) {
                for &alt_j in confusables(caracteres[j]) {
                    let mut intento = caracteres.clone();
                    intento[i] = alt_i;
                    intento[j] = alt_j;
                    let intento_str: String = intento.into_iter().collect();
                    if checksum_valido(&intento_str, check_esperado)
                        && es_numerico_o_relleno(&intento_str)
                    {
                        candidatos_2.push((
                            intento_str,
                            [
                                correccion(campo, i, caracteres[i], alt_i),
                                correccion(campo, j, caracteres[j], alt_j),
                            ],
                        ));
                    }
                }
            }
        }
    }
    if candidatos_2.len() == 1 {
        let (corregido, correcciones) = candidatos_2.into_iter().next().unwrap();
        return (corregido, true, correcciones.into());
    }

    (datos.to_string(), false, Vec::new())
}

fn correccion(
    campo: CampoMrz,
    posicion: usize,
    leido: char,
    corregido: char,
) -> CorreccionAplicada {
    CorreccionAplicada {
        campo,
        // Los campos MRZ nunca superan 15 caracteres -- cabe holgado en u8.
        posicion: u8::try_from(posicion).expect("posición de campo MRZ siempre < 256"),
        caracter_leido: leido.to_string(),
        caracter_corregido: corregido.to_string(),
    }
}

/// El estándar ICAO no fija el siglo -- se decide contra el año actual
/// inyectado (nunca contra una constante que envejezca dentro del binario).
/// Sólo usa el AÑO de "hoy" (igual que la versión Kotlin original, que sólo
/// leía `hoy.year`), no el día/mes.
fn anio_completo(yy: u8, es_nacimiento: bool, anio_actual: i32) -> i32 {
    let yy = i32::from(yy);
    if !es_nacimiento {
        return 2000 + yy;
    }
    if 2000 + yy <= anio_actual {
        2000 + yy
    } else {
        1900 + yy
    }
}

fn parsear_fecha_mrz(yymmdd: &str, es_nacimiento: bool, anio_actual: i32) -> Option<FechaMrz> {
    if yymmdd.len() != 6 || !yymmdd.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let yy: u8 = yymmdd[0..2].parse().ok()?;
    let mm: u8 = yymmdd[2..4].parse().ok()?;
    let dd: u8 = yymmdd[4..6].parse().ok()?;
    let anio = anio_completo(yy, es_nacimiento, anio_actual);
    chrono::NaiveDate::from_ymd_opt(anio, u32::from(mm), u32::from(dd))?;
    Some(FechaMrz {
        dia: dd,
        mes: mm,
        anio,
    })
}

fn separar_nombres(campo_nombres: &str) -> (String, String) {
    let mut partes = campo_nombres.splitn(2, "<<");
    let limpiar = |s: &str| -> String {
        let sin_relleno = s.replace('<', " ");
        sin_relleno.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let apellidos = limpiar(partes.next().unwrap_or(""));
    let nombres = limpiar(partes.next().unwrap_or(""));
    (apellidos, nombres)
}

fn es_alfabeto_mrz(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit() || c == '<'
}

/// Sexo del MRZ ('M'/'F'/'<') como `String` de 0 o 1 carácter -- `<`
/// (sin especificar) se expone como cadena vacía, igual que el resto de
/// los campos opcionales de este `Record` usan vacío/`None` en vez de
/// propagar el carácter de relleno de ICAO hacia Kotlin.
fn sexo_desde_mrz(c: char) -> String {
    if c == 'M' || c == 'F' {
        c.to_string()
    } else {
        String::new()
    }
}

enum ResolucionNumeroTd1 {
    Resuelto {
        numero: String,
        valido: bool,
        correcciones: Vec<CorreccionAplicada>,
    },
    ExtendidoSinSoporte,
}

/// Resuelve el número de documento de un TD1 -- separado de [`parsear_td1`]
/// sólo para no superar el límite de líneas por función del repo (ver
/// `Cargo.toml`, `too_many_lines`), es la misma lógica que antes vivía en
/// línea, sin ningún cambio de comportamiento.
fn resolver_numero_documento_td1(
    bloque_numero: &str,
    check_numero: char,
    pais_emisor: &str,
    opcional1: &str,
) -> ResolucionNumeroTd1 {
    if check_numero == '<' {
        let tras_relleno = opcional1.trim_end_matches('<');
        if tras_relleno.is_empty() {
            // Extensión declarada (posición 15 = '<') pero sin continuación
            // ni check digit legibles -- MRZ incompleto, no un número normal.
            return ResolucionNumeroTd1::ExtendidoSinSoporte;
        }
        let continuacion = tras_relleno[..tras_relleno.len() - 1].to_string();
        let check_extendido = tras_relleno.chars().last().unwrap();
        let numero = format!("{bloque_numero}{continuacion}");
        // Mecanismo "long document number" de ICAO -- el check digit de la
        // extensión cubre bloque+relleno('<')+continuación. Sin corrección
        // acotada acá: el `'<'` de relleno no es un carácter "leído mal",
        // es la señal misma del mecanismo, y el punto de decisión ya está
        // resuelto por `check_numero == '<'` antes de llegar acá.
        let valido = checksum_valido(&format!("{bloque_numero}<{continuacion}"), check_extendido);
        return ResolucionNumeroTd1::Resuelto {
            numero,
            valido,
            correcciones: Vec::new(),
        };
    }

    let continuacion_digitos: String = opcional1.chars().take_while(|&c| c != '<').collect();
    if pais_emisor == "CRI"
        && checksum_valido(bloque_numero, check_numero)
        && !continuacion_digitos.is_empty()
        && continuacion_digitos.chars().all(|c| c.is_ascii_digit())
    {
        // Convención costarricense del DIMEX -- ver doc-comment del
        // `MrzParser.kt` original para el detalle completo.
        let numero = format!("{bloque_numero}{continuacion_digitos}");
        return ResolucionNumeroTd1::Resuelto {
            numero,
            valido: true,
            correcciones: Vec::new(),
        };
    }

    // Dígito (no '<') en la posición 15 con más dígitos en el campo
    // opcional, pero sin calzar ni el mecanismo estándar de ICAO ni la
    // convención de DIMEX verificada -- no se arriesga un algoritmo sin
    // poder confirmarlo.
    let parece_extendido_no_estandar = continuacion_digitos.chars().any(|c| c.is_ascii_digit());
    if parece_extendido_no_estandar {
        return ResolucionNumeroTd1::ExtendidoSinSoporte;
    }

    // Único caso con corrección acotada de confusables: número de documento
    // estándar de 9 caracteres + su check digit simple.
    let (numero, valido, correcciones) =
        corregir_campo(bloque_numero, check_numero, CampoMrz::NumeroDocumento);
    ResolucionNumeroTd1::Resuelto {
        numero,
        valido,
        correcciones,
    }
}

fn parsear_td1(lineas: &[String], anio_actual: i32) -> RegistroMrz {
    let l1: Vec<char> = lineas[0].chars().collect();
    let l2: Vec<char> = lineas[1].chars().collect();
    let l3 = &lineas[2];

    let sub =
        |v: &[char], desde: usize, hasta: usize| -> String { v[desde..hasta].iter().collect() };

    let codigo_documento = sub(&l1, 0, 2);
    let pais_emisor = sub(&l1, 2, 5);
    let bloque_numero = sub(&l1, 5, 14); // 9 caracteres
    let check_numero = l1[14];
    let opcional1 = sub(&l1, 15, 30); // 15 caracteres

    let resolucion =
        resolver_numero_documento_td1(&bloque_numero, check_numero, &pais_emisor, &opcional1);
    let (numero_documento, numero_valido, mut correcciones) = match resolucion {
        ResolucionNumeroTd1::ExtendidoSinSoporte => {
            return RegistroMrz {
                formato_reconocido: true,
                formato: Some(FormatoMrz::Td1),
                codigo_documento,
                pais_emisor,
                numero_documento: bloque_numero,
                numero_documento_extendido_sin_soporte: true,
                ..RegistroMrz::no_reconocido()
            };
        }
        ResolucionNumeroTd1::Resuelto {
            numero,
            valido,
            correcciones,
        } => (numero, valido, correcciones),
    };

    let nacimiento_str = sub(&l2, 0, 6);
    let check_nacimiento = l2[6];
    let sexo = l2[7];
    let vencimiento_str = sub(&l2, 8, 14);
    let check_vencimiento = l2[14];
    let nacionalidad = sub(&l2, 15, 18);
    let opcional2 = sub(&l2, 18, 29);
    let check_compuesto = l2[29];

    let (nacimiento_corregida, nacimiento_valida, mut c) =
        corregir_campo(&nacimiento_str, check_nacimiento, CampoMrz::FechaNacimiento);
    correcciones.append(&mut c);
    let (vencimiento_corregida, vencimiento_valida, mut c) = corregir_campo(
        &vencimiento_str,
        check_vencimiento,
        CampoMrz::FechaVencimiento,
    );
    correcciones.append(&mut c);

    // El checksum compuesto cubre número+check+opcional1+nacimiento+check+
    // vencimiento+check+opcional2 completo -- se valida tal cual llegó
    // (sin corrección propia, ver doc-comment de `CampoMrz`), pero contra
    // los campos YA corregidos arriba, igual que el resto de la validación.
    let compuesto_input = format!(
        "{bloque_numero}{check_numero}{opcional1}{nacimiento_corregida}{check_nacimiento}{vencimiento_corregida}{check_vencimiento}{opcional2}"
    );

    let checksums_validos = numero_valido
        && nacimiento_valida
        && vencimiento_valida
        && checksum_valido(&compuesto_input, check_compuesto);

    let (apellidos, nombres) = separar_nombres(l3);

    RegistroMrz {
        formato_reconocido: true,
        formato: Some(FormatoMrz::Td1),
        codigo_documento,
        pais_emisor,
        numero_documento,
        apellidos,
        nombres,
        nacionalidad,
        fecha_nacimiento: parsear_fecha_mrz(&nacimiento_corregida, true, anio_actual),
        sexo: sexo_desde_mrz(sexo),
        fecha_vencimiento: parsear_fecha_mrz(&vencimiento_corregida, false, anio_actual),
        checksums_validos,
        numero_documento_extendido_sin_soporte: false,
        correcciones,
    }
}

fn parsear_td3(lineas: &[String], anio_actual: i32) -> RegistroMrz {
    let l1: Vec<char> = lineas[0].chars().collect();
    let l2: Vec<char> = lineas[1].chars().collect();
    let sub =
        |v: &[char], desde: usize, hasta: usize| -> String { v[desde..hasta].iter().collect() };

    let codigo_documento = sub(&l1, 0, 2);
    let pais_emisor = sub(&l1, 2, 5);
    let (apellidos, nombres) = separar_nombres(&sub(&l1, 5, l1.len()));

    let bloque_numero = sub(&l2, 0, 9);
    let check_numero = l2[9];
    let nacionalidad = sub(&l2, 10, 13);
    let nacimiento_str = sub(&l2, 13, 19);
    let check_nacimiento = l2[19];
    let sexo = l2[20];
    let vencimiento_str = sub(&l2, 21, 27);
    let check_vencimiento = l2[27];
    let datos_personales = sub(&l2, 28, 42);
    let check_datos_personales = l2[42];
    let check_compuesto = l2[43];

    let mut correcciones = Vec::new();
    let (numero_corregido, numero_valido, mut c) =
        corregir_campo(&bloque_numero, check_numero, CampoMrz::NumeroDocumento);
    correcciones.append(&mut c);
    let (nacimiento_corregida, nacimiento_valida, mut c) =
        corregir_campo(&nacimiento_str, check_nacimiento, CampoMrz::FechaNacimiento);
    correcciones.append(&mut c);
    let (vencimiento_corregida, vencimiento_valida, mut c) = corregir_campo(
        &vencimiento_str,
        check_vencimiento,
        CampoMrz::FechaVencimiento,
    );
    correcciones.append(&mut c);
    let (datos_personales_corregidos, datos_personales_validos, mut c) = corregir_campo(
        &datos_personales,
        check_datos_personales,
        CampoMrz::DatosPersonales,
    );
    correcciones.append(&mut c);

    let compuesto_input = format!(
        "{numero_corregido}{check_numero}{nacimiento_corregida}{check_nacimiento}{vencimiento_corregida}{check_vencimiento}{datos_personales_corregidos}{check_datos_personales}"
    );

    let checksums_validos = numero_valido
        && nacimiento_valida
        && vencimiento_valida
        && datos_personales_validos
        && checksum_valido(&compuesto_input, check_compuesto);

    RegistroMrz {
        formato_reconocido: true,
        formato: Some(FormatoMrz::Td3),
        codigo_documento,
        pais_emisor,
        numero_documento: numero_corregido.trim_end_matches('<').to_string(),
        apellidos,
        nombres,
        nacionalidad,
        fecha_nacimiento: parsear_fecha_mrz(&nacimiento_corregida, true, anio_actual),
        sexo: sexo_desde_mrz(sexo),
        fecha_vencimiento: parsear_fecha_mrz(&vencimiento_corregida, false, anio_actual),
        checksums_validos,
        numero_documento_extendido_sin_soporte: false,
        correcciones,
    }
}

/// Punto de entrada FFI: recibe las 2-3 líneas de MRZ que Kotlin ya aisló
/// con `buscarLineasMrz` (extracción mecánica, se queda en Kotlin) y
/// devuelve el resultado resuelto -- válido/inválido y, si corrigió algo,
/// qué corrigió.
///
/// `lineas.len()` decide el formato (3 → TD1, 2 → TD3) -- Kotlin ya filtró
/// por longitud de línea (30 para TD1, 44 para TD3) y alfabeto antes de
/// aislarlas, pero se revalida acá también (nunca confiar ciegamente en la
/// frontera FFI): cualquier otra cosa, o líneas que no respeten el alfabeto
/// MRZ estricto (A-Z, 0-9, '<'), devuelve `formato_reconocido = false` en
/// vez de entrar en pánico o adivinar.
///
/// `anio_actual` en vez de una fecha completa: es lo único que el
/// desambiguado de siglo de nacimiento necesita (ver `anio_completo`) --
/// Kotlin lo saca de `java.time.LocalDate.now()` al llamar.
#[uniffi::export]
pub fn leer_mrz(lineas: Vec<String>, anio_actual: i32) -> RegistroMrz {
    let normalizadas: Vec<String> = lineas
        .iter()
        .map(|l| l.trim().to_ascii_uppercase().replace(' ', ""))
        .collect();

    let longitud_ok = |lineas: &[String], longitud: usize| -> bool {
        lineas
            .iter()
            .all(|l| l.chars().count() == longitud && l.chars().all(es_alfabeto_mrz))
    };

    if normalizadas.len() == 3 && longitud_ok(&normalizadas, 30) {
        parsear_td1(&normalizadas, anio_actual)
    } else if normalizadas.len() == 2 && longitud_ok(&normalizadas, 44) {
        parsear_td3(&normalizadas, anio_actual)
    } else {
        RegistroMrz::no_reconocido()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digito(datos: &str) -> char {
        char::from_digit(digito_verificador_mrz(datos).unwrap(), 10).unwrap()
    }

    fn td1_valido() -> Vec<String> {
        vec![
            "C<CRI9998887774<<<<<<<<<<<<<<<".to_string(),
            "9001011F3001019NIC<<<<<<<<<<<8".to_string(),
            "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_string(),
        ]
    }

    #[test]
    fn digito_verificador_coincide_con_algoritmo_icao() {
        // "999888777" -> dígito verificador 4 (pesos 7,3,1, A-Z=10-35, '<'=0),
        // mismo caso que el test Kotlin original.
        assert_eq!(digito_verificador_mrz("999888777"), Some(4));
    }

    #[test]
    fn parsea_td1_valido_completo() {
        let r = leer_mrz(td1_valido(), 2026);
        assert!(r.formato_reconocido);
        assert_eq!(r.formato, Some(FormatoMrz::Td1));
        assert_eq!(r.pais_emisor, "CRI");
        assert_eq!(r.numero_documento, "999888777");
        assert_eq!(r.apellidos, "PEREZ");
        assert_eq!(r.nombres, "MARIA JOSE");
        assert_eq!(r.nacionalidad, "NIC");
        assert_eq!(r.sexo, "F");
        assert_eq!(
            r.fecha_nacimiento,
            Some(FechaMrz {
                dia: 1,
                mes: 1,
                anio: 1990
            })
        );
        assert_eq!(
            r.fecha_vencimiento,
            Some(FechaMrz {
                dia: 1,
                mes: 1,
                anio: 2030
            })
        );
        assert!(r.checksums_validos);
        assert!(r.correcciones.is_empty());
    }

    #[test]
    fn rechaza_td1_con_digito_alterado_sin_confusable_posible() {
        // Número de documento armado sólo con dígitos FUERA de todo grupo
        // confusable (3,4,6,7,9 -- ninguno es 0/1/2/5/8/B/I/L/O/S/Z), para
        // que la corrección acotada no tenga ningún candidato que probar
        // y el campo quede simplemente rechazado, sin inventar nada.
        let bloque_numero = "346793467";
        let check_numero = digito(bloque_numero);
        let opcional1 = "<".repeat(15);
        let l1 = format!("C<CRI{bloque_numero}{check_numero}{opcional1}");

        let nacimiento = "900101";
        let check_nacimiento = digito(nacimiento);
        let vencimiento = "300101";
        let check_vencimiento = digito(vencimiento);
        let opcional2 = "<".repeat(11);
        let compuesto = format!(
            "{bloque_numero}{check_numero}{opcional1}{nacimiento}{check_nacimiento}{vencimiento}{check_vencimiento}{opcional2}"
        );
        let check_compuesto = digito(&compuesto);
        let l2 = format!(
            "{nacimiento}{check_nacimiento}F{vencimiento}{check_vencimiento}NIC{opcional2}{check_compuesto}"
        );

        // Altera un dígito del bloque (3->4 en la primera posición) sin
        // tocar ningún carácter confusable de los demás campos.
        let mut l1_corrupto: Vec<char> = l1.chars().collect();
        l1_corrupto[5] = '4';
        let l1_corrupto: String = l1_corrupto.into_iter().collect();

        let r = leer_mrz(
            vec![
                l1_corrupto,
                l2,
                "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_string(),
            ],
            2026,
        );
        assert!(!r.checksums_validos);
        assert!(r.correcciones.is_empty());
    }

    #[test]
    fn devuelve_no_reconocido_si_no_hay_tres_lineas_mrz() {
        let r = leer_mrz(vec!["esto no es un MRZ".to_string()], 2026);
        assert!(!r.formato_reconocido);
    }

    #[test]
    fn siglo_de_nacimiento_depende_del_anio_inyectado_no_de_una_constante() {
        let l1 = "IDCRI1011101119<<<<<<<<<<<<<<<".to_string();
        let nacimiento = "270101";
        let check_nacimiento = digito(nacimiento);
        let vencimiento = "300101";
        let check_vencimiento = digito(vencimiento);
        let opcional2 = "<<<<<<<<<<<";
        let compuesto = format!(
            "{}{}{}{}{}{}",
            &l1[5..],
            nacimiento,
            check_nacimiento,
            vencimiento,
            check_vencimiento,
            opcional2
        );
        let check_compuesto = digito(&compuesto);
        let l2 = format!(
            "{nacimiento}{check_nacimiento}F{vencimiento}{check_vencimiento}CRI{opcional2}{check_compuesto}"
        );
        let mrz = vec![l1, l2, "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_string()];

        assert_eq!(
            leer_mrz(mrz.clone(), 2026).fecha_nacimiento,
            Some(FechaMrz {
                dia: 1,
                mes: 1,
                anio: 1927
            })
        );
        assert_eq!(
            leer_mrz(mrz, 2027).fecha_nacimiento,
            Some(FechaMrz {
                dia: 1,
                mes: 1,
                anio: 2027
            })
        );
    }

    #[test]
    fn fecha_mrz_inexistente_no_se_expone_como_fecha_valida() {
        let l1 = "IDCRI1011101119<<<<<<<<<<<<<<<".to_string();
        let nacimiento = "900231"; // 31 de febrero -- no existe.
        let check_nacimiento = digito(nacimiento);
        let vencimiento = "300101";
        let check_vencimiento = digito(vencimiento);
        let opcional2 = "<<<<<<<<<<<";
        let compuesto = format!(
            "{}{}{}{}{}{}",
            &l1[5..],
            nacimiento,
            check_nacimiento,
            vencimiento,
            check_vencimiento,
            opcional2
        );
        let check_compuesto = digito(&compuesto);
        let l2 = format!(
            "{nacimiento}{check_nacimiento}F{vencimiento}{check_vencimiento}CRI{opcional2}{check_compuesto}"
        );
        let r = leer_mrz(
            vec![l1, l2, "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_string()],
            2026,
        );
        assert!(r.checksums_validos);
        assert_eq!(r.fecha_nacimiento, None);
    }

    #[test]
    fn ignora_espacios_que_el_ocr_puede_insertar_en_la_mrz() {
        let con_ruido = vec![
            "C<CRI 9998887774<<<<<<<<<<<<<<<".to_string(),
            "9001011F3001019NIC<<<<<<<<<<<8".to_string(),
            "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_string(),
        ];
        let r = leer_mrz(con_ruido, 2026);
        assert_eq!(r.numero_documento, "999888777");
        assert!(r.checksums_validos);
    }

    #[test]
    fn resuelve_numero_extendido_de_dimex_costarricense() {
        let td1_dimex = vec![
            "C<CRI1999888772701<<<<<<<<<<<<".to_string(),
            "9001011M3001019NIC<<<<<<<<<<<0".to_string(),
            "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_string(),
        ];
        let r = leer_mrz(td1_dimex, 2026);
        assert!(!r.numero_documento_extendido_sin_soporte);
        assert_eq!(r.numero_documento, "199988877701");
        assert!(r.checksums_validos);
    }

    #[test]
    fn dimex_con_check_digit_del_bloque_base_alterado_no_confirma() {
        let td1_dimex_corrupto = vec![
            "C<CRI1999888773701<<<<<<<<<<<<".to_string(),
            "9001011M3001019NIC<<<<<<<<<<<0".to_string(),
            "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_string(),
        ];
        let r = leer_mrz(td1_dimex_corrupto, 2026);
        assert!(!r.checksums_validos);
    }

    #[test]
    fn digito_en_posicion15_sin_checksum_valido_y_sin_ser_cri_queda_sin_soporte() {
        let td1_otro_pais = vec![
            "C<ARG1999888772701<<<<<<<<<<<<".to_string(),
            "9001011M3001019ARG<<<<<<<<<<<0".to_string(),
            "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_string(),
        ];
        let r = leer_mrz(td1_otro_pais, 2026);
        assert!(r.numero_documento_extendido_sin_soporte);
        assert!(!r.checksums_validos);
    }

    #[test]
    fn parsea_numero_extendido_estandar_icao_con_checksum_valido() {
        // Cédula belga, issue Arg0s1080/mrz#4 -- mismo fixture que el test
        // Kotlin original.
        let td1_extendido_estandar = vec![
            "IDBEL123456789<1233<<<<<<<<<<<".to_string(),
            "9001011F3001019BEL<<<<<<<<<<<8".to_string(),
            "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_string(),
        ];
        let r = leer_mrz(td1_extendido_estandar, 2026);
        assert!(!r.numero_documento_extendido_sin_soporte);
        assert_eq!(r.numero_documento, "123456789123");
        assert!(r.checksums_validos);
    }

    #[test]
    fn rechaza_numero_extendido_estandar_con_check_digit_alterado() {
        let corrupto = vec![
            "IDBEL123456789<1234<<<<<<<<<<<".to_string(),
            "9001011F3001019BEL<<<<<<<<<<<8".to_string(),
            "PEREZ<<MARIA<JOSE<<<<<<<<<<<<<".to_string(),
        ];
        let r = leer_mrz(corrupto, 2026);
        assert!(!r.numero_documento_extendido_sin_soporte);
        assert!(!r.checksums_validos);
    }

    #[test]
    fn parsea_td3_valido_completo() {
        let td3_valido = vec![
            "P<CRIPEREZ<<MARIA<JOSE<<<<<<<<<<<<<<<<<<<<<<".to_string(),
            "A1234567<6CRI9001011F3001019<<<<<<<<<<<<<<04".to_string(),
        ];
        let r = leer_mrz(td3_valido, 2026);
        assert_eq!(r.formato, Some(FormatoMrz::Td3));
        assert_eq!(r.pais_emisor, "CRI");
        assert_eq!(r.numero_documento, "A1234567");
        assert_eq!(r.apellidos, "PEREZ");
        assert_eq!(r.nombres, "MARIA JOSE");
        assert!(r.checksums_validos);
    }

    // --- Corrección acotada de confusables (nuevo respecto al Kotlin) ---

    #[test]
    fn corrige_un_confusable_en_numero_de_documento_dentro_del_limite() {
        // Único carácter confusable del campo es el '0' -- el resto son
        // dígitos fuera de todo grupo confusable (3,4,6,7,9), así que no
        // hay ambigüedad posible: sólo hay un candidato que puede calzar
        // el checksum.
        let base_numero = "346793407";
        let check = digito(base_numero);
        let con_error = base_numero.replacen('0', "O", 1);
        assert_ne!(con_error, base_numero);

        let (corregido, valido, correcciones) =
            corregir_campo(&con_error, check, CampoMrz::NumeroDocumento);
        assert!(valido);
        assert_eq!(corregido, base_numero);
        assert_eq!(correcciones.len(), 1);
        assert_eq!(correcciones[0].caracter_leido, "O");
        assert_eq!(correcciones[0].caracter_corregido, "0");
    }

    #[test]
    fn corrige_dos_confusables_en_el_mismo_campo_dentro_del_limite() {
        // Exactamente dos posiciones confusables en todo el campo (dos
        // '1' iniciales, cada uno leído distinto por el OCR: 'I' y 'L',
        // ambos del mismo grupo confusable de '1'), el resto fuera de
        // cualquier grupo -- combinación única encontrada por búsqueda
        // exhaustiva, sin ambigüedad.
        let base = "116793467";
        let check = digito(base);
        let con_error = "IL6793467";
        assert_ne!(con_error, base);

        let (corregido, valido, correcciones) =
            corregir_campo(con_error, check, CampoMrz::NumeroDocumento);
        assert!(valido);
        assert_eq!(corregido, base);
        assert_eq!(correcciones.len(), 2);
    }

    #[test]
    fn se_rinde_con_tres_confusables_en_el_mismo_campo_y_no_inventa_nada() {
        // Tres caracteres confusables alterados a la vez (tres '1' que el
        // OCR leyó como 'I') -- supera el límite de 2 por campo, así que
        // debe rendirse sin importar que técnicamente exista alguna
        // combinación de 3 que calzaría.
        let base = "314161793";
        let check = digito(base);
        let con_error = "3I4I6I793";
        assert_ne!(con_error, base);

        let (corregido, valido, correcciones) =
            corregir_campo(con_error, check, CampoMrz::NumeroDocumento);
        assert!(!valido);
        assert_eq!(corregido, con_error, "sin corrección forzada al rendirse");
        assert!(correcciones.is_empty());
    }

    #[test]
    fn corrige_campos_distintos_de_forma_independiente_sin_combinarlos() {
        // Número de documento con 1 error propio y fecha de nacimiento con
        // otro error propio, en el mismo MRZ -- deben resolverse cada uno
        // con SU checksum, nunca mezclando datos de un campo para validar
        // el otro.
        let numero_base = "100988577";
        let check_numero = digito(numero_base);
        let numero_con_error = numero_base.replacen('1', "I", 1);

        let nacimiento_base = "900101";
        let check_nacimiento = digito(nacimiento_base);
        let nacimiento_con_error = nacimiento_base.replacen('0', "O", 1);

        let (num_corregido, num_valido, num_corr) =
            corregir_campo(&numero_con_error, check_numero, CampoMrz::NumeroDocumento);
        let (nac_corregida, nac_valida, nac_corr) = corregir_campo(
            &nacimiento_con_error,
            check_nacimiento,
            CampoMrz::FechaNacimiento,
        );

        assert!(num_valido && nac_valida);
        assert_eq!(num_corregido, numero_base);
        assert_eq!(nac_corregida, nacimiento_base);
        assert_eq!(num_corr[0].campo, CampoMrz::NumeroDocumento);
        assert_eq!(nac_corr[0].campo, CampoMrz::FechaNacimiento);
    }

    #[test]
    fn leer_mrz_prueba_td1_antes_que_td3() {
        let r = leer_mrz(td1_valido(), 2026);
        assert_eq!(r.formato, Some(FormatoMrz::Td1));
    }
}
