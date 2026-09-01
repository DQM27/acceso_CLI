//! Layout de columnas compartido por las tablas de búsqueda, activos e
//! historial: qué columnas están visibles, cuánto ancho le toca a cada una
//! y cómo concatenar una fila ya resuelta a texto.

use crate::cli::columnas::Columna;

use super::util::recortar;

pub(super) fn columnas_visibles<C: Columna>(
    columnas: &crate::cli::columnas::SelectorColumnas<C>,
) -> impl Iterator<Item = C> + '_ {
    columnas.iter().filter(|(_, v)| *v).map(|(c, _)| c)
}

/// Ancho de cada columna visible: la fija (`Some`) se conserva tal cual; la
/// flexible (`None`) reparte el espacio que sobra por "water-filling" según
/// el tope propio que le da `maximo_flexible` — no siempre partes iguales.
/// Una columna cuyo tope entra en el reparto igualitario de lo que queda se
/// satisface con ese tope exacto y libera el resto para las que sí lo
/// necesitan (p. ej. Empresa con nombres cortos no le roba ancho a Nombre,
/// que con personas puede ser bastante más largo, ni deja ese sobrante sin
/// usar) — a diferencia del reparto anterior, siempre a partes iguales con
/// un único tope global.
pub(super) fn anchos_columnas<C: Columna>(
    ancho_total: u16,
    visibles: impl Iterator<Item = C>,
    ancho_fijo: impl Fn(C) -> Option<usize>,
    maximo_flexible: impl Fn(C) -> usize,
) -> Vec<(C, usize)> {
    let visibles: Vec<C> = visibles.collect();
    let fijo_total: u16 = visibles
        .iter()
        .filter_map(|c| ancho_fijo(*c))
        .map(|a| u16::try_from(a).unwrap_or(u16::MAX))
        .sum();
    let mut resultado: Vec<(C, usize)> = visibles
        .iter()
        .map(|c| (*c, ancho_fijo(*c).unwrap_or(0)))
        .collect();
    let mut pendientes: Vec<(usize, usize)> = visibles
        .iter()
        .enumerate()
        .filter(|(_, c)| ancho_fijo(**c).is_none())
        .map(|(posicion, c)| (posicion, maximo_flexible(*c)))
        .collect();
    if pendientes.is_empty() {
        return resultado;
    }
    let pendientes_u16 = u16::try_from(pendientes.len()).unwrap_or(u16::MAX);
    let mut restante = ancho_total
        .saturating_sub(fijo_total + 2)
        .max(12 * pendientes_u16) as usize;
    loop {
        if pendientes.is_empty() {
            break;
        }
        let igual = (restante / pendientes.len()).max(12);
        let (satisfechos, siguen): (Vec<_>, Vec<_>) =
            pendientes.into_iter().partition(|(_, tope)| *tope <= igual);
        if satisfechos.is_empty() {
            for (posicion, _) in siguen {
                resultado[posicion].1 = igual;
            }
            break;
        }
        for (posicion, tope) in satisfechos {
            resultado[posicion].1 = tope;
            restante = restante.saturating_sub(tope);
        }
        pendientes = siguen;
    }
    resultado
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ColumnaPrueba {
        Fija,
        Nombre,
        Empresa,
    }

    impl Columna for ColumnaPrueba {
        const TODAS: &'static [Self] = &[Self::Fija, Self::Nombre, Self::Empresa];
        fn etiqueta(self) -> &'static str {
            match self {
                Self::Fija => "Fija",
                Self::Nombre => "Nombre",
                Self::Empresa => "Empresa",
            }
        }
        fn clave(self) -> &'static str {
            match self {
                Self::Fija => "fija",
                Self::Nombre => "nombre",
                Self::Empresa => "empresa",
            }
        }
    }

    fn fijo(columna: ColumnaPrueba) -> Option<usize> {
        match columna {
            ColumnaPrueba::Fija => Some(10),
            _ => None,
        }
    }

    fn maximo(columna: ColumnaPrueba) -> usize {
        match columna {
            ColumnaPrueba::Empresa => 20,
            _ => 60,
        }
    }

    fn ancho_de(anchos: &[(ColumnaPrueba, usize)], columna: ColumnaPrueba) -> usize {
        anchos.iter().find(|(c, _)| *c == columna).unwrap().1
    }

    #[test]
    fn columna_con_tope_bajo_libera_espacio_para_la_de_tope_alto() {
        // 100 de ancho, 10 fijos + 2 de separación = 88 para repartir entre
        // dos flexibles. Con reparto igualitario tocarían 44 y 44: Empresa
        // (tope 20) no necesita tanto y se satisface con su tope exacto,
        // liberando el resto para que Nombre (tope 60) crezca hasta el
        // suyo — ninguna se queda en el 44 de partes iguales.
        let anchos = anchos_columnas(100, ColumnaPrueba::TODAS.iter().copied(), fijo, maximo);
        assert_eq!(ancho_de(&anchos, ColumnaPrueba::Empresa), 20);
        assert_eq!(ancho_de(&anchos, ColumnaPrueba::Nombre), 60);
    }

    #[test]
    fn ambas_flexibles_exceden_el_tope_quedan_en_su_propio_maximo() {
        // Con un ancho total enorme, el reparto igualitario ya supera el
        // tope de las dos en la primera pasada — cada una queda tapada en
        // su propio máximo, no crecen más allá aunque sobre espacio: nadie
        // lo necesita.
        let anchos = anchos_columnas(300, ColumnaPrueba::TODAS.iter().copied(), fijo, maximo);
        assert_eq!(ancho_de(&anchos, ColumnaPrueba::Empresa), 20);
        assert_eq!(ancho_de(&anchos, ColumnaPrueba::Nombre), 60);
    }

    #[test]
    fn poco_espacio_reparte_en_partes_iguales_con_piso_de_doce() {
        // Ancho angosto: ninguna columna cabe en su tope, así que ambas
        // vuelven al reparto por partes iguales (con el piso de 12 de
        // siempre, ver `paginacion_*` — mismo criterio de no desbordar en
        // terminales chicas).
        let anchos = anchos_columnas(20, ColumnaPrueba::TODAS.iter().copied(), fijo, maximo);
        assert_eq!(ancho_de(&anchos, ColumnaPrueba::Nombre), 12);
        assert_eq!(ancho_de(&anchos, ColumnaPrueba::Empresa), 12);
    }

    #[test]
    fn columna_fija_se_conserva_tal_cual() {
        let anchos = anchos_columnas(100, ColumnaPrueba::TODAS.iter().copied(), fijo, maximo);
        assert_eq!(ancho_de(&anchos, ColumnaPrueba::Fija), 10);
    }
}
