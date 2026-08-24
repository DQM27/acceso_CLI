//! Selector de columnas ocultables para las tablas de `--comandos` (`F4`) —
//! misma mecánica que la TUI clásica (lista con casillero, guardrail de "al
//! menos una visible"), reescrita acá porque `src/tui/` no se toca ni se
//! comparte (DEC-002/DEC-014). Cada pantalla define su propio enum de
//! columnas; este módulo no sabe qué significa ninguna de ellas.

/// Un conjunto de columnas propio de una tabla: orden fijo (`TODAS`),
/// etiqueta visible y clave estable de persistencia (la clave nunca cambia
/// aunque la etiqueta se retoque — es lo que se escribe a disco).
pub trait Columna: Copy + PartialEq + 'static {
    const TODAS: &'static [Self];
    fn etiqueta(self) -> &'static str;
    fn clave(self) -> &'static str;
}

/// Qué columnas de `C` están visibles ahora mismo, en el mismo orden que
/// `C::TODAS` — ese orden es lo que el render recorre y lo que el índice del
/// picker (`alternar`) direcciona.
#[derive(Debug, Clone)]
pub struct SelectorColumnas<C: Columna> {
    columnas: Vec<(C, bool)>,
}

impl<C: Columna> SelectorColumnas<C> {
    pub fn todas_visibles() -> Self {
        Self {
            columnas: C::TODAS.iter().map(|c| (*c, true)).collect(),
        }
    }

    pub fn visible(&self, columna: C) -> bool {
        self.columnas
            .iter()
            .find(|(c, _)| *c == columna)
            .map(|(_, visible)| *visible)
            .unwrap_or(true)
    }

    pub fn iter(&self) -> impl Iterator<Item = (C, bool)> + '_ {
        self.columnas.iter().copied()
    }

    /// Marca/desmarca la columna en `indice` (posición dentro de
    /// `C::TODAS`). Rechaza dejar la tabla sin ninguna columna visible —
    /// mismo guardrail que la TUI clásica ("Debe conservar al menos una
    /// columna").
    pub fn alternar(&mut self, indice: usize) -> Result<(), &'static str> {
        let Some((_, visible)) = self.columnas.get(indice) else {
            return Ok(());
        };
        let visibles = self.columnas.iter().filter(|(_, v)| *v).count();
        if *visible && visibles <= 1 {
            return Err("Debe conservar al menos una columna visible");
        }
        self.columnas[indice].1 = !self.columnas[indice].1;
        Ok(())
    }

    /// Serializa las columnas visibles como `clave,clave,clave` — formato de
    /// persistencia (ver `preferencias.rs`).
    pub fn preferencia(&self) -> String {
        self.columnas
            .iter()
            .filter_map(|(c, visible)| visible.then_some(c.clave()))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Aplica una preferencia guardada. Una preferencia vacía o que no
    /// reconoce ninguna clave se ignora (queda todo visible, el default) en
    /// vez de vaciar la tabla por un archivo corrupto o de otra versión.
    pub fn aplicar_preferencia(&mut self, valor: &str) {
        if valor.is_empty() {
            return;
        }
        let claves: Vec<&str> = valor.split(',').collect();
        if !self
            .columnas
            .iter()
            .any(|(c, _)| claves.contains(&c.clave()))
        {
            return;
        }
        for (c, visible) in &mut self.columnas {
            *visible = claves.contains(&c.clave());
        }
    }
}

/// Columnas de la tabla de coincidencias (búsqueda de contratistas /
/// `/ingreso` / `/editar`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnaBusqueda {
    Cedula,
    Nombre,
    Empresa,
    Tipo,
}

impl Columna for ColumnaBusqueda {
    const TODAS: &'static [Self] = &[Self::Cedula, Self::Nombre, Self::Empresa, Self::Tipo];

    fn etiqueta(self) -> &'static str {
        match self {
            Self::Cedula => "Cédula",
            Self::Nombre => "Nombre",
            Self::Empresa => "Empresa",
            Self::Tipo => "Tipo",
        }
    }

    fn clave(self) -> &'static str {
        match self {
            Self::Cedula => "cedula",
            Self::Nombre => "nombre",
            Self::Empresa => "empresa",
            Self::Tipo => "tipo",
        }
    }
}

/// Columnas de la tabla de `/activos`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnaActivos {
    Gafete,
    Nombre,
    Empresa,
    Ingreso,
}

impl Columna for ColumnaActivos {
    const TODAS: &'static [Self] = &[Self::Gafete, Self::Nombre, Self::Empresa, Self::Ingreso];

    fn etiqueta(self) -> &'static str {
        match self {
            Self::Gafete => "Gafete",
            Self::Nombre => "Nombre",
            Self::Empresa => "Empresa",
            Self::Ingreso => "Ingreso",
        }
    }

    fn clave(self) -> &'static str {
        match self {
            Self::Gafete => "gafete",
            Self::Nombre => "nombre",
            Self::Empresa => "empresa",
            Self::Ingreso => "ingreso",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todas_visibles_por_defecto() {
        let selector = SelectorColumnas::<ColumnaBusqueda>::todas_visibles();
        for columna in ColumnaBusqueda::TODAS {
            assert!(selector.visible(*columna));
        }
    }

    #[test]
    fn alternar_oculta_y_muestra() {
        let mut selector = SelectorColumnas::<ColumnaBusqueda>::todas_visibles();
        selector.alternar(0).unwrap();
        assert!(!selector.visible(ColumnaBusqueda::Cedula));
        selector.alternar(0).unwrap();
        assert!(selector.visible(ColumnaBusqueda::Cedula));
    }

    #[test]
    fn no_permite_ocultar_la_ultima_columna_visible() {
        let mut selector = SelectorColumnas::<ColumnaBusqueda>::todas_visibles();
        for indice in 0..3 {
            selector.alternar(indice).unwrap();
        }
        // Sólo Tipo (índice 3) sigue visible.
        assert_eq!(selector.iter().filter(|(_, v)| *v).count(), 1);
        assert!(selector.alternar(3).is_err());
        assert!(selector.visible(ColumnaBusqueda::Tipo));
    }

    #[test]
    fn preferencia_serializa_solo_las_visibles_en_orden() {
        let mut selector = SelectorColumnas::<ColumnaBusqueda>::todas_visibles();
        selector.alternar(0).unwrap(); // oculta Cédula
        assert_eq!(selector.preferencia(), "nombre,empresa,tipo");
    }

    #[test]
    fn aplicar_preferencia_reconstruye_visibilidad() {
        let mut selector = SelectorColumnas::<ColumnaBusqueda>::todas_visibles();
        selector.aplicar_preferencia("nombre,tipo");
        assert!(!selector.visible(ColumnaBusqueda::Cedula));
        assert!(selector.visible(ColumnaBusqueda::Nombre));
        assert!(!selector.visible(ColumnaBusqueda::Empresa));
        assert!(selector.visible(ColumnaBusqueda::Tipo));
    }

    #[test]
    fn aplicar_preferencia_vacia_o_irreconocible_no_cambia_nada() {
        let mut selector = SelectorColumnas::<ColumnaBusqueda>::todas_visibles();
        selector.aplicar_preferencia("");
        assert_eq!(selector.preferencia(), "cedula,nombre,empresa,tipo");
        selector.aplicar_preferencia("xyz,otra");
        assert_eq!(selector.preferencia(), "cedula,nombre,empresa,tipo");
    }
}
