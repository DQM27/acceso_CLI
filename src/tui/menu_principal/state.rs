use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    models::usuario::RolUsuario,
    tui::ui_kit::{StandardCommand, TabBar, TabItem, standard_command},
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
    GestionGafetes,
    Cli,
    CerrarSesion,
    Salir,
}

impl OpcionMenu {
    pub const TODAS: [Self; 13] = [
        Self::NuevoIngreso,
        Self::IngresosActivos,
        Self::Historial,
        Self::Contratistas,
        Self::Empresas,
        Self::Usuarios,
        Self::Auditoria,
        Self::Respaldos,
        Self::CambiarPassword,
        Self::GestionGafetes,
        Self::Cli,
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
            Self::GestionGafetes => "G   Gestión de gafetes",
            Self::Cli => "M   Modo CLI",
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
            Self::GestionGafetes => "Catálogo de gafetes: alta, baja, pérdidas y deudas.",
            Self::Cli => "Reiniciar en la interfaz CLI y dejarla como default.",
            Self::CerrarSesion => "Volver a la pantalla de autenticación.",
            Self::Salir => "Cerrar BRISAS CLI.",
        }
    }

    /// Sólo ROOT y Administrador administran usuarios y ajustes del sistema — un
    /// Operador con acceso a Usuarios podría autopromoverse a Administrador o Root.
    fn visible_para(self, rol: RolUsuario) -> bool {
        match self {
            Self::Usuarios | Self::Auditoria => rol != RolUsuario::Operador,
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

    pub fn pestanas_para(rol: RolUsuario) -> Vec<Self> {
        Self::TODAS
            .into_iter()
            .filter(|opcion| opcion.es_pestana() && opcion.visible_para(rol))
            .collect()
    }

    pub fn barra_pestanas(rol: RolUsuario, activa: Self) -> TabBar {
        let opciones = Self::pestanas_para(rol);
        let selected = opciones
            .iter()
            .position(|opcion| *opcion == activa)
            .unwrap_or_default();
        let items = opciones.into_iter().filter_map(Self::tab_item).collect();
        TabBar::new(items, selected)
    }

    pub const fn desde_atajo(character: char) -> Option<Self> {
        match character {
            '1' => Some(Self::NuevoIngreso),
            '2' => Some(Self::IngresosActivos),
            '3' => Some(Self::Historial),
            '4' => Some(Self::Contratistas),
            '5' => Some(Self::Empresas),
            '6' => Some(Self::Usuarios),
            '7' => Some(Self::Auditoria),
            '8' => Some(Self::Respaldos),
            '9' => Some(Self::CambiarPassword),
            _ => None,
        }
    }

    pub const fn indice_pestana(self) -> Option<usize> {
        match self {
            Self::NuevoIngreso => Some(0),
            Self::IngresosActivos => Some(1),
            Self::Historial => Some(2),
            Self::Contratistas => Some(3),
            Self::Empresas => Some(4),
            Self::Usuarios => Some(5),
            Self::Auditoria => Some(6),
            Self::Respaldos => Some(7),
            Self::CambiarPassword => Some(8),
            Self::GestionGafetes | Self::Cli | Self::CerrarSesion | Self::Salir => None,
        }
    }

    const fn es_pestana(self) -> bool {
        self.indice_pestana().is_some()
    }

    const fn tab_item(self) -> Option<TabItem> {
        let item = match self {
            Self::NuevoIngreso => TabItem::new("1", "Nuevo ingreso", "Nuevo"),
            Self::IngresosActivos => TabItem::new("2", "Ingresos activos", "Activos"),
            Self::Historial => TabItem::new("3", "Historial", "Hist."),
            Self::Contratistas => TabItem::new("4", "Contratistas", "Contr."),
            Self::Empresas => TabItem::new("5", "Empresas", "Emp."),
            Self::Usuarios => TabItem::new("6", "Usuarios", "Usr."),
            Self::Auditoria => TabItem::new("7", "Auditoría", "Aud."),
            Self::Respaldos => TabItem::new("8", "Respaldos", "Resp."),
            Self::CambiarPassword => TabItem::new("9", "Mi contraseña", "Clave"),
            Self::GestionGafetes | Self::Cli | Self::CerrarSesion | Self::Salir => {
                return None;
            }
        };
        Some(item)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmacionMenu {
    CerrarSesion,
    Salir,
    Cli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccionMenu {
    Ninguna,
    Abrir(OpcionMenu),
    CerrarSesion,
    Salir,
    Cli,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuPrincipalState {
    pub seleccion: OpcionMenu,
    pub confirmacion: Option<ConfirmacionMenu>,
    pub ayuda_expandida: bool,
    /// Mensaje de por qué falló el último intento de respaldo automático —
    /// `None` si el más reciente tuvo éxito (o aún no hubo ninguno). Sólo se
    /// usa para decidir si mostrar el aviso genérico; el detalle vive en la
    /// pantalla Respaldos, no aquí.
    pub fallo_respaldo_automatico: Option<String>,
}

impl Default for MenuPrincipalState {
    fn default() -> Self {
        Self {
            seleccion: OpcionMenu::NuevoIngreso,
            confirmacion: None,
            ayuda_expandida: false,
            fallo_respaldo_automatico: None,
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
                        ConfirmacionMenu::Cli => AccionMenu::Cli,
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
            KeyCode::Char('g' | 'G') => return AccionMenu::Abrir(OpcionMenu::GestionGafetes),
            KeyCode::Char('m' | 'M') => self.solicitar(ConfirmacionMenu::Cli),
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
            OpcionMenu::Cli => {
                self.solicitar(ConfirmacionMenu::Cli);
                AccionMenu::Ninguna
            }
            opcion => AccionMenu::Abrir(opcion),
        }
    }

    fn solicitar(&mut self, confirmacion: ConfirmacionMenu) {
        self.confirmacion = Some(confirmacion);
    }
}
