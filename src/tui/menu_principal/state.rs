use crossterm::event::{KeyCode, KeyEvent};

use crate::tui::ui_kit::{StandardCommand, standard_command};

#[path = "render.rs"]
pub(super) mod render;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpcionMenu {
    NuevoIngreso,
    IngresosActivos,
    Historial,
    Contratistas,
    Empresas,
    Usuarios,
    CerrarSesion,
    Salir,
}

impl OpcionMenu {
    pub const TODAS: [Self; 8] = [
        Self::NuevoIngreso,
        Self::IngresosActivos,
        Self::Historial,
        Self::Contratistas,
        Self::Empresas,
        Self::Usuarios,
        Self::CerrarSesion,
        Self::Salir,
    ];

    pub fn etiqueta(self) -> &'static str {
        match self {
            Self::NuevoIngreso => "1   Nuevo ingreso",
            Self::IngresosActivos => "2   Ingresos activos",
            Self::Historial => "3   Historial",
            Self::Contratistas => "4   Contratistas",
            Self::Empresas => "5   Empresas",
            Self::Usuarios => "6   Usuarios",
            Self::CerrarSesion => "L   Cerrar sesión",
            Self::Salir => "Q   Salir",
        }
    }

    pub fn descripcion(self) -> &'static str {
        match self {
            Self::NuevoIngreso => "Registrar la entrada de un contratista.",
            Self::IngresosActivos => "Consultar personas actualmente dentro.",
            Self::Historial => "Consultar movimientos de ingreso y salida.",
            Self::Contratistas => "Administrar la base de contratistas.",
            Self::Empresas => "Administrar empresas registradas.",
            Self::Usuarios => "Administrar usuarios del sistema.",
            Self::CerrarSesion => "Volver a la pantalla de autenticación.",
            Self::Salir => "Cerrar BRISAS CLI.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmacionMenu {
    CerrarSesion,
    Salir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionMenu {
    Ninguna,
    Abrir(OpcionMenu),
    CerrarSesion,
    Salir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuPrincipalState {
    pub seleccion: OpcionMenu,
    pub confirmacion: Option<ConfirmacionMenu>,
    pub ayuda_expandida: bool,
}

impl Default for MenuPrincipalState {
    fn default() -> Self {
        Self {
            seleccion: OpcionMenu::NuevoIngreso,
            confirmacion: None,
            ayuda_expandida: false,
        }
    }
}

impl MenuPrincipalState {
    pub fn nueva_sesion(&mut self) {
        self.seleccion = OpcionMenu::NuevoIngreso;
        self.confirmacion = None;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AccionMenu {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionMenu::Ninguna;
        }
        if let Some(confirmacion) = self.confirmacion {
            return match key.code {
                KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
                    self.confirmacion = None;
                    match confirmacion {
                        ConfirmacionMenu::CerrarSesion => AccionMenu::CerrarSesion,
                        ConfirmacionMenu::Salir => AccionMenu::Salir,
                    }
                }
                KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                    self.confirmacion = None;
                    AccionMenu::Ninguna
                }
                _ => AccionMenu::Ninguna,
            };
        }
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => return self.abrir_seleccion(),
            KeyCode::Char('1') => return AccionMenu::Abrir(OpcionMenu::NuevoIngreso),
            KeyCode::Char('2') => return AccionMenu::Abrir(OpcionMenu::IngresosActivos),
            KeyCode::Char('3') => return AccionMenu::Abrir(OpcionMenu::Historial),
            KeyCode::Char('4') => return AccionMenu::Abrir(OpcionMenu::Contratistas),
            KeyCode::Char('5') => return AccionMenu::Abrir(OpcionMenu::Empresas),
            KeyCode::Char('6') => return AccionMenu::Abrir(OpcionMenu::Usuarios),
            KeyCode::Char('l' | 'L') => self.solicitar(ConfirmacionMenu::CerrarSesion),
            KeyCode::Char('q' | 'Q') => self.solicitar(ConfirmacionMenu::Salir),
            _ => {}
        }
        AccionMenu::Ninguna
    }

    fn mover(&mut self, delta: isize) {
        let actual = OpcionMenu::TODAS
            .iter()
            .position(|o| *o == self.seleccion)
            .unwrap_or(0);
        let nuevo = if delta < 0 {
            actual.saturating_sub(1)
        } else {
            (actual + 1).min(OpcionMenu::TODAS.len() - 1)
        };
        self.seleccion = OpcionMenu::TODAS[nuevo];
    }

    fn abrir_seleccion(&mut self) -> AccionMenu {
        match self.seleccion {
            OpcionMenu::CerrarSesion => {
                self.solicitar(ConfirmacionMenu::CerrarSesion);
                AccionMenu::Ninguna
            }
            OpcionMenu::Salir => {
                self.solicitar(ConfirmacionMenu::Salir);
                AccionMenu::Ninguna
            }
            opcion => AccionMenu::Abrir(opcion),
        }
    }

    fn solicitar(&mut self, confirmacion: ConfirmacionMenu) {
        self.confirmacion = Some(confirmacion);
    }
}
