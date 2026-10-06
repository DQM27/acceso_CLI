//! Regla de negocio: todo nombre, de persona o de empresa, se guarda en
//! MAYÚSCULAS con los espacios de más colapsados (pedido del dueño
//! 2026-10-06). La aplica el núcleo al guardar y la usan las interfaces
//! mientras se escribe, para que lo que se ve sea lo que queda.

/// "  ana  maría solano " → "ANA MARÍA SOLANO". Mayúscula de Unicode
/// completa: las tildes y la ñ también ("josé peña" → "JOSÉ PEÑA").
pub fn nombre_en_mayusculas(texto: &str) -> String {
    texto
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

/// Mientras se escribe: sólo pasa a mayúscula, sin tocar los espacios (si
/// no, no se podría escribir el espacio entre nombre y apellido). Al guardar
/// manda [`nombre_en_mayusculas`].
pub fn nombre_mientras_se_escribe(texto: &str) -> String {
    texto.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_nombre_queda_en_mayusculas_y_sin_espacios_de_mas() {
        assert_eq!(
            nombre_en_mayusculas("  ana  maría solano "),
            "ANA MARÍA SOLANO"
        );
        assert_eq!(nombre_en_mayusculas("josé peña"), "JOSÉ PEÑA");
        assert_eq!(nombre_en_mayusculas("Acme s.a."), "ACME S.A.");
        assert_eq!(nombre_en_mayusculas("   "), "");
    }

    #[test]
    fn mientras_se_escribe_conserva_el_espacio_del_final() {
        assert_eq!(nombre_mientras_se_escribe("ana "), "ANA ");
        assert_eq!(nombre_mientras_se_escribe("peña"), "PEÑA");
    }
}
