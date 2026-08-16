//! Mockup de fluidez para usuarios: mismo panel único que contratistas,
//! pero con dos particularidades propias de esta entidad.
//!
//! 1. El cambio de contraseña es un flujo separado del formulario de datos
//!    (cédula/nombre/rol/estado) — igual que en la pantalla productiva —,
//!    así que comparte el mismo panel con un tercer modo (`Panel::Password`)
//!    en vez de mezclarse con la edición normal.
//! 2. Activar/desactivar pide confirmación en la línea de estado, igual
//!    que Salir en `menu_v2` — nunca un modal.
//!
//! Como el buscador necesita convivir con CUATRO atajos de letra (N/P/A,
//! Enter ya cubre "editar"), usa `/` para entrar a modo búsqueda —igual
//! que se corrigió en `empresas_v2`— en vez de estar siempre activo.
//!
//! cargo run --example brisas_cli -- usuarios-v2

use chrono::Utc;
use chrono_tz::America::Costa_Rica;
use control_acceso::tui::ui_kit::{
    QUICK_EXIT_HINT, StandardCommand, Theme, ThemePreset, render_terminal_too_small,
    standard_command,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

use super::quick_exit::{QuickExitOutcome, QuickExitOverlay};

const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 20;
const WIDE_LAYOUT_WIDTH: u16 = 100;
const PANEL_HEIGHT: u16 = 10;
/// Operador con sesión simulada, igual que en el resto de las pantallas v2.
const CURRENT_OPERATOR: &str = "Daniel Quintana";
/// Coincide con el id de `CURRENT_OPERATOR` en `demo_users()`.
const CURRENT_OPERATOR_ID: u32 = 1;
/// Contraseña simulada de la sesión actual: no hay backend real detrás de
/// este piloto, así que se compara contra un valor fijo únicamente para
/// poder probar el camino de error y el de éxito.
const DEMO_CURRENT_PASSWORD: &str = "brisas2024";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Root,
    Admin,
    Operator,
}

impl Role {
    const ALL: [Self; 3] = [Self::Root, Self::Admin, Self::Operator];

