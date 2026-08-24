//! Formulario de alta de usuario para `--comandos` — mismo patrón de
//! Surface enclavada que `formulario.rs` (contratista): campos, navegación,
//! validación local y una tarjeta de Resumen antes de persistir (acá sí
//! vale la pena: password/confirmación/rol son varios campos con
//! consecuencias reales — a diferencia de Empresa, un solo campo, donde
//! una segunda pantalla habría sido fricción sin valor).
//!
//! El Resumen nunca muestra la contraseña en texto — sólo confirma que se
//! definió, igual criterio que el enmascarado del propio campo mientras se
//! teclea (nunca se ve en pantalla, ni siquiera en la revisión final).

use crate::domain::autorizacion::puede_gestionar_usuario;
use crate::models::usuario::RolUsuario;
use crate::services::usuario_service::CrearUsuarioInput;

/// Mismo mínimo que exige `usuario_service` (constante privada ahí, no
/// importable) — validar acá evita un viaje a `AppCore` por algo que ya se
/// sabe de antemano, igual criterio que ya usaba la TUI clásica.
const LONGITUD_MINIMA_PASSWORD: usize = 8;
pub const MAX_CEDULA: usize = 30;
pub const MAX_NOMBRE: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampoUsuario {
    Cedula,
    Nombre,
    Rol,
    Password,
    ConfirmarPassword,
}

impl CampoUsuario {
    pub const ORDEN: [Self; 5] = [
        Self::Cedula,
        Self::Nombre,
        Self::Rol,
        Self::Password,
        Self::ConfirmarPassword,
    ];

