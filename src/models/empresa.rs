#[derive(Debug, Clone)]
pub struct Empresa {
    pub id: i64,
    pub nombre: String,
    /// Dar de baja una empresa no toca `Contratista::tiene_acceso` de sus
    /// contratistas (eso sigue siendo una decisión individual) — en cambio
    /// `domain::acceso::verificar_acceso` deniega a cualquiera de ellos
    /// mientras la empresa esté inactiva, sin importar su acceso individual.
    pub activo: bool,
}
