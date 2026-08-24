//! Formulario de alta de empresa para `--comandos` — mismo patrón de Surface
//! enclavada que `formulario.rs` (contratista), reducido a lo que hace
//! falta para un único campo: sin `Campo`/`Subfase`/selector/resumen, esas
//! abstracciones existen para navegar y revisar varios campos y acá sólo
//! hay uno. Sin resumen intermedio a propósito (§2 principio 8: "¿realmente
//! hace falta?") — con un solo campo, una segunda pantalla para revisar lo
//! mismo que ya se ve sería fricción sin valor.
//!
//! A diferencia del nombre de un contratista, el nombre de una empresa NO
//! se filtra por tipo de carácter — nombres reales de empresa llevan
//! números, puntos, `&` ("3M", "S.A.", "Import & Export") — sólo se limita
//! el largo.

/// Mismo largo máximo que ya usaba la TUI clásica para este campo.
pub const MAX_NOMBRE_EMPRESA: usize = 80;

#[derive(Debug, Clone, Default)]
pub struct FormularioEmpresa {
    pub nombre: String,
    pub error: Option<String>,
}

impl FormularioEmpresa {
    pub fn nuevo() -> Self {
        Self::default()
    }

    pub fn asignar_texto(&mut self, texto: &str) {
        self.nombre = texto.chars().take(MAX_NOMBRE_EMPRESA).collect();
        self.error = None;
    }

    /// Sólo valida "no vacío" — lo mismo que ya validaba la TUI clásica en
    /// el cliente; el nombre duplicado lo detecta `AppCore` al guardar (no
    /// hay forma de saberlo sin consultar la base, y acá no vale la pena
    /// una consulta proactiva como la de cédula: un campo, un intento, el
    /// costo de equivocarse es un solo Enter más).
    pub fn validar(&mut self) -> Result<String, String> {
        let nombre = self.nombre.trim().to_string();
        if nombre.is_empty() {
            let mensaje = "Escriba el nombre de la empresa".to_string();
            self.error = Some(mensaje.clone());
            return Err(mensaje);
        }
        self.error = None;
        Ok(nombre)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admite_numeros_puntos_y_ampersand() {
        let mut form = FormularioEmpresa::nuevo();
        form.asignar_texto("3M Import & Export S.A.");
        assert_eq!(form.nombre, "3M Import & Export S.A.");
    }

    #[test]
    fn respeta_el_largo_maximo() {
        let mut form = FormularioEmpresa::nuevo();
        form.asignar_texto(&"a".repeat(120));
        assert_eq!(form.nombre.chars().count(), MAX_NOMBRE_EMPRESA);
    }

    #[test]
    fn nombre_vacio_es_error() {
        let mut form = FormularioEmpresa::nuevo();
        assert!(form.validar().is_err());
        assert!(form.error.is_some());
    }

    #[test]
    fn escribir_limpia_el_error() {
        let mut form = FormularioEmpresa::nuevo();
        form.validar().unwrap_err();
        form.asignar_texto("Brisas del Oeste");
        assert!(form.error.is_none());
    }

    #[test]
    fn nombre_valido_recorta_espacios() {
        let mut form = FormularioEmpresa::nuevo();
        form.asignar_texto("  Brisas del Oeste  ");
        assert_eq!(form.validar(), Ok("Brisas del Oeste".to_string()));
    }
}
