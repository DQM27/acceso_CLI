#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoIngreso {
    Praind,
    InHouse,
    PorCorreo,
    Swat,
}

impl TipoIngreso {
    pub const ALL: [Self; 4] = [Self::Praind, Self::InHouse, Self::PorCorreo, Self::Swat];
}
