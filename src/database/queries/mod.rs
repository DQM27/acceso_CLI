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
pub mod auditoria;

/// Filtro de igualdad que también admite su negación (`-clave:valor` en el
/// lenguaje `clave:valor` de la TUI) — `Incluye(v)` arma `col = v`,
/// `Excluye(v)` arma `col <> v`. Compartido por `empresa_id`/`gafete_numero`
/// en `contratistas.rs`/`ingresos.rs` para no repetir el mismo par
/// valor+bandera-de-negación en cada `Filtro*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Igualdad<T> {
    Incluye(T),
    Excluye(T),
}

impl<T> Igualdad<T> {
    pub fn valor(&self) -> &T {
        match self {
            Igualdad::Incluye(v) | Igualdad::Excluye(v) => v,
        }
    }

    pub fn negado(&self) -> bool {
        matches!(self, Igualdad::Excluye(_))
    }

    pub fn operador_sql(&self) -> &'static str {
        if self.negado() { "<>" } else { "=" }
    }
}
