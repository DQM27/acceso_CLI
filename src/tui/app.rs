use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{Terminal, backend::Backend};

mod actions;
mod auth_jobs;

use auth_jobs::{HiloUsuarioPendiente, ReceptorAutenticacion, ReceptorCambioPropio, ReceptorHash};

use crate::application::{AppCore, EstadoRespaldoAutomatico};
use crate::models::usuario::RolUsuario;
use crate::services::autenticacion_service::UsuarioSesion;

use super::{
    activos::{self, AccionActivos, ActivosState},
    auditoria::{self, AuditoriaState},
    cambio_password::{self, AccionCambioPassword, CambioPasswordState},
    configuracion::{self, ConfiguracionState},
    configuracion_inicial::{self, AccionConfiguracion, ConfiguracionInicialState, SolicitudRoot},
    contratistas::{self, AccionContratistas, ContratistasState},
    elegir_interfaz::{self, AccionElegirInterfaz, ElegirInterfazState},
    empresas::{self, AccionEmpresas, EmpresasState},
    historial::{self, AccionHistorial, HistorialState},
    login::{self, AccionLogin, LoginState},
    menu_principal::{self, AccionMenu, ConfirmacionMenu, MenuPrincipalState, OpcionMenu},
    nuevo_ingreso::{self, AccionNuevoIngreso, NuevoIngresoState},
    preferences::{PreferencesStore, UiPreferences},
    salida_rapida::{self, AccionSalidaRapida, SalidaRapidaState},
    ui_kit::{StandardCommand, ThemePreset, standard_command},
    usuarios::{self, AccionUsuarios, UsuariosState},
};

const EVENT_POLL: Duration = Duration::from_millis(50);
/// Las pantallas muestran un reloj con precisión de minutos. Un refresco por segundo
/// deja margen de sobra para el cambio de minuto sin volver a construir toda la TUI
/// veinte veces por segundo cuando el operador no está haciendo nada.
const CLOCK_REFRESH: Duration = Duration::from_secs(1);
/// `AppCore::respaldo_automatico_diario_si_hace_falta` sólo hace algo una vez
/// por día (después de la 01:00 Costa Rica), así que revisarla una vez por
/// minuto —no en cada vuelta del bucle— alcanza de sobra; evita golpear el
/// directorio de respaldos veinte veces por segundo mientras la app se queda
/// abierta varios días seguidos sin reiniciar.
const REVISION_RESPALDO_AUTOMATICO: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vista {
    ConfiguracionInicial,
    Login,
    ElegirInterfaz,
    MenuPrincipal,
    IngresosActivos,
    Historial,
    Contratistas,
    Empresas,
    Usuarios,
    CambiarPassword,
    Auditoria,
    Respaldos,
    NuevoIngreso,
}

impl Vista {
    const fn opcion_menu(self) -> Option<OpcionMenu> {
        match self {
            Self::NuevoIngreso => Some(OpcionMenu::NuevoIngreso),
            Self::IngresosActivos => Some(OpcionMenu::IngresosActivos),
            Self::Historial => Some(OpcionMenu::Historial),
            Self::Contratistas => Some(OpcionMenu::Contratistas),
            Self::Empresas => Some(OpcionMenu::Empresas),
            Self::Usuarios => Some(OpcionMenu::Usuarios),
            Self::Auditoria => Some(OpcionMenu::Auditoria),
            Self::Respaldos => Some(OpcionMenu::Respaldos),
            Self::CambiarPassword => Some(OpcionMenu::CambiarPassword),
            Self::ConfiguracionInicial
            | Self::Login
            | Self::ElegirInterfaz
            | Self::MenuPrincipal => None,
        }
    }
}

/// Cómo terminó el bucle principal: cierre normal, o una restauración de
/// respaldo confirmada que exige que `main.rs` cierre la conexión SQLite,
/// aplique el reemplazo de archivo y vuelva a arrancar la TUI desde cero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SalidaApp {
    Cerrar,
    Restaurar {
        candidata: std::path::PathBuf,
    },
    /// El operador ya se autenticó y eligió el modo CLI en `ElegirInterfaz`:
    /// `main.rs` arranca `comandos::run` con esta misma sesión, sin volver a
    /// pedir cédula/contraseña.
    ModoComandos {
        sesion: UsuarioSesion,
    },
    /// "Modo comandos" del Menú Principal (`OpcionMenu::ModoComandos`): a
    /// diferencia de `ModoComandos` (que reusa la sesión sin reiniciar), acá
    /// el operador pidió dejar `--comandos` como interfaz por defecto — la
    /// preferencia ya se guardó (`interfaz_preferida::guardar`) antes de
    /// llegar a esta variante; `main.rs` sólo tiene que cerrar esta conexión
    /// y relanzar el ejecutable.
    ReiniciarEnComandos,
}

