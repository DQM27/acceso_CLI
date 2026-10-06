//! Por qué vía está adentro una persona: como contratista, como proveedor o
//! por correo. Cada vía tiene su propia tabla de movimientos, y una misma
//! persona (misma cédula) no puede estar adentro por dos a la vez (revisión
//! del 2026-10-04, `docs/auditorias/revision-carreras-2026-10-04.md`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViaIngreso {
    Contratista,
    Proveedor,
    PorCorreo,
}

impl ViaIngreso {
    /// "como contratista", "como proveedor", "por correo" -- para armar
    /// mensajes del tipo "ya está adentro {texto}".
    pub fn texto(self) -> &'static str {
        match self {
            Self::Contratista => "como contratista",
            Self::Proveedor => "como proveedor",
            Self::PorCorreo => "por correo",
        }
    }
}
