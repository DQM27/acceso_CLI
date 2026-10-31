//! Las últimas operaciones antes de un error: el nombre técnico de cada una
//! (comando del escritorio o llamada al núcleo del móvil), si salió bien y
//! cuándo. Nunca datos.

use std::collections::VecDeque;

use super::formato::Operacion;

/// Cuántas operaciones viajan con cada error.
pub const MAXIMO_OPERACIONES: usize = 20;
const LARGO_MAXIMO_NOMBRE: usize = 60;

/// Un nombre técnico razonable: letras, dígitos y `_ . : -`, recortado. Lo
/// demás se descarta (devuelve `None`): un nombre con otra cosa no es un
/// nombre de operación.
fn nombre_valido(nombre: &str) -> Option<String> {
    let nombre = nombre.trim();
    (!nombre.is_empty()
        && nombre
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-')))
    .then(|| nombre.chars().take(LARGO_MAXIMO_NOMBRE).collect())
}

#[derive(Debug, Default)]
pub struct Rastro {
    operaciones: VecDeque<Operacion>,
}

impl Rastro {
    /// Anota una operación; la más vieja sale si ya hay
    /// [`MAXIMO_OPERACIONES`]. Un nombre inválido no se anota.
    pub fn anotar(&mut self, nombre: &str, ok: Option<bool>, en: String) {
        let Some(nombre) = nombre_valido(nombre) else {
            return;
        };
        if self.operaciones.len() >= MAXIMO_OPERACIONES {
            self.operaciones.pop_front();
        }
        self.operaciones.push_back(Operacion { nombre, ok, en });
    }

    /// Las anotadas, de la más vieja a la más nueva.
    pub fn copia(&self) -> Vec<Operacion> {
        self.operaciones.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{MAXIMO_OPERACIONES, Rastro};

    #[test]
    fn guarda_las_ultimas_veinte_en_orden() {
        let mut rastro = Rastro::default();
        for i in 0..25 {
            rastro.anotar(&format!("op_{i}"), Some(true), format!("t{i}"));
        }
        let copia = rastro.copia();
        assert_eq!(copia.len(), MAXIMO_OPERACIONES);
        assert_eq!(copia[0].nombre, "op_5");
        assert_eq!(copia[19].nombre, "op_24");
    }

    #[test]
    fn un_nombre_con_datos_no_se_anota() {
        let mut rastro = Rastro::default();
        rastro.anotar("buscar 1-0123-0456", None, String::new());
        rastro.anotar("", None, String::new());
        rastro.anotar("registrar_ingreso", Some(false), String::new());
        rastro.anotar("nube::sincronizar", None, String::new());
        let nombres: Vec<_> = rastro.copia().into_iter().map(|o| o.nombre).collect();
        assert_eq!(nombres, ["registrar_ingreso", "nube::sincronizar"]);
    }
}
