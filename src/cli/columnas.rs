//! Selector de columnas ocultables para las tablas de `--cli` (`F4`) —
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
            .is_none_or(|(_, visible)| *visible)
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
/// `/ingreso` / `/editar`). Mismas 7 columnas que `tui::contratistas::Columna`
/// — los mismos datos ya viven en `ContratistaResumen`, sólo faltaba
/// exponerlos acá (DEC-030) — más `Estado` (DENTRO/FUERA, `tiene_ingreso_activo`),
/// para que el operador vea antes de confirmar si ya hay un ingreso activo,
/// sin tener que abrir la ficha para enterarse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnaBusqueda {
    Cedula,
    Nombre,
    Empresa,
    Tipo,
    Praind,
    Ruta,
    Acceso,
    Estado,
}

impl Columna for ColumnaBusqueda {
    const TODAS: &'static [Self] = &[
        Self::Cedula,
        Self::Nombre,
        Self::Empresa,
        Self::Tipo,
        Self::Praind,
        Self::Ruta,
        Self::Acceso,
        Self::Estado,
    ];

    fn etiqueta(self) -> &'static str {
        match self {
            Self::Cedula => "Cédula",
            Self::Nombre => "Nombre",
            Self::Empresa => "Empresa",
            Self::Tipo => "Tipo",
            Self::Praind => "Praind",
            Self::Ruta => "Ruta",
            Self::Acceso => "Acceso",
            Self::Estado => "Estado",
        }
    }

    fn clave(self) -> &'static str {
        match self {
            Self::Cedula => "cedula",
            Self::Nombre => "nombre",
            Self::Empresa => "empresa",
            Self::Tipo => "tipo",
            Self::Praind => "praind",
            Self::Ruta => "ruta",
            Self::Acceso => "acceso",
            Self::Estado => "estado",
        }
    }
}

/// Columnas de la tabla de `/activos`. Mismas 8 columnas que
/// `tui::activos::Columna` — los mismos datos ya viven en
/// `IngresoActivoResumen` (DEC-030).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnaActivos {
    Cedula,
    Nombre,
    Empresa,
    Tipo,
    Hora,
    Gafete,
    Medio,
    Usuario,
}

impl Columna for ColumnaActivos {
    const TODAS: &'static [Self] = &[
        Self::Cedula,
        Self::Nombre,
        Self::Empresa,
        Self::Tipo,
        Self::Hora,
        Self::Gafete,
        Self::Medio,
        Self::Usuario,
    ];

    fn etiqueta(self) -> &'static str {
        match self {
            Self::Cedula => "Cédula",
            Self::Nombre => "Nombre",
            Self::Empresa => "Empresa",
            Self::Tipo => "Tipo",
            Self::Hora => "Hora",
            Self::Gafete => "Gafete",
            Self::Medio => "Medio",
            Self::Usuario => "Da ingreso",
        }
    }

    fn clave(self) -> &'static str {
        match self {
            Self::Cedula => "cedula",
            Self::Nombre => "nombre",
            Self::Empresa => "empresa",
            Self::Tipo => "tipo",
            Self::Hora => "hora",
            Self::Gafete => "gafete",
            Self::Medio => "medio",
            Self::Usuario => "usuario",
        }
    }
}

/// Columnas de la tabla de Historial. A diferencia de `ColumnaBusqueda`/
/// `ColumnaActivos`, todavía no hay un `SelectorColumnas<ColumnaHistorial>`
/// en `AppState` — la primera versión muestra siempre las 7, sin F4. El
/// enum ya implementa `Columna` para reusar `anchos_columnas`/
/// `fila_columnas` de `render.rs` tal cual; sumarle F4 más adelante es sólo
/// agregar el campo a `AppState`, no rehacer el render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnaHistorial {
    Ingreso,
    Nombre,
    Empresa,
    Tipo,
    Gafete,
    Salida,
    Usuario,
}

impl Columna for ColumnaHistorial {
    const TODAS: &'static [Self] = &[
        Self::Ingreso,
        Self::Nombre,
        Self::Empresa,
        Self::Tipo,
        Self::Gafete,
        Self::Salida,
        Self::Usuario,
    ];

    fn etiqueta(self) -> &'static str {
        match self {
            Self::Ingreso => "Ingreso",
            Self::Nombre => "Nombre",
            Self::Empresa => "Empresa",
            Self::Tipo => "Tipo",
            Self::Gafete => "Gafete",
            Self::Salida => "Salida",
            Self::Usuario => "Da ingreso",
        }
    }

    fn clave(self) -> &'static str {
        match self {
            Self::Ingreso => "ingreso_col",
            Self::Nombre => "nombre",
            Self::Empresa => "empresa",
            Self::Tipo => "tipo",
            Self::Gafete => "gafete",
            Self::Salida => "salida_col",
            Self::Usuario => "usuario",
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
        for indice in 0..7 {
            selector.alternar(indice).unwrap();
        }
        // Sólo Estado (índice 7) sigue visible.
        assert_eq!(selector.iter().filter(|(_, v)| *v).count(), 1);
        assert!(selector.alternar(7).is_err());
        assert!(selector.visible(ColumnaBusqueda::Estado));
    }

    #[test]
    fn preferencia_serializa_solo_las_visibles_en_orden() {
        let mut selector = SelectorColumnas::<ColumnaBusqueda>::todas_visibles();
        selector.alternar(0).unwrap(); // oculta Cédula
        assert_eq!(
            selector.preferencia(),
            "nombre,empresa,tipo,praind,ruta,acceso,estado"
        );
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
        let todas = "cedula,nombre,empresa,tipo,praind,ruta,acceso,estado";
        selector.aplicar_preferencia("");
        assert_eq!(selector.preferencia(), todas);
        selector.aplicar_preferencia("xyz,otra");
        assert_eq!(selector.preferencia(), todas);
    }
}
