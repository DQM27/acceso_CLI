#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoIngreso {
    Praind,
    InHouse,
    PorCorreo,
    Swat,
}

impl TipoIngreso {
    pub fn requiere_praind(&self) -> bool {
        match self {
            TipoIngreso::Praind => true,
            TipoIngreso::InHouse => false,
            TipoIngreso::PorCorreo => true,
            TipoIngreso::Swat => false,
        }
    }

    pub fn requiere_gafete(&self) -> bool {
        match self {
            TipoIngreso::Praind => true,
            TipoIngreso::InHouse => false,
            TipoIngreso::PorCorreo => true,
            TipoIngreso::Swat => false,
        }
    }
}