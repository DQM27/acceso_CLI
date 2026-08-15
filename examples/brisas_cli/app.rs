use chrono::Utc;
use chrono_tz::America::Costa_Rica;
use control_acceso::tui::ui_kit::{
    CommandHint, ScreenShell, SelectMenu, SelectMenuState, StatusKind, TextInput, Theme,
    ThemePreset, auxiliary_panel, centered_content, panel,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Cell, Clear, Paragraph, Row, Table, TableState},
};

const COMPANIES: [&str; 5] = [
    "Brisas del Oeste",
    "Aldama Servicios",
    "Expenic Industrial",
    "Logística Central",
    "Mantenimiento CR",
];
const ENTRY_TYPES: [&str; 4] = ["PRAIND", "IN HOUSE", "POR CORREO", "SWAT"];
const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Contractor {
    id: u32,
    identity: String,
    name: String,
    company: String,
    entry_type: String,
    route_staff: bool,
    has_access: bool,
}

#[derive(Debug, Clone)]
enum Mode {
    Browse,
    Search {
        original: String,
        selected_id: Option<u32>,
    },
    Detail,
    Form(Box<ContractorForm>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InteractionLayer {
    Browse,
    Search,
    Detail,
    Form,
    CompanyDropdown,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatOrigin {
    code: KeyCode,
    modifiers: KeyModifiers,
    layer: InteractionLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    MoveSelection(isize),
    MovePage(isize),
    SelectFirst,
    SelectLast,
    OpenDetail,
    OpenNewForm,
    OpenEditForm,
    BeginSearch,
    ApplySearch,
    CancelSearch,
    CloseDetail,
    MoveField(bool),
    ChangeChoice(bool),
    ToggleChoice,
    MoveDropdown(isize),
    DropdownFirst,
    DropdownLast,
    AcceptDropdown,
    CloseDropdown,
    SaveForm,
    CancelForm,
    ToggleHelp,
    ToggleTheme,
    Quit,
}

#[derive(Debug, Clone, Copy)]
enum BindingAction {
    Key(KeyCode, Action),
    KeyPair(KeyCode, KeyCode, Action),
    Char(char, Action),
    CharOrKey(char, KeyCode, Action),
    CtrlCharOrKey(char, KeyCode, Action),
    VerticalSelection,
    PagingSelection,
    EdgeSelection,
    FieldNavigation,
    ChoiceNavigation,
    ToggleChoice,
    DropdownNavigation,
    DropdownEdges,
}

#[derive(Debug, Clone, Copy)]
struct Binding {
    on: BindingAction,
    hint_key: &'static str,
    label: &'static str,
    repeatable: bool,
}

impl Binding {
    const fn new(
        on: BindingAction,
        hint_key: &'static str,
        label: &'static str,
        repeatable: bool,
    ) -> Self {
        Self {
            on,
            hint_key,
            label,
            repeatable,
        }
    }

    fn resolve(self, key: KeyEvent) -> Option<Action> {
        match self.on {
            BindingAction::Key(code, action) => (key.code == code).then_some(action),
            BindingAction::KeyPair(first, second, action) => {
                matches!(key.code, code if code == first || code == second).then_some(action)
            }
            BindingAction::Char(expected, action) => key_char(key, expected).then_some(action),
            BindingAction::CharOrKey(character, code, action) => {
                (key_char(key, character) || key.code == code).then_some(action)
            }
            BindingAction::CtrlCharOrKey(character, code, action) => {
                ((key.modifiers.contains(KeyModifiers::CONTROL) && key_char(key, character))
                    || key.code == code)
                    .then_some(action)
            }
            BindingAction::VerticalSelection => match key.code {
                KeyCode::Up => Some(Action::MoveSelection(-1)),
                KeyCode::Down => Some(Action::MoveSelection(1)),
                _ => None,
            },
            BindingAction::PagingSelection => match key.code {
                KeyCode::PageUp => Some(Action::MovePage(-1)),
                KeyCode::PageDown => Some(Action::MovePage(1)),
                _ => None,
            },
            BindingAction::EdgeSelection => match key.code {
                KeyCode::Home => Some(Action::SelectFirst),
                KeyCode::End => Some(Action::SelectLast),
                _ => None,
            },
            BindingAction::FieldNavigation => match key.code {
                KeyCode::BackTab | KeyCode::Up => Some(Action::MoveField(true)),
                KeyCode::Tab | KeyCode::Down => Some(Action::MoveField(false)),
                _ => None,
            },
            BindingAction::ChoiceNavigation => match key.code {
                KeyCode::Left => Some(Action::ChangeChoice(true)),
                KeyCode::Right => Some(Action::ChangeChoice(false)),
                _ => None,
            },
            BindingAction::ToggleChoice => matches!(key.code, KeyCode::Enter | KeyCode::Char(' '))
                .then_some(Action::ToggleChoice),
            BindingAction::DropdownNavigation => match key.code {
                KeyCode::Up => Some(Action::MoveDropdown(-1)),
                KeyCode::Down => Some(Action::MoveDropdown(1)),
                _ => None,
            },
            BindingAction::DropdownEdges => match key.code {
                KeyCode::Home => Some(Action::DropdownFirst),
                KeyCode::End => Some(Action::DropdownLast),
                _ => None,
            },
        }
    }
}

const THEME_KEY: KeyCode = KeyCode::F(7);
const THEME_BINDING: Binding = Binding::new(
    BindingAction::Key(THEME_KEY, Action::ToggleTheme),
    "F7",
    "Tema",
    false,
);

const BROWSE_BINDINGS: &[Binding] = &[
    Binding::new(BindingAction::VerticalSelection, "↑↓", "Mover", true),
    Binding::new(BindingAction::PagingSelection, "PGUP/PGDN", "Página", true),
    Binding::new(BindingAction::EdgeSelection, "HOME/END", "Extremos", true),
    Binding::new(
        BindingAction::Key(KeyCode::Enter, Action::OpenDetail),
        "ENTER",
        "Detalle",
        false,
    ),
    Binding::new(
        BindingAction::Char('n', Action::OpenNewForm),
        "N",
        "Nuevo",
        false,
    ),
    Binding::new(
        BindingAction::Char('e', Action::OpenEditForm),
        "E",
        "Editar",
        false,
    ),
    Binding::new(
        BindingAction::Char('/', Action::BeginSearch),
        "/",
        "Buscar",
        false,
    ),
    Binding::new(
        BindingAction::Key(KeyCode::F(1), Action::ToggleHelp),
        "F1",
        "Ayuda",
        false,
    ),
    THEME_BINDING,
    Binding::new(
        BindingAction::CharOrKey('q', KeyCode::Esc, Action::Quit),
        "Q/ESC",
        "Salir",
        false,
    ),
];

const SEARCH_BINDINGS: &[Binding] = &[
    Binding::new(BindingAction::VerticalSelection, "↑↓", "Mover", true),
    Binding::new(BindingAction::PagingSelection, "PGUP/PGDN", "Página", true),
    Binding::new(
        BindingAction::Key(KeyCode::Enter, Action::ApplySearch),
        "ENTER",
        "Aplicar",
        false,
    ),
    Binding::new(
        BindingAction::Key(KeyCode::Esc, Action::CancelSearch),
        "ESC",
        "Cancelar",
        false,
    ),
    Binding::new(
        BindingAction::Key(KeyCode::F(1), Action::ToggleHelp),
        "F1",
        "Ayuda",
        false,
    ),
    THEME_BINDING,
];

const DETAIL_BINDINGS: &[Binding] = &[
    Binding::new(
        BindingAction::Char('e', Action::OpenEditForm),
        "E",
        "Editar",
        false,
    ),
    Binding::new(
        BindingAction::KeyPair(KeyCode::Enter, KeyCode::Esc, Action::CloseDetail),
        "ENTER/ESC",
        "Cerrar",
        false,
    ),
    Binding::new(
        BindingAction::Key(KeyCode::F(1), Action::ToggleHelp),
        "F1",
        "Ayuda",
        false,
    ),
    THEME_BINDING,
];

const FORM_BINDINGS: &[Binding] = &[
    Binding::new(BindingAction::FieldNavigation, "TAB/↑↓", "Campo", true),
    Binding::new(BindingAction::ChoiceNavigation, "←→", "Cambiar", true),
    Binding::new(
        BindingAction::ToggleChoice,
        "ENTER/ESP",
        "Abrir/alternar",
        false,
    ),
    Binding::new(
        BindingAction::CtrlCharOrKey('s', KeyCode::F(10), Action::SaveForm),
        "F10/CTRL+S",
        "Guardar",
        false,
    ),
    Binding::new(
        BindingAction::Key(KeyCode::Esc, Action::CancelForm),
        "ESC",
        "Cancelar",
        false,
    ),
    Binding::new(
        BindingAction::Key(KeyCode::F(1), Action::ToggleHelp),
        "F1",
        "Ayuda",
        false,
    ),
    THEME_BINDING,
];

const DROPDOWN_BINDINGS: &[Binding] = &[
    Binding::new(BindingAction::DropdownNavigation, "↑↓", "Mover", true),
    Binding::new(BindingAction::DropdownEdges, "HOME/END", "Extremos", true),
    Binding::new(
        BindingAction::Key(KeyCode::Enter, Action::AcceptDropdown),
        "ENTER",
        "Seleccionar",
        false,
    ),
    Binding::new(
        BindingAction::Key(KeyCode::Esc, Action::CloseDropdown),
        "ESC",
        "Cerrar",
        false,
    ),
    Binding::new(
        BindingAction::Key(KeyCode::F(1), Action::ToggleHelp),
        "F1",
        "Ayuda",
        false,
    ),
    THEME_BINDING,
];

const HELP_BINDINGS: &[Binding] = &[
    Binding::new(
        BindingAction::KeyPair(KeyCode::Esc, KeyCode::F(1), Action::ToggleHelp),
        "ESC/F1",
        "Cerrar ayuda",
        false,
    ),
    THEME_BINDING,
];

fn key_char(key: KeyEvent, expected: char) -> bool {
    matches!(key.code, KeyCode::Char(actual) if actual.eq_ignore_ascii_case(&expected))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormField {
    Identity,
    Name,
    Company,
    EntryType,
    RouteStaff,
    Access,
}

impl FormField {
    const NEW_FIELDS: [Self; 6] = [
        Self::Identity,
        Self::Name,
        Self::Company,
        Self::EntryType,
        Self::RouteStaff,
        Self::Access,
    ];
    const EDIT_FIELDS: [Self; 5] = [
        Self::Name,
        Self::Company,
        Self::EntryType,
        Self::RouteStaff,
        Self::Access,
    ];
}

#[derive(Debug, Clone)]
struct ContractorForm {
    id: Option<u32>,
    identity: TextInput,
    name: TextInput,
    company: usize,
    company_dropdown: SelectMenuState,
    entry_type: usize,
    route_staff: bool,
    has_access: bool,
    focus: usize,
    error: Option<String>,
}

impl ContractorForm {
    fn new() -> Self {
        Self {
            id: None,
            identity: TextInput::default().with_max_chars(30),
            name: TextInput::default().with_max_chars(80),
            company: 0,
            company_dropdown: SelectMenuState::closed(),
            entry_type: 0,
            route_staff: false,
            has_access: true,
            focus: 0,
            error: None,
        }
    }

    fn edit(contractor: &Contractor) -> Self {
        Self {
            id: Some(contractor.id),
            identity: TextInput::new(contractor.identity.clone()).with_max_chars(30),
            name: TextInput::new(contractor.name.clone()).with_max_chars(80),
            company: COMPANIES
                .iter()
                .position(|company| *company == contractor.company)
                .unwrap_or(0),
            company_dropdown: SelectMenuState::closed(),
            entry_type: ENTRY_TYPES
                .iter()
                .position(|entry_type| *entry_type == contractor.entry_type)
                .unwrap_or(0),
            route_staff: contractor.route_staff,
            has_access: contractor.has_access,
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

    fn move_focus(&mut self, backwards: bool) {
        let last = self.fields().len().saturating_sub(1);
        self.focus = if backwards {
            self.focus.saturating_sub(1)
        } else {
            (self.focus + 1).min(last)
        };
        self.error = None;
    }

    fn cycle_value(&mut self, backwards: bool) {
        match self.field() {
            FormField::Company => {
                self.company = cycle(self.company, COMPANIES.len(), backwards);
            }
            FormField::EntryType => {
                self.entry_type = cycle(self.entry_type, ENTRY_TYPES.len(), backwards);
            }
            FormField::RouteStaff => self.route_staff = !self.route_staff,
            FormField::Access => self.has_access = !self.has_access,
            FormField::Identity | FormField::Name => {}
        }
        self.error = None;
    }
}

#[derive(Debug)]
pub struct PilotApp {
    contractors: Vec<Contractor>,
    selected: usize,
    table_offset: usize,
    page_size: usize,
    search: TextInput,
    mode: Mode,
    help_visible: bool,
    theme: ThemePreset,
    status: String,
    status_kind: StatusKind,
    running: bool,
    terminal_size: (u16, u16),
    repeat_origin: Option<RepeatOrigin>,
}

impl Default for PilotApp {
    fn default() -> Self {
        Self {
            contractors: demo_contractors(),
            selected: 0,
            table_offset: 0,
            page_size: 8,
            search: TextInput::default().with_max_chars(120),
            mode: Mode::Browse,
            help_visible: false,
            theme: ThemePreset::Classic,
            status: "PILOTO AISLADO · Datos ficticios; nada se guarda en la base".into(),
            status_kind: StatusKind::Normal,
            running: true,
            terminal_size: (u16::MAX, u16::MAX),
            repeat_origin: None,
        }
    }
}

impl PilotApp {
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

        if key.is_some_and(|key| {
            key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
        }) {
            self.running = false;
            return true;
        }

        if !self.viewport_is_valid() {
            if key.is_some_and(|key| {
                key.kind == KeyEventKind::Press && (key.code == KeyCode::Esc || key_char(key, 'q'))
            }) {
                self.running = false;
                return true;
            }
            return false;
        }

        let Some(key) = key else {
            if self.help_visible {
                return false;
            }
            return self.handle_text_input(event);
        };
        let origin = RepeatOrigin {
            code: key.code,
            modifiers: key.modifiers,
            layer: self.interaction_layer(),
        };
        match key.kind {
            KeyEventKind::Press => self.repeat_origin = Some(origin),
            KeyEventKind::Repeat if self.repeat_origin != Some(origin) => return false,
            KeyEventKind::Repeat | KeyEventKind::Release => {}
        }

        let resolved = self.active_bindings().iter().find_map(|binding| {
            (key.kind == KeyEventKind::Press || binding.repeatable)
                .then(|| binding.resolve(key))
                .flatten()
        });
        if let Some(action) = resolved
            && self.apply_action(action)
        {
            return true;
        }

        if self.help_visible {
            return false;
        }

        self.handle_text_input(event)
    }

    fn active_bindings(&self) -> &'static [Binding] {
        if self.help_visible {
            HELP_BINDINGS
        } else {
            match &self.mode {
                Mode::Browse => BROWSE_BINDINGS,
                Mode::Search { .. } => SEARCH_BINDINGS,
                Mode::Detail => DETAIL_BINDINGS,
                Mode::Form(form) if form.company_dropdown.is_open() => DROPDOWN_BINDINGS,
                Mode::Form(_) => FORM_BINDINGS,
            }
        }
    }

    fn interaction_layer(&self) -> InteractionLayer {
        if self.help_visible {
            InteractionLayer::Help
        } else {
            match &self.mode {
                Mode::Browse => InteractionLayer::Browse,
                Mode::Search { .. } => InteractionLayer::Search,
                Mode::Detail => InteractionLayer::Detail,
                Mode::Form(form) if form.company_dropdown.is_open() => {
                    InteractionLayer::CompanyDropdown
                }
                Mode::Form(_) => InteractionLayer::Form,
            }
        }
    }

    fn viewport_is_valid(&self) -> bool {
        self.terminal_size.0 >= MIN_WIDTH && self.terminal_size.1 >= MIN_HEIGHT
    }

    fn apply_action(&mut self, action: Action) -> bool {
        match action {
            Action::MoveSelection(delta) => self.move_selection(delta),
            Action::MovePage(direction) => {
                self.move_selection(direction * self.page_size.max(1) as isize)
            }
            Action::SelectFirst => self.select_first(),
            Action::SelectLast => self.select_last(),
            Action::OpenDetail => {
                if self.selected_contractor().is_none() {
                    return false;
                }
                self.mode = Mode::Detail;
            }
            Action::OpenNewForm => self.mode = Mode::Form(Box::new(ContractorForm::new())),
            Action::OpenEditForm => {
                if self.selected_contractor().is_none() {
                    return false;
                }
                self.open_edit_form();
            }
            Action::BeginSearch => {
                self.mode = Mode::Search {
                    original: self.search.value().to_owned(),
                    selected_id: self.selected_contractor().map(|contractor| contractor.id),
                };
                self.set_status("Búsqueda activa · escriba para filtrar", StatusKind::Normal);
            }
            Action::ApplySearch => {
                self.mode = Mode::Browse;
                let count = self.filtered_indices().len();
                self.set_status(
                    format!("Filtro aplicado · {count} resultados"),
                    StatusKind::Success,
                );
            }
            Action::CancelSearch => {
                let (original, selected_id) = match &self.mode {
                    Mode::Search {
                        original,
                        selected_id,
                    } => (original.clone(), *selected_id),
                    _ => return false,
                };
                self.search.set_value(original);
                self.table_offset = 0;
                self.select_id(selected_id);
                self.mode = Mode::Browse;
                self.set_status("Búsqueda cancelada", StatusKind::Normal);
            }
            Action::CloseDetail => self.mode = Mode::Browse,
            Action::MoveField(backwards) => {
                let Mode::Form(form) = &mut self.mode else {
                    return false;
                };
                form.move_focus(backwards);
            }
            Action::ChangeChoice(backwards) => {
                let Mode::Form(form) = &mut self.mode else {
                    return false;
                };
                if matches!(form.field(), FormField::Identity | FormField::Name) {
                    return false;
                }
                form.cycle_value(backwards);
            }
            Action::ToggleChoice => {
                let Mode::Form(form) = &mut self.mode else {
                    return false;
                };
                if matches!(form.field(), FormField::Identity | FormField::Name) {
                    return false;
                }
                if form.field() == FormField::Company {
                    form.company_dropdown.open(form.company, COMPANIES.len());
                } else {
                    form.cycle_value(false);
                }
            }
            Action::MoveDropdown(delta) => {
                let Mode::Form(form) = &mut self.mode else {
                    return false;
                };
                return form.company_dropdown.move_by(delta, COMPANIES.len());
            }
            Action::DropdownFirst => {
                let Mode::Form(form) = &mut self.mode else {
                    return false;
                };
                return form.company_dropdown.first(COMPANIES.len());
            }
            Action::DropdownLast => {
                let Mode::Form(form) = &mut self.mode else {
                    return false;
                };
                return form.company_dropdown.last(COMPANIES.len());
            }
            Action::AcceptDropdown => {
                let Mode::Form(form) = &mut self.mode else {
                    return false;
                };
                let Some(highlighted) = form.company_dropdown.accept(COMPANIES.len()) else {
                    return false;
                };
                form.company = highlighted;
                form.error = None;
            }
            Action::CloseDropdown => {
                let Mode::Form(form) = &mut self.mode else {
                    return false;
                };
                return form.company_dropdown.close();
            }
            Action::SaveForm => self.save_form(),
            Action::CancelForm => {
                self.mode = Mode::Browse;
                self.set_status("Edición cancelada · no hubo cambios", StatusKind::Warning);
            }
            Action::ToggleHelp => self.help_visible = !self.help_visible,
            Action::ToggleTheme => {
                self.theme = self.theme.next();
                self.set_status(
                    format!(
                        "Tema {} aplicado sin cambiar la pantalla",
                        self.theme.name()
                    ),
                    StatusKind::Success,
                );
            }
            Action::Quit => self.running = false,
        }
        true
    }

    fn handle_text_input(&mut self, event: &Event) -> bool {
        let changed = match &mut self.mode {
            Mode::Search { .. } => self.search.handle_event(event),
            Mode::Form(form) if form.company_dropdown.is_open() => false,
            Mode::Form(form) => match form.field() {
                FormField::Identity if !form.is_editing() => form.identity.handle_event(event),
                FormField::Name => form.name.handle_event(event),
                _ => false,
            },
            Mode::Browse | Mode::Detail => false,
        };
        if !changed {
            return false;
        }

        match &mut self.mode {
            Mode::Search { .. } => {
                self.selected = 0;
                self.table_offset = 0;
                let count = self.filtered_indices().len();
                self.set_status(
                    format!("Filtrando en memoria · {count} resultados"),
                    StatusKind::Normal,
                );
            }
            Mode::Form(form) => form.error = None,
            Mode::Browse | Mode::Detail => {}
        }
        true
    }

    fn open_edit_form(&mut self) {
        if let Some(contractor) = self.selected_contractor().cloned() {
            self.mode = Mode::Form(Box::new(ContractorForm::edit(&contractor)));
        }
    }

    fn save_form(&mut self) {
        let Some(form) = (match &self.mode {
            Mode::Form(form) => Some(form.clone()),
            _ => None,
        }) else {
            return;
        };

        let identity = form.identity.value().trim();
        let name = form.name.value().trim();
        let error = if identity.is_empty() {
            Some("La cédula es obligatoria")
        } else if name.is_empty() {
            Some("El nombre es obligatorio")
        } else if form.id.is_none()
            && self
                .contractors
                .iter()
                .any(|contractor| contractor.identity.eq_ignore_ascii_case(identity))
        {
            Some("Ya existe un contratista con esa cédula")
        } else {
            None
        };

        if let Some(error) = error {
            if let Mode::Form(current) = &mut self.mode {
                current.error = Some(error.into());
            }
            return;
        }

        let saved_id = if let Some(id) = form.id {
            if let Some(contractor) = self.contractors.iter_mut().find(|item| item.id == id) {
                contractor.name = name.into();
                contractor.company = COMPANIES[form.company].into();
                contractor.entry_type = ENTRY_TYPES[form.entry_type].into();
                contractor.route_staff = form.route_staff;
                contractor.has_access = form.has_access;
            }
            id
        } else {
            let id = self
                .contractors
                .iter()
                .map(|contractor| contractor.id)
                .max()
                .unwrap_or(0)
                + 1;
            self.contractors.push(Contractor {
                id,
                identity: identity.into(),
                name: name.into(),
                company: COMPANIES[form.company].into(),
                entry_type: ENTRY_TYPES[form.entry_type].into(),
                route_staff: form.route_staff,
                has_access: form.has_access,
            });
            id
        };

        self.search.clear();
        self.table_offset = 0;
        self.selected = self
            .contractors
            .iter()
            .position(|contractor| contractor.id == saved_id)
            .unwrap_or(0);
        self.mode = Mode::Browse;
        self.set_status(
            format!("Contratista guardado en la memoria del piloto · {name}"),
            StatusKind::Success,
        );
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let filter = self.search.value().trim().to_lowercase();
        self.contractors
            .iter()
            .enumerate()
            .filter_map(|(index, contractor)| {
                (filter.is_empty()
                    || contractor.identity.to_lowercase().contains(&filter)
                    || contractor.name.to_lowercase().contains(&filter)
                    || contractor.company.to_lowercase().contains(&filter)
                    || contractor.entry_type.to_lowercase().contains(&filter))
                .then_some(index)
            })
            .collect()
    }

    fn selected_contractor(&self) -> Option<&Contractor> {
        let indices = self.filtered_indices();
        self.contractors.get(*indices.get(self.selected)?)
    }

    fn select_id(&mut self, id: Option<u32>) {
        let indices = self.filtered_indices();
        self.selected = id
            .and_then(|id| {
                indices
                    .iter()
                    .position(|index| self.contractors[*index].id == id)
            })
            .unwrap_or(0);
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.filtered_indices().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(count.saturating_sub(1));
    }

    fn select_first(&mut self) {
        self.selected = 0;
    }

    fn select_last(&mut self) {
        self.selected = self.filtered_indices().len().saturating_sub(1);
    }

    fn set_status(&mut self, message: impl Into<String>, kind: StatusKind) {
        self.status = message.into();
        self.status_kind = kind;
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.terminal_size = (area.width, area.height);
        let theme = self.theme.theme();
        if !self.viewport_is_valid() {
            render_terminal_too_small(frame, area, theme);
            return;
        }

        let clock = Utc::now()
            .with_timezone(&Costa_Rica)
            .format("%d/%m/%Y  %H:%M:%S")
            .to_string();
        let context = format!("PILOTO · SIN BASE · {}", self.theme.name());
        let commands = self.command_hints();
        let shell = ScreenShell {
            product: "B R I S A S   C L I",
            screen: "CONTRATISTAS",
            context: &context,
            clock: &clock,
            status: &self.status,
            status_kind: self.status_kind,
            commands: &commands,
        };
        let areas = shell.render(frame, area, theme);
        self.render_body(frame, areas.body, theme);

        match &self.mode {
            Mode::Detail => self.render_detail_overlay(frame, area, theme),
            Mode::Form(form) => self.render_form(frame, area, form, theme, !self.help_visible),
            Mode::Browse | Mode::Search { .. } => {}
        }
        if self.help_visible {
            self.render_help(frame, area, theme);
        }
    }

    fn command_hints(&self) -> Vec<CommandHint<'static>> {
        self.active_bindings()
            .iter()
            .map(|binding| CommandHint::new(binding.hint_key, binding.label))
            .collect()
    }

    fn render_body(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let rows = Layout::vertical([Constraint::Length(3), Constraint::Min(4)]).split(area);
        let search_label = format!("BUSCAR · {} RESULTADOS", self.filtered_indices().len());
        self.search.render(
            frame,
            rows[0],
            &search_label,
            "Presione / para buscar por cédula, nombre o empresa",
            matches!(self.mode, Mode::Search { .. }) && !self.help_visible,
            theme,
        );

        if area.width >= 100 {
            let columns =
                Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)])
                    .split(rows[1]);
            self.render_table(frame, columns[0], theme);
            self.render_detail_panel(frame, columns[1], theme);
        } else {
            self.render_table(frame, rows[1], theme);
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect, theme: Theme) {
        let indices = self.filtered_indices();
        let table_block = panel("LISTADO", theme, true);
        let table_inner = table_block.inner(area);
        let capacity = table_inner.height.saturating_sub(2).max(1) as usize;
        self.page_size = capacity;

        if indices.is_empty() {
            self.selected = 0;
            self.table_offset = 0;
        } else {
            self.selected = self.selected.min(indices.len() - 1);
            if self.selected < self.table_offset {
                self.table_offset = self.selected;
            } else if self.selected >= self.table_offset.saturating_add(capacity) {
                self.table_offset = self.selected + 1 - capacity;
            }
            self.table_offset = self
                .table_offset
                .min(indices.len().saturating_sub(capacity));
        }

        let wide = area.width >= 76;
        let rows = indices
            .iter()
            .skip(self.table_offset)
            .take(capacity)
            .map(|index| {
                let contractor = &self.contractors[*index];
                let access = if contractor.has_access { "SÍ" } else { "NO" };
                let access_style = if contractor.has_access {
                    theme.success()
                } else {
                    theme.danger()
                };
                let cells = if wide {
                    vec![
                        Cell::from(contractor.identity.as_str()),
                        Cell::from(contractor.name.as_str()),
                        Cell::from(contractor.company.as_str()),
                        Cell::from(contractor.entry_type.as_str()),
                        Cell::from(access).style(access_style),
                    ]
                } else {
                    vec![
                        Cell::from(contractor.identity.as_str()),
                        Cell::from(contractor.name.as_str()),
                        Cell::from(access).style(access_style),
                    ]
                };
                Row::new(cells).style(theme.base())
            });
        let headers = if wide {
            vec!["CÉDULA", "NOMBRE", "EMPRESA", "TIPO", "ACCESO"]
        } else {
            vec!["CÉDULA", "NOMBRE", "ACCESO"]
        };
        let widths = if wide {
            vec![
                Constraint::Length(14),
                Constraint::Fill(3),
                Constraint::Fill(2),
                Constraint::Length(12),
                Constraint::Length(8),
            ]
        } else {
            vec![
                Constraint::Length(14),
                Constraint::Fill(1),
                Constraint::Length(8),
            ]
        };
        let header = Row::new(headers).style(theme.accent()).bottom_margin(1);
        let table = Table::new(rows, widths)
            .header(header)
            .block(table_block)
            .row_highlight_style(theme.selected())
            .highlight_symbol("> ")
            .column_spacing(1);
        let mut state = TableState::default().with_selected(
            (!indices.is_empty()).then_some(self.selected.saturating_sub(self.table_offset)),
        );
        frame.render_stateful_widget(table, area, &mut state);

        if indices.is_empty() {
            let inner = Rect::new(
                area.x.saturating_add(2),
                area.y + area.height / 2,
                area.width.saturating_sub(4),
                1,
            );
            frame.render_widget(
                Paragraph::new("Sin coincidencias · ESC cancela el filtro")
                    .style(theme.warning())
                    .alignment(Alignment::Center),
                inner,
            );
        }
    }

    fn render_detail_panel(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let block = panel("DETALLE", theme, false);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let Some(contractor) = self.selected_contractor() else {
            frame.render_widget(
                Paragraph::new("No hay un registro seleccionado").style(theme.muted()),
                inner,
            );
            return;
        };
        frame.render_widget(
            Paragraph::new(detail_lines(contractor, theme))
                .wrap(ratatui::widgets::Wrap { trim: false }),
            inner,
        );
    }

    fn render_detail_overlay(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let Some(contractor) = self.selected_contractor() else {
            return;
        };
        let popup = centered(area, 68, 15);
        frame.render_widget(Clear, popup);
        let block = auxiliary_panel("DETALLE DEL CONTRATISTA", theme, true);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let content = centered_content(inner, 54);
        let mut lines = detail_lines(contractor, theme);
        lines.push(Line::from(""));
        lines.push(
            Line::from("Las acciones disponibles están en la barra inferior.").style(theme.muted()),
        );
        frame.render_widget(Paragraph::new(lines), content);
    }

    fn render_form(
        &self,
        frame: &mut Frame,
        area: Rect,
        form: &ContractorForm,
        theme: Theme,
        focus_enabled: bool,
    ) {
        let popup = centered(area, 78, 20);
        frame.render_widget(Clear, popup);
        let title = if form.is_editing() {
            "EDITAR CONTRATISTA · DATOS EN MEMORIA"
        } else {
            "NUEVO CONTRATISTA · DATOS EN MEMORIA"
        };
        let block = auxiliary_panel(title, theme, true);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let content = centered_content(inner, 68);

        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(content);

        let identity_focused =
            focus_enabled && form.field() == FormField::Identity && !form.is_editing();
        form.identity.render(
            frame,
            rows[0],
            if form.is_editing() {
                "CÉDULA · IDENTIDAD NO EDITABLE"
            } else {
                "CÉDULA"
            },
            "Identificador único",
            identity_focused,
            theme,
        );
        form.name.render(
            frame,
            rows[1],
            "NOMBRE",
            "Nombre completo",
            focus_enabled && form.field() == FormField::Name,
            theme,
        );
        render_choice(
            frame,
            rows[2],
            "EMPRESA",
            COMPANIES[form.company],
            focus_enabled && form.field() == FormField::Company,
            theme,
        );
        render_choice(
            frame,
            rows[3],
            "TIPO DE INGRESO",
            ENTRY_TYPES[form.entry_type],
            focus_enabled && form.field() == FormField::EntryType,
            theme,
        );
        render_choice(
            frame,
            rows[4],
            "PERSONAL DE RUTA",
            yes_no(form.route_staff),
            focus_enabled && form.field() == FormField::RouteStaff,
            theme,
        );
        render_choice(
            frame,
            rows[5],
            "TIENE ACCESO",
            yes_no(form.has_access),
            focus_enabled && form.field() == FormField::Access,
            theme,
        );
        frame.render_widget(
            Paragraph::new(form.error.as_deref().unwrap_or_default()).style(theme.danger()),
            rows[6],
        );
        frame.render_widget(
            Paragraph::new("Las acciones disponibles están en la barra inferior.")
                .style(theme.muted()),
            rows[7],
        );

        if form.company_dropdown.is_open() {
            let dropdown_width = 46.min(content.width);
            let dropdown_height = (COMPANIES.len() as u16 + 2).min(content.height);
            let rightmost_x = content.x + content.width.saturating_sub(dropdown_width);
            let lowest_y = content.y + content.height.saturating_sub(dropdown_height);
            let dropdown_area = Rect::new(
                (rows[2].x + 22).min(rightmost_x),
                (rows[2].y + 1).min(lowest_y),
                dropdown_width,
                dropdown_height,
            );

            SelectMenu {
                title: "SELECCIONAR EMPRESA",
                options: &COMPANIES,
                current: Some(form.company),
            }
            .render(frame, dropdown_area, &form.company_dropdown, theme);
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect, theme: Theme) {
        let popup = centered(area, 76, 18);
        frame.render_widget(Clear, popup);
        let block = auxiliary_panel("CONTRATO DE INTERACCIÓN DEL PILOTO", theme, true);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let content = centered_content(inner, 66);
        let lines = vec![
            Line::from("Este ejemplo no abre SQLite ni usa AppCore.").style(theme.warning()),
            Line::from(""),
            Line::from("La barra inferior muestra exactamente los comandos activos."),
            Line::from(""),
            Line::from("• Mantener flechas, PageUp/PageDown o Backspace repite."),
            Line::from("• Los comandos que cambian de pantalla se ejecutan una vez."),
            Line::from("• Enter realiza la acción principal del contexto."),
            Line::from("• Esc retrocede exactamente una capa."),
            Line::from("• Tab y Shift+Tab recorren campos del formulario."),
            Line::from("• Ctrl+C queda reservado como salida de emergencia."),
            Line::from(""),
            Line::from("El cursor es el cursor real del terminal; no se dibuja “_”.")
                .style(theme.success()),
            Line::from("ESC / F1 cerrar ayuda").style(theme.accent()),
        ];
        frame.render_widget(Paragraph::new(lines), content);
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

fn render_choice(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    theme: Theme,
) {
    let marker = if focused { ">" } else { " " };
    let style = if focused {
        theme.accent()
    } else {
        theme.base()
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{marker} {label:<22}"), style),
            Span::styled(if focused { "◀ " } else { "  " }, theme.muted()),
            Span::styled(format!("{value:<24}"), style),
            Span::styled(if focused { " ▶" } else { "  " }, theme.muted()),
        ])),
        Rect::new(area.x, area.y, area.width, 1),
    );
}

fn detail_lines(contractor: &Contractor, theme: Theme) -> Vec<Line<'_>> {
    vec![
        Line::from(contractor.name.as_str()).style(theme.title()),
        Line::from(""),
        labeled("Cédula", &contractor.identity, theme),
        labeled("Empresa", &contractor.company, theme),
        labeled("Tipo", &contractor.entry_type, theme),
        labeled("Personal de ruta", yes_no(contractor.route_staff), theme),
        Line::from(vec![
            Span::styled("Acceso              ", theme.muted()),
            Span::styled(
                yes_no(contractor.has_access),
                if contractor.has_access {
                    theme.success()
                } else {
                    theme.danger()
                },
            ),
        ]),
    ]
}

