//! Regla de validación compartida por las dos contraseñas nuevas que puede
//! escribir un operador: crear usuario (`form.rs::validar_formulario`) y
//! cambiar contraseña (`state.rs::password`).

pub(in crate::tui::usuarios::state) fn validar_password(p: &str, c: &str) -> Result<(), String> {
    if p.is_empty() {
        Err("La contraseña es obligatoria".into())
    } else if c.is_empty() {
        Err("Debe confirmar la contraseña".into())
    } else if p.chars().count() < 8 {
        Err("La contraseña debe tener al menos 8 caracteres".into())
    } else if p != c {
        Err("Las contraseñas no coinciden".into())
    } else {
        Ok(())
    }
}
