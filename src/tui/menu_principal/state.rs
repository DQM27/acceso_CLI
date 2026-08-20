use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    models::usuario::RolUsuario,
    tui::ui_kit::{StandardCommand, standard_command},
};

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
    Auditoria,
    Respaldos,
    CambiarPassword,
    CerrarSesion,
    Salir,
}

impl OpcionMenu {
    pub const TODAS: [Self; 11] = [
        Self::NuevoIngreso,
        Self::IngresosActivos,
        Self::Historial,
        Self::Contratistas,
        Self::Empresas,
        Self::Usuarios,
        Self::Auditoria,
        Self::Respaldos,
        Self::CambiarPassword,
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
            Self::Auditoria => "7   Auditoría",
            Self::Respaldos => "8   Respaldos",
            Self::CambiarPassword => "9   Cambiar mi contraseña",
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
            Self::CambiarPassword => "Actualizar la contraseña de la sesión actual.",
            Self::Auditoria => "Consultar cambios en campos críticos de contratistas.",
            Self::Respaldos => "Crear, validar, exportar y restaurar respaldos.",
            Self::CerrarSesion => "Volver a la pantalla de autenticación.",
            Self::Salir => "Cerrar BRISAS CLI.",
        }
    }

    /// Sólo ROOT y Administrador administran usuarios y ajustes del sistema — un
    /// Operador con acceso a Usuarios podría autopromoverse a Administrador o Root.
    fn visible_para(self, rol: RolUsuario) -> bool {
        match self {
            Self::Usuarios => rol != RolUsuario::Operador,
            Self::Auditoria => rol != RolUsuario::Operador,
            Self::Respaldos => rol == RolUsuario::Root,
            _ => true,
        }
    }

    pub fn visibles_para(rol: RolUsuario) -> Vec<Self> {
        Self::TODAS
            .into_iter()
            .filter(|opcion| opcion.visible_para(rol))
            .collect()
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

    pub fn handle_key(&mut self, key: KeyEvent, rol: RolUsuario) -> AccionMenu {
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionMenu::Ninguna;
        }
        if let Some(confirmacion) = self.confirmacion {
            return match key.code {
                KeyCode::Enter => {
                    self.confirmacion = None;
                    match confirmacion {
                        ConfirmacionMenu::CerrarSesion => AccionMenu::CerrarSesion,
                        ConfirmacionMenu::Salir => AccionMenu::Salir,
                    }
                }
                KeyCode::Esc => {
                    self.confirmacion = None;
                    AccionMenu::Ninguna
                }
                _ => AccionMenu::Ninguna,
            };
        }
        let visibles = OpcionMenu::visibles_para(rol);
        match key.code {
            KeyCode::Up => self.mover(-1, &visibles),
            KeyCode::Down => self.mover(1, &visibles),
            KeyCode::Enter => return self.abrir_seleccion(),
            KeyCode::Char('1') => return AccionMenu::Abrir(OpcionMenu::NuevoIngreso),
            KeyCode::Char('2') => return AccionMenu::Abrir(OpcionMenu::IngresosActivos),
            KeyCode::Char('3') => return AccionMenu::Abrir(OpcionMenu::Historial),
            KeyCode::Char('4') => return AccionMenu::Abrir(OpcionMenu::Contratistas),
            KeyCode::Char('5') => return AccionMenu::Abrir(OpcionMenu::Empresas),
            KeyCode::Char('6') if visibles.contains(&OpcionMenu::Usuarios) => {
                return AccionMenu::Abrir(OpcionMenu::Usuarios);
            }
            KeyCode::Char('7') if visibles.contains(&OpcionMenu::Auditoria) => {
                return AccionMenu::Abrir(OpcionMenu::Auditoria);
            }
            KeyCode::Char('8') if visibles.contains(&OpcionMenu::Respaldos) => {
                return AccionMenu::Abrir(OpcionMenu::Respaldos);
            }
            KeyCode::Char('9') => {
                return AccionMenu::Abrir(OpcionMenu::CambiarPassword);
            }
            KeyCode::Char('l' | 'L') => self.solicitar(ConfirmacionMenu::CerrarSesion),
            KeyCode::Char('q' | 'Q') => self.solicitar(ConfirmacionMenu::Salir),
            _ => {}
        }
        AccionMenu::Ninguna
    }

    fn mover(&mut self, delta: isize, visibles: &[OpcionMenu]) {
        let actual = visibles
            .iter()
            .position(|o| *o == self.seleccion)
            .unwrap_or(0);
        let nuevo = if delta < 0 {
            actual.saturating_sub(1)
        } else {
            (actual + 1).min(visibles.len() - 1)
        };
        self.seleccion = visibles[nuevo];
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
