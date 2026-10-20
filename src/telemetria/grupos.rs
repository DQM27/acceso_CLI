//! Los estudios de telemetría y el catálogo de eventos de cada uno.
//!
//! La lista de grupos es fija y la misma que valida la nube
//! (`telemetria_ordenes.grupos`, migración `20261020120000`). Agregar un grupo
//! es un cambio en las dos puntas.

/// Un estudio de telemetría.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Grupo {
    /// Estado de la flota y caídas. Va siempre y no se puede apagar.
    Base,
    /// Tiempo de las operaciones y de los flujos que vive el guarda.
    Rendimiento,
    Fluidez,
    Sincronizacion,
    Recursos,
    Escaneo,
    /// Cuesta (`StrictMode`, vigilante de fugas): sólo por poco tiempo.
    Depuracion,
}

impl Grupo {
    pub const TODOS: [Self; 7] = [
        Self::Base,
        Self::Rendimiento,
        Self::Fluidez,
        Self::Sincronizacion,
        Self::Recursos,
        Self::Escaneo,
        Self::Depuracion,
    ];

    /// El nombre con que viaja en la orden y en la base.
    pub const fn nombre(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Rendimiento => "rendimiento",
            Self::Fluidez => "fluidez",
            Self::Sincronizacion => "sincronizacion",
            Self::Recursos => "recursos",
            Self::Escaneo => "escaneo",
            Self::Depuracion => "depuracion",
        }
    }

    pub fn desde_nombre(nombre: &str) -> Option<Self> {
        Self::TODOS.into_iter().find(|g| g.nombre() == nombre)
    }

    const fn bit(self) -> u8 {
        match self {
            Self::Base => 1,
            Self::Rendimiento => 1 << 1,
            Self::Fluidez => 1 << 2,
            Self::Sincronizacion => 1 << 3,
            Self::Recursos => 1 << 4,
            Self::Escaneo => 1 << 5,
            Self::Depuracion => 1 << 6,
        }
    }
}

/// Un conjunto de grupos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grupos(u8);

impl Grupos {
    pub const VACIO: Self = Self(0);

    /// Lo que mide un equipo sin orden vigente: `base` y `rendimiento`.
    pub const POR_DEFECTO: Self = Self(Grupo::Base.bit() | Grupo::Rendimiento.bit());

    pub const fn con(self, grupo: Grupo) -> Self {
        Self(self.0 | grupo.bit())
    }

    pub const fn contiene(self, grupo: Grupo) -> bool {
        self.0 & grupo.bit() != 0
    }

    pub(crate) const fn bits(self) -> u8 {
        self.0
    }

    pub(crate) const fn desde_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// Los grupos de una lista de nombres. Un nombre desconocido se ignora
    /// (una nube más nueva puede mandar un grupo que esta versión no conoce) y
    /// `base` también: va siempre y no forma parte de la orden.
    pub fn desde_nombres<'a>(nombres: impl IntoIterator<Item = &'a str>) -> Self {
        nombres
            .into_iter()
            .filter_map(|n| Grupo::desde_nombre(n.trim()))
            .filter(|g| *g != Grupo::Base)
            .fold(Self::VACIO, Self::con)
    }

    /// Los nombres de la orden, sin `base`, en un orden estable.
    pub fn nombres_de_la_orden(self) -> Vec<&'static str> {
        Grupo::TODOS
            .into_iter()
            .filter(|g| *g != Grupo::Base && self.contiene(*g))
            .map(Grupo::nombre)
            .collect()
    }
}

/// A qué grupo pertenece un tipo de evento, o `None` si no está en el catálogo.
/// Un evento nuevo se asigna a un grupo acá antes de usarse.
pub fn grupo_de_evento(tipo: &str) -> Option<Grupo> {
    match tipo {
        "sesion_inicio" | "arranque" | "salida_anterior" | "cierre" | "panic" | "error_js"
        | "error_telemetria" => Some(Grupo::Base),
        "llamada_nucleo" | "transicion_ingreso" | "pantalla" => Some(Grupo::Rendimiento),
        "frames" | "ui_bloqueos" => Some(Grupo::Fluidez),
        "realtime" | "sincronizacion_auto" | "sesion_unidad" | "reloj" => {
            Some(Grupo::Sincronizacion)
        }
        "muestra_sistema" | "muestra_webview" | "memoria_baja" => Some(Grupo::Recursos),
        "ocr_sesion" | "camara_info" => Some(Grupo::Escaneo),
        "strictmode" | "retencion" => Some(Grupo::Depuracion),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_grupo_se_reconoce_por_su_nombre() {
        for grupo in Grupo::TODOS {
            assert_eq!(Grupo::desde_nombre(grupo.nombre()), Some(grupo));
        }
        assert_eq!(Grupo::desde_nombre("inventado"), None);
    }

    #[test]
    fn los_bits_de_los_grupos_no_se_pisan() {
        let mut vistos = 0u8;
        for grupo in Grupo::TODOS {
            assert_eq!(vistos & grupo.bit(), 0, "{grupo:?} repite un bit");
            vistos |= grupo.bit();
        }
    }

    #[test]
    fn por_defecto_es_base_y_rendimiento() {
        assert!(Grupos::POR_DEFECTO.contiene(Grupo::Base));
        assert!(Grupos::POR_DEFECTO.contiene(Grupo::Rendimiento));
        assert!(!Grupos::POR_DEFECTO.contiene(Grupo::Fluidez));
    }

    #[test]
    fn de_los_nombres_se_ignoran_los_desconocidos_y_base() {
        let grupos = Grupos::desde_nombres(["rendimiento", " fluidez ", "futuro", "base", ""]);
        assert!(grupos.contiene(Grupo::Rendimiento));
        assert!(grupos.contiene(Grupo::Fluidez));
        assert!(!grupos.contiene(Grupo::Base));
        assert_eq!(grupos.nombres_de_la_orden(), ["rendimiento", "fluidez"]);
    }

    #[test]
    fn los_nombres_de_la_orden_salen_en_orden_estable_y_sin_base() {
        let grupos = Grupos::desde_nombres(["depuracion", "recursos", "rendimiento"]);
        assert_eq!(
            grupos.nombres_de_la_orden(),
            ["rendimiento", "recursos", "depuracion"]
        );
        assert!(Grupos::VACIO.nombres_de_la_orden().is_empty());
    }

    #[test]
    fn el_catalogo_asigna_cada_evento_conocido() {
        assert_eq!(grupo_de_evento("llamada_nucleo"), Some(Grupo::Rendimiento));
        assert_eq!(grupo_de_evento("panic"), Some(Grupo::Base));
        assert_eq!(grupo_de_evento("frames"), Some(Grupo::Fluidez));
        assert_eq!(grupo_de_evento("reloj"), Some(Grupo::Sincronizacion));
        assert_eq!(grupo_de_evento("muestra_sistema"), Some(Grupo::Recursos));
        assert_eq!(grupo_de_evento("ocr_sesion"), Some(Grupo::Escaneo));
        assert_eq!(grupo_de_evento("strictmode"), Some(Grupo::Depuracion));
        assert_eq!(grupo_de_evento("desconocido"), None);
    }
}
