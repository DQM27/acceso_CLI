//! Qué busca el campo único de Activos: personas (nombre o cédula) o números
//! de gafete. Antes eran dos pestañas, "Persona" y "Gafete", porque la
//! búsqueda de texto libre mezcla cédula con gafete: buscar "7" como gafete
//! también traería cualquier cédula que lo contenga. Ahora lo decide el
//! largo de cada número: un gafete tiene a lo sumo [`MAX_DIGITOS_GAFETE`]
//! dígitos y una cédula al menos nueve, así que el resultado sigue siendo
//! exacto en los dos casos. Lo único que se pierde es buscar una cédula por
//! sus primeros tres dígitos, que de todos modos traería media lista.

/// Gafetes de 1 a 999 (pedido del usuario 2026-10-28: hoy son 25 y no se
/// manejan más de 999).
const MAX_DIGITOS_GAFETE: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum BusquedaActivos {
    /// Campo vacío: se ve la lista de quién está adentro.
    Vacia,
    /// Nombre o cédula, contra el catálogo de contratistas.
    Personas,
    /// Uno o más gafetes separados por coma ("2, 25, 85"), en el orden escrito.
    Gafetes { numeros: Vec<i32> },
}

/// Lee lo escrito en el buscador de Activos. Son gafetes sólo si cada número
/// separado por coma tiene de 1 a [`MAX_DIGITOS_GAFETE`] dígitos; cualquier
/// otra cosa (letras, una cédula, "117 900 735" con espacios) es una persona.
#[uniffi::export]
pub fn interpretar_busqueda_activos(texto: String) -> BusquedaActivos {
    let texto = texto.trim();
    if texto.is_empty() {
        return BusquedaActivos::Vacia;
    }
    let partes: Vec<&str> = texto
        .split(',')
        .map(str::trim)
        .filter(|parte| !parte.is_empty())
        .collect();
    let son_gafetes = !partes.is_empty()
        && partes.iter().all(|parte| {
            parte.len() <= MAX_DIGITOS_GAFETE && parte.bytes().all(|b| b.is_ascii_digit())
        });
    if !son_gafetes {
        return BusquedaActivos::Personas;
    }
    BusquedaActivos::Gafetes {
        numeros: partes
            .iter()
            .filter_map(|parte| parte.parse().ok())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interpretar(texto: &str) -> BusquedaActivos {
        interpretar_busqueda_activos(texto.to_string())
    }

    fn gafetes(numeros: &[i32]) -> BusquedaActivos {
        BusquedaActivos::Gafetes {
            numeros: numeros.to_vec(),
        }
    }

    #[test]
    fn vacio_o_solo_espacios_muestra_la_lista() {
        assert_eq!(interpretar(""), BusquedaActivos::Vacia);
        assert_eq!(interpretar("   "), BusquedaActivos::Vacia);
    }

    #[test]
    fn de_uno_a_tres_digitos_es_un_gafete() {
        assert_eq!(interpretar("7"), gafetes(&[7]));
        assert_eq!(interpretar("25"), gafetes(&[25]));
        assert_eq!(interpretar(" 999 "), gafetes(&[999]));
        assert_eq!(interpretar("007"), gafetes(&[7]));
    }

    #[test]
    fn desde_cuatro_digitos_es_una_cedula() {
        assert_eq!(interpretar("1179"), BusquedaActivos::Personas);
        assert_eq!(interpretar("117900735"), BusquedaActivos::Personas);
    }

    #[test]
    fn varios_gafetes_separados_por_coma_en_el_orden_escrito() {
        assert_eq!(interpretar("2, 25, 85"), gafetes(&[2, 25, 85]));
        assert_eq!(interpretar("85,2"), gafetes(&[85, 2]));
        // El mismo número dos veces se deja: la lista muestra lo escrito.
        assert_eq!(interpretar("2, 2"), gafetes(&[2, 2]));
        // Una coma de más mientras se escribe no cambia el sentido.
        assert_eq!(interpretar("2, 25,"), gafetes(&[2, 25]));
        assert_eq!(interpretar(",3"), gafetes(&[3]));
    }

    #[test]
    fn un_numero_largo_en_la_lista_la_vuelve_busqueda_de_persona() {
        assert_eq!(interpretar("2, 1179"), BusquedaActivos::Personas);
    }

    #[test]
    fn letras_o_separadores_de_cedula_son_una_persona() {
        assert_eq!(interpretar("lid"), BusquedaActivos::Personas);
        assert_eq!(interpretar("Berta 25"), BusquedaActivos::Personas);
        assert_eq!(interpretar("1-1790-0735"), BusquedaActivos::Personas);
        // Con espacios no se parte en gafetes: sólo la coma separa.
        assert_eq!(interpretar("117 900 735"), BusquedaActivos::Personas);
    }

    #[test]
    fn solo_comas_es_una_persona_sin_resultados() {
        assert_eq!(interpretar(","), BusquedaActivos::Personas);
    }
}
