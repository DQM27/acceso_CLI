#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoIngreso {
    Praind,
    InHouse,
    PorCorreo,
    Swat,
}

impl TipoIngreso {
    pub const ALL: [Self; 4] = [Self::Praind, Self::InHouse, Self::PorCorreo, Self::Swat];

    /// Codificación canónica usada para persistir/filtrar en SQLite (columna
    /// `tipo_ingreso`). Única fuente de verdad para este mapeo — antes estaba
    /// copiado a mano en 4 archivos de `database/`, dos veces en uno de ellos.
    pub fn as_str_sql(self) -> &'static str {
        match self {
            Self::Praind => "PRAIND",
            Self::InHouse => "IN_HOUSE",
            Self::PorCorreo => "POR_CORREO",
            Self::Swat => "SWAT",
        }
    }

    /// Inverso de [`Self::as_str_sql`]. `None` si el texto no es ninguno de
    /// los 4 valores conocidos.
    pub fn from_str_sql(texto: &str) -> Option<Self> {
        match texto {
            "PRAIND" => Some(Self::Praind),
            "IN_HOUSE" => Some(Self::InHouse),
            "POR_CORREO" => Some(Self::PorCorreo),
            "SWAT" => Some(Self::Swat),
            _ => None,
        }
    }

    /// Reconoce el texto que un operador escribe en el buscador
    /// (`tipo:praind`, `tipo:in-house`...) — alias más flexibles y sin
    /// distinguir mayúsculas, pensados para teclado humano. Antes copiado
    /// igual en 3 pantallas (`activos`, `contratistas`, `historial`).
    pub fn from_str_filtro(texto: &str) -> Option<Self> {
        match texto.to_lowercase().as_str() {
            "praind" => Some(Self::Praind),
            "inhouse" | "in-house" | "in_house" => Some(Self::InHouse),
            "correo" | "porcorreo" => Some(Self::PorCorreo),
            "swat" => Some(Self::Swat),
            _ => None,
        }
    }
}
