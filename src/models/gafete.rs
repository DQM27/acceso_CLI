#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EstadoGafete {
    Disponible,
    Perdido,
    DeBaja,
}

impl EstadoGafete {
    /// Codificación canónica usada para persistir/filtrar en SQLite (columna
    /// `gafetes.estado`).
    pub fn as_str_sql(self) -> &'static str {
        match self {
            Self::Disponible => "DISPONIBLE",
            Self::Perdido => "PERDIDO",
            Self::DeBaja => "DE_BAJA",
        }
    }

    /// Inverso de [`Self::as_str_sql`]. `None` si el texto no es ninguno de
    /// los 3 valores conocidos.
    pub fn from_str_sql(texto: &str) -> Option<Self> {
        match texto {
            "DISPONIBLE" => Some(Self::Disponible),
            "PERDIDO" => Some(Self::Perdido),
            "DE_BAJA" => Some(Self::DeBaja),
            _ => None,
        }
    }

    /// Reconoce el texto que un operador escribe en el buscador
    /// (`estado:disponible|perdido|de_baja`) — alias flexibles, sin
    /// distinguir mayúsculas, mismo criterio que `TipoIngreso::from_str_filtro`.
    pub fn from_str_filtro(texto: &str) -> Option<Self> {
        match texto.to_lowercase().as_str() {
            "disponible" | "disponibles" => Some(Self::Disponible),
            "perdido" | "perdidos" => Some(Self::Perdido),
            "de_baja" | "debaja" | "de-baja" | "baja" => Some(Self::DeBaja),
            _ => None,
        }
    }
}

/// Motivo por el que se cierra un incidente de pérdida (`gafetes_incidentes`,
/// registro `RESUELTO`). Sólo estado — sin monto ni nota, decisión explícita
/// del usuario (`docs/plan-gafetes.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MotivoResolucionGafete {
    Pagado,
    Aparecido,
}

impl MotivoResolucionGafete {
    pub fn as_str_sql(self) -> &'static str {
        match self {
            Self::Pagado => "PAGADO",
            Self::Aparecido => "APARECIDO",
        }
    }

    pub fn from_str_sql(texto: &str) -> Option<Self> {
        match texto {
            "PAGADO" => Some(Self::Pagado),
            "APARECIDO" => Some(Self::Aparecido),
            _ => None,
        }
    }
}

/// Fila del catálogo (`gafetes`): sólo el estado vigente, no el historial —
/// eso vive en `gafetes_incidentes` (`database::queries::gafetes_incidentes`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Gafete {
    pub id: i64,
    pub numero: i64,
    pub estado: EstadoGafete,
    /// `Some` únicamente cuando `estado == Perdido` — el `CHECK` del esquema
    /// impone la misma regla del lado de SQLite, esto sólo la refleja en Rust.
    pub contratista_deudor_id: Option<i64>,
}
