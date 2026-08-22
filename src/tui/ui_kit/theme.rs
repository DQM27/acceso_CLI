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
    /// Verde fósforo y ámbar sobre fondo oscuro: consola clásica intencional.
    Classic,
    /// Cian sobrio, cercano a la identidad visual actual de Brisas.
    Brisas,
    /// Carbón y lavanda: interfaz oscura inspirada en herramientas de terminal modernas.
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
            Self::Classic => Theme {
                background: Color::Black,
                text: Color::Rgb(196, 220, 199),
                muted: Color::Rgb(105, 135, 111),
                accent: Color::Rgb(91, 224, 123),
                success: Color::Rgb(91, 224, 123),
                warning: Color::Rgb(238, 184, 78),
                danger: Color::Rgb(239, 105, 101),
                border: Color::Rgb(76, 119, 85),
                selection_foreground: Color::Black,
                selection_background: Color::Rgb(91, 224, 123),
                navegacion_pestanas: false,
            },
            Self::Brisas => Theme {
                background: Color::Black,
                text: Color::Rgb(220, 225, 228),
                muted: Color::Rgb(145, 158, 164),
                accent: Color::Rgb(70, 200, 215),
                success: Color::Rgb(95, 190, 125),
                warning: Color::Rgb(220, 170, 70),
                danger: Color::Rgb(220, 95, 100),
                border: Color::Rgb(100, 120, 126),
                selection_foreground: Color::Black,
                selection_background: Color::Rgb(70, 200, 215),
                navegacion_pestanas: false,
            },
            Self::Negro => Theme {
                background: Color::Rgb(36, 36, 39),
                text: Color::Rgb(232, 232, 235),
                muted: Color::Rgb(148, 148, 158),
                accent: Color::Rgb(184, 177, 255),
                success: Color::Rgb(134, 217, 160),
                warning: Color::Rgb(231, 198, 107),
                danger: Color::Rgb(240, 140, 140),
                border: Color::Rgb(86, 86, 94),
                selection_foreground: Color::Rgb(24, 24, 27),
                selection_background: Color::Rgb(232, 232, 235),
                navegacion_pestanas: true,
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
            Self::Classic => "CLÁSICO",
            Self::Brisas => "BRISAS",
            Self::Negro => "NEGRO",
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

    /// Selección no cromática para pestañas: invierte fondo y texto incluso
    /// cuando la terminal no distingue bien los colores del preset.
    pub fn selected_tab(self) -> Style {
        Style::default()
            .fg(self.background)
            .bg(self.text)
            .add_modifier(Modifier::BOLD)
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
