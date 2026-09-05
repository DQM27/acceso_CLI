use ratatui::style::{Color, Modifier, Style};

/// Paleta semántica: las vistas expresan intención, no colores concretos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub background: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub border: Color,
    pub selection_foreground: Color,
    pub selection_background: Color,
    /// Sólo el tema Negro navega por pestañas — Classic/Brisas conservan el
    /// Menú Principal de siempre. No es una preferencia de color: cambia qué
    /// pantalla aparece tras iniciar sesión (`App::sincronizar_vista_con_tema`).
    pub navegacion_pestanas: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    /// Brisas claro; conserva la clave histórica de preferencias.
    Classic,
    /// Brisas oscuro.
    Brisas,
    /// Brisas oscuro con navegación por pestañas; conserva la clave histórica.
    Negro,
}

impl ThemePreset {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Brisas => "brisas",
            Self::Negro => "negro",
        }
    }

    pub fn from_key(value: &str) -> Option<Self> {
        match value {
            "classic" => Some(Self::Classic),
            "brisas" => Some(Self::Brisas),
            "negro" => Some(Self::Negro),
            _ => None,
        }
    }

    pub const fn theme(self) -> Theme {
        match self {
            Self::Classic => crate::diseno_generado::LIGHT,
            Self::Brisas => crate::diseno_generado::DARK,
            Self::Negro => Theme {
                navegacion_pestanas: true,
                ..crate::diseno_generado::DARK
            },
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Classic => Self::Brisas,
            Self::Brisas => Self::Negro,
            Self::Negro => Self::Classic,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Classic => "BRISAS CLARO",
            Self::Brisas => "BRISAS OSCURO",
            Self::Negro => "BRISAS OSCURO · PESTAÑAS",
        }
    }
}

impl Theme {
    pub fn base(self) -> Style {
        Style::default().fg(self.text).bg(self.background)
    }

    pub fn title(self) -> Style {
        self.base().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn muted(self) -> Style {
        self.base().fg(self.muted)
    }

    pub fn accent(self) -> Style {
        self.base().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn border(self) -> Style {
        self.base().fg(self.border)
    }

    pub fn selected(self) -> Style {
        Style::default()
            .fg(self.selection_foreground)
            .bg(self.selection_background)
            .add_modifier(Modifier::BOLD)
    }

    /// Pestaña activa con el mismo azul marino de las demás selecciones.
    pub fn selected_tab(self) -> Style {
        self.selected()
    }

    pub fn success(self) -> Style {
        self.base().fg(self.success)
    }

    pub fn warning(self) -> Style {
        self.base().fg(self.warning)
    }

    pub fn danger(self) -> Style {
        self.base().fg(self.danger)
    }
}
