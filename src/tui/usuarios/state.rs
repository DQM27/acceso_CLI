use std::fmt;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    database::queries::usuarios::UsuarioResumen,
    models::usuario::RolUsuario,
    services::usuario_service::{ActualizarUsuarioInput, CrearUsuarioInput},
    tui::ui_kit::{Debounce, StandardCommand, TextInput, standard_command},
};
use std::time::Instant;

const DURACION_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(250);

#[path = "form.rs"]
mod form;
#[path = "password.rs"]
mod password;
#[path = "render.rs"]
pub(super) mod render;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use form::{ROLES, indice_rol, validar_formulario};
use password::validar_password;

/// Igual que `TextInput`, pero sin depender de `tui_input` — se usa sólo
/// para contraseñas, donde el valor real jamás debe imprimirse ni siquiera
/// en `Debug` (`TextInput` ya redacta su `Debug`, pero mantener este tipo
/// aparte deja explícito, por el nombre, cuáles campos son secretos).
#[derive(Clone, Default, PartialEq, Eq)]
struct Secreto {
    valor: String,
    cursor: usize,
}
impl fmt::Debug for Secreto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[OCULTA]")
    }
}
impl Secreto {
    #[cfg(test)]
    fn nuevo(valor: impl Into<String>) -> Self {
        let valor = valor.into();
        let cursor = valor.chars().count();
        Self { valor, cursor }
    }
    fn limpiar(&mut self) {
        self.valor.clear();
        self.cursor = 0;
    }
    /// Se lleva el valor real y deja el campo vacío — reemplaza al
    /// `std::mem::take` que se usaba cuando esto era un `String` desnudo.
    fn tomar(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.valor)
    }
    fn valor(&self) -> &str {
        &self.valor
    }
    fn cursor(&self) -> usize {
        self.cursor
    }
    fn mascara(&self) -> String {
        "•".repeat(self.valor.chars().count())
    }
    fn longitud(&self) -> usize {
        self.valor.chars().count()
    }
    fn indice_byte(&self, indice_char: usize) -> usize {
        self.valor
            .char_indices()
            .nth(indice_char)
            .map(|(i, _)| i)
            .unwrap_or(self.valor.len())
    }
    /// Devuelve `true` sólo si el contenido cambió (inserción o borrado) —
    /// el movimiento de cursor solo no cuenta, así el llamador sabe cuándo
    /// limpiar un error de validación previo.
    fn handle_key(&mut self, key: KeyEvent, max_chars: usize) -> bool {
        match key.code {
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                false
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.longitud());
                false
            }
            KeyCode::Home => {
                self.cursor = 0;
                false
            }
            KeyCode::End => {
                self.cursor = self.longitud();
                false
            }
            KeyCode::Backspace => {
                if self.cursor == 0 {
                    return false;
                }
                let idx = self.indice_byte(self.cursor - 1);
                self.valor.remove(idx);
                self.cursor -= 1;
                true
            }
            KeyCode::Delete => {
                if self.cursor >= self.longitud() {
                    return false;
                }
                let idx = self.indice_byte(self.cursor);
                self.valor.remove(idx);
                true
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && self.longitud() < max_chars =>
            {
                let idx = self.indice_byte(self.cursor);
                self.valor.insert(idx, c);
                self.cursor += 1;
                true
            }
            _ => false,
        }
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
    cedula: TextInput,
    nombre: TextInput,
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
            cedula: TextInput::default().with_max_chars(30),
            nombre: TextInput::default().with_max_chars(60),
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
            cedula: TextInput::new(u.cedula.clone()).with_max_chars(30),
            nombre: TextInput::new(u.nombre.clone()).with_max_chars(60),
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
    Busqueda { texto: TextInput },
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
    ayuda_expandida: bool,
    busqueda_debounce: Debounce,
    /// `true` mientras se espera el resultado real de crear un usuario o
    /// cambiar una contraseña (Argon2 corriendo en un hilo aparte) — bloquea
    /// la edición del formulario, mismo criterio que `EstadoLogin::Validando`.
    guardando: bool,
}
impl Default for UsuariosState {
    fn default() -> Self {
        Self {
            usuarios: vec![],
            seleccion: None,
            modo: ModoUsuarios::Normal,
            filtro: String::new(),
            mensaje: None,
            ayuda_expandida: false,
            busqueda_debounce: Debounce::default(),
            guardando: false,
        }
    }
}

