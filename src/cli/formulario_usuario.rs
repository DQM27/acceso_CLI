//! Formulario de alta de usuario para `--cli` — mismo patrón de
//! Surface enclavada que `formulario.rs` (contratista): campos, navegación,
//! validación local y una tarjeta de Resumen antes de persistir (acá sí
//! vale la pena: password/confirmación/rol son varios campos con
//! consecuencias reales — a diferencia de Empresa, un solo campo, donde
//! una segunda pantalla habría sido fricción sin valor).
//!
//! El Resumen nunca muestra la contraseña en texto — sólo confirma que se
//! definió, igual criterio que el enmascarado del propio campo mientras se
//! teclea (nunca se ve en pantalla, ni siquiera en la revisión final).

use crate::database::queries::usuarios::UsuarioResumen;
use crate::domain::autorizacion::puede_gestionar_usuario;
use crate::models::usuario::RolUsuario;
use crate::services::usuario_service::{ActualizarUsuarioInput, CrearUsuarioInput};

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

/// `Editar` guarda el `id` (para `AppCore::actualizar_usuario`) y el
/// `activo` vigente — no hay campo para tocarlo en este formulario (alta
/// separada del activar/desactivar, mismo corte de alcance que Empresa),
/// así que se conserva tal cual para no pisarlo al guardar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoFormularioUsuario {
    Nuevo,
    Editar { id: i64, activo: bool },
}

