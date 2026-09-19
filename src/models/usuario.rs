#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usuario {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub password_hash: String,
    pub rol: RolUsuario,
    pub activo: bool,
    /// `None` -- el default de siempre -- significa que `password_hash` (si
    /// es real, no `SIN_PASSWORD_LOCAL`) es permanente: no vence nunca, ROOT
    /// y cualquier cuenta local de antes de la migración a Supabase Auth
    /// (ver `docs/planes-implementados/plan-autenticacion-supabase-auth.md`)
    /// siguen funcionando así, sin cambios.
    ///
    /// `Some(marca)` marca en cambio un hash CACHEADO tras un login online
    /// exitoso contra Supabase (usuario global, Administrador/Operador) --
    /// ver `docs/decisiones-tecnicas.md`, entrada 2026-09-18. Ese hash sólo
    /// es válido por 24h desde `marca` (`AutenticacionService::buscar_candidato`
    /// lo rechaza pasado ese tope, aunque la contraseña sea correcta):
    /// existe para poder operar sin internet ante un corte, acotado a una
    /// ventana conocida en vez de dejarlo indefinido. Texto UTC serializado
    /// con `crate::tiempo::serializar_utc`/`parsear_utc`, igual que el resto
    /// de las marcas de tiempo de este crate (`catalogo_actualizado_hasta`,
    /// etc.) -- nunca un timestamp crudo del sistema operativo sin corregir.
    pub password_hash_confirmado_en: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RolUsuario {
    Root,
    Administrador,
    Operador,
}
