#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EstadoGafete {
    Disponible,
    /// No se puede asignar: el gafete no está físicamente (se perdió, no se
    /// sabe dónde está o espera reimpresión). Puede tener responsable o no.
    Perdido,
}

impl EstadoGafete {
    /// Codificación canónica usada para persistir/filtrar en `SQLite` (columna
    /// `gafetes.estado`).
    pub fn as_str_sql(self) -> &'static str {
        match self {
            Self::Disponible => "DISPONIBLE",
            Self::Perdido => "PERDIDO",
        }
    }

    /// Inverso de [`Self::as_str_sql`]. `None` si el texto no es ninguno de
    /// los valores conocidos. `DE_BAJA` (estado retirado en `MIGRACION_65`)
    /// se lee como `Perdido`: es lo que hace la migración con los locales y
    /// lo que hace la nube con lo que manda una app sin actualizar.
    pub fn from_str_sql(texto: &str) -> Option<Self> {
        match texto {
            "DISPONIBLE" => Some(Self::Disponible),
            "PERDIDO" | "DE_BAJA" => Some(Self::Perdido),
            _ => None,
        }
    }

    /// Reconoce el texto que un operador escribe en el buscador
    /// (`estado:disponible|perdido`) — alias flexibles, sin distinguir
    /// mayúsculas, mismo criterio que `TipoIngreso::from_str_filtro`.
    pub fn from_str_filtro(texto: &str) -> Option<Self> {
        match texto.to_lowercase().as_str() {
            "disponible" | "disponibles" => Some(Self::Disponible),
            "perdido" | "perdidos" => Some(Self::Perdido),
            _ => None,
        }
    }
}

/// Pool físico al que pertenece un gafete (`gafetes.tipo`) — gafetes de
/// distinto tipo son objetos físicos distintos que repiten la misma
/// numeración (el "7 verde" de contratista y el "7 rojo" de visita
/// coexisten), por eso la unicidad real es `(numero, tipo)`, no `numero`
/// solo. `ProvisionalKof` tiene su columna de portador propia
/// (`encargado_portador_id`, hacia `encargados_ruta`) desde `MIGRACION_39`
/// -- ver `docs/features-futuras/plan-gafetes-provisionales-kof.md`.
/// `Proveedor` tiene la suya (`proveedor_portador_id`, hacia
/// `registro_ingresos_proveedor`) desde `MIGRACION_41` -- ver
/// `docs/features-futuras/plan-control-proveedores.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TipoGafete {
    Contratista,
    Visita,
    Proveedor,
    ProvisionalKof,
}

impl TipoGafete {
    pub fn as_str_sql(self) -> &'static str {
        match self {
            Self::Contratista => "CONTRATISTA",
            Self::Visita => "VISITA",
            Self::Proveedor => "PROVEEDOR",
            Self::ProvisionalKof => "PROVISIONAL_KOF",
        }
    }

    pub fn from_str_sql(texto: &str) -> Option<Self> {
        match texto {
            "CONTRATISTA" => Some(Self::Contratista),
            "VISITA" => Some(Self::Visita),
            "PROVEEDOR" => Some(Self::Proveedor),
            "PROVISIONAL_KOF" => Some(Self::ProvisionalKof),
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
            "provisional_kof" | "provisional" | "kof" => Some(Self::ProvisionalKof),
            _ => None,
        }
    }
}

/// Quién responde por un gafete perdido -- para trazabilidad, no para
/// llevar cuentas de dinero (por eso no se llama "deudor"). Cada variante
/// corresponde a una de las columnas FK reales de `gafetes`
/// (`contratista_portador_id`/`visita_portador_id`/`encargado_portador_id`/
/// `proveedor_portador_id`). `ProvisionalKof(i64)` apunta a un catálogo real
/// (`encargados_ruta`, la persona KOF sí se repite); `Proveedor(i64)` en
/// cambio apunta a un registro TRANSACCIONAL (`registro_ingresos_proveedor.id`,
/// la visita puntual) y no a un catálogo de personas -- pedido explícito del
/// usuario: el colaborador de un proveedor nunca se repite, así que no existe
/// (ni debe existir) una tabla de personas para este caso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortadorGafete {
    Contratista(i64),
    Visita(i64),
    ProvisionalKof(i64),
    Proveedor(i64),
}

/// Motivo con que se cerraba un incidente antes de `MIGRACION_65` (filas
/// `RESUELTO`). Sólo se lee, para mostrar el historial viejo: los reportes
/// nuevos son `Pagado`/`Recuperado` (ver [`TipoIncidenteGafete`]).
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

/// Tipo de reporte del historial (`gafetes_incidentes.tipo`). Cada reporte
/// queda a nombre de quien lo hizo y no se edita ni se borra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TipoIncidenteGafete {
    /// Se reportó perdido, con o sin responsable.
    Perdido,
    /// El responsable pagó: se le quita la marca, el gafete sigue perdido
    /// hasta que se reimprima.
    Pagado,
    /// Apareció o se reimprimió: vuelve a estar disponible.
    Recuperado,
    /// Cierre anterior a `MIGRACION_65` (pagado o apareció en un solo paso).
    Resuelto,
}

impl TipoIncidenteGafete {
    pub fn as_str_sql(self) -> &'static str {
        match self {
            Self::Perdido => "PERDIDO",
            Self::Pagado => "PAGADO",
            Self::Recuperado => "RECUPERADO",
            Self::Resuelto => "RESUELTO",
        }
    }

    pub fn from_str_sql(texto: &str) -> Option<Self> {
        match texto {
            "PERDIDO" => Some(Self::Perdido),
            "PAGADO" => Some(Self::Pagado),
            "RECUPERADO" => Some(Self::Recuperado),
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
    /// A lo sumo una puede ser `Some`, y únicamente cuando `estado ==
    /// Perdido` (un perdido puede no tener ninguna: responsable desconocido o
    /// ya pagó) -- los `CHECK`s del esquema imponen las mismas reglas del
    /// lado de `SQLite`, esto sólo las refleja en Rust. Usar
    /// [`Gafete::portador`] en vez de leer estos campos directamente.
    pub contratista_portador_id: Option<i64>,
    pub visita_portador_id: Option<i64>,
    pub encargado_portador_id: Option<i64>,
    pub proveedor_portador_id: Option<i64>,
}

impl Gafete {
    /// Quién responde por este gafete perdido, si alguien -- `None` si está
    /// `Disponible`, si no se sabe quién lo tenía o si ya pagó.
    pub fn portador(&self) -> Option<PortadorGafete> {
        if let Some(id) = self.contratista_portador_id {
            return Some(PortadorGafete::Contratista(id));
        }
        if let Some(id) = self.visita_portador_id {
            return Some(PortadorGafete::Visita(id));
        }
        if let Some(id) = self.encargado_portador_id {
            return Some(PortadorGafete::ProvisionalKof(id));
        }
        self.proveedor_portador_id.map(PortadorGafete::Proveedor)
    }
}