    const fn label(self) -> &'static str {
        match self {
            Self::Root => "ROOT",
            Self::Admin => "ADMINISTRADOR",
            Self::Operator => "OPERADOR",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct User {
    id: u32,
    identity: String,
    name: String,
    role: Role,
    active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormField {
    Identity,
    Name,
    Role,
    Password,
    ConfirmPassword,
    Active,
}

impl FormField {
    const NEW_FIELDS: [Self; 6] = [
        Self::Identity,
        Self::Name,
        Self::Role,
        Self::Password,
        Self::ConfirmPassword,
        Self::Active,
    ];
    const EDIT_FIELDS: [Self; 4] = [Self::Identity, Self::Name, Self::Role, Self::Active];
}

#[derive(Debug)]
struct UserForm {
    id: Option<u32>,
    identity: Input,
    name: Input,
    role: usize,
    role_expanded: Option<usize>,
    password: Input,
    confirm_password: Input,
    active: bool,
    focus: usize,
    error: Option<String>,
}

impl UserForm {
    fn new() -> Self {
        Self {
            id: None,
            identity: Input::default(),
            name: Input::default(),
            role: 2, // Operador por defecto, como el productivo
            role_expanded: None,
            password: Input::default(),
            confirm_password: Input::default(),
            active: true,
            focus: 0,
            error: None,
        }
    }

    fn edit(user: &User) -> Self {
        Self {
            id: Some(user.id),
            identity: Input::new(user.identity.clone()),
            name: Input::new(user.name.clone()),
            role: Role::ALL.iter().position(|role| *role == user.role).unwrap_or(2),
            role_expanded: None,
            password: Input::default(),
            confirm_password: Input::default(),
            active: user.active,
            focus: 0,
            error: None,
        }
    }

    fn is_editing(&self) -> bool {
        self.id.is_some()
    }

    fn fields(&self) -> &'static [FormField] {
        if self.is_editing() {
            &FormField::EDIT_FIELDS
        } else {
            &FormField::NEW_FIELDS
        }
    }

    fn field(&self) -> FormField {
        self.fields()[self.focus]
    }
}

#[derive(Debug)]
struct PasswordForm {
    id: u32,
    user_name: String,
    current_password: Input,
    password: Input,
    confirm: Input,
    focus: usize,
    error: Option<String>,
}

impl PasswordForm {
    fn new(user: &User) -> Self {
        Self {
            id: user.id,
            user_name: user.name.clone(),
            current_password: Input::default(),
            password: Input::default(),
            confirm: Input::default(),
            focus: 0,
            error: None,
        }
    }

    /// Sólo se exige la contraseña actual cuando la sesión activa se
    /// cambia la suya propia. Un reset administrativo de OTRO usuario no
    /// puede pedirla — el admin nunca la supo, esa es la razón del reset.
    fn requires_current_password(&self) -> bool {
        self.id == CURRENT_OPERATOR_ID
    }

    fn field_count(&self) -> usize {
        if self.requires_current_password() { 3 } else { 2 }
    }
}

#[derive(Debug)]
enum Panel {
    Detail,
    Form(Box<UserForm>),
    Password(Box<PasswordForm>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingStatus {
    id: u32,
    activate: bool,
}

#[derive(Debug, Clone)]
enum Mode {
    Browse,
    Search {
        original: String,
        selected_id: Option<u32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    Browse,
    Search,
    Form,
    RoleExpand,
    Password,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatOrigin {
    code: KeyCode,
    modifiers: KeyModifiers,
    layer: Layer,
}

#[derive(Debug)]
pub struct UsuariosV2Pilot {
    users: Vec<User>,
    search: Input,
    mode: Mode,
    selected: usize,
    panel: Panel,
    confirming: Option<PendingStatus>,
    prepared_message: Option<String>,
    help_expanded: bool,
    theme: ThemePreset,
    running: bool,
    terminal_size: (u16, u16),
    repeat_origin: Option<RepeatOrigin>,
    quick_exit: QuickExitOverlay,
}

impl Default for UsuariosV2Pilot {
    fn default() -> Self {
        Self {
            users: demo_users(),
            search: Input::default(),
            mode: Mode::Browse,
            selected: 0,
            panel: Panel::Detail,
            confirming: None,
            prepared_message: None,
            help_expanded: false,
            theme: ThemePreset::Classic,
            running: true,
            terminal_size: (u16::MAX, u16::MAX),
            repeat_origin: None,
            quick_exit: QuickExitOverlay::demo(),
        }
    }
}

impl UsuariosV2Pilot {
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::Resize(width, height) => {
                self.terminal_size = (*width, *height);
                return true;
            }
            Event::FocusGained | Event::FocusLost => return true,
            Event::Key(key) if key.kind == KeyEventKind::Release => {
                if self.repeat_origin.is_some_and(|origin| {
                    origin.code == key.code && origin.modifiers == key.modifiers
                }) {
                    self.repeat_origin = None;
                }
                return false;
            }
            _ => {}
        }

        let key = match event {
            Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                Some(*key)
            }
            Event::Paste(_) => None,
            _ => return false,
        };

        if key.is_some_and(|key| standard_command(key) == Some(StandardCommand::EmergencyExit)) {
            self.running = false;
            return true;
        }
        if !self.viewport_is_valid() {
            if key.is_some_and(|key| {
                key.kind == KeyEventKind::Press && standard_command(key) == Some(StandardCommand::Cancel)
            }) {
                self.running = false;
                return true;
            }
            return false;
        }

        if self.quick_exit.is_open() {
            match self.quick_exit.handle_event(event) {
                QuickExitOutcome::Ignored => {}
                QuickExitOutcome::Consumed => return true,
                QuickExitOutcome::ExitRegistered(message) => {
                    self.prepared_message = Some(message);
                    return true;
                }
            }
        }
        if key.is_some_and(|key| {
            key.kind == KeyEventKind::Press && standard_command(key) == Some(StandardCommand::QuickExit)
        }) {
            self.quick_exit.open();
            return true;
        }

        let Some(key) = key else {
            return self.handle_text_input(event);
        };

        let origin = RepeatOrigin {
            code: key.code,
            modifiers: key.modifiers,
            layer: self.layer(),
        };
        match key.kind {
            KeyEventKind::Press => self.repeat_origin = Some(origin),
            KeyEventKind::Repeat if self.repeat_origin != Some(origin) => return false,
            KeyEventKind::Repeat | KeyEventKind::Release => {}
        }

        let handled = match self.layer() {
            Layer::Browse => self.handle_browse_key(key),
            Layer::Search => self.handle_search_key(key),
            Layer::Form => self.handle_form_key(key),
            Layer::RoleExpand => self.handle_role_expand_key(key),
            Layer::Password => self.handle_password_key(key),
            Layer::Confirm => self.handle_confirm_key(key),
        };
        if handled {
            return true;
        }
        self.handle_text_input(event)
    }

    fn handle_browse_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
        );
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => self.open_edit_form(),
            Some(StandardCommand::Cancel) => {
                self.running = false;
                return true;
            }
            Some(StandardCommand::Help) if key.kind == KeyEventKind::Press => {
                self.help_expanded = !self.help_expanded;
            }
            Some(StandardCommand::Theme) if key.kind == KeyEventKind::Press => self.toggle_theme(),
            _ => match key.code {
                KeyCode::Up => self.move_selection(-1),
                KeyCode::Down => self.move_selection(1),
                KeyCode::Home => self.select(0),
                KeyCode::End => self.select(self.filtered_indices().len().saturating_sub(1)),
                KeyCode::Char('/') => self.begin_search(),
                KeyCode::Char(character) if character.eq_ignore_ascii_case(&'n') => {
                    self.panel = Panel::Form(Box::new(UserForm::new()));
                    self.prepared_message = None;
                }
                KeyCode::Char(character) if character.eq_ignore_ascii_case(&'p') => {
                    self.open_password_form();
                }
                KeyCode::Char(character) if character.eq_ignore_ascii_case(&'a') => {
                    self.request_toggle_active();
                }
                _ => return false,
            },
        }
        true
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
        );
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => self.apply_search(),
            Some(StandardCommand::Cancel) => self.cancel_search(),
            Some(StandardCommand::Help) if key.kind == KeyEventKind::Press => {
                self.help_expanded = !self.help_expanded;
            }
            Some(StandardCommand::Theme) if key.kind == KeyEventKind::Press => self.toggle_theme(),
            _ => match key.code {
                KeyCode::Up => self.move_selection(-1),
                KeyCode::Down => self.move_selection(1),
                KeyCode::Home => self.select(0),
                KeyCode::End => self.select(self.filtered_indices().len().saturating_sub(1)),
                _ => return false,
            },
        }
        true
    }

    fn handle_form_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Repeat && !matches!(key.code, KeyCode::Left | KeyCode::Right) {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => self.save_form(),
            Some(StandardCommand::Activate) => {
                if matches!(
                    self.form_field(),
                    Some(FormField::Identity | FormField::Name | FormField::Password | FormField::ConfirmPassword)
                ) {
                    return false;
                }
                self.activate_form_field();
            }
            Some(StandardCommand::Cancel) => {
                self.panel = Panel::Detail;
                self.prepared_message = Some("Edición cancelada · no hubo cambios".into());
            }
            Some(StandardCommand::FocusNext) => self.move_form_focus(false),
            Some(StandardCommand::FocusPrevious) => self.move_form_focus(true),
            Some(StandardCommand::Help) if key.kind == KeyEventKind::Press => {
                self.help_expanded = !self.help_expanded;
            }
            Some(StandardCommand::Theme) if key.kind == KeyEventKind::Press => self.toggle_theme(),
            _ => match key.code {
                KeyCode::Left => self.cycle_form_value(true),
                KeyCode::Right => self.cycle_form_value(false),
                _ => return false,
            },
        }
        true
    }

    fn handle_role_expand_key(&mut self, key: KeyEvent) -> bool {
        let repeatable = matches!(key.code, KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End);
        if key.kind == KeyEventKind::Repeat && !repeatable {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => {
                if let Panel::Form(form) = &mut self.panel
                    && let Some(highlighted) = form.role_expanded
                {
                    form.role = highlighted;
                    form.error = None;
                }
                if let Panel::Form(form) = &mut self.panel {
                    form.role_expanded = None;
                }
            }
            Some(StandardCommand::Cancel) => {
                if let Panel::Form(form) = &mut self.panel {
                    form.role_expanded = None;
                }
            }
            Some(StandardCommand::Help) if key.kind == KeyEventKind::Press => {
                self.help_expanded = !self.help_expanded;
            }
            Some(StandardCommand::Theme) if key.kind == KeyEventKind::Press => self.toggle_theme(),
            _ => {
                let Panel::Form(form) = &mut self.panel else {
                    return false;
                };
                let Some(highlighted) = &mut form.role_expanded else {
                    return false;
                };
                match key.code {
                    KeyCode::Up => *highlighted = highlighted.checked_sub(1).unwrap_or(Role::ALL.len() - 1),
                    KeyCode::Down => *highlighted = (*highlighted + 1) % Role::ALL.len(),
                    KeyCode::Home => *highlighted = 0,
                    KeyCode::End => *highlighted = Role::ALL.len() - 1,
                    _ => return false,
                }
            }
        }
        true
    }

    fn handle_password_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Repeat {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => self.save_password(),
            Some(StandardCommand::Cancel) => {
                self.panel = Panel::Detail;
                self.prepared_message = Some("Cambio de contraseña cancelado".into());
            }
            Some(StandardCommand::FocusNext) => self.move_password_focus(false),
            Some(StandardCommand::FocusPrevious) => self.move_password_focus(true),
            Some(StandardCommand::Help) => self.help_expanded = !self.help_expanded,
            Some(StandardCommand::Theme) => self.toggle_theme(),
            _ => return false,
        }
        true
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Repeat {
            return false;
        }
        match standard_command(key) {
            Some(StandardCommand::Primary) => self.confirm_toggle_active(),
            Some(StandardCommand::Cancel) => {
                self.confirming = None;
                self.prepared_message = Some("Acción cancelada · no hubo cambios".into());
            }
            Some(StandardCommand::Theme) => self.toggle_theme(),
            _ => return false,
        }
        true
    }

    fn handle_text_input(&mut self, event: &Event) -> bool {
        match &mut self.panel {
            Panel::Form(form) if form.role_expanded.is_none() => match form.field() {
                FormField::Identity => {
                    let changed = apply_event(&mut form.identity, event);
                    if changed {
                        form.error = None;
                    }
                    changed
                }
                FormField::Name => {
                    let changed = apply_event(&mut form.name, event);
                    if changed {
                        form.error = None;
                    }
                    changed
                }
                FormField::Password => {
                    let changed = apply_event(&mut form.password, event);
                    if changed {
                        form.error = None;
                    }
                    changed
                }
                FormField::ConfirmPassword => {
                    let changed = apply_event(&mut form.confirm_password, event);
                    if changed {
                        form.error = None;
                    }
                    changed
                }
                FormField::Role | FormField::Active => false,
            },
            Panel::Password(form) => {
                let requires_current = form.requires_current_password();
                let target = match (requires_current, form.focus) {
                    (true, 0) => &mut form.current_password,
                    (true, 1) | (false, 0) => &mut form.password,
                    _ => &mut form.confirm,
                };
                let changed = apply_event(target, event);
                if changed {
                    form.error = None;
                }
                changed
            }
            Panel::Detail if matches!(self.mode, Mode::Search { .. }) && self.confirming.is_none() => {
                let changed = apply_event(&mut self.search, event);
                if changed {
                    self.selected = 0;
                    self.prepared_message = None;
                }
                changed
            }
            Panel::Form(_) | Panel::Detail => false,
        }
    }

    fn begin_search(&mut self) {
        self.mode = Mode::Search {
            original: self.search.value().to_owned(),
            selected_id: self.selected_user().map(|user| user.id),
        };
    }

    fn apply_search(&mut self) {
        self.mode = Mode::Browse;
    }

    fn cancel_search(&mut self) {
        let Mode::Search {
            original,
            selected_id,
        } = self.mode.clone()
        else {
            return;
        };
        self.search = Input::new(original);
        self.select_id(selected_id);
        self.mode = Mode::Browse;
    }

    fn select_id(&mut self, id: Option<u32>) {
        let indices = self.filtered_indices();
        self.selected = id
            .and_then(|id| indices.iter().position(|&index| self.users[index].id == id))
            .unwrap_or(0);
    }

    fn open_edit_form(&mut self) {
        let Some(user) = self.selected_user().cloned() else {
            return;
        };
        self.panel = Panel::Form(Box::new(UserForm::edit(&user)));
    }

    fn open_password_form(&mut self) {
        let Some(user) = self.selected_user().cloned() else {
            return;
        };
        self.panel = Panel::Password(Box::new(PasswordForm::new(&user)));
    }

    fn request_toggle_active(&mut self) {
        let Some(user) = self.selected_user() else {
            return;
        };
        self.confirming = Some(PendingStatus {
            id: user.id,
            activate: !user.active,
        });
    }

    fn confirm_toggle_active(&mut self) {
        let Some(pending) = self.confirming.take() else {
            return;
        };
        if let Some(user) = self.users.iter_mut().find(|user| user.id == pending.id) {
            user.active = pending.activate;
            self.prepared_message = Some(format!(
                "Usuario {} · {}",
                if pending.activate { "activado" } else { "desactivado" },
                user.name
            ));
        }
    }

    fn form_field(&self) -> Option<FormField> {
        match &self.panel {
            Panel::Form(form) => Some(form.field()),
            _ => None,
        }
    }

    fn move_form_focus(&mut self, backwards: bool) {
        let Panel::Form(form) = &mut self.panel else {
            return;
        };
        let len = form.fields().len();
        form.focus = if backwards {
            form.focus.checked_sub(1).unwrap_or(len - 1)
        } else {
            (form.focus + 1) % len
        };
        form.error = None;
    }

    fn move_password_focus(&mut self, backwards: bool) {
        let Panel::Password(form) = &mut self.panel else {
            return;
        };
        let len = form.field_count();
        form.focus = if backwards {
            form.focus.checked_sub(1).unwrap_or(len - 1)
        } else {
            (form.focus + 1) % len
        };
        form.error = None;
    }

    fn activate_form_field(&mut self) {
        let Panel::Form(form) = &mut self.panel else {
            return;
        };
        match form.field() {
            FormField::Role => form.role_expanded = Some(form.role),
            FormField::Active => {
                form.active = !form.active;
                form.error = None;
            }
            FormField::Identity | FormField::Name | FormField::Password | FormField::ConfirmPassword => {}
        }
    }

    fn cycle_form_value(&mut self, backwards: bool) {
        let Panel::Form(form) = &mut self.panel else {
            return;
        };
        match form.field() {
            FormField::Role => form.role = cycle(form.role, Role::ALL.len(), backwards),
            FormField::Active => form.active = !form.active,
            FormField::Identity | FormField::Name | FormField::Password | FormField::ConfirmPassword => return,
        }
        form.error = None;
    }

    fn save_form(&mut self) {
        let Panel::Form(form) = &self.panel else {
            return;
        };
        let identity = form.identity.value().trim().to_owned();
        let name = form.name.value().trim().to_owned();
        let error = if identity.is_empty() {
            Some("La cédula es obligatoria".to_owned())
        } else if name.is_empty() {
            Some("El nombre es obligatorio".to_owned())
        } else if self
            .users
            .iter()
            .any(|user| user.identity.eq_ignore_ascii_case(&identity) && Some(user.id) != form.id)
        {
            Some("Ya existe un usuario con esa cédula".to_owned())
        } else if !form.is_editing() {
            validate_password(form.password.value(), form.confirm_password.value()).err()
        } else {
            None
        };
        if let Some(error) = error {
            if let Panel::Form(form) = &mut self.panel {
                form.error = Some(error);
            }
            return;
        }

        let (id, role_index, active) = (form.id, form.role, form.active);
        let saved_id = if let Some(id) = id {
            if let Some(user) = self.users.iter_mut().find(|user| user.id == id) {
                user.identity = identity.clone();
                user.name = name.clone();
                user.role = Role::ALL[role_index];
                user.active = active;
            }
            id
        } else {
            let id = self.users.iter().map(|user| user.id).max().unwrap_or(0) + 1;
            self.users.push(User {
                id,
                identity: identity.clone(),
                name: name.clone(),
                role: Role::ALL[role_index],
                active,
            });
            id
        };

        self.search = Input::default();
        self.selected = self.users.iter().position(|user| user.id == saved_id).unwrap_or(0);
        self.panel = Panel::Detail;
        self.prepared_message = Some(format!("Usuario guardado en memoria · {name}"));
    }

    fn save_password(&mut self) {
        let Panel::Password(form) = &self.panel else {
            return;
        };
        if form.requires_current_password() && form.current_password.value() != DEMO_CURRENT_PASSWORD {
            if let Panel::Password(form) = &mut self.panel {
                form.error = Some("La contraseña actual no es correcta.".into());
            }
            return;
        }
        if let Err(error) = validate_password(form.password.value(), form.confirm.value()) {
            if let Panel::Password(form) = &mut self.panel {
                form.error = Some(error);
            }
            return;
        }
        // Defensivo: el usuario pudo haberse borrado en otra sesión mientras
        // se escribía la contraseña. Se confirma por id, no sólo por nombre.
        let Some(user) = self.users.iter().find(|user| user.id == form.id) else {
            self.panel = Panel::Detail;
            self.prepared_message = Some("El usuario ya no está disponible.".into());
            return;
        };
        let name = user.name.clone();
        self.panel = Panel::Detail;
        self.prepared_message = Some(format!("Contraseña actualizada · {name}"));
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let query = self.search.value().trim().to_lowercase();
        self.users
            .iter()
            .enumerate()
            .filter_map(|(index, user)| {
                (query.is_empty()
                    || user.identity.to_lowercase().contains(&query)
                    || user.name.to_lowercase().contains(&query))
                .then_some(index)
            })
            .collect()
    }

    fn selected_user(&self) -> Option<&User> {
        let indices = self.filtered_indices();
        self.users.get(*indices.get(self.selected)?)
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.filtered_indices().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self.selected.saturating_add_signed(delta).min(count - 1);
    }

    fn select(&mut self, index: usize) {
        let count = self.filtered_indices().len();
        self.selected = index.min(count.saturating_sub(1));
    }

    fn toggle_theme(&mut self) {
        self.theme = self.theme.next();
    }

    fn layer(&self) -> Layer {
        if self.confirming.is_some() {
            return Layer::Confirm;
        }
        match &self.panel {
            Panel::Form(form) if form.role_expanded.is_some() => Layer::RoleExpand,
            Panel::Form(_) => Layer::Form,
            Panel::Password(_) => Layer::Password,
            Panel::Detail => match self.mode {
                Mode::Search { .. } => Layer::Search,
                Mode::Browse => Layer::Browse,
            },
        }
    }

    fn viewport_is_valid(&self) -> bool {
        self.terminal_size.0 >= MIN_WIDTH && self.terminal_size.1 >= MIN_HEIGHT
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.terminal_size = (area.width, area.height);
        let theme = self.theme.theme();

        if !self.viewport_is_valid() {
            render_terminal_too_small(frame, area, MIN_WIDTH, MIN_HEIGHT, "ESC salir", theme);
            return;
        }

        frame.render_widget(Block::default().style(theme.base()), area);

        let hint_lines = self.hint_lines(theme);
        let hint_height = hint_lines.len() as u16;
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(1),
            Constraint::Length(hint_height),
        ])
        .split(area.inner(Margin {
            horizontal: 2,
            vertical: 1,
        }));

        self.render_top_bar(frame, rows[0], theme);
        self.render_body(frame, rows[1], theme);
        frame.render_widget(self.status_line(theme), rows[2]);
        frame.render_widget(Paragraph::new(hint_lines), rows[3]);

        self.quick_exit.render(frame, area, theme);
    }

    fn render_top_bar(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let clock = Utc::now()
            .with_timezone(&Costa_Rica)
            .format("%H:%M")
            .to_string();
        let columns =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(area);
        frame.render_widget(
            Paragraph::new("brisas cli · usuarios (v2)").style(theme.muted()),
            columns[0],
        );
        frame.render_widget(
            Paragraph::new(format!("{CURRENT_OPERATOR} · {clock}"))
                .style(theme.muted())
                .alignment(Alignment::Right),
            columns[1],
        );
    }

    fn render_body(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let rows = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);
        let count = self.filtered_indices().len();
        render_field(
            frame,
            rows[0],
            FieldSpec {
                label: &format!("BUSCAR · {count} RESULTADOS"),
                input: &self.search,
                focused: matches!(self.mode, Mode::Search { .. }),
                masked: false,
                theme,
            },
        );

        let panel_height = self.panel_row_count();
        if area.width >= WIDE_LAYOUT_WIDTH {
            let columns = Layout::horizontal([
                Constraint::Percentage(63),
                Constraint::Length(1),
                Constraint::Percentage(35),
            ])
            .split(rows[1]);
            self.render_table(frame, columns[0], theme);
            render_vertical_separator(frame, columns[1], theme);
            self.render_panel(frame, columns[2], theme);
        } else {
            let stacked = Layout::vertical([
                Constraint::Min(4),
                Constraint::Length(1),
                Constraint::Length(panel_height.min(rows[1].height.saturating_sub(5))),
            ])
            .split(rows[1]);
            self.render_table(frame, stacked[0], theme);
            render_horizontal_separator(frame, stacked[1], theme);
            self.render_panel(frame, stacked[2], theme);
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let indices = self.filtered_indices();
        self.selected = self.selected.min(indices.len().saturating_sub(1));
        let capacity = area.height.saturating_sub(2) as usize;
        let start = self.selected.saturating_sub(capacity.saturating_sub(1));
        let rows = indices.iter().skip(start).take(capacity).map(|&index| {
            let user = &self.users[index];
            let role_style = if user.role == Role::Root { theme.warning() } else { theme.base() };
            let status_style = if user.active { theme.success() } else { theme.danger() };
            Row::new([
                Cell::from(user.identity.as_str()),
                Cell::from(user.name.as_str()),
                Cell::from(user.role.label()).style(role_style),
                Cell::from(if user.active { "ACTIVO" } else { "INACTIVO" }).style(status_style),
            ])
            .style(theme.base())
        });
        let header = Row::new(["CÉDULA", "NOMBRE", "ROL", "ESTADO"])
            .style(theme.muted())
            .bottom_margin(1);
        let table = Table::new(
            rows,
            [
                Constraint::Length(14),
                Constraint::Fill(3),
                Constraint::Length(16),
                Constraint::Length(10),
            ],
        )
        .header(header)
        .row_highlight_style(theme.selected())
        .highlight_symbol("> ")
        .column_spacing(1);
        let selected = (!indices.is_empty()).then_some(self.selected.saturating_sub(start));
        frame.render_stateful_widget(
            table,
            area,
            &mut TableState::default().with_selected(selected),
        );

        if indices.is_empty() {
            frame.render_widget(
                Paragraph::new("Sin usuarios con ese filtro · Esc limpia la búsqueda")
                    .style(theme.warning())
                    .alignment(Alignment::Center),
                Rect::new(area.x, area.y + area.height / 2, area.width, 1),
            );
        }
    }

    fn panel_row_count(&self) -> u16 {
        match &self.panel {
            Panel::Detail => PANEL_HEIGHT,
            Panel::Password(form) => {
                if form.requires_current_password() {
                    11 // usuario + actual + nueva + confirmar + error
                } else {
                    8
                }
            }
            Panel::Form(form) => {
                let mut total: u16 = 3 + 3 + 1 + 1; // identity + name + rol + activo
                if form.role_expanded.is_some() {
                    total += Role::ALL.len() as u16;
                }
                if !form.is_editing() {
                    total += 3 + 3; // password + confirmar
                }
                total += 1; // error
                total
            }
        }
    }

    fn render_panel(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        match &self.panel {
            Panel::Detail => self.render_detail(frame, area, theme),
            Panel::Form(form) => self.render_form(frame, area, form, theme),
            Panel::Password(form) => self.render_password(frame, area, form, theme),
        }
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let Some(user) = self.selected_user() else {
            frame.render_widget(
                Paragraph::new("No hay un usuario seleccionado").style(theme.muted()),
                area,
            );
            return;
        };
        let role_style = if user.role == Role::Root { theme.warning() } else { theme.base() };
        let status_style = if user.active { theme.success() } else { theme.danger() };
        let lines = vec![
            Line::from(user.name.as_str()).style(theme.title()),
            Line::from(user.identity.as_str()).style(theme.muted()),
            Line::from(""),
            Line::from(user.role.label()).style(role_style),
            Line::from(if user.active { "ACTIVO" } else { "INACTIVO" }).style(status_style),
        ];
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_form(&self, frame: &mut Frame, area: Rect, form: &UserForm, theme: Theme) {
        let mut constraints = vec![Constraint::Length(3), Constraint::Length(3), Constraint::Length(1)];
        if form.role_expanded.is_some() {
            constraints.push(Constraint::Length(Role::ALL.len() as u16));
        }
        if !form.is_editing() {
            constraints.push(Constraint::Length(3));
            constraints.push(Constraint::Length(3));
        }
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Length(1));
        let rows = Layout::vertical(constraints).split(area);
        let mut cursor = 0;

        render_field(
            frame,
            rows[cursor],
            FieldSpec {
                label: "CÉDULA",
                input: &form.identity,
                focused: form.field() == FormField::Identity,
                masked: false,
                theme,
            },
        );
        cursor += 1;
        render_field(
            frame,
            rows[cursor],
            FieldSpec {
                label: "NOMBRE",
                input: &form.name,
                focused: form.field() == FormField::Name,
                masked: false,
                theme,
            },
        );
        cursor += 1;

        render_choice(frame, rows[cursor], "ROL", Role::ALL[form.role].label(), form.field() == FormField::Role, theme);
        cursor += 1;

        if let Some(highlighted) = form.role_expanded {
            let options: Vec<&str> = Role::ALL.iter().map(|role| role.label()).collect();
            render_inline_list(frame, rows[cursor], &options, highlighted, theme);
            cursor += 1;
        }

        if !form.is_editing() {
            render_field(
                frame,
                rows[cursor],
                FieldSpec {
                    label: "CONTRASEÑA",
                    input: &form.password,
                    focused: form.field() == FormField::Password,
                    masked: true,
                    theme,
                },
            );
            cursor += 1;
            render_field(
                frame,
                rows[cursor],
                FieldSpec {
                    label: "CONFIRMAR CONTRASEÑA",
                    input: &form.confirm_password,
                    focused: form.field() == FormField::ConfirmPassword,
                    masked: true,
                    theme,
                },
            );
            cursor += 1;
        }

        render_choice(
            frame,
            rows[cursor],
            "ACTIVO",
            yes_no(form.active),
            form.field() == FormField::Active,
            theme,
        );
        cursor += 1;
        frame.render_widget(
            Paragraph::new(form.error.as_deref().unwrap_or_default()).style(theme.danger()),
            rows[cursor],
        );
    }

    fn render_password(&self, frame: &mut Frame, area: Rect, form: &PasswordForm, theme: Theme) {
        let requires_current = form.requires_current_password();
        let mut constraints = vec![Constraint::Length(1)];
        if requires_current {
            constraints.push(Constraint::Length(3));
        }
        constraints.push(Constraint::Length(3));
        constraints.push(Constraint::Length(3));
        constraints.push(Constraint::Length(1));
        let rows = Layout::vertical(constraints).split(area);
        let mut cursor = 0;

        frame.render_widget(
            Paragraph::new(format!("Usuario: {}", form.user_name)).style(theme.title()),
            rows[cursor],
        );
        cursor += 1;

        if requires_current {
            render_field(
                frame,
                rows[cursor],
                FieldSpec {
                    label: "CONTRASEÑA ACTUAL",
                    input: &form.current_password,
                    focused: form.focus == 0,
                    masked: true,
                    theme,
                },
            );
            cursor += 1;
        }

        let new_password_focus = if requires_current { 1 } else { 0 };
        render_field(
            frame,
            rows[cursor],
            FieldSpec {
                label: "NUEVA CONTRASEÑA",
                input: &form.password,
                focused: form.focus == new_password_focus,
                masked: true,
                theme,
            },
        );
        cursor += 1;
        render_field(
            frame,
            rows[cursor],
            FieldSpec {
                label: "CONFIRMAR CONTRASEÑA",
                input: &form.confirm,
                focused: form.focus == new_password_focus + 1,
                masked: true,
                theme,
            },
        );
        cursor += 1;
        frame.render_widget(
            Paragraph::new(form.error.as_deref().unwrap_or_default()).style(theme.danger()),
            rows[cursor],
        );
    }

    fn status_line(&self, theme: Theme) -> Line<'static> {
        if self.confirming.is_some()
            && let Some(user) = self.selected_user()
        {
            let accion = if self.confirming.is_some_and(|pending| pending.activate) {
                "activar"
            } else {
                "desactivar"
            };
            return Line::from(format!("¿Confirma {accion} a {}?", user.name)).style(theme.warning());
        }
        if let Panel::Form(form) = &self.panel
            && let Some(error) = &form.error
        {
            return Line::from(error.clone()).style(theme.danger());
        }
        if let Panel::Password(form) = &self.panel
            && let Some(error) = &form.error
        {
            return Line::from(error.clone()).style(theme.danger());
        }
        if let Some(message) = &self.prepared_message {
            return Line::from(message.clone()).style(theme.success());
        }
        Line::from("").style(theme.muted())
    }

    fn hint_lines(&self, theme: Theme) -> Vec<Line<'static>> {
        let primary = match self.layer() {
            Layer::Browse => vec![
                Span::styled("↑↓", theme.accent()),
                Span::styled(" mover   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" editar   ", theme.base()),
                Span::styled("N", theme.accent()),
                Span::styled(" nuevo   ", theme.base()),
                Span::styled("P", theme.accent()),
                Span::styled(" contraseña   ", theme.base()),
                Span::styled("A", theme.accent()),
                Span::styled(" activar/desactivar   ", theme.base()),
                Span::styled("/", theme.accent()),
                Span::styled(" buscar   ", theme.base()),
                Span::styled(QUICK_EXIT_HINT.key, theme.accent()),
                Span::styled(format!(" {}   ", QUICK_EXIT_HINT.label), theme.base()),
                Span::styled("F1", theme.accent()),
                Span::styled(
                    if self.help_expanded { " cerrar ayuda" } else { " más" },
                    theme.base(),
                ),
            ],
            Layer::Search => vec![
                Span::styled("ENTER", theme.accent()),
                Span::styled(" aplicar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cancelar", theme.base()),
            ],
            Layer::Form => vec![
                Span::styled("TAB", theme.accent()),
                Span::styled(" campo   ", theme.base()),
                Span::styled("←→", theme.accent()),
                Span::styled(" cambiar   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" guardar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cancelar", theme.base()),
            ],
            Layer::RoleExpand => vec![
                Span::styled("↑↓", theme.accent()),
                Span::styled(" mover   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" elegir   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cerrar", theme.base()),
            ],
            Layer::Password => vec![
                Span::styled("TAB", theme.accent()),
                Span::styled(" campo   ", theme.base()),
                Span::styled("ENTER", theme.accent()),
                Span::styled(" guardar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cancelar", theme.base()),
            ],
            Layer::Confirm => vec![
                Span::styled("ENTER", theme.accent()),
                Span::styled(" confirmar   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" cancelar", theme.base()),
            ],
        };
        let mut lines = vec![Line::from(primary)];
        if self.help_expanded && self.layer() == Layer::Browse {
            lines.push(Line::from(vec![
                Span::styled("HOME/END", theme.accent()),
                Span::styled(" extremos   ", theme.base()),
                Span::styled("F7", theme.accent()),
                Span::styled(" tema   ", theme.base()),
                Span::styled("ESC", theme.accent()),
                Span::styled(" salir", theme.base()),
            ]));
        }
        lines
    }
}

fn validate_password(password: &str, confirm: &str) -> Result<(), String> {
    if password.is_empty() {
        Err("La contraseña es obligatoria".into())
    } else if confirm.is_empty() {
        Err("Debe confirmar la contraseña".into())
    } else if password.chars().count() < 8 {
        Err("La contraseña debe tener al menos 8 caracteres".into())
    } else if password != confirm {
        Err("Las contraseñas no coinciden".into())
    } else {
        Ok(())
    }
}

fn cycle(current: usize, length: usize, backwards: bool) -> usize {
    if backwards {
        current.checked_sub(1).unwrap_or(length.saturating_sub(1))
    } else {
        (current + 1) % length.max(1)
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "SÍ" } else { "NO" }
}

fn apply_event(input: &mut Input, event: &Event) -> bool {
    if let Event::Paste(text) = event {
        let mut changed = false;
        for character in text.chars().filter(|character| !character.is_control()) {
            input.handle(InputRequest::InsertChar(character));
            changed = true;
        }
        changed
    } else {
        input.handle_event(event).is_some()
    }
}

struct FieldSpec<'a> {
    label: &'a str,
    input: &'a Input,
    focused: bool,
    masked: bool,
    theme: Theme,
}

/// Misma silueta enfocado o no: etiqueta, valor, línea. Sólo cambia color.
fn render_field(frame: &mut Frame, area: Rect, spec: FieldSpec<'_>) {
    let FieldSpec {
        label,
        input,
        focused,
        masked,
        theme,
    } = spec;
    let label_style = if focused { theme.accent() } else { theme.muted() };
    let line_style = if focused { theme.accent() } else { theme.border() };
    let value_y = area.y.saturating_add(1);
    let line_y = area.y.saturating_add(2);

    frame.render_widget(
        Paragraph::new(label).style(label_style),
        Rect::new(area.x, area.y, area.width, 1),
    );
    let displayed = if masked {
        "•".repeat(input.value().chars().count())
    } else {
        input.value().to_owned()
    };
    let visual_cursor = if masked { input.value().chars().count() } else { input.visual_cursor() };
    let viewport_width = area.width.saturating_sub(1) as usize;
    let scroll = visual_cursor.saturating_sub(viewport_width);
    frame.render_widget(
        Paragraph::new(Line::from(displayed).style(theme.base())).scroll((0, scroll as u16)),
        Rect::new(area.x, value_y, area.width, 1),
    );
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(line_style),
        Rect::new(area.x, line_y, area.width, 1),
    );
    if focused {
        let column = visual_cursor.saturating_sub(scroll).min(viewport_width);
        frame.set_cursor_position((area.x + column as u16, value_y));
    }
}