fn labeled<'a>(label: &'a str, value: &'a str, theme: Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<20}"), theme.muted()),
        Span::styled(value, theme.base()),
    ])
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(2));
    let height = height.min(area.height.saturating_sub(2));
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn render_terminal_too_small(frame: &mut Frame, area: Rect, theme: Theme) {
    frame.render_widget(ratatui::widgets::Block::default().style(theme.base()), area);
    let y = area.y + area.height.saturating_sub(3) / 2;
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("TERMINAL DEMASIADO PEQUEÑA").style(theme.warning()),
            Line::from(format!(
                "Actual: {}×{} · mínimo: {}×{}",
                area.width, area.height, MIN_WIDTH, MIN_HEIGHT
            ))
            .style(theme.base()),
            Line::from("Amplíe para continuar · Q/ESC salir").style(theme.muted()),
        ])
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 3),
    );
}

fn demo_contractors() -> Vec<Contractor> {
    [
        (1, "1-1042-0881", "José Peña", 0, 0, false, true),
        (2, "2-0731-0440", "Ana María Solís", 1, 1, false, true),
        (3, "3-0520-0917", "Juan Rodríguez", 2, 0, true, false),
        (4, "4-0198-0772", "Mónica Quesada", 3, 2, false, true),
        (5, "1-1550-0239", "Carlos Méndez", 4, 3, true, true),
        (6, "2-0611-0854", "Sofía Núñez", 0, 1, false, true),
        (7, "3-0488-0312", "Edgar Chacón", 2, 0, false, false),
        (8, "1-1270-0641", "María Fernanda Rojas", 1, 2, false, true),
        (9, "4-0221-0178", "Luis Ángel Mora", 3, 0, true, true),
        (10, "2-0810-0305", "Valeria Jiménez", 4, 1, false, true),
        (11, "1-0991-0518", "Óscar Zamora", 0, 3, false, true),
        (12, "3-0633-0720", "Gabriela Araya", 2, 0, false, true),
    ]
    .into_iter()
    .map(
        |(id, identity, name, company, entry_type, route_staff, has_access)| Contractor {
            id,
            identity: identity.into(),
            name: name.into(),
            company: COMPANIES[company].into(),
            entry_type: ENTRY_TYPES[entry_type].into(),
            route_staff,
            has_access,
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use super::{COMPANIES, FormField, Mode, PilotApp, THEME_KEY, ThemePreset};

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn repeat(code: KeyCode) -> Event {
        let mut key = KeyEvent::new(code, KeyModifiers::NONE);
        key.kind = KeyEventKind::Repeat;
        Event::Key(key)
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
    fn la_busqueda_filtra_unicode_en_memoria() {
        let mut app = PilotApp::default();

        app.handle_event(&key(KeyCode::Char('/')));
        app.handle_event(&Event::Paste("josé".into()));

        assert_eq!(app.filtered_indices().len(), 1);
        assert_eq!(
            app.selected_contractor().map(|item| item.name.as_str()),
            Some("José Peña")
        );
    }

    #[test]
    fn una_g_mayuscula_se_escribe_y_no_guarda_el_formulario() {
        let mut app = PilotApp::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('G'),
            KeyModifiers::SHIFT,
        )));

        let Mode::Form(form) = &app.mode else {
            panic!("el formulario no debe cerrarse al escribir G");
        };
        assert_eq!(form.field(), FormField::Name);
        assert_eq!(form.name.value(), "G");
    }

    #[test]
    fn el_tema_cambia_sin_recrear_el_estado() {
        let mut app = PilotApp::default();
        let count = app.contractors.len();

        app.handle_event(&key(THEME_KEY));
        app.handle_event(&repeat(THEME_KEY));

        assert_eq!(app.theme, ThemePreset::Brisas);
        assert_eq!(app.contractors.len(), count);
    }

    #[test]
    fn renderiza_los_tamanos_objetivo_sin_salirse_del_area() {
        for (width, height) in [(60, 20), (80, 24), (120, 30)] {
            let mut app = PilotApp::default();
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("backend de prueba");

            terminal
                .draw(|frame| app.render(frame))
                .expect("la pantalla debe renderizar");

            assert_eq!(terminal.backend().buffer().area.width, width);
            assert_eq!(terminal.backend().buffer().area.height, height);
            let rendered = buffer_text(terminal.backend());
            assert!(rendered.contains("CONTRATISTAS"), "ancho {width}");
            assert!(rendered.contains("Q/ESC Salir"), "ancho {width}");
            assert!(!terminal.backend().cursor_visible());
        }
    }

    #[test]
    fn muestra_un_cursor_real_dentro_del_campo_de_busqueda() {
        let mut app = PilotApp::default();
        app.handle_event(&key(KeyCode::Char('/')));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| app.render(frame))
            .expect("la pantalla debe renderizar");

        let cursor = terminal.backend().cursor_position();
        assert!(terminal.backend().cursor_visible());
        assert!(cursor.x < 80);
        assert!(cursor.y < 24);
    }

    #[test]
    fn repeat_no_atraviesa_la_capa_que_abrio_el_press() {
        let mut app = PilotApp::default();

        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&repeat(KeyCode::Char('n')));

        let Mode::Form(form) = &app.mode else {
            panic!("debe permanecer en el formulario");
        };
        assert!(form.identity.value().is_empty());

        let mut app = PilotApp::default();
        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&repeat(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Detail));
    }

    #[test]
    fn desplegable_de_empresa_permite_elegir_una_opcion() {
        let mut app = PilotApp::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&key(KeyCode::Tab));

        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Enter));

        let Mode::Form(form) = &app.mode else {
            panic!("debe permanecer en el formulario");
        };
        assert!(!form.company_dropdown.is_open());
        assert_eq!(COMPANIES[form.company], "Expenic Industrial");
    }

    #[test]
    fn escape_cierra_el_desplegable_sin_cambiar_la_empresa() {
        let mut app = PilotApp::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&key(KeyCode::Tab));

        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Esc));

        let Mode::Form(form) = &app.mode else {
            panic!("debe permanecer en el formulario");
        };
        assert!(!form.company_dropdown.is_open());
        assert_eq!(form.company, 0);
    }

    #[test]
    fn repeat_de_enter_no_confirma_el_desplegable_que_acaba_de_abrir() {
        let mut app = PilotApp::default();
        app.handle_event(&key(KeyCode::Char('n')));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&key(KeyCode::Tab));

        app.handle_event(&key(KeyCode::Enter));
        app.handle_event(&repeat(KeyCode::Enter));

        let Mode::Form(form) = &app.mode else {
            panic!("debe permanecer en el formulario");
        };
        assert_eq!(form.company_dropdown.highlighted(), Some(0));
    }

    #[test]
    fn renderiza_el_desplegable_de_empresa_en_el_tamano_minimo() {
        let mut app = PilotApp::default();
        app.handle_event(&key(KeyCode::Char('e')));
        app.handle_event(&key(KeyCode::Tab));
        app.handle_event(&key(KeyCode::Enter));
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| app.render(frame))
            .expect("el desplegable debe renderizar");

        let rendered = buffer_text(terminal.backend());
        assert!(rendered.contains("SELECCIONAR EMPRESA"));
        assert!(rendered.contains("Expenic Industrial"));
    }

    #[test]
    fn ayuda_oculta_el_cursor_de_la_capa_inferior() {
        let mut app = PilotApp::default();
        app.handle_event(&key(KeyCode::Char('/')));
        app.handle_event(&key(KeyCode::F(1)));
        app.handle_event(&Event::Paste("no debe pasar".into()));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("backend de prueba");

        terminal
            .draw(|frame| app.render(frame))
            .expect("la ayuda debe renderizar");

        assert!(app.help_visible);
        assert!(app.search.value().is_empty());
        assert!(!terminal.backend().cursor_visible());
    }

    #[test]
    fn cancelar_busqueda_restaura_filtro_y_seleccion() {
        let mut app = PilotApp::default();
        app.handle_event(&key(KeyCode::Down));
        app.handle_event(&key(KeyCode::Down));
        let selected_id = app.selected_contractor().map(|contractor| contractor.id);

        app.handle_event(&key(KeyCode::Char('/')));
        app.handle_event(&Event::Paste("ana".into()));
        app.handle_event(&key(KeyCode::Esc));

        assert!(app.search.value().is_empty());
        assert_eq!(
            app.selected_contractor().map(|contractor| contractor.id),
            selected_id
        );
    }

    #[test]
    fn terminal_pequena_bloquea_mutaciones_invisibles() {
        let mut app = PilotApp::default();
        app.handle_event(&key(KeyCode::Char('n')));
        let count = app.contractors.len();

        app.handle_event(&Event::Resize(40, 10));
        app.handle_event(&Event::Paste("1-9999".into()));
        app.handle_event(&key(KeyCode::F(10)));

        assert_eq!(app.contractors.len(), count);
        assert!(matches!(app.mode, Mode::Form(_)));
    }
}