    pub fn etiqueta(self) -> &'static str {
        match self {
            Self::Cedula => "Cédula",
            Self::Nombre => "Nombre",
            Self::Rol => "Rol",
            Self::Password => "Contraseña",
            Self::ConfirmarPassword => "Confirmar",
        }
    }

    pub fn es_texto(self) -> bool {
        matches!(
            self,
            Self::Cedula | Self::Nombre | Self::Password | Self::ConfirmarPassword
        )
    }

    /// Password/Confirmar nunca se muestran en texto plano — ni mientras se
    /// teclea ni en el Resumen.
    pub fn es_secreto(self) -> bool {
        matches!(self, Self::Password | Self::ConfirmarPassword)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubfaseUsuario {
    Editando,
    Resumen,
}

#[derive(Debug, Clone)]
pub struct FormularioUsuario {
    pub campo: CampoUsuario,
    pub subfase: SubfaseUsuario,
    pub cedula: String,
    pub nombre: String,
    pub rol: RolUsuario,
    pub password: String,
    pub confirmar_password: String,
    /// Rol de quien está creando — determina qué roles puede asignar
    /// (`puede_gestionar_usuario`: nadie salvo Root puede crear otro Root).
    rol_actor: RolUsuario,
    pub errores: Vec<(CampoUsuario, String)>,
}

impl FormularioUsuario {
    pub fn nuevo(rol_actor: RolUsuario) -> Self {
        Self {
            campo: CampoUsuario::Cedula,
            subfase: SubfaseUsuario::Editando,
            cedula: String::new(),
            nombre: String::new(),
            // Menor privilegio por defecto — quien crea el usuario tiene
            // que subir el rol a propósito, nunca al revés.
            rol: RolUsuario::Operador,
            password: String::new(),
            confirmar_password: String::new(),
            rol_actor,
            errores: Vec::new(),
        }
    }

    pub fn mover_campo(&mut self, delta: isize) -> bool {
        let total = CampoUsuario::ORDEN.len() as isize;
        let actual = CampoUsuario::ORDEN
            .iter()
            .position(|c| *c == self.campo)
            .unwrap_or(0) as isize;
        let nuevo = (actual + delta).clamp(0, total - 1) as usize;
        let cambio = CampoUsuario::ORDEN[nuevo] != self.campo;
        self.campo = CampoUsuario::ORDEN[nuevo];
        cambio
    }

    /// Space/←/→ sobre Rol: cicla entre los roles que `rol_actor` puede
    /// asignar (`puede_gestionar_usuario`) — un Administrador nunca ve Root
    /// como opción, ni por accidente.
    pub fn alternar(&mut self) {
        if self.campo != CampoUsuario::Rol {
            return;
        }
        const ROLES: [RolUsuario; 3] = [
            RolUsuario::Operador,
            RolUsuario::Administrador,
            RolUsuario::Root,
        ];
        let permitidos: Vec<RolUsuario> = ROLES
            .into_iter()
            .filter(|r| puede_gestionar_usuario(self.rol_actor, *r))
            .collect();
        if permitidos.is_empty() {
            return;
        }
        let indice = permitidos.iter().position(|r| *r == self.rol).unwrap_or(0);
        self.rol = permitidos[(indice + 1) % permitidos.len()];
    }

    pub fn texto_campo(&self) -> Option<&str> {
        match self.campo {
            CampoUsuario::Cedula => Some(&self.cedula),
            CampoUsuario::Nombre => Some(&self.nombre),
            CampoUsuario::Password => Some(&self.password),
            CampoUsuario::ConfirmarPassword => Some(&self.confirmar_password),
            CampoUsuario::Rol => None,
        }
    }

    /// Mismo filtro que Contratista: cédula sólo dígitos, nombre sólo
    /// letras/espacios/guion/apóstrofo. La contraseña no se filtra por
    /// carácter (cualquier símbolo es válido en una contraseña).
    pub fn asignar_texto(&mut self, texto: &str) {
        match self.campo {
            CampoUsuario::Cedula => {
                self.cedula = texto
                    .chars()
                    .filter(char::is_ascii_digit)
                    .take(MAX_CEDULA)
                    .collect();
            }
            CampoUsuario::Nombre => {
                self.nombre = texto
                    .chars()
                    .filter(|c| c.is_alphabetic() || c.is_whitespace() || *c == '-' || *c == '\'')
                    .take(MAX_NOMBRE)
                    .collect();
            }
            CampoUsuario::Password => self.password = texto.to_string(),
            CampoUsuario::ConfirmarPassword => self.confirmar_password = texto.to_string(),
            CampoUsuario::Rol => {}
        }
        self.errores.retain(|(campo, _)| *campo != self.campo);
    }

    pub fn error_de(&self, campo: CampoUsuario) -> Option<&str> {
        self.errores
            .iter()
            .find(|(c, _)| *c == campo)
            .map(|(_, m)| m.as_str())
    }

    pub fn validar(&mut self) -> Result<CrearUsuarioInput, Vec<(CampoUsuario, String)>> {
        let mut errores = Vec::new();
        let cedula = self.cedula.trim().to_string();
        if cedula.is_empty() {
            errores.push((CampoUsuario::Cedula, "Escriba la cédula".to_string()));
        }
        let nombre = self.nombre.trim().to_string();
        if nombre.is_empty() {
            errores.push((CampoUsuario::Nombre, "Escriba el nombre".to_string()));
        }
        if self.password.chars().count() < LONGITUD_MINIMA_PASSWORD {
            errores.push((
                CampoUsuario::Password,
                format!("Mínimo {LONGITUD_MINIMA_PASSWORD} caracteres"),
            ));
        } else if self.password != self.confirmar_password {
            errores.push((
                CampoUsuario::ConfirmarPassword,
                "Las contraseñas no coinciden".to_string(),
            ));
        }

        if errores.is_empty() {
            self.errores.clear();
            Ok(CrearUsuarioInput {
                cedula,
                nombre,
                password: self.password.clone(),
                rol: self.rol,
                activo: true,
            })
        } else {
            self.errores = errores.clone();
            Err(errores)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> FormularioUsuario {
        FormularioUsuario::nuevo(RolUsuario::Root)
    }

    fn valido() -> FormularioUsuario {
        let mut f = form();
        f.cedula = "119430546".to_string();
        f.nombre = "Carlos Pérez".to_string();
        f.password = "clave1234".to_string();
        f.confirmar_password = "clave1234".to_string();
        f
    }

    #[test]
    fn navegacion_recorre_los_cinco_campos() {
        let mut f = form();
        let esperado = [
            CampoUsuario::Nombre,
            CampoUsuario::Rol,
            CampoUsuario::Password,
            CampoUsuario::ConfirmarPassword,
        ];
        for campo in esperado {
            assert!(f.mover_campo(1));
            assert_eq!(f.campo, campo);
        }
        assert!(!f.mover_campo(1));
    }

    #[test]
    fn rol_por_defecto_es_operador() {
        assert_eq!(form().rol, RolUsuario::Operador);
    }

    #[test]
    fn administrador_no_puede_ciclar_hasta_root() {
        let mut f = FormularioUsuario::nuevo(RolUsuario::Administrador);
        f.campo = CampoUsuario::Rol;
        f.alternar();
        assert_eq!(f.rol, RolUsuario::Administrador);
        f.alternar();
        assert_eq!(f.rol, RolUsuario::Operador);
        assert_ne!(f.rol, RolUsuario::Root);
    }

    #[test]
    fn root_puede_ciclar_los_tres_roles() {
        let mut f = FormularioUsuario::nuevo(RolUsuario::Root);
        f.campo = CampoUsuario::Rol;
        f.alternar();
        assert_eq!(f.rol, RolUsuario::Administrador);
        f.alternar();
        assert_eq!(f.rol, RolUsuario::Root);
    }

    #[test]
    fn cedula_solo_admite_digitos() {
        let mut f = form();
        f.campo = CampoUsuario::Cedula;
        f.asignar_texto("1a1b9c4-3 0546");
        assert_eq!(f.cedula, "119430546");
    }

    #[test]
    fn password_no_filtra_caracteres() {
        let mut f = form();
        f.campo = CampoUsuario::Password;
        f.asignar_texto("Clave#123!ñ");
        assert_eq!(f.password, "Clave#123!ñ");
    }

    #[test]
    fn password_corta_es_error() {
        let mut f = valido();
        f.password = "corta".to_string();
        let errores = f.validar().err().expect("password corta no valida");
        assert!(errores.iter().any(|(c, _)| *c == CampoUsuario::Password));
    }

    #[test]
    fn passwords_distintas_es_error() {
        let mut f = valido();
        f.confirmar_password = "otraclave".to_string();
        let errores = f.validar().err().expect("no coinciden");
        assert!(
            errores
                .iter()
                .any(|(c, _)| *c == CampoUsuario::ConfirmarPassword)
        );
    }

    #[test]
    fn formulario_valido_produce_input_con_activo_true() {
        let mut f = valido();
        let input = f.validar().expect("valido");
        assert_eq!(input.cedula, "119430546");
        assert!(input.activo);
        assert_eq!(input.rol, RolUsuario::Operador);
    }
}
