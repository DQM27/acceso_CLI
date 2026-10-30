//! Plegado de diacríticos, compartido entre la TUI (comparaciones en
//! memoria, p. ej. el filtro `empresa:`) y la base de datos (función SQL
//! `PLEGAR`, registrada en `database::schema::initialize_database` para las
//! búsquedas `LIKE` de menos de 3 caracteres — ver `docs/hallazgos-buscador.md`).
//! FTS5 ya pliega diacríticos por su cuenta vía `remove_diacritics = 1`; esta
//! función sólo cubre los caminos que no pasan por FTS.
//!
//! Cubre el alfabeto latino con acentos que puede aparecer en nombres reales
//! de esta app (español); no es una normalización Unicode NFD general.
//!
//! `ñ` se pliega a `n` a propósito, igual que el tokenizador FTS5 —
//! confirmado como comportamiento intencional en
//! `docs/hallazgos-buscador.md`, no un descuido de esta función.
pub fn plegar_diacriticos(texto: &str) -> String {
    texto
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'Á' | 'À' | 'Ä' | 'Â' => 'A',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'É' | 'È' | 'Ë' | 'Ê' => 'E',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'Ó' | 'Ò' | 'Ö' | 'Ô' => 'O',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'Ú' | 'Ù' | 'Ü' | 'Û' => 'U',
            'ñ' => 'n',
            'Ñ' => 'N',
            'ç' => 'c',
            'Ç' => 'C',
            otro => otro,
        })
        .collect()
}

/// Minúsculas + diacríticos plegados — el criterio de comparación completo
/// usado por la función SQL `PLEGAR` (equivalente a `COLLATE NOCASE` más
/// tolerancia a tildes/eñes, ninguna de las dos cosas por separado).
pub fn plegar_para_busqueda(texto: &str) -> String {
    plegar_diacriticos(&texto.to_lowercase())
}

/// Letras y números sin tildes, en minúscula y sin espacios ni signos:
/// "Mariscos Jerusalén S.A." → "mariscosjerusalensa".
fn compacto(texto: &str) -> String {
    plegar_para_busqueda(texto)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect()
}