#[cfg(test)]
#[path = "app/tests.rs"]
mod tests;

#[derive(Debug)]
pub struct App {
    vista: Vista,
    login: LoginState,
    elegir_interfaz: ElegirInterfazState,
    menu: MenuPrincipalState,
    configuracion_inicial: ConfiguracionInicialState,
    activos: ActivosState,
    historial: HistorialState,
    contratistas: ContratistasState,
    empresas: EmpresasState,
    usuarios: UsuariosState,
    cambio_password: CambioPasswordState,
    auditoria: AuditoriaState,
    configuracion: ConfiguracionState,
    nuevo_ingreso: NuevoIngresoState,
    salida_rapida: SalidaRapidaState,
    salir: bool,
    salida: SalidaApp,
    sesion: Option<UsuarioSesion>,
    pestanas_visitadas: [bool; 9],
    tema: ThemePreset,
    preferencias: Option<PreferencesStore>,
    /// Resultado en camino de un hilo aparte que verifica la contraseña
    /// (Argon2) sin bloquear este bucle. `None` cuando no hay ningún login
    /// en curso.
    autenticacion_pendiente: Option<ReceptorAutenticacion>,
    /// Hash de Argon2 en camino para crear un usuario o cambiar una
    /// contraseña. Un único `Option` en vez de dos campos independientes: la
    /// exclusión mutua entre ambos flujos es estructural (no puede haber
    /// creación y cambio de contraseña en vuelo a la vez), no depende de que
    /// nada valide `UsuariosState::guardando` desde aquí.
    hilo_usuario_pendiente: Option<HiloUsuarioPendiente>,
    cambio_password_pendiente: Option<ReceptorCambioPropio>,
    /// Hash de Argon2 en camino para crear el usuario ROOT inicial.
    root_inicial_pendiente: Option<(ReceptorHash, SolicitudRoot)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            vista: Vista::Login,
            login: LoginState::default(),
            elegir_interfaz: ElegirInterfazState::default(),
            menu: MenuPrincipalState::default(),
            configuracion_inicial: ConfiguracionInicialState::default(),
            activos: ActivosState::default(),
            historial: HistorialState::default(),
            contratistas: ContratistasState::default(),
            empresas: EmpresasState::default(),
            usuarios: UsuariosState::default(),
            cambio_password: CambioPasswordState::default(),
            auditoria: AuditoriaState::default(),
            configuracion: ConfiguracionState::default(),
            nuevo_ingreso: NuevoIngresoState::default(),
            salida_rapida: SalidaRapidaState::default(),
            salir: false,
            salida: SalidaApp::Cerrar,
            sesion: None,
            pestanas_visitadas: [false; 9],
            tema: ThemePreset::Brisas,
            preferencias: None,
            autenticacion_pendiente: None,
            hilo_usuario_pendiente: None,
            cambio_password_pendiente: None,
            root_inicial_pendiente: None,
        }
    }
}

impl App {
    pub fn new(requiere_configuracion_inicial: bool, mensaje_inicial: Option<String>) -> Self {
        let mut app = Self {
            vista: if requiere_configuracion_inicial {
                Vista::ConfiguracionInicial
            } else {
                Vista::Login
            },
            ..Self::default()
        };
        if let Some(mensaje) = mensaje_inicial {
            app.login.preset_error(mensaje);
        }
        if let Some(store) = PreferencesStore::load_default() {
            app.aplicar_preferencias(store.current());
            app.preferencias = Some(store);
        }
        app
    }

    fn aplicar_preferencias(&mut self, preferences: &UiPreferences) {
        self.tema = preferences.theme;
        if !preferences.activos_columns.is_empty() {
            self.activos
                .aplicar_columnas_preferencia(&preferences.activos_columns);
        }
        if !preferences.contratistas_columns.is_empty() {
            self.contratistas
                .aplicar_columnas_preferencia(&preferences.contratistas_columns);
        }
        self.historial
            .aplicar_vista_preferencia(&preferences.historial_view);
        if !preferences.historial_columns.is_empty() {
            self.historial
                .aplicar_columnas_preferencia(&preferences.historial_columns);
        }
    }