fn render_choice(frame: &mut Frame, area: Rect, label: &str, value: &str, focused: bool, theme: Theme) {
    let marker = if focused { ">" } else { " " };
    let style = if focused { theme.accent() } else { theme.base() };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{marker} {label:<18}"), style),
            Span::styled(if focused { "◀ " } else { "  " }, theme.muted()),
            Span::styled(value.to_owned(), style),
            Span::styled(if focused { " ▶" } else { "" }, theme.muted()),
        ])),
        area,
    );
}

fn render_inline_list(frame: &mut Frame, area: Rect, options: &[&str], highlighted: usize, theme: Theme) {
    let lines: Vec<Line<'_>> = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let selected = index == highlighted;
            let marker = if selected { "  >" } else { "   " };
            Line::from(format!("{marker} {option}")).style(if selected { theme.selected() } else { theme.muted() })
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_vertical_separator(frame: &mut Frame, area: Rect, theme: Theme) {
    let lines: Vec<Line<'static>> = (0..area.height).map(|_| Line::from("│")).collect();
    frame.render_widget(Paragraph::new(lines).style(theme.border()), area);
}

fn render_horizontal_separator(frame: &mut Frame, area: Rect, theme: Theme) {
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(theme.border()),
        area,
    );
}

fn demo_users() -> Vec<User> {
    [
        (1, "1-1042-0881", CURRENT_OPERATOR, 0, true),
        (2, "2-0731-0440", "Marta Solano", 1, true),
        (3, "3-0520-0917", "José Ramírez", 2, true),
        (4, "4-0198-0772", "Ana Vindas", 2, false),
        (5, "1-1550-0239", "Carlos Rojas", 1, true),
        (6, "2-0611-0854", "Sofía Brenes", 2, true),
    ]
    .into_iter()
    .map(|(id, identity, name, role_index, active)| User {
        id,
        identity: identity.into(),
        name: name.into(),
        role: Role::ALL[role_index],
        active,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{FormField, Layer, Panel, UsuariosV2Pilot, ThemePreset};

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn buffer_text(backend: &TestBackend) -> String {
        backend
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn el_detalle_esta_visible_desde_el_inicio() {
        let app = UsuariosV2Pilot::default();
        assert!(matches!(app.panel, Panel::Detail));
        assert_eq!(app.layer(), Layer::Browse);
    }

    #[test]
    fn buscar_un_nombre_con_letras_de_atajos_no_dispara_ningun_atajo() {
        // "Ana Vindas" contiene n, a y... nada de p, pero cubre el caso que
        // rompía en empresas_v2 antes del arreglo.
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('/')));
        app.handle_event(&Event::Paste("Ana Vindas".into()));

        assert_eq!(app.search.value(), "Ana Vindas");
        assert!(matches!(app.panel, Panel::Detail));
        assert!(app.confirming.is_none());
    }

    #[test]
    fn enter_abre_edicion_con_cedula_editable_junto_a_la_tabla() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter));
        assert!(matches!(app.panel, Panel::Form(_)));

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains(CURRENT_OPERATOR_NAME));
        assert!(rendered.contains("CÉDULA"));
    }

    const CURRENT_OPERATOR_NAME: &str = "Daniel Quintana";

    #[test]
    fn n_abre_formulario_nuevo_y_exige_contrasena_de_al_menos_8_caracteres() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&Event::Paste("9-9999-9999".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("Persona Nueva".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("corta".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("corta".into()));
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("8 caracteres")));
    }

    #[test]
    fn crear_usuario_con_contrasenas_que_no_coinciden_falla() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&Event::Paste("9-9999-9999".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("Persona Nueva".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("contraseña1".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("contraseña2".into()));
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("no coinciden")));
    }

    #[test]
    fn tab_da_la_vuelta_en_el_formulario_nuevo() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        for _ in 0..6 {
            app.handle_event(&key(KeyCode::Tab));
        }
        let Panel::Form(form) = &app.panel else {
            panic!("debe permanecer en el formulario");
        };
        assert_eq!(form.focus, 0);
    }

    #[test]
    fn el_rol_se_expande_inline_sin_ocultar_el_resto_del_formulario() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Enter)); // edita Daniel Quintana
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&key(KeyCode::Tab)); // llega a Rol
        app.handle_event(&key(KeyCode::Char(' ')));
        assert_eq!(app.layer(), Layer::RoleExpand);

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("OPERADOR"));
        assert!(rendered.contains("ACTIVO") || rendered.contains("CÉDULA"));
    }

    #[test]
    fn p_abre_el_cambio_de_contrasena_separado_del_formulario_de_datos() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('p')));

        assert!(matches!(app.panel, Panel::Password(_)));
        assert_eq!(app.layer(), Layer::Password);
    }

    #[test]
    fn reset_administrativo_de_otro_usuario_no_pide_contrasena_actual() {
        // Marta Solano (id 2) no es la sesión activa: es un reset de admin,
        // no requiere saber la contraseña que ella olvidó.
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Char('p')));

        let Panel::Password(form) = &app.panel else {
            panic!("debe abrir el cambio de contraseña");
        };
        assert!(!form.requires_current_password());
        assert_eq!(form.field_count(), 2);
    }

    #[test]
    fn cambiar_la_propia_contrasena_exige_la_actual_correcta() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('p'))); // Daniel Quintana: la sesión activa

        let Panel::Password(form) = &app.panel else {
            panic!("debe abrir el cambio de contraseña");
        };
        assert!(form.requires_current_password());
        assert_eq!(form.field_count(), 3);

        app.handle_event(&Event::Paste("clave-incorrecta".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("nuevaClave123".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("nuevaClave123".into()));
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Password(form) = &app.panel else {
            panic!("debe permanecer en el formulario de contraseña");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("no es correcta")));
    }

    #[test]
    fn cambiar_la_propia_contrasena_con_la_actual_correcta_funciona() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('p')));
        app.handle_event(&Event::Paste(super::DEMO_CURRENT_PASSWORD.into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("nuevaClave123".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("nuevaClave123".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(matches!(app.panel, Panel::Detail));
        assert!(
            app.prepared_message
                .as_deref()
                .is_some_and(|m| m.contains("Contraseña actualizada"))
        );
    }

    #[test]
    fn cambiar_contrasena_corta_falla_y_no_pierde_el_usuario_objetivo() {
        // Marta Solano (id 2): reset administrativo, sin campo de
        // contraseña actual, así que el flujo original de 2 campos sigue
        // aplicando tal cual.
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Char('p')));
        app.handle_event(&Event::Paste("corta".into()));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Paste("corta".into()));
        app.handle_event(&key(KeyCode::Enter));

        let Panel::Password(form) = &app.panel else {
            panic!("debe permanecer en el formulario de contraseña");
        };
        assert!(form.error.as_deref().is_some_and(|e| e.contains("8 caracteres")));
        assert_eq!(form.user_name, "Marta Solano");
    }

    #[test]
    fn a_pide_confirmacion_inline_sin_modal_antes_de_cambiar_el_estado() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Down)); // Marta Solano, activa
        app.handle_event(&key(KeyCode::Char('a')));

        assert_eq!(app.layer(), Layer::Confirm);

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");
        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("Marta Solano"));
        assert!(rendered.contains("desactivar"));
    }

    #[test]
    fn confirmar_el_cambio_de_estado_lo_aplica_y_escape_lo_cancela() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Char('a')));
        app.handle_event(&key(KeyCode::Enter));

        assert!(!app.users[1].active);
        assert!(app.confirming.is_none());

        let estado_antes = app.users[1].active;
        app.handle_event(&key(KeyCode::Char('a')));
        app.handle_event(&key(KeyCode::Esc));
        assert_eq!(app.users[1].active, estado_antes, "escape no debe aplicar el cambio");
    }

    #[test]
    fn el_usuario_root_se_resalta_en_la_tabla() {
        let mut app = UsuariosV2Pilot::default();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");
        let rendered = buffer_text(terminal.backend());

        assert!(rendered.contains("ROOT"));
    }

    #[test]
    fn f2_funciona_en_medio_de_un_formulario_de_usuario_abierto() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&Event::Paste("9-1111-1111".into()));

        app.handle_event(&key(KeyCode::F(2)));
        assert!(app.quick_exit.is_open());
        app.handle_event(&Event::Paste("12".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert!(!app.quick_exit.is_open());
        let Panel::Form(form) = &app.panel else {
            panic!("debe seguir en el formulario, con lo ya escrito");
        };
        assert_eq!(form.identity.value(), "9-1111-1111");
        assert_eq!(form.field(), FormField::Identity);
    }

    #[test]
    fn pinta_todo_el_lienzo_sin_dejar_parches_sin_color() {
        let mut app = UsuariosV2Pilot::default();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal.draw(|frame| app.render(frame)).expect("debe renderizar");

        let theme = ThemePreset::Classic.theme();
        let buffer = terminal.backend().buffer();
        for (x, y) in [(0, 0), (119, 0), (0, 29), (119, 29), (60, 15)] {
            assert_eq!(
                buffer[(x, y)].bg,
                theme.background,
                "celda ({x},{y}) quedó con el fondo por defecto de la terminal"
            );
        }
    }

    #[test]
    fn terminal_pequena_bloquea_mutaciones_invisibles() {
        let mut app = UsuariosV2Pilot::default();
        app.handle_event(&key(KeyCode::Char('n')));
        let count = app.users.len();

        app.handle_event(&Event::Resize(40, 10));
        app.handle_event(&Event::Paste("9-0000-0000".into()));
        app.handle_event(&key(KeyCode::Enter));

        assert_eq!(app.users.len(), count);
    }
}