/// Coeficiente de Dice sobre pares de letras seguidas (0 a 1): tolera una
/// letra cambiada, faltante o de más ("procesos" ~ "procesados").
fn dice(a: &str, b: &str) -> f64 {
    let pares = |t: &str| -> Vec<(char, char)> {
        let letras: Vec<char> = t.chars().collect();
        letras.windows(2).map(|par| (par[0], par[1])).collect()
    };
    let (x, mut y) = (pares(a), pares(b));
    if x.is_empty() || y.is_empty() {
        return 0.0;
    }
    let total = x.len() + y.len();
    let mut comunes = 0;
    for par in &x {
        if let Some(posicion) = y.iter().position(|otro| otro == par) {
            y.swap_remove(posicion);
            comunes += 1;
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let similitud = 2.0 * f64::from(comunes) / total as f64;
    similitud
}

/// Consonantes sin repetir seguidas: "fuddy" → "fd", "foodie" → "fd". Agarra
/// los nombres escritos "como suenan" que no comparten pares de letras.
fn esqueleto(palabra: &str) -> String {
    let mut esqueleto = String::new();
    for letra in palabra.chars().filter(|c| !"aeiouy".contains(*c)) {
        if !esqueleto.ends_with(letra) {
            esqueleto.push(letra);
        }
    }
    esqueleto
}

/// Desde qué similitud dos nombres de empresa se consideran el mismo mal
/// escrito. Calibrado con el catálogo real de proveedores de Brisas
/// (2026-10-30, 53 empresas): marca sus 5 duplicados ("FUDDY FACTORY" /
/// "FOODIE FACTORY" 0,52, "PROCESOS FRESCOS" / "PROCESADOS FRESCOS",
/// "TEHE CHIKEN" / "THE CHIKEN", "MARISCO JERUSALEN" / "MARISCOS
/// JERUSALÉN", "LA YEMA DORADA" / "YEMA DORADA") y un solo falso positivo
/// inofensivo ("CAYCHA" / "MAYCA").
pub const UMBRAL_NOMBRE_PARECIDO: f64 = 0.5;

/// Qué tan parecidos son dos nombres completos (0 a 1), sin mirar
/// mayúsculas, tildes, espacios ni signos. Si uno contiene al otro (de 4
/// letras o más, "YEMA DORADA" en "LA YEMA DORADA") cuenta como 1.
pub fn similitud_nombres(a: &str, b: &str) -> f64 {
    let (a, b) = (compacto(a), compacto(b));
    let (corto, largo) = if a.len() <= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    if corto.len() >= 4 && largo.contains(corto.as_str()) {
        return 1.0;
    }
    dice(&a, &b)
}

/// Qué tan bien coincide lo escrito en un buscador con un nombre (0 a 1),
/// palabra por palabra y tolerando errores: cada palabra escrita busca la
/// mejor del nombre (empieza igual, se parece, o suena igual) y se promedia.
/// Las palabras de menos de 3 letras ("LA", "DE") no cuentan, salvo que no
/// haya otras. 0 si nada se parece.
pub fn puntaje_busqueda(texto: &str, nombre: &str) -> f64 {
    let palabras = |t: &str| -> Vec<String> {
        plegar_para_busqueda(t)
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|palabra| !palabra.is_empty())
            .map(str::to_string)
            .collect()
    };
    let escritas = palabras(texto);
    let largas: Vec<&String> = escritas.iter().filter(|p| p.chars().count() >= 3).collect();
    let escritas: Vec<&String> = if largas.is_empty() {
        escritas.iter().collect()
    } else {
        largas
    };
    let del_nombre = palabras(nombre);
    if escritas.is_empty() || del_nombre.is_empty() {
        return 0.0;
    }
    let puntaje_palabra = |escrita: &str| -> f64 {
        del_nombre
            .iter()
            .map(|palabra| {
                if palabra == escrita {
                    1.0
                } else if palabra.starts_with(escrita) {
                    0.9
                } else if dice(escrita, palabra) >= UMBRAL_NOMBRE_PARECIDO {
                    0.7
                } else if escrita.chars().count() >= 4
                    && esqueleto(escrita).chars().count() >= 2
                    && esqueleto(escrita) == esqueleto(palabra)
                {
                    0.6
                } else {
                    0.0
                }
            })
            .fold(0.0, f64::max)
    };
    #[allow(clippy::cast_precision_loss)]
    let promedio = escritas.iter().map(|p| puntaje_palabra(p)).sum::<f64>() / escritas.len() as f64;
    promedio
}

/// Regla de todo nombre de persona o empresa (en mayúscula), del crate de
/// reglas compartido: la usan el núcleo al guardar y las interfaces mientras
/// se escribe.
pub use control_acceso_reglas::nombre::{nombre_en_mayusculas, nombre_mientras_se_escribe};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pliega_vocales_acentuadas_y_dieresis() {
        assert_eq!(
            plegar_diacriticos("Álvarez Ingeniería"),
            "Alvarez Ingenieria"
        );
    }

    #[test]
    fn pliega_ene_a_n_igual_que_fts() {
        assert_eq!(plegar_diacriticos("Niño"), "Nino");
    }

    #[test]
    fn deja_intacto_el_texto_sin_diacriticos() {
        assert_eq!(plegar_diacriticos("Brisas del Oeste"), "Brisas del Oeste");
    }

    #[test]
    fn plegar_para_busqueda_combina_minusculas_y_diacriticos() {
        assert_eq!(plegar_para_busqueda("ÁLVAREZ"), "alvarez");
        assert_eq!(plegar_para_busqueda("Niño"), "nino");
    }

    /// Los duplicados reales del catálogo de proveedores de Brisas
    /// (2026-10-30), cada uno escrito distinto por un operador.
    const DUPLICADOS_REALES: [(&str, &str); 5] = [
        ("FUDDY FACTORY", "FOODIE FACTORY"),
        ("PROCESOS FRESCOS", "PROCESADOS FRESCOS"),
        ("TEHE CHIKEN", "THE CHIKEN"),
        ("MARISCO JERUSALEN", "MARISCOS JERUSALÉN"),
        ("LA YEMA DORADA", "YEMA DORADA"),
    ];

    /// Empresas distintas del mismo catálogo que no deben confundirse.
    const DISTINTAS_REALES: [(&str, &str); 6] = [
        ("BIMBO", "BELCA"),
        ("LA GRAN BODEGA", "LA GRANJITA DEL ABUELO"),
        ("CARNES DORADAS", "YEMA DORADA"),
        ("DOS PINOS", "DEMASA"),
        ("SIGMA", "SYSCO"),
        ("TIQUESO", "TORTIRICAS"),
    ];

    #[test]
    fn los_duplicados_reales_se_reconocen_como_el_mismo_nombre() {
        for (escrito, existente) in DUPLICADOS_REALES {
            let similitud = similitud_nombres(escrito, existente);
            assert!(
                similitud >= UMBRAL_NOMBRE_PARECIDO,
                "{escrito} ~ {existente}: {similitud}"
            );
        }
    }

    #[test]
    fn empresas_distintas_no_se_confunden() {
        for (a, b) in DISTINTAS_REALES {
            let similitud = similitud_nombres(a, b);
            assert!(similitud < UMBRAL_NOMBRE_PARECIDO, "{a} ~ {b}: {similitud}");
        }
    }

    #[test]
    fn la_similitud_ignora_mayusculas_tildes_y_signos() {
        assert!(
            (similitud_nombres("Corporación Decaprim S.A.", "CORPORACION DECAPRIM SA") - 1.0).abs()
                < 1e-9
        );
    }

    #[test]
    fn el_buscador_encuentra_la_empresa_aunque_este_mal_escrita() {
        for (escrito, existente) in DUPLICADOS_REALES {
            let puntaje = puntaje_busqueda(escrito, existente);
            assert!(
                puntaje >= UMBRAL_NOMBRE_PARECIDO,
                "{escrito} → {existente}: {puntaje}"
            );
        }
        // Mientras se escribe, con la palabra a medias.
        assert!(puntaje_busqueda("FUDDY", "FOODIE FACTORY") >= UMBRAL_NOMBRE_PARECIDO);
        assert!(puntaje_busqueda("proces fres", "PROCESADOS FRESCOS") >= UMBRAL_NOMBRE_PARECIDO);
    }

    #[test]
    fn el_buscador_no_trae_empresas_que_no_se_parecen() {
        for (a, b) in DISTINTAS_REALES {
            let puntaje = puntaje_busqueda(a, b);
            assert!(puntaje < UMBRAL_NOMBRE_PARECIDO, "{a} → {b}: {puntaje}");
        }
        assert!(puntaje_busqueda("CARNES", "CAYCHA") < UMBRAL_NOMBRE_PARECIDO);
        assert!(puntaje_busqueda("", "BIMBO").abs() < 1e-9);
    }
}
