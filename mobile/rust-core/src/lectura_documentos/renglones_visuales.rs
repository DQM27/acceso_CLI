//! Renglones visuales a partir de la geometría que entrega ML Kit
//! (auditoría OCR 2026-09-28, punto E-3).
//!
//! `Text.text` de ML Kit concatena los bloques en un orden que cambia
//! entre frames: la etiqueta "Nombre:" y su valor pueden llegar en bloques
//! distintos, las columnas del DIMEX pegadas a otra cosa, una línea del
//! MRZ partida en dos bloques. Acá se reconstruye el texto como lo ve una
//! persona: las líneas que están a la MISMA altura forman un renglón,
//! ordenadas de izquierda a derecha, y los renglones de arriba hacia
//! abajo.
//!
//! El documento puede venir inclinado: la inclinación se estima con las
//! palabras de cada línea (la recta que une la primera y la última) y la
//! altura se mide sobre esa recta, no sobre el eje de la imagen.
//!
//! Las palabras con confianza muy baja (reflejos, el borde del estuche,
//! texto de fondo) se descartan en esta versión del texto. El umbral es
//! conservador a propósito: falta calibrarlo con muestras del A25, y el
//! texto original de ML Kit se sigue probando después (ver
//! [`textos_de_frame`]), así que descartar de más nunca deja al lector sin
//! leer algo que antes leía.

/// Una palabra (`Element` de ML Kit) con su caja en píxeles de la imagen
/// analizada y su confianza (0 si el modelo no la informa).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PalabraOcr {
    pub texto: String,
    pub izquierda: f32,
    pub arriba: f32,
    pub derecha: f32,
    pub abajo: f32,
    pub confianza: f32,
}

/// Una línea (`Line` de ML Kit) con su caja y sus palabras, en el orden en
/// que ML Kit las entregó.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct LineaOcr {
    pub texto: String,
    pub izquierda: f32,
    pub arriba: f32,
    pub derecha: f32,
    pub abajo: f32,
    pub palabras: Vec<PalabraOcr>,
}

/// Debajo de esto una palabra se considera ruido. Punto de partida
/// conservador, pendiente de calibrar con muestras reales.
const CONFIANZA_MINIMA: f32 = 0.25;

/// El umbral de [`CONFIANZA_MINIMA`], para que la telemetría de diagnóstico
/// cuente las palabras descartadas con el mismo número.
#[uniffi::export]
pub fn confianza_minima_palabra() -> f32 {
    CONFIANZA_MINIMA
}
/// Dos líneas están en el mismo renglón si sus centros (corregidos por la
/// inclinación) distan menos que esta fracción de su altura.
const FRACCION_ALTURA_MISMO_RENGLON: f32 = 0.5;
/// Inclinación máxima que se corrige (pendiente; ~19°). Más que eso no es
/// un documento sostenido frente a la cámara sino una lectura rara, y
/// corregirla desordenaría el texto.
const PENDIENTE_MAXIMA: f32 = 0.35;

struct LineaUbicada {
    texto: String,
    izquierda: f32,
    centro_corregido: f32,
    altura: f32,
}

fn caja_valida(izquierda: f32, arriba: f32, derecha: f32, abajo: f32) -> bool {
    [izquierda, arriba, derecha, abajo]
        .iter()
        .all(|v| v.is_finite())
        && derecha >= izquierda
        && abajo >= arriba
}

fn mediana(mut valores: Vec<f32>) -> Option<f32> {
    if valores.is_empty() {
        return None;
    }
    valores.sort_by(f32::total_cmp);
    Some(valores[valores.len() / 2])
}

/// Pendiente de la línea según sus palabras (primera y última), o `None` si
/// tiene menos de dos o están en la misma columna.
fn pendiente_de(linea: &LineaOcr) -> Option<f32> {
    let (primera, ultima) = (linea.palabras.first()?, linea.palabras.last()?);
    let centro = |p: &PalabraOcr| {
        (
            f32::midpoint(p.izquierda, p.derecha),
            f32::midpoint(p.arriba, p.abajo),
        )
    };
    let ((x1, y1), (x2, y2)) = (centro(primera), centro(ultima));
    let dx = x2 - x1;
    (linea.palabras.len() >= 2 && dx > f32::EPSILON).then(|| (y2 - y1) / dx)
}

