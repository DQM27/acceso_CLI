//! Layout de columnas compartido por las tablas de búsqueda, activos e
//! historial: qué columnas están visibles, cuánto ancho le toca a cada una
//! y cómo concatenar una fila ya resuelta a texto.

use crate::comandos::columnas::Columna;

use super::util::recortar;

pub(super) fn columnas_visibles<C: Columna>(
    columnas: &crate::comandos::columnas::SelectorColumnas<C>,
) -> impl Iterator<Item = C> + '_ {
    columnas.iter().filter(|(_, v)| *v).map(|(c, _)| c)
}

/// Tope al ancho de una columna flexible (Nombre/Empresa/Usuario): sin este
/// tope, en una terminal ancha se comen todo el espacio sobrante y dejan un
/// hueco enorme antes de las columnas fijas — que es lo que se veía "feo".
/// Un nombre de persona o empresa rara vez necesita más que esto; el que se
/// pase igual se trunca con "…", no se pierde información silenciosamente.
const ANCHO_FLEXIBLE_MAXIMO: usize = 28;

/// Ancho de cada columna visible: la fija (`Some`) se conserva tal cual, la
/// flexible (`None`, típicamente Nombre/Empresa) se reparte en partes
/// iguales el espacio que sobra, con tope (`ANCHO_FLEXIBLE_MAXIMO`) — así
/// una tabla con 3 columnas visibles aprovecha el ancho que dejó libre la 4ª
/// que se ocultó con F4, sin desbordarse en una terminal ancha.
pub(super) fn anchos_columnas<C: Columna>(
    ancho_total: u16,
    visibles: impl Iterator<Item = C>,
    ancho_fijo: impl Fn(C) -> Option<usize>,
) -> Vec<(C, usize)> {
    let visibles: Vec<C> = visibles.collect();
    let fijo_total: u16 = visibles
        .iter()
        .filter_map(|c| ancho_fijo(*c))
        .map(|a| a as u16)
        .sum();
    let n_flex = visibles
        .iter()
        .filter(|c| ancho_fijo(**c).is_none())
        .count();
    let flex_ancho = if n_flex == 0 {
        0
    } else {
        let disponible = ancho_total
            .saturating_sub(fijo_total + 2)
            .max(12 * n_flex as u16);
        (disponible / n_flex as u16)
            .max(12)
            .min(ANCHO_FLEXIBLE_MAXIMO as u16) as usize
    };
    visibles
        .into_iter()
        .map(|c| (c, ancho_fijo(c).unwrap_or(flex_ancho)))
        .collect()
}

/// Concatena celdas ya resueltas a `(ancho, columna)`, salvo la última (que
/// crece sin relleno hasta el borde) — como sólo se listan las visibles,
/// "la última" cambia sola según cuál quede más a la derecha. Cada columna
/// reserva 2 espacios de separación (no 1): con columnas numéricas
/// consecutivas (p. ej. Hora/Gafete en `/activos`) un solo espacio las hacía
/// leerse como un número pegado. `derecha` alinea a la derecha las columnas
/// numéricas (Gafete) — separa visualmente sus dígitos de los de la columna
/// anterior en vez de quedar pegados contra el borde izquierdo.
pub(super) fn fila_columnas<C: Columna>(
    anchos: &[(C, usize)],
    derecha: impl Fn(C) -> bool,
    valor: impl Fn(C) -> String,
) -> String {
    let ultimo = anchos.len().saturating_sub(1);
    anchos
        .iter()
        .enumerate()
        .map(|(indice, (columna, ancho))| {
            let texto = valor(*columna);
            if indice == ultimo {
                return texto;
            }
            let contenido = ancho.saturating_sub(2);
            let recortado = recortar(&texto, contenido);
            if derecha(*columna) {
                format!("{recortado:>contenido$}  ")
            } else {
                format!("{recortado:<contenido$}  ")
            }
        })
        .collect()
}
