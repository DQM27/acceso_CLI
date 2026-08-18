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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    /// Verde fósforo y ámbar sobre fondo oscuro: consola clásica intencional.
    Classic,
    /// Cian sobrio, cercano a la identidad visual actual de Brisas.
    Brisas,
}

impl ThemePreset {
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
            },
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Classic => Self::Brisas,
            Self::Brisas => Self::Classic,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Classic => "CLÁSICO",
            Self::Brisas => "BRISAS",
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
