//! Helpers del formulario de alta/edición de Usuarios: validación y el
//! selector de rol.

use crate::models::usuario::RolUsuario;

use super::password::validar_password;
use super::{FormularioUsuario, ModoFormularioUsuario};

pub(in crate::tui::usuarios::state) const ROLES: [RolUsuario; 3] = [
    RolUsuario::Root,
    RolUsuario::Administrador,
    RolUsuario::Operador,
];

pub(super) fn validar_formulario(f: &FormularioUsuario) -> Result<(), String> {
    if f.cedula.value().trim().is_empty() {
        return Err("La cédula es obligatoria".into());
    }
    if f.nombre.value().trim().is_empty() {
        return Err("El nombre es obligatorio".into());
    }
    if matches!(f.modo, ModoFormularioUsuario::Crear) {
        validar_password(f.password.valor(), f.confirmar_password.valor())?;
    }
    Ok(())
}

pub(super) fn indice_rol(r: RolUsuario) -> usize {
    ROLES.iter().position(|x| *x == r).unwrap_or(2)
}

pub(in crate::tui::usuarios::state) fn texto_rol(r: RolUsuario) -> &'static str {
    match r {
        RolUsuario::Root => "ROOT",
        RolUsuario::Administrador => "ADMINISTRADOR",
        RolUsuario::Operador => "OPERADOR",
    }
}

pub(in crate::tui::usuarios::state) fn si_no(v: bool) -> &'static str {
    if v { "SÍ" } else { "NO" }
}
