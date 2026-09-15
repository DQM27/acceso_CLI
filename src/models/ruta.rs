/// Catálogo de números de ruta válidos (`docs/planes-implementados/plan-control-rutas.md`)
/// -- a diferencia de `VehiculoRuta`/`EncargadoRuta` (consultivos, nunca
/// bloquean un registro), este catálogo SÍ restringe: pedido explícito del
/// usuario, 2026-09-15 ("sin restricción podrías poner la ruta 222 y no
/// existe, sino un número acotado de rutas"). Por sitio (como `gafetes`,
/// no global como vehículos/encargados) -- una ruta pertenece a un sitio a
/// la vez, aunque pueda reasignarse a otro administrativamente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Ruta {
    pub id: i64,
    pub numero: i64,
    pub activo: bool,
}