/// Lo que produce `validar()` según el modo — alta necesita una contraseña
/// siempre; edición la deja opcional (`None` = no cambiarla).
pub enum DatosUsuario {
    Crear(CrearUsuarioInput),
    Actualizar {
        id: i64,
        datos: ActualizarUsuarioInput,
        activo: bool,
        /// `Some` sólo si el operador escribió algo en Contraseña/Confirmar.
        password: Option<String>,
    },
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
    pub modo: ModoFormularioUsuario,
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
            modo: ModoFormularioUsuario::Nuevo,
            rol_actor,
            errores: Vec::new(),
        }
    }

    /// Precarga cédula/nombre/rol desde la búsqueda que trajo hasta acá —
    /// contraseña siempre en blanco (blanco = no cambiarla, ver `validar`).
    pub fn editar(resumen: &UsuarioResumen, rol_actor: RolUsuario) -> Self {
        Self {
            campo: CampoUsuario::Cedula,
            subfase: SubfaseUsuario::Editando,
            cedula: resumen.cedula.clone(),
            nombre: resumen.nombre.clone(),
            rol: resumen.rol,
            password: String::new(),
            confirmar_password: String::new(),
            modo: ModoFormularioUsuario::Editar {
                id: resumen.id,
                activo: resumen.activo,
            },
            rol_actor,
            errores: Vec::new(),
        }
    }

    pub fn mover_campo(&mut self, delta: isize) -> bool {
        let total = isize::try_from(CampoUsuario::ORDEN.len()).unwrap_or(isize::MAX);
        let actual = isize::try_from(
            CampoUsuario::ORDEN
                .iter()
                .position(|c| *c == self.campo)
                .unwrap_or(0),
        )
        .unwrap_or(isize::MAX);
        let nuevo = usize::try_from((actual + delta).clamp(0, total - 1)).unwrap_or(0);
        let cambio = CampoUsuario::ORDEN[nuevo] != self.campo;
        self.campo = CampoUsuario::ORDEN[nuevo];
        cambio
    }

    /// Space/←/→ sobre Rol: cicla entre los roles que `rol_actor` puede
    /// asignar (`puede_gestionar_usuario`) — un Administrador nunca ve Root
    /// como opción, ni por accidente.
    pub fn alternar(&mut self) {
        const ROLES: [RolUsuario; 3] = [
            RolUsuario::Operador,
            RolUsuario::Administrador,
            RolUsuario::Root,
        ];

        if self.campo != CampoUsuario::Rol {
            return;
        }
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

    pub fn validar(&mut self) -> Result<DatosUsuario, Vec<(CampoUsuario, String)>> {
        let mut errores = Vec::new();
        let cedula = self.cedula.trim().to_string();
        if cedula.is_empty() {
            errores.push((CampoUsuario::Cedula, "Escriba la cédula".to_string()));
        }
        let nombre = self.nombre.trim().to_string();
        if nombre.is_empty() {
            errores.push((CampoUsuario::Nombre, "Escriba el nombre".to_string()));
        }

        // En alta la contraseña siempre hace falta; en edición, dejar ambos
        // campos en blanco significa "no cambiarla" — sólo se valida si el
        // operador escribió algo en cualquiera de los dos.
        let cambia_password = matches!(self.modo, ModoFormularioUsuario::Nuevo)
            || !self.password.is_empty()
            || !self.confirmar_password.is_empty();
        if cambia_password {
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
        }

        if !errores.is_empty() {
            self.errores.clone_from(&errores);
            return Err(errores);
        }
        self.errores.clear();

        Ok(match self.modo {
            ModoFormularioUsuario::Nuevo => DatosUsuario::Crear(CrearUsuarioInput {
                cedula,
                nombre,
                password: self.password.clone(),
                rol: self.rol,
                activo: true,
            }),
            ModoFormularioUsuario::Editar { id, activo } => DatosUsuario::Actualizar {
                id,
                datos: ActualizarUsuarioInput {
                    cedula,
                    nombre,
                    rol: self.rol,
                },
                activo,
                password: cambia_password.then(|| self.password.clone()),
            },
        })
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
        let datos = f.validar().expect("valido");
        let DatosUsuario::Crear(input) = datos else {
            panic!("se esperaba DatosUsuario::Crear");
        };
        assert_eq!(input.cedula, "119430546");
        assert!(input.activo);
        assert_eq!(input.rol, RolUsuario::Operador);
    }

    fn resumen_usuario() -> UsuarioResumen {
        UsuarioResumen {
            id: 3,
            cedula: "119430546".to_string(),
            nombre: "Carlos Pérez".to_string(),
            rol: RolUsuario::Operador,
            activo: true,
        }
    }

    #[test]
    fn editar_precarga_campos_y_deja_password_en_blanco() {
        let f = FormularioUsuario::editar(&resumen_usuario(), RolUsuario::Root);
        assert_eq!(f.cedula, "119430546");
        assert_eq!(f.nombre, "Carlos Pérez");
        assert_eq!(f.password, "");
        assert_eq!(
            f.modo,
            ModoFormularioUsuario::Editar {
                id: 3,
                activo: true
            }
        );
    }

    #[test]
    fn editar_con_passwords_en_blanco_no_las_cambia() {
        let mut f = FormularioUsuario::editar(&resumen_usuario(), RolUsuario::Root);
        let datos = f.validar().expect("valido sin tocar la contraseña");
        let DatosUsuario::Actualizar { password, .. } = datos else {
            panic!("se esperaba DatosUsuario::Actualizar");
        };
        assert_eq!(password, None);
    }

    #[test]
    fn editar_con_password_nueva_la_valida_igual_que_en_alta() {
        let mut f = FormularioUsuario::editar(&resumen_usuario(), RolUsuario::Root);
        f.password = "corta".to_string();
        let errores = f.validar().err().expect("password corta no valida");
        assert!(errores.iter().any(|(c, _)| *c == CampoUsuario::Password));
    }

    #[test]
    fn editar_con_password_valida_la_incluye_para_cambiarla() {
        let mut f = FormularioUsuario::editar(&resumen_usuario(), RolUsuario::Root);
        f.password = "clave1234".to_string();
        f.confirmar_password = "clave1234".to_string();
        let datos = f.validar().expect("valido");
        let DatosUsuario::Actualizar { password, .. } = datos else {
            panic!("se esperaba DatosUsuario::Actualizar");
        };
        assert_eq!(password, Some("clave1234".to_string()));
    }
}
