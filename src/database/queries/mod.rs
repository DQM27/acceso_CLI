pub mod contratistas;
pub mod empresas;
pub mod ingresos;
pub mod usuarios;

/// Límites de página compartidos por Contratistas, Empresas y Usuarios —
/// antes cada módulo repetía la misma pareja de constantes con el mismo
/// nombre y valor (`docs/hallazgos-buscador.md`, "límites de paginación
/// inconsistentes"). Historial e Ingresos Activos usan los suyos propios a
/// propósito: Historial pagina en ventanas más chicas sobre una tabla
/// append-only de crecimiento indefinido, y Activos no pagina en absoluto
/// (es un tope de seguridad, no una página) — no comparten este criterio.
pub(crate) const LIMITE_LISTADO_PREDETERMINADO: usize = 100;
pub(crate) const LIMITE_LISTADO_MAXIMO: usize = 500;
