/// Catálogo liviano (`docs/planes-implementados/plan-control-rutas.md`) --
/// mismo molde que [`super::empresa::Empresa`]. `numero_unidad` es
/// `None` para camiones de apoyo/particulares (no tienen número de unidad
/// interno, sólo placa) -- ver `TipoVehiculoRuta` del lado mobile
/// (`LectorVehiculoRuta.kt`), que distingue el mismo caso por OCR.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct VehiculoRuta {
    pub id: i64,
    pub numero_unidad: Option<String>,
    pub placa: String,
    pub activo: bool,
}