/// Texto de la línea sin las palabras de confianza baja (si el modelo
/// informa confianza). `None` si no queda nada.
fn texto_confiable(linea: &LineaOcr, hay_confianza: bool) -> Option<String> {
    if !hay_confianza
        || linea
            .palabras
            .iter()
            .all(|p| p.confianza >= CONFIANZA_MINIMA)
    {
        return (!linea.texto.trim().is_empty()).then(|| linea.texto.clone());
    }
    let texto = linea
        .palabras
        .iter()
        .filter(|p| p.confianza >= CONFIANZA_MINIMA)
        .map(|p| p.texto.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    (!texto.trim().is_empty()).then_some(texto)
}

/// Texto del frame en renglones visuales. Si alguna caja es inválida
/// (NaN, invertida) no se puede ubicar nada con seguridad y se devuelven
/// las líneas en el orden de ML Kit.
#[uniffi::export]
pub fn reconstruir_texto_visual(lineas: Vec<LineaOcr>) -> String {
    let geometria_valida = lineas.iter().all(|l| {
        caja_valida(l.izquierda, l.arriba, l.derecha, l.abajo)
            && l.palabras
                .iter()
                .all(|p| caja_valida(p.izquierda, p.arriba, p.derecha, p.abajo))
    });
    if !geometria_valida {
        return lineas
            .iter()
            .map(|l| l.texto.as_str())
            .collect::<Vec<_>>()
            .join("\n");
    }
    let hay_confianza = lineas
        .iter()
        .flat_map(|l| &l.palabras)
        .any(|p| p.confianza > 0.0);
    let pendiente = mediana(lineas.iter().filter_map(pendiente_de).collect())
        .unwrap_or(0.0)
        .clamp(-PENDIENTE_MAXIMA, PENDIENTE_MAXIMA);

    let mut ubicadas: Vec<LineaUbicada> = lineas
        .iter()
        .filter_map(|l| {
            let texto = texto_confiable(l, hay_confianza)?;
            let centro_x = f32::midpoint(l.izquierda, l.derecha);
            let centro_y = f32::midpoint(l.arriba, l.abajo);
            // La caja de una línea inclinada es más alta que sus letras: la
            // altura se toma de las palabras.
            let altura = mediana(l.palabras.iter().map(|p| p.abajo - p.arriba).collect())
                .unwrap_or(l.abajo - l.arriba)
                .max(1.0);
            Some(LineaUbicada {
                texto,
                izquierda: l.izquierda,
                centro_corregido: pendiente.mul_add(-centro_x, centro_y),
                altura,
            })
        })
        .collect();
    ubicadas.sort_by(|a, b| a.centro_corregido.total_cmp(&b.centro_corregido));

    let mut renglones: Vec<Vec<LineaUbicada>> = Vec::new();
    for linea in ubicadas {
        let mismo_renglon = renglones.last().is_some_and(|renglon| {
            let n = cantidad_como_f32(renglon.len());
            let centro = renglon.iter().map(|l| l.centro_corregido).sum::<f32>() / n;
            let altura = renglon.iter().map(|l| l.altura).sum::<f32>() / n;
            (linea.centro_corregido - centro).abs()
                <= FRACCION_ALTURA_MISMO_RENGLON * altura.min(linea.altura)
        });
        match renglones.last_mut() {
            Some(renglon) if mismo_renglon => renglon.push(linea),
            _ => renglones.push(vec![linea]),
        }
    }
    renglones
        .into_iter()
        .map(|mut renglon| {
            renglon.sort_by(|a, b| a.izquierda.total_cmp(&b.izquierda));
            renglon
                .into_iter()
                .map(|l| l.texto)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Conteos chicos (líneas de un renglón) a `f32` sin pérdida.
fn cantidad_como_f32(n: usize) -> f32 {
    f32::from(u16::try_from(n).unwrap_or(u16::MAX))
}

/// Las versiones del texto de un frame que prueban los lectores, en orden:
/// renglones visuales y, si difiere, el texto original de ML Kit. Nunca
/// incluye textos vacíos.
#[uniffi::export]
pub fn textos_de_frame(texto_ml_kit: String, lineas: Vec<LineaOcr>) -> Vec<String> {
    let visual = if lineas.is_empty() {
        String::new()
    } else {
        reconstruir_texto_visual(lineas)
    };
    let mut textos = Vec::with_capacity(2);
    if !visual.trim().is_empty() {
        textos.push(visual);
    }
    if !texto_ml_kit.trim().is_empty() && !textos.contains(&texto_ml_kit) {
        textos.push(texto_ml_kit);
    }
    textos
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ancho(texto: &str) -> f32 {
        12.0 * f32::from(u8::try_from(texto.chars().count()).unwrap())
    }

    fn palabra(texto: &str, x: f32, y: f32, confianza: f32) -> PalabraOcr {
        PalabraOcr {
            texto: texto.to_owned(),
            izquierda: x,
            arriba: y,
            derecha: x + ancho(texto),
            abajo: y + 20.0,
            confianza,
        }
    }

    /// Línea con sus palabras a la altura `y` desde `x`, con pendiente `m`
    /// (el centro de cada palabra cae sobre la recta del texto, como en
    /// una foto real).
    fn linea(texto: &str, x: f32, y: f32, m: f32) -> LineaOcr {
        let mut palabras = Vec::new();
        let mut cursor = x;
        for p in texto.split(' ') {
            let centro = cursor + ancho(p) / 2.0;
            palabras.push(palabra(p, cursor, m.mul_add(centro - x, y), 0.9));
            cursor = palabras.last().unwrap().derecha + 10.0;
        }
        let arriba = palabras
            .iter()
            .map(|p| p.arriba)
            .fold(f32::INFINITY, f32::min);
        let abajo = palabras
            .iter()
            .map(|p| p.abajo)
            .fold(f32::NEG_INFINITY, f32::max);
        LineaOcr {
            texto: texto.to_owned(),
            izquierda: x,
            arriba,
            derecha: cursor - 10.0,
            abajo,
            palabras,
        }
    }

    #[test]
    fn etiqueta_y_valor_en_bloques_distintos_quedan_en_un_renglon() {
        // ML Kit entrega primero la columna de etiquetas y después la de
        // valores (cédula azul anterior).
        let lineas = vec![
            linea("Nombre:", 300.0, 100.0, 0.0),
            linea("1° Apellido:", 260.0, 140.0, 0.0),
            linea("JUAN CARLOS", 420.0, 102.0, 0.0),
            linea("GOMEZ", 420.0, 141.0, 0.0),
        ];
        assert_eq!(
            reconstruir_texto_visual(lineas),
            "Nombre: JUAN CARLOS\n1° Apellido: GOMEZ"
        );
    }

    #[test]
    fn una_linea_mrz_partida_se_une_en_su_renglon() {
        let lineas = vec![
            linea("PEREZ<<MARIA<JOSE", 10.0, 300.0, 0.0),
            linea("C<CRI9998887774<<<<<<<<<<<<<<<", 10.0, 220.0, 0.0),
            linea("<<<<<<<<<<<<<", 400.0, 301.0, 0.0),
        ];
        assert_eq!(
            reconstruir_texto_visual(lineas),
            "C<CRI9998887774<<<<<<<<<<<<<<<\nPEREZ<<MARIA<JOSE <<<<<<<<<<<<<"
        );
    }

    #[test]
    fn corrige_la_inclinacion_del_documento() {
        // Pendiente 0,12 (~7°): el valor de la derecha queda más abajo que
        // su etiqueta, casi a la altura del renglón siguiente en el eje de
        // la imagen, pero sobre la recta del texto es el mismo renglón.
        let m = 0.12;
        let lineas = vec![
            linea("Nombre: JUAN CARLOS", 100.0, 100.0, m),
            linea("1° Apellido: GOMEZ", 100.0, 140.0, m),
            linea("Vence: 01/01/2030", 600.0, m.mul_add(500.0, 100.0), m),
        ];
        let texto = reconstruir_texto_visual(lineas);
        assert_eq!(
            texto,
            "Nombre: JUAN CARLOS Vence: 01/01/2030\n1° Apellido: GOMEZ"
        );
    }

    #[test]
    fn descarta_palabras_de_confianza_baja_solo_si_el_modelo_la_informa() {
        let mut l = linea("1 2345 6789 ~,", 10.0, 10.0, 0.0);
        l.palabras[3].confianza = 0.05;
        assert_eq!(reconstruir_texto_visual(vec![l.clone()]), "1 2345 6789");
        for p in &mut l.palabras {
            p.confianza = 0.0;
        }
        assert_eq!(reconstruir_texto_visual(vec![l]), "1 2345 6789 ~,");
    }

    #[test]
    fn con_geometria_invalida_conserva_el_orden_de_ml_kit() {
        let mut l = linea("B", 10.0, 10.0, 0.0);
        l.izquierda = f32::NAN;
        let lineas = vec![linea("A", 10.0, 100.0, 0.0), l];
        assert_eq!(reconstruir_texto_visual(lineas), "A\nB");
    }

    #[test]
    fn textos_de_frame_sin_repetidos_ni_vacios() {
        assert_eq!(
            textos_de_frame("A".into(), vec![linea("A", 0.0, 0.0, 0.0)]),
            vec!["A"]
        );
        assert_eq!(
            textos_de_frame(
                "B\nA".into(),
                vec![linea("B", 0.0, 50.0, 0.0), linea("A", 0.0, 0.0, 0.0)]
            ),
            vec!["A\nB", "B\nA"]
        );
        assert!(textos_de_frame("  ".into(), vec![]).is_empty());
    }
}