    fn preferencias_actuales(&self) -> UiPreferences {
        UiPreferences {
            theme: self.tema,
            activos_columns: self.activos.columnas_preferencia(),
            contratistas_columns: self.contratistas.columnas_preferencia(),
            historial_view: self.historial.vista_preferencia().to_owned(),
            historial_columns: self.historial.columnas_preferencia(),
        }
    }

    fn persistir_preferencias_si_cambiaron(&mut self) {
        let preferences = self.preferencias_actuales();
        if let Some(store) = &mut self.preferencias {
            // Una preferencia visual nunca debe interrumpir una operación de
            // acceso; si el sistema no permite escribir, se conserva en memoria.
            let _ = store.save_if_changed(preferences);
        }
    }

    pub fn run<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<SalidaApp> {
        self.run_internal(terminal, None)
    }

    pub fn run_with_core<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
        core: &AppCore,
    ) -> io::Result<SalidaApp> {
        self.run_internal(terminal, Some(core))
    }

    fn run_internal<B: Backend<Error = io::Error>>(
        &mut self,
        terminal: &mut Terminal<B>,
        core: Option<&AppCore>,
    ) -> io::Result<SalidaApp> {
        let mut redibujar = true;
        let mut ultimo_refresco_reloj = Instant::now();
        // Arranca vencido a propósito: la primera vuelta del bucle ya debe
        // revisar el respaldo automático, no esperar un minuto entero.
        let mut ultima_revision_respaldo = Instant::now() - REVISION_RESPALDO_AUTOMATICO;
        while !self.salir {
            if redibujar {
                let theme = self.tema.theme();
                terminal.draw(|frame| {
                    match self.vista {
                        Vista::ConfiguracionInicial => configuracion_inicial::render(
                            frame,
                            frame.area(),
                            &self.configuracion_inicial,
                            theme,
                        ),
                        Vista::Login => login::render(frame, frame.area(), &self.login, theme),
                        Vista::ElegirInterfaz => elegir_interfaz::render(
                            frame,
                            frame.area(),
                            &self.elegir_interfaz,
                            theme,
                        ),
                        Vista::MenuPrincipal => {
                            if let Some(sesion) = &self.sesion {
                                menu_principal::render(
                                    frame,
                                    frame.area(),
                                    &self.menu,
                                    sesion,
                                    theme,
                                )
                            }
                        }
                        Vista::IngresosActivos => {
                            if let Some(sesion) = &self.sesion {
                                activos::render(frame, frame.area(), &self.activos, sesion, theme)
                            }
                        }
                        Vista::Historial => {
                            if let Some(sesion) = &self.sesion {
                                historial::render(
                                    frame,
                                    frame.area(),
                                    &self.historial,
                                    sesion,
                                    theme,
                                )
                            }
                        }
                        Vista::Contratistas => {
                            if let Some(sesion) = &self.sesion {
                                contratistas::render(
                                    frame,
                                    frame.area(),
                                    &self.contratistas,
                                    sesion,
                                    theme,
                                )
                            }
                        }
                        Vista::Empresas => {
                            if let Some(sesion) = &self.sesion {
                                empresas::render(frame, frame.area(), &self.empresas, sesion, theme)
                            }
                        }
                        Vista::Usuarios => {
                            if let Some(sesion) = &self.sesion {
                                usuarios::render(frame, frame.area(), &self.usuarios, sesion, theme)
                            }
                        }
                        Vista::CambiarPassword => {
                            if let Some(sesion) = &self.sesion {
                                cambio_password::render(
                                    frame,
                                    frame.area(),
                                    &self.cambio_password,
                                    sesion,
                                    theme,
                                )
                            }
                        }
                        Vista::Auditoria => {
                            if let Some(sesion) = &self.sesion {
                                auditoria::render(
                                    frame,
                                    frame.area(),
                                    &self.auditoria,
                                    sesion,
                                    theme,
                                )
                            }
                        }
                        Vista::Respaldos => {
                            if let Some(sesion) = &self.sesion {
                                configuracion::render(
                                    frame,
                                    frame.area(),
                                    &self.configuracion,
                                    sesion,
                                    theme,
                                )
                            }
                        }
                        Vista::NuevoIngreso => {
                            if let Some(sesion) = &self.sesion {
                                nuevo_ingreso::render(
                                    frame,
                                    frame.area(),
                                    &self.nuevo_ingreso,
                                    sesion,
                                    theme,
                                )
                            }
                        }
                    }
                    salida_rapida::render(frame, frame.area(), &self.salida_rapida, theme);
                })?;
                redibujar = false;
            }

            let mut cambio_visible = false;
            if event::poll(EVENT_POLL)? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.procesar_tecla_global(key, core);
                        cambio_visible = true;
                    }
                    // Sin el redibujado continuo, un cambio de tamaño debe invalidar
                    // explícitamente el frame anterior.
                    Event::Resize(_, _) => cambio_visible = true,
                    _ => {}
                }
            }

            let ahora = Instant::now();
            self.configuracion_inicial.tick(ahora);
            self.login.tick(ahora);
            let trabajos_antes = (
                self.autenticacion_pendiente.is_some(),
                self.hilo_usuario_pendiente.is_some(),
                self.cambio_password_pendiente.is_some(),
                self.root_inicial_pendiente.is_some(),
            );
            // Sondeo de los 4 hilos de Argon2 en vuelo, siempre en el mismo lugar
            // del bucle (después de leer teclas): login, ROOT inicial, crear
            // usuario/cambiar contraseña.
            self.recibir_autenticacion_si_lista(core);
            match core {
                Some(core) => {
                    self.procesar_configuracion_pendiente(core);
                    self.recibir_root_inicial_si_lista(core);
                    if ahora.saturating_duration_since(ultima_revision_respaldo)
                        >= REVISION_RESPALDO_AUTOMATICO
                    {
                        ultima_revision_respaldo = ahora;
                        let fallo = match core.respaldo_automatico_diario_si_hace_falta() {
                            EstadoRespaldoAutomatico::Fallo(mensaje) => Some(mensaje),
                            EstadoRespaldoAutomatico::SinCambios
                            | EstadoRespaldoAutomatico::Creado => None,
                        };
                        cambio_visible |=
                            fallo.is_some() != self.menu.fallo_respaldo_automatico.is_some();
                        self.menu.fallo_respaldo_automatico = fallo.clone();
                        self.configuracion
                            .actualizar_fallo_respaldo_automatico(fallo);
                    }
                }
                None => self.abortar_configuracion_inicial_sin_core(),
            }
            self.recibir_hilo_usuario_si_lista(core);
            self.recibir_cambio_password_propio(core);
            let trabajos_despues = (
                self.autenticacion_pendiente.is_some(),
                self.hilo_usuario_pendiente.is_some(),
                self.cambio_password_pendiente.is_some(),
                self.root_inicial_pendiente.is_some(),
            );
            cambio_visible |= trabajos_antes != trabajos_despues;

            // Búsquedas con debounce: cada pantalla decide si ya pasó el
            // tiempo sin tecla nueva; si no, `tick` devuelve `Ninguna` y el
            // despacho de siempre es un no-op.
            let accion = self.historial.tick(ahora);
            cambio_visible |= !matches!(&accion, AccionHistorial::Ninguna);
            self.procesar_accion_historial(accion, core);
            let accion = self.contratistas.tick(ahora);
            cambio_visible |= !matches!(&accion, AccionContratistas::Ninguna);
            self.procesar_accion_contratistas(accion, core);
            let accion = self.activos.tick(ahora);
            cambio_visible |= !matches!(&accion, AccionActivos::Ninguna);
            self.procesar_accion_activos(accion, core);
            let accion = self.empresas.tick(ahora);
            cambio_visible |= !matches!(&accion, AccionEmpresas::Ninguna);
            self.procesar_accion_empresas(accion, core);
            let accion = self.usuarios.tick(ahora);
            cambio_visible |= !matches!(&accion, AccionUsuarios::Ninguna);
            self.procesar_accion_usuarios(accion, core);
            let accion = self.nuevo_ingreso.tick(ahora);
            cambio_visible |= !matches!(&accion, AccionNuevoIngreso::Ninguna);
            self.procesar_accion_nuevo_ingreso(accion, core);
            let accion = self.salida_rapida.tick(ahora);
            cambio_visible |= !matches!(&accion, AccionSalidaRapida::Ninguna);
            self.procesar_accion_salida_rapida(accion, core);

            // Login y configuración inicial sí tienen animación (spinner/cursor).
            // El resto sólo se invalida por una entrada, un resultado, un resize o
            // el reloj; así la app ociosa deja de reconstruir 20 frames por segundo.
            cambio_visible |= matches!(self.vista, Vista::Login | Vista::ConfiguracionInicial);
            if ahora.saturating_duration_since(ultimo_refresco_reloj) >= CLOCK_REFRESH {
                ultimo_refresco_reloj = ahora;
                cambio_visible = true;
            }
            redibujar |= cambio_visible;
        }

        Ok(self.salida.clone())
    }

    #[cfg(test)]
    fn procesar_tecla_vista(&mut self, key: crossterm::event::KeyEvent) {
        self.procesar_tecla_global(key, None);
    }

    /// Fuerza que se dispare cualquier búsqueda con debounce pendiente,
    /// simulando que pasó tiempo de sobra desde la última tecla. Para
    /// pruebas que necesitan el resultado real de una búsqueda sin esperar
    /// el reloj de verdad.
    #[cfg(test)]
    fn agotar_debounce_busquedas(&mut self, core: Option<&AppCore>) {
        let futuro = std::time::Instant::now() + std::time::Duration::from_secs(1);
        let accion = self.historial.tick(futuro);
        self.procesar_accion_historial(accion, core);
        let accion = self.contratistas.tick(futuro);
        self.procesar_accion_contratistas(accion, core);
        let accion = self.activos.tick(futuro);
        self.procesar_accion_activos(accion, core);
        let accion = self.empresas.tick(futuro);
        self.procesar_accion_empresas(accion, core);
        let accion = self.usuarios.tick(futuro);
        self.procesar_accion_usuarios(accion, core);
        let accion = self.nuevo_ingreso.tick(futuro);
        self.procesar_accion_nuevo_ingreso(accion, core);
        let accion = self.salida_rapida.tick(futuro);
        self.procesar_accion_salida_rapida(accion, core);
    }

    /// Comandos transversales (salida de emergencia, tema, salida rápida) que se
    /// resuelven antes de despachar por vista, sin importar cuál esté activa.
    fn procesar_tecla_global(&mut self, key: crossterm::event::KeyEvent, core: Option<&AppCore>) {
        match standard_command(key) {
            Some(StandardCommand::EmergencyExit) => {
                self.finalizar_hilos_pendientes(core);
                self.salir = true;
                return;
            }
            // Ctrl+Q: cerrar sesión desde cualquier pantalla, con la misma
            // confirmación (Enter/Esc) que ya usa la opción "L" del Menú —
            // se arma directamente sobre `menu.confirmacion` y se salta a
            // esa pantalla para reusar su tarjeta, en vez de duplicar un
            // segundo mecanismo de confirmación sólo para este atajo.
            Some(StandardCommand::CerrarSesion) if self.sesion.is_some() => {
                self.menu.confirmacion = Some(ConfirmacionMenu::CerrarSesion);
                self.vista = Vista::MenuPrincipal;
                return;
            }
            Some(StandardCommand::Theme) => {
                self.tema = self.tema.next();
                self.persistir_preferencias_si_cambiaron();
                self.sincronizar_vista_con_tema(core);
                return;
            }
            Some(StandardCommand::PreviousTab)
                if self.sesion.is_some()
                    && self.tema.theme().navegacion_pestanas
                    && !self.salida_rapida.abierto() =>
            {
                self.mover_pestana(-1, core);
                return;
            }
            Some(StandardCommand::NextTab)
                if self.sesion.is_some()
                    && self.tema.theme().navegacion_pestanas
                    && !self.salida_rapida.abierto() =>
            {
                self.mover_pestana(1, core);
                return;
            }
            // Requiere sesión iniciada: en Login/ConfiguracionInicial no hay a quién
            // atribuir la salida ni personal "adentro" que buscar todavía.
            Some(StandardCommand::QuickExit)
                if !self.salida_rapida.abierto() && self.sesion.is_some() =>
            {
                let accion = self.salida_rapida.abrir();
                self.procesar_accion_salida_rapida(accion, core);
                return;
            }
            _ => {}
        }
        // Ctrl+1..Ctrl+9 saltan directo a una pestaña. La relación entre
        // número, pantalla y permisos vive en `OpcionMenu`, igual que la barra.
        // Sólo tiene sentido en el tema con navegación por pestañas — en los
        // demás no hay barra visible ni Menú Principal fuera de circulación.
        if self.sesion.is_some()
            && self.tema.theme().navegacion_pestanas
            && !self.salida_rapida.abierto()
            && key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
            && !key.modifiers.contains(crossterm::event::KeyModifiers::ALT)
            && matches!(key.code, crossterm::event::KeyCode::Char(c) if c.is_ascii_digit() && c != '0')
        {
            let crossterm::event::KeyCode::Char(numero) = key.code else {
                unreachable!();
            };
            if let Some(opcion) = OpcionMenu::desde_atajo(numero) {
                self.navegar_a_pestana(opcion, core);
            }
            return;
        }
        if self.salida_rapida.abierto() {
            let accion = self.salida_rapida.handle_key(key);
            self.procesar_accion_salida_rapida(accion, core);
            return;
        }
        self.procesar_tecla_vista_con_core(key, core);
        self.persistir_preferencias_si_cambiaron();
    }

    fn procesar_tecla_vista_con_core(
        &mut self,
        key: crossterm::event::KeyEvent,
        core: Option<&AppCore>,
    ) {
        match self.vista {
            Vista::ConfiguracionInicial => {
                if self.configuracion_inicial.handle_key(key) == AccionConfiguracion::Salir {
                    self.salir = true;
                }
            }
            Vista::Login => match self.login.handle_key(key) {
                AccionLogin::Salir => self.salir = true,
                AccionLogin::Autenticar { cedula, password } => {
                    self.iniciar_autenticacion(cedula, password, core)
                }
                AccionLogin::Ninguna => {}
            },
            Vista::ElegirInterfaz => match self.elegir_interfaz.handle_key(key) {
                AccionElegirInterfaz::Ninguna => {}
                AccionElegirInterfaz::Tui => {
                    self.vista = Vista::MenuPrincipal;
                    self.sincronizar_vista_con_tema(core);
                }
                AccionElegirInterfaz::Cli => {
                    if let Some(sesion) = self.sesion.clone() {
                        self.salida = SalidaApp::ModoComandos { sesion };
                        self.salir = true;
                    }
                }
            },
            Vista::MenuPrincipal => self.procesar_accion_menu_con_core(key, core),
            Vista::IngresosActivos => {
                let accion = self.activos.handle_key(key);
                self.procesar_accion_activos(accion, core);
            }
            Vista::Historial => {
                let accion = self.historial.handle_key(key);
                self.procesar_accion_historial(accion, core);
            }
            Vista::Contratistas => {
                let rol = self
                    .sesion
                    .as_ref()
                    .map_or(RolUsuario::Operador, |sesion| sesion.rol);
                let accion = self.contratistas.handle_key_con_rol(key, rol);
                self.procesar_accion_contratistas(accion, core);
            }
            Vista::Empresas => {
                let accion = self.empresas.handle_key(key);
                self.procesar_accion_empresas(accion, core);
            }
            Vista::Usuarios => {
                let accion = self.usuarios.handle_key(key);
                self.procesar_accion_usuarios(accion, core);
            }
            Vista::CambiarPassword => {
                let accion = self.cambio_password.handle_key(key);
                match accion {
                    AccionCambioPassword::Ninguna => {}
                    AccionCambioPassword::Volver => self.volver_de_pestana(core),
                    AccionCambioPassword::Cambiar {
                        password_actual,
                        nueva_password,
                    } => self.iniciar_cambio_password_propio(password_actual, nueva_password, core),
                }
            }
            Vista::Auditoria => {
                let accion = self.auditoria.handle_key(key);
                self.procesar_accion_auditoria(accion, core);
            }
            Vista::Respaldos => {
                let accion = self.configuracion.handle_key(key);
                self.procesar_accion_configuracion(accion, core);
            }
            Vista::NuevoIngreso => {
                let accion = self.nuevo_ingreso.handle_key(key);
                self.procesar_accion_nuevo_ingreso(accion, core);
            }
        }
    }

    pub fn sesion(&self) -> Option<&UsuarioSesion> {
        self.sesion.as_ref()
    }

    #[cfg(test)]
    fn procesar_accion_menu(&mut self, key: crossterm::event::KeyEvent) {
        self.procesar_accion_menu_con_core(key, None);
    }

    fn procesar_accion_menu_con_core(
        &mut self,
        key: crossterm::event::KeyEvent,
        core: Option<&AppCore>,
    ) {
        let rol = self
            .sesion
            .as_ref()
            .map_or(RolUsuario::Operador, |sesion| sesion.rol);
        match self.menu.handle_key(key, rol) {
            AccionMenu::Ninguna => {}
            AccionMenu::Abrir(opcion) => {
                self.abrir_opcion(opcion, core);
            }
            AccionMenu::CerrarSesion => {
                self.sesion = None;
                self.pestanas_visitadas = [false; 9];
                self.login.reiniciar();
                self.vista = Vista::Login;
            }
            AccionMenu::Salir => self.salir = true,
            AccionMenu::ModoComandos => {
                crate::interfaz_preferida::guardar(crate::interfaz_preferida::Interfaz::Comandos);
                self.salida = SalidaApp::ReiniciarEnComandos;
                self.salir = true;
            }
        }
    }

    fn mover_pestana(&mut self, desplazamiento: isize, core: Option<&AppCore>) {
        let Some(actual) = self.vista.opcion_menu() else {
            return;
        };
        let Some(sesion) = self.sesion.as_ref() else {
            return;
        };
        let visibles = OpcionMenu::pestanas_para(sesion.rol);
        let Some(indice) = visibles.iter().position(|opcion| *opcion == actual) else {
            return;
        };
        let destino =
            (indice as isize + desplazamiento).rem_euclid(visibles.len() as isize) as usize;
        self.navegar_a_pestana(visibles[destino], core);
    }

    fn navegar_a_pestana(&mut self, opcion: OpcionMenu, core: Option<&AppCore>) {
        let Some(sesion) = self.sesion.as_ref() else {
            return;
        };
        if !OpcionMenu::pestanas_para(sesion.rol).contains(&opcion) {
            return;
        }
        let Some(indice) = opcion.indice_pestana() else {
            return;
        };
        if !self.pestanas_visitadas[indice] {
            self.preparar_opcion(opcion, core);
            self.pestanas_visitadas[indice] = true;
        }
        self.menu.seleccion = opcion;
        self.vista = Self::vista_de_opcion(opcion);
    }

    /// Abrir desde el menú conserva el comportamiento histórico: refresca o
    /// reinicia la pantalla. Cambiar entre pestañas sólo hace esa preparación
    /// en la primera visita de la sesión y después conserva filtros/formularios.
    fn abrir_opcion(&mut self, opcion: OpcionMenu, core: Option<&AppCore>) {
        let Some(indice) = opcion.indice_pestana() else {
            return;
        };
        self.preparar_opcion(opcion, core);
        self.pestanas_visitadas[indice] = true;
        self.menu.seleccion = opcion;
        self.vista = Self::vista_de_opcion(opcion);
    }

    fn preparar_opcion(&mut self, opcion: OpcionMenu, core: Option<&AppCore>) {
        match opcion {
            OpcionMenu::NuevoIngreso => {
                self.nuevo_ingreso = NuevoIngresoState::new();
                if core.is_some() {
                    self.procesar_accion_nuevo_ingreso(self.nuevo_ingreso.solicitud_carga(), core);
                }
            }
            OpcionMenu::IngresosActivos => {
                if let Some(core) = core {
                    self.activos.completar_empresas(
                        core.listar_empresas()
                            .map_err(|_| "No se pudieron cargar las empresas".into()),
                    );
                    self.procesar_accion_activos(self.activos.solicitud_carga(), Some(core));
                }
            }
            OpcionMenu::Historial => {
                if let Some(core) = core {
                    self.historial.completar_empresas(
                        core.listar_empresas()
                            .map_err(|_| "No se pudieron cargar las empresas".into()),
                    );
                    let accion = self.historial.solicitud_carga();
                    self.procesar_accion_historial(accion, Some(core));
                }
            }
            OpcionMenu::Contratistas => {
                if let Some(core) = core {
                    self.contratistas.completar_empresas(
                        core.listar_empresas()
                            .map_err(|_| "No se pudieron cargar las empresas".into()),
                    );
                    self.procesar_accion_contratistas(
                        self.contratistas.solicitud_carga(),
                        Some(core),
                    );
                }
            }
            OpcionMenu::Empresas => {
                if core.is_some() {
                    self.procesar_accion_empresas(self.empresas.solicitar_carga(), core);
                }
            }
            OpcionMenu::Usuarios => {
                if core.is_some() {
                    self.procesar_accion_usuarios(self.usuarios.solicitud_carga(), core);
                }
            }
            OpcionMenu::CambiarPassword => self.cambio_password.reiniciar(),
            OpcionMenu::Auditoria => {
                let accion = self.auditoria.reiniciar();
                self.procesar_accion_auditoria(accion, core);
            }
            OpcionMenu::Respaldos => {
                let accion = self.configuracion.reiniciar();
                self.procesar_accion_configuracion(accion, core);
            }
            // Ninguna abre pantalla propia: `ModoComandos` sale por la
            // confirmación (como `CerrarSesion`/`Salir`), nunca llega acá
            // como `AccionMenu::Abrir`.
            OpcionMenu::ModoComandos | OpcionMenu::CerrarSesion | OpcionMenu::Salir => {}
        }
    }

    const fn vista_de_opcion(opcion: OpcionMenu) -> Vista {
        match opcion {
            OpcionMenu::NuevoIngreso => Vista::NuevoIngreso,
            OpcionMenu::IngresosActivos => Vista::IngresosActivos,
            OpcionMenu::Historial => Vista::Historial,
            OpcionMenu::Contratistas => Vista::Contratistas,
            OpcionMenu::Empresas => Vista::Empresas,
            OpcionMenu::Usuarios => Vista::Usuarios,
            OpcionMenu::CambiarPassword => Vista::CambiarPassword,
            OpcionMenu::Auditoria => Vista::Auditoria,
            OpcionMenu::Respaldos => Vista::Respaldos,
            OpcionMenu::ModoComandos | OpcionMenu::CerrarSesion | OpcionMenu::Salir => {
                Vista::MenuPrincipal
            }
        }
    }

    fn iniciar_sesion(&mut self, sesion: UsuarioSesion, core: Option<&AppCore>) {
        self.sesion = Some(sesion);
        self.pestanas_visitadas = [false; 9];
        self.menu.nueva_sesion();
        // Ya no pregunta TUI o CLI en cada login (`ElegirInterfaz`): con la
        // preferencia persistente de interfaz (`interfaz_preferida`) y
        // "Modo comandos" ya disponible como opción explícita del Menú
        // Principal, esa pantalla extra antes de cada sesión sólo repetía
        // una elección que el operador ya hizo (o puede hacer cuando
        // quiera) sin necesidad de pasar por acá.
        self.vista = Vista::MenuPrincipal;
        self.sincronizar_vista_con_tema(core);
    }

    /// El Menú Principal y la navegación por pestañas son dos modos
    /// excluyentes atados al tema activo (sólo Negro usa pestañas) — se
    /// llama al iniciar sesión y cada vez que cambia el tema (F7) para que
    /// nunca convivan: entrar a Negro salta directo a la primera pestaña
    /// visible para el rol; salir de Negro vuelve al Menú, sincronizado con
    /// la pantalla que se estaba viendo.
    fn sincronizar_vista_con_tema(&mut self, core: Option<&AppCore>) {
        let Some(sesion) = self.sesion.as_ref() else {
            return;
        };
        if self.tema.theme().navegacion_pestanas {
            if self.vista == Vista::MenuPrincipal
                && let Some(primera) = OpcionMenu::pestanas_para(sesion.rol).first().copied()
            {
                self.navegar_a_pestana(primera, core);
            }
        } else if let Some(opcion) = self.vista.opcion_menu() {
            self.menu.seleccion = opcion;
            self.vista = Vista::MenuPrincipal;
        }
    }

    /// Volver desde una pantalla de pestaña (hoy sólo Cambiar contraseña) al
    /// punto de partida correcto según el tema: el Menú en Classic/Brisas, o
    /// la primera pestaña en Negro, donde el Menú no es alcanzable.
    fn volver_de_pestana(&mut self, core: Option<&AppCore>) {
        if self.tema.theme().navegacion_pestanas
            && let Some(sesion) = self.sesion.as_ref()
            && let Some(primera) = OpcionMenu::pestanas_para(sesion.rol).first().copied()
        {
            self.navegar_a_pestana(primera, core);
            return;
        }
        self.vista = Vista::MenuPrincipal;
    }

    /// Contraparte de `procesar_configuracion_pendiente` cuando no hay `core`
    /// (`App::run`, sin base de datos): sin esto, un ROOT inicial enviado se
    /// queda para siempre en "Creando" — `EstadoConfiguracion::Creando`
    /// bloquea hasta el `Esc` porque nadie vuelve a tomar la solicitud pendiente.
    fn abortar_configuracion_inicial_sin_core(&mut self) {
        if self.vista != Vista::ConfiguracionInicial {
            return;
        }
        if self.configuracion_inicial.tomar_solicitud().is_some() {
            self.configuracion_inicial
                .completar_con_error("No se pudo crear el usuario ROOT");
        }
    }

    fn procesar_configuracion_pendiente(&mut self, core: &AppCore) {
        if self.vista != Vista::ConfiguracionInicial {
            return;
        }
        let Some(solicitud) = self.configuracion_inicial.tomar_solicitud() else {
            return;
        };
        self.iniciar_root_inicial(solicitud, core);
    }
}
