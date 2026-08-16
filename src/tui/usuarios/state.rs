use std::fmt;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    database::queries::usuarios::UsuarioResumen,
    models::usuario::RolUsuario,
    services::usuario_service::{ActualizarUsuarioInput, CrearUsuarioInput},
};

#[path = "render.rs"]
pub(super) mod render;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const ROLES: [RolUsuario; 3] = [
    RolUsuario::Root,
    RolUsuario::Administrador,
    RolUsuario::Operador,
];

#[derive(Clone, Default, PartialEq, Eq)]
struct Secreto(String);
impl fmt::Debug for Secreto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[OCULTA]")
    }
}
impl Secreto {
    fn limpiar(&mut self) {
        self.0.clear();
    }
    fn mascara(&self) -> String {
        "•".repeat(self.0.chars().count())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModoFormularioUsuario {
    Crear,
    Editar { id: i64 },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CampoUsuario {
    Cedula,
    Nombre,
    Rol,
    Password,
    ConfirmarPassword,
    Activo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FormularioUsuario {
    modo: ModoFormularioUsuario,
    cedula: String,
    nombre: String,
    rol: RolUsuario,
    activo: bool,
    password: Secreto,
    confirmar_password: Secreto,
    campo: usize,
    selector_rol: Option<usize>,
    error: Option<String>,
}
impl FormularioUsuario {
    fn nuevo() -> Self {
        Self {
            modo: ModoFormularioUsuario::Crear,
            cedula: String::new(),
            nombre: String::new(),
            rol: RolUsuario::Operador,
            activo: true,
            password: Secreto::default(),
            confirmar_password: Secreto::default(),
            campo: 0,
            selector_rol: None,
            error: None,
        }
    }
    fn editar(u: &UsuarioResumen) -> Self {
        Self {
            modo: ModoFormularioUsuario::Editar { id: u.id },
            cedula: u.cedula.clone(),
            nombre: u.nombre.clone(),
            rol: u.rol,
            activo: u.activo,
            password: Secreto::default(),
            confirmar_password: Secreto::default(),
            campo: 0,
            selector_rol: None,
            error: None,
        }
    }
    fn campos(&self) -> &'static [CampoUsuario] {
        match self.modo {
            ModoFormularioUsuario::Crear => &[
                CampoUsuario::Cedula,
                CampoUsuario::Nombre,
                CampoUsuario::Rol,
                CampoUsuario::Password,
                CampoUsuario::ConfirmarPassword,
                CampoUsuario::Activo,
            ],
            ModoFormularioUsuario::Editar { .. } => &[
                CampoUsuario::Cedula,
                CampoUsuario::Nombre,
                CampoUsuario::Rol,
                CampoUsuario::Activo,
            ],
        }
    }
    fn campo_actual(&self) -> CampoUsuario {
        self.campos()[self.campo]
    }
    fn limpiar_secretos(&mut self) {
        self.password.limpiar();
        self.confirmar_password.limpiar();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FormularioPassword {
    id: i64,
    usuario_nombre: String,
    password: Secreto,
    confirmar: Secreto,
    campo: usize,
    error: Option<String>,
}
impl FormularioPassword {
    fn limpiar(&mut self) {
        self.password.limpiar();
        self.confirmar.limpiar();
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConfirmacionEstado {
    id: i64,
    activar: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum ModoUsuarios {
    Normal,
    Busqueda { texto: String },
    Detalle { id: i64 },
    Formulario(FormularioUsuario),
    CambioPassword(FormularioPassword),
    ConfirmacionEstado(ConfirmacionEstado),
}

pub enum AccionUsuarios {
    Ninguna,
    Volver,
    Buscar {
        texto: Option<String>,
        seleccionar_id: Option<i64>,
    },
    Crear {
        input: CrearUsuarioInput,
        nombre: String,
    },
    Actualizar {
        id: i64,
        input: ActualizarUsuarioInput,
        activo: bool,
        nombre: String,
    },
    CambiarPassword {
        id: i64,
        password: String,
        nombre: String,
    },
    EstablecerActivo {
        id: i64,
        activar: bool,
        nombre: String,
    },
}
impl fmt::Debug for AccionUsuarios {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ninguna => f.write_str("Ninguna"),
            Self::Volver => f.write_str("Volver"),
            Self::Buscar {
                texto,
                seleccionar_id,
            } => f
                .debug_struct("Buscar")
                .field("texto", texto)
                .field("seleccionar_id", seleccionar_id)
                .finish(),
            Self::Crear { nombre, .. } => f
                .debug_struct("Crear")
                .field("nombre", nombre)
                .field("password", &"[OCULTA]")
                .finish(),
            Self::Actualizar { id, nombre, .. } => f
                .debug_struct("Actualizar")
                .field("id", id)
                .field("nombre", nombre)
                .finish(),
            Self::CambiarPassword { id, nombre, .. } => f
                .debug_struct("CambiarPassword")
                .field("id", id)
                .field("nombre", nombre)
                .field("password", &"[OCULTA]")
                .finish(),
            Self::EstablecerActivo {
                id,
                activar,
                nombre,
            } => f
                .debug_struct("EstablecerActivo")
                .field("id", id)
                .field("activar", activar)
                .field("nombre", nombre)
                .finish(),
        }
    }
}

#[derive(Debug)]
pub struct UsuariosState {
    usuarios: Vec<UsuarioResumen>,
    seleccion: Option<usize>,
    modo: ModoUsuarios,
    filtro: String,
    mensaje: Option<String>,
    usuario_nombre: String,
}
impl Default for UsuariosState {
    fn default() -> Self {
        Self {
            usuarios: vec![],
            seleccion: None,
            modo: ModoUsuarios::Normal,
            filtro: String::new(),
            mensaje: None,
            usuario_nombre: "Quintana".into(),
        }
    }
}

impl UsuariosState {
    pub fn set_usuario_nombre(&mut self, nombre: impl Into<String>) {
        self.usuario_nombre = nombre.into();
    }
    pub fn resumen_por_id(&self, id: i64) -> Option<&UsuarioResumen> {
        self.usuario(id)
    }
    pub fn solicitud_carga(&self) -> AccionUsuarios {
        AccionUsuarios::Buscar {
            texto: None,
            seleccionar_id: None,
        }
    }
    pub fn completar_busqueda(
        &mut self,
        resultado: Result<Vec<UsuarioResumen>, String>,
        seleccionar_id: Option<i64>,
    ) {
        match resultado {
            Ok(items) => {
                self.usuarios = items;
                self.seleccion = seleccionar_id
                    .and_then(|id| self.usuarios.iter().position(|u| u.id == id))
                    .or((!self.usuarios.is_empty()).then_some(0));
            }
            Err(e) => {
                self.usuarios.clear();
                self.seleccion = None;
                self.mensaje = Some(e);
            }
        }
    }
    pub fn completar_guardado(
        &mut self,
        resultado: Result<Option<i64>, String>,
        id: Option<i64>,
        nombre: &str,
    ) -> AccionUsuarios {
        match resultado {
            Ok(nuevo) => {
                self.modo = ModoUsuarios::Normal;
                self.filtro.clear();
                self.mensaje = Some(format!(
                    "✓ Usuario {} — {nombre}",
                    if nuevo.is_some() {
                        "creado"
                    } else {
                        "actualizado"
                    }
                ));
                AccionUsuarios::Buscar {
                    texto: None,
                    seleccionar_id: nuevo.or(id),
                }
            }
            Err(e) => {
                if let ModoUsuarios::Formulario(f) = &mut self.modo {
                    f.error = Some(e);
                }
                AccionUsuarios::Ninguna
            }
        }
    }
    pub fn completar_estado(
        &mut self,
        resultado: Result<(), String>,
        id: i64,
        activar: bool,
        nombre: &str,
    ) -> AccionUsuarios {
        match resultado {
            Ok(()) => {
                self.modo = ModoUsuarios::Normal;
                self.mensaje = Some(format!(
                    "✓ Usuario {} — {nombre}",
                    if activar { "activado" } else { "desactivado" }
                ));
                AccionUsuarios::Buscar {
                    texto: texto_filtro(&self.filtro),
                    seleccionar_id: Some(id),
                }
            }
            Err(e) => {
                self.modo = ModoUsuarios::Normal;
                self.mensaje = Some(e);
                AccionUsuarios::Ninguna
            }
        }
    }
    pub fn completar_password(&mut self, resultado: Result<(), String>, nombre: &str) {
        match resultado {
            Ok(()) => {
                if let ModoUsuarios::CambioPassword(f) = &mut self.modo {
                    f.limpiar();
                }
                self.modo = ModoUsuarios::Normal;
                self.mensaje = Some(format!("✓ Contraseña actualizada — {nombre}"));
            }
            Err(e) => {
                if let ModoUsuarios::CambioPassword(f) = &mut self.modo {
                    f.limpiar();
                    f.error = Some(e);
                }
            }
        }
    }
    pub fn handle_key(&mut self, key: KeyEvent) -> AccionUsuarios {
        match self.modo.clone() {
            ModoUsuarios::Normal => self.normal(key),
            ModoUsuarios::Busqueda { .. } => self.busqueda(key),
            ModoUsuarios::Detalle { id } => match key.code {
                KeyCode::Esc => {
                    self.modo = ModoUsuarios::Normal;
                    AccionUsuarios::Ninguna
                }
                KeyCode::Char('e' | 'E') => {
                    self.abrir_edicion(id);
                    AccionUsuarios::Ninguna
                }
                KeyCode::Char('p' | 'P') => {
                    self.abrir_password(id);
                    AccionUsuarios::Ninguna
                }
                KeyCode::Char('a' | 'A') => {
                    self.solicitar_estado(id);
                    AccionUsuarios::Ninguna
                }
                _ => AccionUsuarios::Ninguna,
            },
            ModoUsuarios::Formulario(f) => self.formulario(key, f),
            ModoUsuarios::CambioPassword(f) => self.password(key, f),
            ModoUsuarios::ConfirmacionEstado(c) => self.confirmacion(key, c),
        }
    }
    fn normal(&mut self, key: KeyEvent) -> AccionUsuarios {
        self.mensaje = None;
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => {
                if let Some(id) = self.id_seleccionado() {
                    self.modo = ModoUsuarios::Detalle { id }
                }
            }
            KeyCode::Char('n' | 'N') => {
                self.modo = ModoUsuarios::Formulario(FormularioUsuario::nuevo())
            }
            KeyCode::Char('e' | 'E') => {
                if let Some(id) = self.id_seleccionado() {
                    self.abrir_edicion(id)
                }
            }
            KeyCode::Char('p' | 'P') => {
                if let Some(id) = self.id_seleccionado() {
                    self.abrir_password(id)
                }
            }
            KeyCode::Char('a' | 'A') => {
                if let Some(id) = self.id_seleccionado() {
                    self.solicitar_estado(id)
                }
            }
            KeyCode::Char('/') => {
                self.modo = ModoUsuarios::Busqueda {
                    texto: self.filtro.clone(),
                }
            }
            KeyCode::Esc if !self.filtro.is_empty() => {
                self.filtro.clear();
                return AccionUsuarios::Buscar {
                    texto: None,
                    seleccionar_id: None,
                };
            }
            KeyCode::Esc => return AccionUsuarios::Volver,
            _ => {}
        }
        AccionUsuarios::Ninguna
    }
    fn busqueda(&mut self, key: KeyEvent) -> AccionUsuarios {
        match key.code {
            KeyCode::Esc => {
                self.filtro.clear();
                self.modo = ModoUsuarios::Normal;
                AccionUsuarios::Buscar {
                    texto: None,
                    seleccionar_id: None,
                }
            }
            KeyCode::Enter => {
                self.modo = ModoUsuarios::Normal;
                AccionUsuarios::Ninguna
            }
            KeyCode::Up => {
                self.mover(-1);
                AccionUsuarios::Ninguna
            }
            KeyCode::Down => {
                self.mover(1);
                AccionUsuarios::Ninguna
            }
            KeyCode::Backspace => {
                if let ModoUsuarios::Busqueda { texto } = &mut self.modo {
                    texto.pop();
                    self.filtro = texto.clone()
                }
                AccionUsuarios::Buscar {
                    texto: texto_filtro(&self.filtro),
                    seleccionar_id: None,
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let ModoUsuarios::Busqueda { texto } = &mut self.modo {
                    texto.push(c);
                    self.filtro = texto.clone()
                }
                AccionUsuarios::Buscar {
                    texto: texto_filtro(&self.filtro),
                    seleccionar_id: None,
                }
            }
            _ => AccionUsuarios::Ninguna,
        }
    }
    fn formulario(&mut self, key: KeyEvent, mut f: FormularioUsuario) -> AccionUsuarios {
        if let Some(i) = f.selector_rol {
            match key.code {
                KeyCode::Up => f.selector_rol = Some(i.saturating_sub(1)),
                KeyCode::Down => f.selector_rol = Some((i + 1).min(2)),
                KeyCode::Enter => {
                    f.rol = ROLES[i];
                    f.selector_rol = None
                }
                KeyCode::Esc => f.selector_rol = None,
                _ => {}
            }
            self.modo = ModoUsuarios::Formulario(f);
            return AccionUsuarios::Ninguna;
        }
        match key.code {
            KeyCode::Esc => {
                f.limpiar_secretos();
                self.modo = ModoUsuarios::Normal;
                return AccionUsuarios::Ninguna;
            }
            KeyCode::Up | KeyCode::BackTab => f.campo = f.campo.saturating_sub(1),
            KeyCode::Down | KeyCode::Tab => f.campo = (f.campo + 1).min(f.campos().len() - 1),
            KeyCode::Enter => match f.campo_actual() {
                CampoUsuario::Rol => f.selector_rol = Some(indice_rol(f.rol)),
                CampoUsuario::Activo => f.activo = !f.activo,
                _ => {}
            },
            KeyCode::Left | KeyCode::Right if f.campo_actual() == CampoUsuario::Activo => {
                f.activo = !f.activo
            }
            KeyCode::Char('G') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                return self.emitir_guardado(f);
            }
            KeyCode::Backspace => match f.campo_actual() {
                CampoUsuario::Cedula => {
                    f.cedula.pop();
                }
                CampoUsuario::Nombre => {
                    f.nombre.pop();
                }
                CampoUsuario::Password => {
                    f.password.0.pop();
                }
                CampoUsuario::ConfirmarPassword => {
                    f.confirmar_password.0.pop();
                }
                _ => {}
            },
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                match f.campo_actual() {
                    CampoUsuario::Cedula if f.cedula.chars().count() < 30 => f.cedula.push(c),
                    CampoUsuario::Nombre if f.nombre.chars().count() < 60 => f.nombre.push(c),
                    CampoUsuario::Password if f.password.0.chars().count() < 128 => {
                        f.password.0.push(c)
                    }
                    CampoUsuario::ConfirmarPassword
                        if f.confirmar_password.0.chars().count() < 128 =>
                    {
                        f.confirmar_password.0.push(c)
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        f.error = None;
        self.modo = ModoUsuarios::Formulario(f);
        AccionUsuarios::Ninguna
    }
    fn emitir_guardado(&mut self, mut f: FormularioUsuario) -> AccionUsuarios {
        if let Err(e) = validar_formulario(&f) {
            f.error = Some(e);
            self.modo = ModoUsuarios::Formulario(f);
            return AccionUsuarios::Ninguna;
        }
        let cedula = f.cedula.trim().into();
        let nombre: String = f.nombre.trim().into();
        match f.modo {
            ModoFormularioUsuario::Crear => {
                let password = std::mem::take(&mut f.password.0);
                f.confirmar_password.limpiar();
                self.modo = ModoUsuarios::Formulario(f);
                AccionUsuarios::Crear {
                    input: CrearUsuarioInput {
                        cedula,
                        nombre: nombre.clone(),
                        password,
                        rol: self.formulario_actual().unwrap().rol,
                        activo: self.formulario_actual().unwrap().activo,
                    },
                    nombre,
                }
            }
            ModoFormularioUsuario::Editar { id } => {
                let input = ActualizarUsuarioInput {
                    cedula,
                    nombre: nombre.clone(),
                    rol: f.rol,
                };
                let activo = f.activo;
                self.modo = ModoUsuarios::Formulario(f);
                AccionUsuarios::Actualizar {
                    id,
                    input,
                    activo,
                    nombre,
                }
            }
        }
    }
    fn formulario_actual(&self) -> Option<&FormularioUsuario> {
        if let ModoUsuarios::Formulario(f) = &self.modo {
            Some(f)
        } else {
            None
        }
    }
    fn password(&mut self, key: KeyEvent, mut f: FormularioPassword) -> AccionUsuarios {
        match key.code {
            KeyCode::Esc => {
                f.limpiar();
                self.modo = ModoUsuarios::Normal;
                return AccionUsuarios::Ninguna;
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Tab | KeyCode::BackTab => f.campo = 1 - f.campo,
            KeyCode::Char('G') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Err(e) = validar_password(&f.password.0, &f.confirmar.0) {
                    f.error = Some(e);
                    self.modo = ModoUsuarios::CambioPassword(f);
                    return AccionUsuarios::Ninguna;
                }
                let password = std::mem::take(&mut f.password.0);
                f.confirmar.limpiar();
                let id = f.id;
                let nombre = f.usuario_nombre.clone();
                self.modo = ModoUsuarios::CambioPassword(f);
                return AccionUsuarios::CambiarPassword {
                    id,
                    password,
                    nombre,
                };
            }
            KeyCode::Backspace => {
                if f.campo == 0 {
                    f.password.0.pop();
                } else {
                    f.confirmar.0.pop();
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if f.campo == 0 {
                    f.password.0.push(c)
                } else {
                    f.confirmar.0.push(c)
                }
            }
            _ => {}
        }
        f.error = None;
        self.modo = ModoUsuarios::CambioPassword(f);
        AccionUsuarios::Ninguna
    }
    fn confirmacion(&mut self, key: KeyEvent, c: ConfirmacionEstado) -> AccionUsuarios {
        match key.code {
            KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
                let nombre = self
                    .usuario(c.id)
                    .map(|u| u.nombre.clone())
                    .unwrap_or_default();
                AccionUsuarios::EstablecerActivo {
                    id: c.id,
                    activar: c.activar,
                    nombre,
                }
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.modo = ModoUsuarios::Normal;
                AccionUsuarios::Ninguna
            }
            _ => AccionUsuarios::Ninguna,
        }
    }
    fn abrir_edicion(&mut self, id: i64) {
        if let Some(u) = self.usuario(id) {
            self.modo = ModoUsuarios::Formulario(FormularioUsuario::editar(u))
        }
    }
    fn abrir_password(&mut self, id: i64) {
        if let Some(u) = self.usuario(id) {
            self.modo = ModoUsuarios::CambioPassword(FormularioPassword {
                id,
                usuario_nombre: u.nombre.clone(),
                password: Secreto::default(),
                confirmar: Secreto::default(),
                campo: 0,
                error: None,
            })
        }
    }
    fn solicitar_estado(&mut self, id: i64) {
        if let Some(u) = self.usuario(id) {
            self.modo = ModoUsuarios::ConfirmacionEstado(ConfirmacionEstado {
                id,
                activar: !u.activo,
            })
        }
    }
    fn mover(&mut self, d: isize) {
        if self.usuarios.is_empty() {
            self.seleccion = None
        } else {
            let i = self.seleccion.unwrap_or(0);
            self.seleccion = Some(if d < 0 {
                i.saturating_sub(1)
            } else {
                (i + 1).min(self.usuarios.len() - 1)
            })
        }
    }
    fn id_seleccionado(&self) -> Option<i64> {
        self.usuarios.get(self.seleccion?).map(|u| u.id)
    }
    fn usuario(&self, id: i64) -> Option<&UsuarioResumen> {
        self.usuarios.iter().find(|u| u.id == id)
    }
    fn inicio_visible(&self, capacidad: usize) -> usize {
        self.seleccion
            .unwrap_or(0)
            .saturating_sub(capacidad.saturating_sub(1))
    }
}

fn texto_filtro(s: &str) -> Option<String> {
    (!s.trim().is_empty()).then(|| s.to_owned())
}
fn validar_formulario(f: &FormularioUsuario) -> Result<(), String> {
    if f.cedula.trim().is_empty() {
        return Err("La cédula es obligatoria".into());
    }
    if f.nombre.trim().is_empty() {
        return Err("El nombre es obligatorio".into());
    }
    if matches!(f.modo, ModoFormularioUsuario::Crear) {
        validar_password(&f.password.0, &f.confirmar_password.0)?
    }
    Ok(())
}
fn validar_password(p: &str, c: &str) -> Result<(), String> {
    if p.is_empty() {
        Err("La contraseña es obligatoria".into())
    } else if c.is_empty() {
        Err("Debe confirmar la contraseña".into())
    } else if p.chars().count() < 8 {
        Err("La contraseña debe tener al menos 8 caracteres".into())
    } else if p != c {
        Err("Las contraseñas no coinciden".into())
    } else {
        Ok(())
    }
}
fn indice_rol(r: RolUsuario) -> usize {
    ROLES.iter().position(|x| *x == r).unwrap_or(2)
}
fn texto_rol(r: RolUsuario) -> &'static str {
    match r {
        RolUsuario::Root => "ROOT",
        RolUsuario::Administrador => "ADMINISTRADOR",
        RolUsuario::Operador => "OPERADOR",
    }
}
fn si_no(v: bool) -> &'static str {
    if v { "SÍ" } else { "NO" }
}
