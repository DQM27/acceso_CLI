//! Reglas del catálogo de gafetes (`docs/plan-gafetes.md`): transiciones de
//! estado válidas (`Disponible -> Perdido -> Disponible`,
//! `Disponible -> DeBaja`) y validación de un gafete contra el catálogo
//! antes de asignarlo a un ingreso.

use crate::models::gafete::{EstadoGafete, Gafete};

fn esta_disponible(estado: EstadoGafete) -> bool {
    estado == EstadoGafete::Disponible
}

/// Sólo un gafete `Disponible` puede darse de baja.
pub fn puede_darse_de_baja(estado: EstadoGafete) -> bool {
    esta_disponible(estado)
}

/// Sólo un gafete `Disponible` puede marcarse como perdido.
pub fn puede_marcarse_perdido(estado: EstadoGafete) -> bool {
    esta_disponible(estado)
}

/// Sólo un gafete `Perdido` puede resolverse (pagado/aparecido).
pub fn puede_resolverse(estado: EstadoGafete) -> bool {
    estado == EstadoGafete::Perdido
}

/// Resultado de validar un número de gafete contra el catálogo antes de
/// asignarlo a un ingreso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidacionAsignacion {
    Asignable,
    NoRegistrado,
    NoDisponible(EstadoGafete),
}

/// Regla de negocio (`docs/plan-gafetes.md`): catálogo primero (¿existe y
/// está disponible?). La ocupación (¿está en uso ahora mismo por un ingreso
/// activo?) es un chequeo aparte porque depende de `registro_ingresos`, no
/// del catálogo — vive en el servicio, que sí tiene acceso a ese repositorio.
pub fn validar_para_asignar(gafete: Option<&Gafete>) -> ValidacionAsignacion {
    match gafete {
        None => ValidacionAsignacion::NoRegistrado,
        Some(gafete) if !esta_disponible(gafete.estado) => {
            ValidacionAsignacion::NoDisponible(gafete.estado)
        }
        Some(_) => ValidacionAsignacion::Asignable,
    }
}
