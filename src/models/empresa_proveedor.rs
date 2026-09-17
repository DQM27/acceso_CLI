/// Catálogo de empresas proveedoras (`docs/features-futuras/plan-control-proveedores.md`)
/// -- separado a propósito de [`crate::models::empresa::Empresa`]: son
/// empresas aparte de las que emplean contratistas, pedido explícito del
/// usuario. Mismo shape mínimo, sin campos propios más allá de eso.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct EmpresaProveedor {
    pub id: i64,
    pub nombre: String,
    pub activo: bool,
}