impl UsuariosState {
    pub fn resumen_por_id(&self, id: i64) -> Option<&UsuarioResumen> {
        self.usuario(id)
    }
    /// Marca que ya se disparó el hilo de Argon2 para crear un usuario o
    /// cambiar una contraseña — bloquea la edición hasta que llegue el
    /// resultado real (`completar_guardado`/`completar_password`).
    pub fn marcar_guardando(&mut self) {
        self.guardando = true;
    }

    pub fn guardando(&self) -> bool {
        self.guardando
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
                if !matches!(self.mensaje.as_deref(), Some(mensaje) if mensaje.starts_with('✓')) {
                    self.mensaje = None;
                }
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
        self.guardando = false;
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
                // Mismo criterio que `completar_guardado`: limpiar el filtro
                // tras cualquier escritura exitosa, no sólo al crear/editar.
                self.filtro.clear();
                self.mensaje = Some(format!(
                    "✓ Usuario {} — {nombre}",
                    if activar { "activado" } else { "desactivado" }
                ));
                AccionUsuarios::Buscar {
                    texto: None,
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
        self.guardando = false;
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
        if standard_command(key) == Some(StandardCommand::Help) {
            self.ayuda_expandida = !self.ayuda_expandida;
            return AccionUsuarios::Ninguna;
        }
        if self.guardando {
            return AccionUsuarios::Ninguna;
        }
        match self.modo.clone() {
            ModoUsuarios::Normal => self.normal(key),
            ModoUsuarios::Busqueda { .. } => self.busqueda(key),
            ModoUsuarios::Formulario(f) => self.formulario(key, f),
            ModoUsuarios::CambioPassword(f) => self.password(key, f),
            ModoUsuarios::ConfirmacionEstado(c) => self.confirmacion(key, c),
        }
    }
    fn normal(&mut self, key: KeyEvent) -> AccionUsuarios {
        if matches!(
            key.code,
            KeyCode::Enter | KeyCode::Char('n' | 'N' | 'p' | 'P' | 'a' | 'A' | '/') | KeyCode::Esc
        ) {
            self.mensaje = None;
        }
        match key.code {
            KeyCode::Up => self.mover(-1),
            KeyCode::Down => self.mover(1),
            KeyCode::Enter => {
                if let Some(id) = self.id_seleccionado() {
                    self.abrir_edicion(id)
                }
            }
            KeyCode::Char('n' | 'N') => {
                self.modo = ModoUsuarios::Formulario(FormularioUsuario::nuevo())
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
                    texto: TextInput::new(self.filtro.clone()),
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
            _ => {
                let mut cambio = false;
                if let ModoUsuarios::Busqueda { texto } = &mut self.modo {
                    cambio = texto.handle_key(key);
                    self.filtro = texto.value().to_owned();
                }
                if cambio {
                    self.busqueda_debounce.marcar(Instant::now());
                }
                AccionUsuarios::Ninguna
            }
        }
    }
    /// Se llama en cada vuelta del bucle principal; dispara la búsqueda
    /// diferida sólo una vez que pasa `DURACION_DEBOUNCE` sin una tecla
    /// nueva.
    pub fn tick(&mut self, ahora: Instant) -> AccionUsuarios {
        if self.busqueda_debounce.listo(ahora, DURACION_DEBOUNCE) {
            AccionUsuarios::Buscar {
                texto: texto_filtro(&self.filtro),
                seleccionar_id: None,
            }
        } else {
            AccionUsuarios::Ninguna
        }
    }
    fn formulario(&mut self, key: KeyEvent, mut f: FormularioUsuario) -> AccionUsuarios {
        if let Some(i) = f.selector_rol {
            match key.code {
                KeyCode::Up => f.selector_rol = Some(i.saturating_sub(1)),
                KeyCode::Down => f.selector_rol = Some((i + 1).min(ROLES.len() - 1)),
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
            KeyCode::Up | KeyCode::BackTab => {
                let len = f.campos().len();
                f.campo = (f.campo + len - 1) % len;
            }
            KeyCode::Down | KeyCode::Tab => f.campo = (f.campo + 1) % f.campos().len(),
            KeyCode::Char(' ')
                if matches!(f.campo_actual(), CampoUsuario::Rol | CampoUsuario::Activo) =>
            {
                match f.campo_actual() {
                    CampoUsuario::Rol => f.selector_rol = Some(indice_rol(f.rol)),
                    CampoUsuario::Activo => f.activo = !f.activo,
                    _ => {}
                }
            }
            KeyCode::Enter => {
                return self.emitir_guardado(f);
            }
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Delete
            | KeyCode::Backspace => match f.campo_actual() {
                CampoUsuario::Cedula => {
                    f.cedula.handle_key(key);
                }
                CampoUsuario::Nombre => {
                    f.nombre.handle_key(key);
                }
                CampoUsuario::Password => {
                    f.password.handle_key(key, 128);
                }
                CampoUsuario::ConfirmarPassword => {
                    f.confirmar_password.handle_key(key, 128);
                }
                _ => {}
            },
            KeyCode::Char(_)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                match f.campo_actual() {
                    CampoUsuario::Cedula => {
                        f.cedula.handle_key(key);
                    }
                    CampoUsuario::Nombre => {
                        f.nombre.handle_key(key);
                    }
                    CampoUsuario::Password => {
                        f.password.handle_key(key, 128);
                    }
                    CampoUsuario::ConfirmarPassword => {
                        f.confirmar_password.handle_key(key, 128);
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
        let cedula = f.cedula.value().trim().to_owned();
        let nombre: String = f.nombre.value().trim().to_owned();
        match f.modo {
            ModoFormularioUsuario::Crear => {
                let password = f.password.tomar();
                f.confirmar_password.limpiar();
                let rol = f.rol;
                let activo = f.activo;
                self.modo = ModoUsuarios::Formulario(f);
                AccionUsuarios::Crear {
                    input: CrearUsuarioInput {
                        cedula,
                        nombre: nombre.clone(),
                        password,
                        rol,
                        activo,
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
    fn password(&mut self, key: KeyEvent, mut f: FormularioPassword) -> AccionUsuarios {
        match key.code {
            KeyCode::Esc => {
                f.limpiar();
                self.modo = ModoUsuarios::Normal;
                return AccionUsuarios::Ninguna;
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Tab | KeyCode::BackTab => f.campo = 1 - f.campo,
            KeyCode::Enter => {
                if let Err(e) = validar_password(f.password.valor(), f.confirmar.valor()) {
                    f.error = Some(e);
                    self.modo = ModoUsuarios::CambioPassword(f);
                    return AccionUsuarios::Ninguna;
                }
                let password = f.password.tomar();
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
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Delete
            | KeyCode::Backspace => {
                if f.campo == 0 {
                    f.password.handle_key(key, 128);
                } else {
                    f.confirmar.handle_key(key, 128);
                }
            }
            KeyCode::Char(_)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if f.campo == 0 {
                    f.password.handle_key(key, 128);
                } else {
                    f.confirmar.handle_key(key, 128);
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
            KeyCode::Enter => {
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
            KeyCode::Esc => {
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
    fn seleccionado(&self) -> Option<&UsuarioResumen> {
        self.usuarios.get(self.seleccion?)
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
