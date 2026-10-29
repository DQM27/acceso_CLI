//! Qué busca el campo único de Activos: personas (nombre o cédula) o números
//! de gafete. Antes eran dos pestañas, "Persona" y "Gafete", porque la
//! búsqueda de texto libre mezcla cédula con gafete: buscar "7" como gafete
//! también traería cualquier cédula que lo contenga. Ahora lo decide el
//! largo de cada número: un gafete tiene a lo sumo [`MAX_DIGITOS_GAFETE`]
//! dígitos y una cédula al menos nueve, así que el resultado sigue siendo
//! exacto en los dos casos. Lo único que se pierde es buscar una cédula por
//! sus primeros tres dígitos, que de todos modos traería media lista.

use control_acceso::database::queries::Igualdad;
use control_acceso::database::queries::gafetes::FiltroGafetes;
use control_acceso::models::gafete::{EstadoGafete, TipoGafete};

use crate::{Nucleo, NucleoError, interno};

/// Gafetes de 1 a 999 (pedido del usuario 2026-10-28: hoy son 25 y no se
/// manejan más de 999).
const MAX_DIGITOS_GAFETE: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum BusquedaActivos {
    /// Campo vacío: se ve la lista de quién está adentro.
    Vacia,
    /// Nombre o cédula, contra el catálogo de contratistas.
    Personas,
    /// Uno o más gafetes separados por coma ("2, 25, 85"), en el orden escrito
    /// y sin repetidos.
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
    // Cada gafete una vez: "2, 2" listaba dos veces a la misma persona y la
    // salida en lote la contaba doble (pruebas del usuario 2026-10-29). "02"
    // y "2" son el mismo.
    let mut numeros: Vec<i32> = Vec::new();
    for numero in partes.iter().filter_map(|parte| parte.parse().ok()) {
        if !numeros.contains(&numero) {
            numeros.push(numero);
        }
    }
    BusquedaActivos::Gafetes { numeros }
}

/// Por qué nadie adentro tiene un gafete escrito: "nadie adentro" a secas
/// no distinguía un gafete libre de uno que ni existe (pedido del usuario
/// 2026-10-28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SituacionGafete {
    /// No está en el catálogo de gafetes de contratista: número mal escrito.
    NoExiste,
    /// Existe y está disponible: nadie lo tiene puesto.
    Libre,
    Perdido,
    DeBaja,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GafeteSinPortador {
    pub situacion: SituacionGafete,
    /// "Gafete 130 · no existe", listo para mostrar.
    pub mensaje: String,
}

fn situacion_segun_catalogo(estado: Option<EstadoGafete>) -> SituacionGafete {
    match estado {
        None => SituacionGafete::NoExiste,
        Some(EstadoGafete::Disponible) => SituacionGafete::Libre,
        Some(EstadoGafete::Perdido) => SituacionGafete::Perdido,
        Some(EstadoGafete::DeBaja) => SituacionGafete::DeBaja,
    }
}

fn mensaje_gafete_sin_portador(numero: i32, situacion: SituacionGafete) -> String {
    let estado = match situacion {
        SituacionGafete::NoExiste => "no existe",
        SituacionGafete::Libre => "libre, nadie lo tiene",
        SituacionGafete::Perdido => "reportado perdido",
        SituacionGafete::DeBaja => "dado de baja",
    };
    format!("Gafete {numero} · {estado}")
}

#[uniffi::export]
impl Nucleo {
    /// Para un gafete que nadie adentro tiene puesto: si existe en el
    /// catálogo de contratistas y en qué estado. Sólo local, sin red.
    pub fn gafete_sin_portador(&self, numero: i32) -> Result<GafeteSinPortador, NucleoError> {
        let filtro = FiltroGafetes {
            numero: Some(i64::from(numero)),
            tipo: Some(Igualdad::Incluye(TipoGafete::Contratista)),
            estado: None,
        };
        let encontrados =
            self.core_lock()
                .buscar_gafetes(&filtro)
                .map_err(|error| NucleoError::Interno {
                    mensaje: interno(error),
                })?;
        let situacion = situacion_segun_catalogo(encontrados.first().map(|gafete| gafete.estado));
        Ok(GafeteSinPortador {
            situacion,
            mensaje: mensaje_gafete_sin_portador(numero, situacion),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_situacion_sale_del_estado_en_el_catalogo() {
        assert_eq!(situacion_segun_catalogo(None), SituacionGafete::NoExiste);
        assert_eq!(
            situacion_segun_catalogo(Some(EstadoGafete::Disponible)),
            SituacionGafete::Libre
        );
        assert_eq!(
            situacion_segun_catalogo(Some(EstadoGafete::Perdido)),
            SituacionGafete::Perdido
        );
        assert_eq!(
            situacion_segun_catalogo(Some(EstadoGafete::DeBaja)),
            SituacionGafete::DeBaja
        );
    }

    #[test]
    fn el_mensaje_dice_el_numero_y_por_que_nadie_lo_tiene() {
        assert_eq!(
            mensaje_gafete_sin_portador(130, SituacionGafete::NoExiste),
            "Gafete 130 · no existe"
        );
        assert_eq!(
            mensaje_gafete_sin_portador(11, SituacionGafete::Libre),
            "Gafete 11 · libre, nadie lo tiene"
        );
    }

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
        // El mismo número dos veces cuenta una sola, en su primer lugar.
        assert_eq!(interpretar("2, 2"), gafetes(&[2]));
        assert_eq!(interpretar("2,2,2,2"), gafetes(&[2]));
        assert_eq!(interpretar("5, 2, 5, 02"), gafetes(&[5, 2]));
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
