//! Modelo puro del modo enclavado de salida por gafete (`/gafete`, alias
//! `/g`) — DEC-057. Pensado para el caso más frecuente de la portería:
//! alguien deja el gafete y se va, y no siempre se sabe el nombre para
//! buscarlo. A diferencia de `/salida` (texto libre O gafete, puede
//! devolver varias coincidencias), acá sólo hay dígitos — uno o varios,
//! separados por coma (`2, 25, 85`) para el caso de un grupo que sale
//! junto — y el gafete es único entre los ingresos activos, así que Enter
//! confirma directo, sin una segunda pantalla de "¿está seguro?" que sólo
//! agregaría fricción al paso que más se repite en toda la operación
//! (§2.8 "¿realmente hace falta?"). La vista previa en vivo (quién
//! aparece mientras se teclea, uno por número) ya cumple ese papel.
//!
//! Pensado para uso repetido: tras cada Enter el campo se limpia solo y el
//! modo se queda abierto para el siguiente gafete (o grupo) — Esc es lo
//! único que lo cierra.

use crate::services::registro_ingreso_service::IngresoActivoResumen;

/// Largo del campo de texto — varios gafetes de 2 dígitos separados por
/// ", " caben de sobra (p. ej. "2, 25, 85, 11" son 13 caracteres).
pub const MAX_LARGO_TEXTO: usize = 60;

#[derive(Debug, Clone, Default)]
pub struct SalidaGafeteState {
    pub texto: String,
    /// Un ingreso activo por número reconocido en `texto`, en el mismo
    /// orden — `None` si ese gafete no tiene ingreso activo. Lo resuelve
    /// el controlador tras cada tecla (el modelo no consulta `AppCore`).
    pub coincidencias: Vec<(i64, Option<IngresoActivoResumen>)>,
}

impl SalidaGafeteState {
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// Dígitos, coma (separador de lista) y espacio (para poder escribir
    /// "2, 25" con la coma normal del teclado) — cualquier otro carácter
    /// no se inserta, mismo criterio que el resto de campos numéricos.
    pub fn asignar_texto(&mut self, texto: &str) {
        self.texto = texto
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == ',' || c.is_whitespace())
            .take(MAX_LARGO_TEXTO)
            .collect();
    }

    /// Números de gafete reconocidos en `texto`, en el orden en que
    /// aparecen — separados por coma; espacios alrededor de cada uno se
    /// ignoran. Vacío si no hay ningún dígito todavía.
    pub fn gafetes(&self) -> Vec<i64> {
        self.texto
            .split(',')
            .filter_map(|token| token.trim().parse::<i64>().ok())
            .collect()
    }

    pub fn limpiar_tras_confirmar(&mut self) {
        self.texto.clear();
        self.coincidencias.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_admite_digitos_coma_y_espacio() {
        let mut estado = SalidaGafeteState::nuevo();
        estado.asignar_texto("2a, 25b; 85x");
        assert_eq!(estado.texto, "2, 25 85");
    }

    #[test]
    fn gafetes_separa_por_coma_y_recorta_espacios() {
        let mut estado = SalidaGafeteState::nuevo();
        estado.asignar_texto("2, 25 , 85,11");
        assert_eq!(estado.gafetes(), vec![2, 25, 85, 11]);
    }

    #[test]
    fn un_solo_numero_sin_coma_funciona_igual() {
        let mut estado = SalidaGafeteState::nuevo();
        estado.asignar_texto("27");
        assert_eq!(estado.gafetes(), vec![27]);
    }

    #[test]
    fn texto_vacio_no_produce_gafetes() {
        assert_eq!(SalidaGafeteState::nuevo().gafetes(), Vec::<i64>::new());
    }

    #[test]
    fn coma_suelta_o_doble_no_rompe_el_resto() {
        let mut estado = SalidaGafeteState::nuevo();
        estado.asignar_texto("2,,25,");
        assert_eq!(estado.gafetes(), vec![2, 25]);
    }

    #[test]
    fn limpiar_tras_confirmar_deja_todo_en_blanco() {
        let mut estado = SalidaGafeteState::nuevo();
        estado.asignar_texto("27");
        estado.limpiar_tras_confirmar();
        assert_eq!(estado.texto, "");
        assert!(estado.coincidencias.is_empty());
    }
}
