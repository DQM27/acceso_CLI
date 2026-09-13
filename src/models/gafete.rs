#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EstadoGafete {
    Disponible,
    Perdido,
    DeBaja,
}

impl EstadoGafete {
    /// Codificación canónica usada para persistir/filtrar en `SQLite` (columna
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

/// Pool físico al que pertenece un gafete (`gafetes.tipo`) — gafetes de
/// distinto tipo son objetos físicos distintos que repiten la misma
/// numeración (el "7 verde" de contratista y el "7 rojo" de visita
/// coexisten), por eso la unicidad real es `(numero, tipo)`, no `numero`
/// solo. Sin `Proveedor` con columna de portador todavía -- no existe
/// tabla `proveedores`, pero el valor ya se acepta en el `CHECK` de la
/// base para no tener que volver a tocar el esquema cuando exista.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TipoGafete {
    Contratista,
    Visita,
    Proveedor,
}

impl TipoGafete {
    pub fn as_str_sql(self) -> &'static str {
        match self {
            Self::Contratista => "CONTRATISTA",
            Self::Visita => "VISITA",
            Self::Proveedor => "PROVEEDOR",
        }
    }

    pub fn from_str_sql(texto: &str) -> Option<Self> {
        match texto {
            "CONTRATISTA" => Some(Self::Contratista),
            "VISITA" => Some(Self::Visita),
            "PROVEEDOR" => Some(Self::Proveedor),
            _ => None,
        }
    }

    /// Mismo criterio que `EstadoGafete::from_str_filtro`: alias flexibles
    /// para lo que un operador escribe en el buscador.
    pub fn from_str_filtro(texto: &str) -> Option<Self> {
        match texto.to_lowercase().as_str() {
            "contratista" | "contratistas" => Some(Self::Contratista),
            "visita" | "visitas" => Some(Self::Visita),
            "proveedor" | "proveedores" => Some(Self::Proveedor),
            _ => None,
        }
    }
}

/// A quién se le asignó un gafete la última vez -- para trazabilidad si se
/// pierde, no para llevar cuentas de dinero (por eso no se llama
/// "deudor"). Cada variante corresponde a una de las columnas FK reales de
/// `gafetes` (`contratista_portador_id`/`visita_portador_id`); sin
/// `Proveedor(i64)` todavía por lo mismo que [`TipoGafete::Proveedor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortadorGafete {
    Contratista(i64),
    Visita(i64),
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

/// Tipo de fila de `gafetes_incidentes` — espejo de la columna `tipo`
/// (`PERDIDO`/`RESUELTO`), usado por la lectura del historial
/// (`database::queries::gafetes_incidentes::GafetesIncidentesQuery`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TipoIncidenteGafete {
    Perdido,
    Resuelto,
}

impl TipoIncidenteGafete {
    pub fn as_str_sql(self) -> &'static str {
        match self {
            Self::Perdido => "PERDIDO",
            Self::Resuelto => "RESUELTO",
        }
    }

    pub fn from_str_sql(texto: &str) -> Option<Self> {
        match texto {
            "PERDIDO" => Some(Self::Perdido),
            "RESUELTO" => Some(Self::Resuelto),
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
    pub tipo: TipoGafete,
    pub estado: EstadoGafete,
    /// Sólo una de las dos puede ser `Some`, y únicamente cuando
    /// `estado == Perdido` -- los `CHECK`s del esquema imponen las mismas
    /// reglas del lado de `SQLite` (tipo↔columna y estado↔portador), esto
    /// sólo las refleja en Rust. Usar [`Gafete::portador`] en vez de leer
    /// estos campos directamente.
    pub contratista_portador_id: Option<i64>,
    pub visita_portador_id: Option<i64>,
}

impl Gafete {
    /// A quién se le asignó este gafete la última vez, si a alguien --
    /// `None` si está `Disponible`/`DeBaja`. Deriva de las dos columnas en
    /// vez de guardarse aparte porque el `CHECK` del esquema ya garantiza
    /// que a lo sumo una está seteada.
    pub fn portador(&self) -> Option<PortadorGafete> {
        self.contratista_portador_id.map_or_else(
            || self.visita_portador_id.map(PortadorGafete::Visita),
            |id| Some(PortadorGafete::Contratista(id)),
        )
    }
}
