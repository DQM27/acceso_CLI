use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{Terminal, backend::Backend};

mod error_messages;

use error_messages::{
    mensaje_contratista, mensaje_empresa, mensaje_ingreso, mensaje_salida, mensaje_usuario,
};

use crate::application::AppCore;
use crate::models::usuario::RolUsuario;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::{AutenticacionError, PasswordError, UsuarioServiceError};
use crate::services::usuario_service::CrearRootInicialInput;

use super::{
    activos::{self, AccionActivos, ActivosState},
    auditoria::{self, AccionAuditoria, AuditoriaState},
    cambio_password::{self, AccionCambioPassword, CambioPasswordState},
    configuracion::{self, AccionAjustes, AccionRespaldos, ConfiguracionState},
    configuracion_inicial::{self, AccionConfiguracion, ConfiguracionInicialState, SolicitudRoot},
    contratistas::{self, AccionContratistas, ContratistasState},
    empresas::{self, AccionEmpresas, EmpresasState},
    historial::{self, AccionHistorial, HistorialState},
    login::{self, AccionLogin, LoginState},
    menu_principal::{self, AccionMenu, MenuPrincipalState, OpcionMenu},
    nuevo_ingreso::{self, AccionNuevoIngreso, NuevoIngresoState},
    preferences::{PreferencesStore, UiPreferences},
    salida_rapida::{self, AccionSalidaRapida, SalidaRapidaState},
    ui_kit::{StandardCommand, ThemePreset, standard_command},
    usuarios::{self, AccionUsuarios, UsuariosState},
};

const EVENT_POLL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vista {
    ConfiguracionInicial,
    Login,
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

/// Cómo terminó el bucle principal: cierre normal, o una restauración de
/// respaldo confirmada que exige que `main.rs` cierre la conexión SQLite,
/// aplique el reemplazo de archivo y vuelva a arrancar la TUI desde cero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SalidaApp {
    Cerrar,
    Restaurar { candidata: std::path::PathBuf },
}

#[cfg(test)]
#[path = "app/tests.rs"]
mod tests;

/// Datos ya validados de un usuario nuevo, a la espera del hash de Argon2 —
/// no incluye `password` en texto plano, que ya se movió al hilo que calcula
/// el hash y no hace falta después.
#[derive(Debug)]
enum HiloUsuarioPendiente {
    Creacion(ReceptorHash, UsuarioSesion, DatosUsuarioPendiente, String),
    CambioPassword(ReceptorHash, UsuarioSesion, i64, String),
}

#[derive(Debug, Clone)]
struct DatosUsuarioPendiente {
    cedula: String,
    nombre: String,
    rol: RolUsuario,
    activo: bool,
}

/// Receptor del hilo aparte que sólo calcula un hash de Argon2 — nunca del resultado
/// final de escribir en SQLite, que ocurre después, en el hilo principal.
type ReceptorHash = std::sync::mpsc::Receiver<Result<String, PasswordError>>;
type ReceptorCambioPropio = std::sync::mpsc::Receiver<Result<(String, String), String>>;

#[derive(Debug)]
pub struct App {
    vista: Vista,
    login: LoginState,
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
    tema: ThemePreset,
    preferencias: Option<PreferencesStore>,
    /// Resultado en camino de un hilo aparte que verifica la contraseña
    /// (Argon2) sin bloquear este bucle. `None` cuando no hay ningún login
    /// en curso.
    autenticacion_pendiente:
        Option<std::sync::mpsc::Receiver<Result<UsuarioSesion, AutenticacionError>>>,
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
        while !self.salir {
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
                    Vista::MenuPrincipal => {
                        if let Some(sesion) = &self.sesion {
                            menu_principal::render(frame, frame.area(), &self.menu, sesion, theme)
                        }
                    }
                    Vista::IngresosActivos => {
                        if let Some(sesion) = &self.sesion {
                            activos::render(frame, frame.area(), &self.activos, sesion, theme)
                        }
                    }
                    Vista::Historial => {
                        if let Some(sesion) = &self.sesion {
                            historial::render(frame, frame.area(), &self.historial, sesion, theme)
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
                            auditoria::render(frame, frame.area(), &self.auditoria, sesion, theme)
                        }
                    }
                    Vista::Respaldos => {
                        configuracion::render(frame, frame.area(), &self.configuracion, theme)
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

            if event::poll(EVENT_POLL)?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.procesar_tecla_global(key, core);
            }

            let ahora = std::time::Instant::now();
            self.configuracion_inicial.tick(ahora);
            self.login.tick(ahora);
            // Sondeo de los 4 hilos de Argon2 en vuelo, siempre en el mismo lugar
            // del bucle (después de leer teclas): login, ROOT inicial, crear
            // usuario/cambiar contraseña.
            self.recibir_autenticacion_si_lista(core);
            match core {
                Some(core) => {
                    self.procesar_configuracion_pendiente(core);
                    self.recibir_root_inicial_si_lista(core);
                }
                None => self.abortar_configuracion_inicial_sin_core(),
            }
            self.recibir_hilo_usuario_si_lista(core);
            self.recibir_cambio_password_propio(core);

            // Búsquedas con debounce: cada pantalla decide si ya pasó el
            // tiempo sin tecla nueva; si no, `tick` devuelve `Ninguna` y el
            // despacho de siempre es un no-op.
            let accion = self.historial.tick(ahora);
            self.procesar_accion_historial(accion, core);
            let accion = self.contratistas.tick(ahora);
            self.procesar_accion_contratistas(accion, core);
            let accion = self.activos.tick(ahora);
            self.procesar_accion_activos(accion, core);
            let accion = self.empresas.tick(ahora);
            self.procesar_accion_empresas(accion, core);
            let accion = self.usuarios.tick(ahora);
            self.procesar_accion_usuarios(accion, core);
            let accion = self.nuevo_ingreso.tick(ahora);
            self.procesar_accion_nuevo_ingreso(accion, core);
            let accion = self.salida_rapida.tick(ahora);
            self.procesar_accion_salida_rapida(accion, core);
        }

        Ok(self.salida.clone())
    }

    /// Revisa sin bloquear si el hilo de verificación de contraseña (Argon2) ya terminó.
    ///
    /// La contraseña se verificó contra un `UsuarioSesion`/hash resueltos
    /// *antes* de que corriera Argon2 (potencialmente varios cientos de ms
    /// atrás, ver `iniciar_autenticacion`) — si la cuenta fue desactivada,
    /// degradada o editada mientras tanto, ese snapshot ya está vencido.
    /// Antes de aceptar la sesión se vuelve a resolver el candidato contra
    /// SQLite (rápido, sin Argon2) y se usa ese estado fresco, no el que
    /// llegó por el canal — `buscar_candidato` ya rechaza cuentas inactivas
    /// (`docs/auditoria-dominio-2026-08-20.md`, hallazgo #5).
    fn recibir_autenticacion_si_lista(&mut self, core: Option<&AppCore>) {
        let Some(receptor) = &self.autenticacion_pendiente else {
            return;
        };
        let Ok(resultado) = receptor.try_recv() else {
            return;
        };
        self.autenticacion_pendiente = None;
        match resultado.and_then(|sesion| Self::revalidar_sesion(core, sesion)) {
            Ok(sesion) => {
                self.login.completar_validacion(None);
                self.iniciar_sesion(sesion);
            }
            Err(error) => self.login.completar_validacion(Some(error.to_string())),
        }
    }

    /// Resuelve la cédula de inmediato (rápido, sólo SQLite) y, si existe y está activo,
    /// verifica la contraseña en un hilo aparte para no congelar la UI mientras Argon2 calcula.
    fn iniciar_autenticacion(&mut self, cedula: String, password: String, core: Option<&AppCore>) {
        let Some(core) = core else {
            self.login.completar_validacion(None);
            self.iniciar_sesion(UsuarioSesion {
                id: 0,
                cedula: cedula.clone(),
                nombre: cedula,
                rol: RolUsuario::Operador,
            });
            return;
        };
        match core.buscar_candidato_autenticacion(&cedula) {
            Ok(candidato) => {
                let (emisor, receptor) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let resultado = crate::services::autenticacion_service::verificar_candidato(
                        candidato, &password,
                    );
                    let _ = emisor.send(resultado);
                });
                self.autenticacion_pendiente = Some(receptor);
            }
            Err(error) => self.login.completar_validacion(Some(error.to_string())),
        }
    }

    /// Vuelve a resolver el candidato contra SQLite justo antes de aceptar la
    /// sesión, descartando el snapshot que viajó por el canal — ver el
    /// comentario de `recibir_autenticacion_si_lista`. Sin `core` (modo de
    /// desarrollo sin base) no hay nada que revalidar: esa rama de
    /// `iniciar_autenticacion` nunca llega a poblar `autenticacion_pendiente`,
    /// así que esto es puramente defensivo.
    fn revalidar_sesion(
        core: Option<&AppCore>,
        sesion: UsuarioSesion,
    ) -> Result<UsuarioSesion, AutenticacionError> {
        match core {
            Some(core) => Ok(core.buscar_candidato_autenticacion(&sesion.cedula)?.sesion),
            None => Ok(sesion),
        }
    }

    /// Calcula el hash de Argon2 de `password` en un hilo aparte y devuelve el
    /// receptor para sondear el resultado sin bloquear — usado por los 3 flujos
    /// que crean/cambian una credencial (crear usuario, cambiar contraseña,
    /// ROOT inicial).
    fn generar_hash_en_hilo(password: String) -> ReceptorHash {
        let (emisor, receptor) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = emisor.send(crate::services::password::generar_hash(&password));
        });
        receptor
    }

    /// Valida rápido (sólo SQLite) y, si pasa, calcula el hash de Argon2 en un hilo
    /// aparte — la escritura real ocurre después, en el hilo principal, cuando llega.
    fn iniciar_creacion_usuario(
        &mut self,
        input: crate::services::usuario_service::CrearUsuarioInput,
        nombre: String,
        core: Option<&AppCore>,
    ) {
        let Some(core) = core else {
            let recarga = self.usuarios.completar_guardado(
                Err("No se pudo guardar el usuario".into()),
                None,
                &nombre,
            );
            self.procesar_recarga_usuarios(recarga, core);
            return;
        };
        let Some(actor) = self.sesion.clone() else {
            let recarga = self.usuarios.completar_guardado(
                Err("No hay una sesión activa".into()),
                None,
                &nombre,
            );
            self.procesar_recarga_usuarios(recarga, Some(core));
            return;
        };
        if let Err(error) = core.validar_datos_para_crear_usuario(&actor, &input) {
            let recarga =
                self.usuarios
                    .completar_guardado(Err(mensaje_usuario(error)), None, &nombre);
            self.procesar_recarga_usuarios(recarga, Some(core));
            return;
        }
        let datos = DatosUsuarioPendiente {
            cedula: input.cedula,
            nombre: input.nombre,
            rol: input.rol,
            activo: input.activo,
        };
        let receptor = Self::generar_hash_en_hilo(input.password);
        self.hilo_usuario_pendiente = Some(HiloUsuarioPendiente::Creacion(
            receptor, actor, datos, nombre,
        ));
        self.usuarios.marcar_guardando();
    }

    /// Mismo patrón que `iniciar_creacion_usuario`: valida rápido, hashea en un hilo aparte.
    fn iniciar_cambio_password(
        &mut self,
        id: i64,
        password: String,
        nombre: String,
        core: Option<&AppCore>,
    ) {
        let Some(core) = core else {
            self.usuarios
                .completar_password(Err("No se pudo cambiar la contraseña".into()), &nombre);
            return;
        };
        let Some(actor) = self.sesion.clone() else {
            self.usuarios
                .completar_password(Err("No hay una sesión activa".into()), &nombre);
            return;
        };
        if let Err(error) = core.validar_password_para_cambio(&actor, id, &password) {
            self.usuarios
                .completar_password(Err(mensaje_usuario(error)), &nombre);
            return;
        }
        let receptor = Self::generar_hash_en_hilo(password);
        self.hilo_usuario_pendiente = Some(HiloUsuarioPendiente::CambioPassword(
            receptor, actor, id, nombre,
        ));
        self.usuarios.marcar_guardando();
    }

    /// Revisa sin bloquear si el hilo de Argon2 de creación de usuario o cambio de
    /// contraseña ya terminó — a lo sumo uno de los dos puede estar en vuelo a la
    /// vez, ver el comentario de `hilo_usuario_pendiente`.
    fn recibir_hilo_usuario_si_lista(&mut self, core: Option<&AppCore>) {
        let receptor = match &self.hilo_usuario_pendiente {
            Some(HiloUsuarioPendiente::Creacion(receptor, ..)) => receptor,
            Some(HiloUsuarioPendiente::CambioPassword(receptor, ..)) => receptor,
            None => return,
        };
        let Ok(resultado_hash) = receptor.try_recv() else {
            return;
        };
        match self.hilo_usuario_pendiente.take() {
            Some(HiloUsuarioPendiente::Creacion(_, actor, datos, nombre)) => {
                let resultado = match resultado_hash {
                    Ok(hash) => core
                        .ok_or_else(|| "No se pudo guardar el usuario".to_owned())
                        .and_then(|core| {
                            core.crear_usuario_con_hash(
                                &actor,
                                &datos.cedula,
                                &datos.nombre,
                                datos.rol,
                                datos.activo,
                                hash,
                            )
                            .map(Some)
                            .map_err(mensaje_usuario)
                        }),
                    Err(error) => Err(error.to_string()),
                };
                let recarga = self.usuarios.completar_guardado(resultado, None, &nombre);
                self.procesar_recarga_usuarios(recarga, core);
            }
            Some(HiloUsuarioPendiente::CambioPassword(_, actor, id, nombre)) => {
                let resultado = match resultado_hash {
                    Ok(hash) => core
                        .ok_or_else(|| "No se pudo cambiar la contraseña".to_owned())
                        .and_then(|core| {
                            core.cambiar_password_usuario_con_hash(&actor, id, &hash)
                                .map_err(mensaje_usuario)
                        }),
                    Err(error) => Err(error.to_string()),
                };
                self.usuarios.completar_password(resultado, &nombre);
            }
            None => {}
        }
    }

    fn iniciar_cambio_password_propio(
        &mut self,
        password_actual: String,
        nueva_password: String,
        core: Option<&AppCore>,
    ) {
        let resultado = (|| {
            let core = core.ok_or_else(|| "No se pudo cambiar la contraseña".to_owned())?;
            let actor = self
                .sesion
                .as_ref()
                .ok_or_else(|| "No hay una sesión activa".to_owned())?;
            let candidato = core
                .preparar_cambio_password_propio(actor, &nueva_password)
                .map_err(mensaje_usuario)?;
            let hash_actual = candidato.password_hash.clone();
            let (emisor, receptor) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let resultado = crate::services::autenticacion_service::verificar_candidato(
                    candidato,
                    &password_actual,
                )
                .map_err(|_| "La contraseña actual es incorrecta".to_owned())
                .and_then(|_| {
                    crate::services::password::generar_hash(&nueva_password)
                        .map(|nuevo_hash| (hash_actual, nuevo_hash))
                        .map_err(|error| error.to_string())
                });
                let _ = emisor.send(resultado);
            });
            Ok(receptor)
        })();
        match resultado {
            Ok(receptor) => self.cambio_password_pendiente = Some(receptor),
            Err(error) => self.cambio_password.completar(Err(error)),
        }
    }

    fn recibir_cambio_password_propio(&mut self, core: Option<&AppCore>) {
        let Some(receptor) = &self.cambio_password_pendiente else {
            return;
        };
        let Ok(resultado_hilo) = receptor.try_recv() else {
            return;
        };
        self.cambio_password_pendiente = None;
        let resultado = resultado_hilo.and_then(|(hash_actual, nuevo_hash)| {
            let core = core.ok_or_else(|| "No se pudo cambiar la contraseña".to_owned())?;
            let actor = self
                .sesion
                .as_ref()
                .ok_or_else(|| "No hay una sesión activa".to_owned())?;
            core.cambiar_mi_password_con_hash(actor, &hash_actual, &nuevo_hash)
                .map_err(mensaje_usuario)
        });
        self.cambio_password.completar(resultado);
    }

    /// Mismo patrón para el ROOT inicial: valida rápido (sin la comprobación de "ya
    /// existe un ROOT", que sigue siendo atómica con el insert), hashea aparte, y crea
    /// el usuario cuando llega el hash — ver `recibir_root_inicial_si_lista`.
    fn iniciar_root_inicial(&mut self, solicitud: SolicitudRoot, core: &AppCore) {
        if let Err(error) = core.validar_datos_para_root_inicial(&CrearRootInicialInput {
            cedula: solicitud.cedula.clone(),
            nombre: solicitud.nombre.clone(),
            password: solicitud.password.clone(),
        }) {
            self.configuracion_inicial
                .completar_con_error(error.to_string());
            return;
        }
        let receptor = Self::generar_hash_en_hilo(solicitud.password.clone());
        self.root_inicial_pendiente = Some((receptor, solicitud));
    }

    fn recibir_root_inicial_si_lista(&mut self, core: &AppCore) {
        let Some((receptor, ..)) = &self.root_inicial_pendiente else {
            return;
        };
        let Ok(resultado_hash) = receptor.try_recv() else {
            return;
        };
        let Some((_, solicitud)) = self.root_inicial_pendiente.take() else {
            return;
        };
        match resultado_hash {
            Ok(hash) => {
                let input = CrearRootInicialInput {
                    cedula: solicitud.cedula,
                    nombre: solicitud.nombre,
                    password: solicitud.password,
                };
                match core.crear_root_inicial_con_hash(input, hash) {
                    Ok(_) | Err(UsuarioServiceError::ConfiguracionInicialYaRealizada) => {
                        self.configuracion_inicial.limpiar_secretos();
                        self.vista = Vista::Login;
                    }
                    Err(UsuarioServiceError::Database(_)) => self
                        .configuracion_inicial
                        .completar_con_error("No se pudo crear el usuario ROOT"),
                    Err(error) => self
                        .configuracion_inicial
                        .completar_con_error(error.to_string()),
                }
            }
            Err(error) => self
                .configuracion_inicial
                .completar_con_error(error.to_string()),
        }
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
            Some(StandardCommand::Theme) => {
                self.tema = self.tema.next();
                self.persistir_preferencias_si_cambiaron();
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
        // Atajo global sin documentar en ninguna pantalla de ayuda (a pedido
        // explícito): Ctrl+1..Ctrl+9 saltan directo a la pantalla
        // correspondiente sin pasar por el menú principal. Reusa la misma
        // tabla de números y el mismo chequeo de rol que ya tiene
        // `MenuPrincipalState::handle_key` (armando el `KeyEvent` sin el
        // modificador) en vez de duplicar la relación número→pantalla acá,
        // para que no puedan desincronizarse.
        if self.sesion.is_some()
            && !self.salida_rapida.abierto()
            && key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
            && !key.modifiers.contains(crossterm::event::KeyModifiers::ALT)
            && matches!(key.code, crossterm::event::KeyCode::Char(c) if c.is_ascii_digit() && c != '0')
        {
            let sin_modificador =
                crossterm::event::KeyEvent::new(key.code, crossterm::event::KeyModifiers::NONE);
            self.procesar_accion_menu_con_core(sin_modificador, core);
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

    /// Espera (bloqueando, con reintentos cortos) cualquier hilo de Argon2 en vuelo
    /// antes de la salida de emergencia — sin esto, la escritura ya validada se
    /// pierde en silencio porque el bucle principal termina sin volver a sondear el
    /// canal. El login no escribe nada y se abandona sin esperar.
    fn finalizar_hilos_pendientes(&mut self, core: Option<&AppCore>) {
        while self.hilo_usuario_pendiente.is_some() {
            self.recibir_hilo_usuario_si_lista(core);
            if self.hilo_usuario_pendiente.is_some() {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        match core {
            Some(core) => {
                while self.root_inicial_pendiente.is_some() {
                    self.recibir_root_inicial_si_lista(core);
                    if self.root_inicial_pendiente.is_some() {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                }
            }
            None => self.root_inicial_pendiente = None,
        }
    }

    fn procesar_accion_salida_rapida(
        &mut self,
        accion: AccionSalidaRapida,
        core: Option<&AppCore>,
    ) {
        match accion {
            AccionSalidaRapida::Ninguna => {}
            AccionSalidaRapida::Buscar { texto } => {
                let resultado = core
                    .ok_or_else(|| "No se pudieron cargar los ingresos activos".into())
                    .and_then(|c| {
                        c.listar_ingresos_activos(
                            &crate::database::queries::ingresos::FiltroIngresosActivos {
                                texto,
                                ..Default::default()
                            },
                        )
                        .map_err(|_| "No se pudieron cargar los ingresos activos".into())
                    });
                self.salida_rapida.completar_busqueda(resultado);
            }
            AccionSalidaRapida::Confirmar {
                registro_id,
                nombre,
            } => {
                let resultado = match (&self.sesion, core) {
                    (Some(s), _) if s.id == 0 => {
                        Err("La sesión de desarrollo no puede registrar movimientos reales".into())
                    }
                    (Some(s), Some(c)) => c
                        .registrar_salida(s, registro_id)
                        .map(|()| format!("✓ Salida registrada — {nombre}"))
                        .map_err(mensaje_salida),
                    _ => Err("No se pudo registrar la salida".into()),
                };
                let recarga = self.salida_rapida.completar_confirmacion(resultado);
                self.procesar_accion_salida_rapida(recarga, core);
                // La salida se registra desde el overlay global (F2), sin
                // pasar por la pantalla que el operador tiene abierta
                // debajo — a diferencia de registrarla directo desde
                // Ingresos Activos, acá nada más recarga esa pantalla en
                // particular. Sin este refresco, Historial/Activos/Nuevo
                // Ingreso se quedaban mostrando datos viejos (p. ej.
                // `tiene_ingreso_activo` desactualizado) hasta que el
                // operador navegaba a otra pantalla y volvía.
                match self.vista {
                    Vista::IngresosActivos => {
                        let recarga_activos = self.activos.solicitud_carga();
                        self.procesar_accion_activos(recarga_activos, core);
                    }
                    Vista::Historial => {
                        let recarga_historial = self.historial.refrescar();
                        self.procesar_accion_historial(recarga_historial, core);
                    }
                    Vista::NuevoIngreso => {
                        let recarga_nuevo_ingreso = self.nuevo_ingreso.refrescar();
                        self.procesar_accion_nuevo_ingreso(recarga_nuevo_ingreso, core);
                    }
                    _ => {}
                }
            }
        }
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
                let accion = self.contratistas.handle_key(key);
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
                    AccionCambioPassword::Volver => self.vista = Vista::MenuPrincipal,
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
                self.menu.seleccion = opcion;
                self.vista = match opcion {
                    OpcionMenu::NuevoIngreso => {
                        // El menú sólo es alcanzable con `self.sesion` ya
                        // establecida (`Vista::MenuPrincipal` no renderiza sin
                        // ella) — este fallback es defensivo, no debería
                        // dispararse nunca en un flujo real.
                        self.nuevo_ingreso = NuevoIngresoState::new();
                        if core.is_some() {
                            self.procesar_accion_nuevo_ingreso(
                                self.nuevo_ingreso.solicitud_carga(),
                                core,
                            );
                        }
                        Vista::NuevoIngreso
                    }
                    OpcionMenu::IngresosActivos => {
                        if let Some(core) = core {
                            self.activos.completar_empresas(
                                core.listar_empresas()
                                    .map_err(|_| "No se pudieron cargar las empresas".into()),
                            );
                            self.procesar_accion_activos(self.activos.solicitud_carga(), Some(core))
                        }
                        Vista::IngresosActivos
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
                        Vista::Historial
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
                        Vista::Contratistas
                    }
                    OpcionMenu::Empresas => {
                        if core.is_some() {
                            self.procesar_accion_empresas(self.empresas.solicitar_carga(), core);
                        }
                        Vista::Empresas
                    }
                    OpcionMenu::Usuarios => {
                        if core.is_some() {
                            self.procesar_accion_usuarios(self.usuarios.solicitud_carga(), core);
                        }
                        Vista::Usuarios
                    }
                    OpcionMenu::CambiarPassword => {
                        self.cambio_password.reiniciar();
                        Vista::CambiarPassword
                    }
                    OpcionMenu::Auditoria => {
                        let accion = self.auditoria.reiniciar();
                        self.procesar_accion_auditoria(accion, core);
                        Vista::Auditoria
                    }
                    OpcionMenu::Respaldos => {
                        let accion = self.configuracion.reiniciar();
                        self.procesar_accion_configuracion(accion, core);
                        Vista::Respaldos
                    }
                    OpcionMenu::CerrarSesion | OpcionMenu::Salir => Vista::MenuPrincipal,
                };
            }
            AccionMenu::CerrarSesion => {
                self.sesion = None;
                self.login.reiniciar();
                self.vista = Vista::Login;
            }
            AccionMenu::Salir => self.salir = true,
        }
    }

    fn procesar_accion_empresas(&mut self, accion: AccionEmpresas, core: Option<&AppCore>) {
        let actor = self.sesion.clone();
        match accion {
            AccionEmpresas::Ninguna => {}
            AccionEmpresas::Volver => self.vista = Vista::MenuPrincipal,
            AccionEmpresas::Buscar {
                texto,
                seleccionar_id,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo cargar la base de empresas".to_owned())
                    .and_then(|core| {
                        core.buscar_empresas(&crate::database::queries::empresas::FiltroEmpresas {
                            texto,
                            ..Default::default()
                        })
                        .map_err(|_| "No se pudo cargar la base de empresas".to_owned())
                    });
                self.empresas.completar_busqueda(resultado, seleccionar_id);
            }
            AccionEmpresas::Crear { nombre } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar la empresa".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.crear_empresa(actor, &nombre).map_err(mensaje_empresa)
                    });
                let recarga = self.empresas.completar_creacion(resultado, &nombre);
                if !matches!(recarga, AccionEmpresas::Ninguna) {
                    self.procesar_accion_empresas(recarga, core);
                }
            }
            AccionEmpresas::Actualizar { id, nombre } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar la empresa".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.actualizar_empresa(actor, id, &nombre)
                            .map_err(mensaje_empresa)
                    });
                let recarga = self
                    .empresas
                    .completar_actualizacion(resultado, id, &nombre);
                if !matches!(recarga, AccionEmpresas::Ninguna) {
                    self.procesar_accion_empresas(recarga, core);
                }
            }
            AccionEmpresas::EstablecerActivo {
                id,
                activar,
                nombre,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo actualizar el estado de la empresa".into())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        if activar {
                            core.activar_empresa(actor, id)
                        } else {
                            core.desactivar_empresa(actor, id)
                        }
                        .map_err(mensaje_empresa)
                    });
                let recarga = self
                    .empresas
                    .completar_estado(resultado, id, activar, &nombre);
                if !matches!(recarga, AccionEmpresas::Ninguna) {
                    self.procesar_accion_empresas(recarga, core);
                }
            }
        }
    }

    fn procesar_accion_auditoria(&mut self, accion: AccionAuditoria, core: Option<&AppCore>) {
        match accion {
            AccionAuditoria::Ninguna => {}
            AccionAuditoria::Volver => self.vista = Vista::MenuPrincipal,
            AccionAuditoria::Cargar { offset } => {
                let resultado = (|| {
                    let core = core.ok_or_else(|| "No se pudo cargar la auditoría".to_owned())?;
                    let actor = self
                        .sesion
                        .as_ref()
                        .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                    core.buscar_auditoria_contratistas(
                        actor,
                        &crate::database::queries::auditoria_contratistas::FiltroAuditoriaContratistas {
                            offset,
                            ..Default::default()
                        },
                    )
                    .map_err(|error| error.to_string())
                })();
                self.auditoria.completar(resultado);
            }
        }
    }

    fn procesar_accion_contratistas(&mut self, accion: AccionContratistas, core: Option<&AppCore>) {
        let actor = self.sesion.clone();
        match accion {
            AccionContratistas::Ninguna => {}
            AccionContratistas::Volver => self.vista = Vista::MenuPrincipal,
            AccionContratistas::Buscar {
                texto,
                seleccionar_id,
                empresa_id,
                tipos,
                praind,
                praind_negado,
                personal_ruta,
                tiene_acceso,
                offset,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo cargar la base de contratistas".into())
                    .and_then(|core| {
                        core.buscar_contratistas(
                            &crate::database::queries::contratistas::FiltroContratistas {
                                texto,
                                empresa_id,
                                tipos_incluidos: tipos,
                                praind,
                                praind_negado,
                                personal_ruta,
                                tiene_acceso,
                                offset,
                                ..Default::default()
                            },
                        )
                        .map_err(|_| "No se pudo cargar la base de contratistas".into())
                    });
                self.contratistas
                    .completar_busqueda(resultado, seleccionar_id);
            }
            AccionContratistas::Crear { datos, nombre } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar el contratista".into())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.crear_contratista(actor, datos)
                            .map(Some)
                            .map_err(mensaje_contratista)
                    });
                let recarga = self
                    .contratistas
                    .completar_guardado(resultado, None, &nombre);
                if !matches!(recarga, AccionContratistas::Ninguna) {
                    self.procesar_accion_contratistas(recarga, core);
                }
            }
            AccionContratistas::Actualizar { id, datos, nombre } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar el contratista".into())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.actualizar_contratista(actor, id, datos)
                            .map(|_| None)
                            .map_err(mensaje_contratista)
                    });
                let recarga = self
                    .contratistas
                    .completar_guardado(resultado, Some(id), &nombre);
                if !matches!(recarga, AccionContratistas::Ninguna) {
                    self.procesar_accion_contratistas(recarga, core);
                }
            }
        }
    }

    fn procesar_accion_usuarios(&mut self, accion: AccionUsuarios, core: Option<&AppCore>) {
        let actor = self.sesion.clone();
        match accion {
            AccionUsuarios::Ninguna => {}
            AccionUsuarios::Volver => self.vista = Vista::MenuPrincipal,
            AccionUsuarios::Buscar {
                texto,
                seleccionar_id,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo cargar la base de usuarios".into())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.buscar_usuarios(
                            actor,
                            &crate::database::queries::usuarios::FiltroUsuarios {
                                texto,
                                ..Default::default()
                            },
                        )
                        .map_err(|_| "No se pudo cargar la base de usuarios".into())
                    });
                self.usuarios.completar_busqueda(resultado, seleccionar_id);
            }
            AccionUsuarios::Crear { input, nombre } => {
                self.iniciar_creacion_usuario(input, nombre, core)
            }
            AccionUsuarios::Actualizar {
                id,
                input,
                activo,
                nombre,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar el usuario".into())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.actualizar_usuario(actor, id, input, activo)
                            .map_err(mensaje_usuario)
                    })
                    .map(|_| None);
                let recarga = self
                    .usuarios
                    .completar_guardado(resultado, Some(id), &nombre);
                self.procesar_recarga_usuarios(recarga, core);
                self.actualizar_sesion_desde_tabla(id);
            }
            AccionUsuarios::CambiarPassword {
                id,
                password,
                nombre,
            } => self.iniciar_cambio_password(id, password, nombre, core),
            AccionUsuarios::EstablecerActivo {
                id,
                activar,
                nombre,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo actualizar el estado del usuario".into())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        if activar {
                            core.activar_usuario(actor, id)
                        } else {
                            core.desactivar_usuario(actor, id)
                        }
                        .map_err(mensaje_usuario)
                    });
                let recarga = self
                    .usuarios
                    .completar_estado(resultado, id, activar, &nombre);
                self.procesar_recarga_usuarios(recarga, core);
            }
        }
    }

    fn procesar_recarga_usuarios(&mut self, accion: AccionUsuarios, core: Option<&AppCore>) {
        if !matches!(accion, AccionUsuarios::Ninguna) {
            self.procesar_accion_usuarios(accion, core);
        }
    }

    fn procesar_accion_configuracion(&mut self, accion: AccionAjustes, core: Option<&AppCore>) {
        match accion {
            AccionAjustes::Ninguna => {}
            AccionAjustes::Volver => self.vista = Vista::MenuPrincipal,
            AccionAjustes::Respaldos(accion) => self.procesar_accion_respaldos(accion, core),
        }
    }

    fn procesar_accion_respaldos(&mut self, accion: AccionRespaldos, core: Option<&AppCore>) {
        let actor = self.sesion.clone();
        match accion {
            AccionRespaldos::Ninguna | AccionRespaldos::Volver => {}
            AccionRespaldos::Cargar => {
                let resultado = core
                    .ok_or_else(|| "No se pudieron listar los respaldos".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.listar_respaldos(actor)
                            .map_err(|error| error.to_string())
                    });
                self.configuracion.completar_listado(resultado);
            }
            AccionRespaldos::Crear => {
                let resultado = core
                    .ok_or_else(|| "No se pudo crear el respaldo".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.crear_respaldo(actor, crate::database::backup::TipoRespaldo::Manual)
                            .map_err(|error| error.to_string())
                    });
                self.configuracion.completar_creacion(resultado);
            }
            AccionRespaldos::Revalidar { ruta } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo validar el respaldo".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.validar_respaldo(actor, &ruta)
                            .map_err(|error| error.to_string())
                    });
                self.configuracion.completar_validacion(&ruta, resultado);
            }
            AccionRespaldos::Exportar { ruta, destino } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo exportar el respaldo".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.exportar_respaldo(actor, &ruta, &destino)
                            .map_err(|error| error.to_string())
                    });
                self.configuracion
                    .completar_exportacion(resultado, &destino);
            }
            AccionRespaldos::Restaurar { ruta } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo respaldar la base antes de restaurar".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.crear_respaldo(
                            actor,
                            crate::database::backup::TipoRespaldo::PreRestauracion,
                        )
                        .map_err(|error| error.to_string())
                    });
                match resultado {
                    Ok(_) => {
                        self.salida = SalidaApp::Restaurar { candidata: ruta };
                        self.salir = true;
                    }
                    Err(error) => self.configuracion.completar_creacion(Err(error)),
                }
            }
        }
    }

    fn actualizar_sesion_desde_tabla(&mut self, id: i64) {
        let Some(sesion) = &mut self.sesion else {
            return;
        };
        if sesion.id != id {
            return;
        }
        if let Some(usuario) = self.usuarios.resumen_por_id(id) {
            sesion.cedula = usuario.cedula.clone();
            sesion.nombre = usuario.nombre.clone();
            sesion.rol = usuario.rol;
        }
    }

    fn procesar_accion_nuevo_ingreso(
        &mut self,
        accion: AccionNuevoIngreso,
        core: Option<&AppCore>,
    ) {
        match accion {
            AccionNuevoIngreso::Ninguna => {}
            AccionNuevoIngreso::Volver => self.vista = Vista::MenuPrincipal,
            AccionNuevoIngreso::Buscar { texto } => {
                let r = core
                    .ok_or_else(|| "No se pudieron cargar los contratistas".into())
                    .and_then(|c| {
                        c.buscar_contratistas(
                            &crate::database::queries::contratistas::FiltroContratistas {
                                texto,
                                ..Default::default()
                            },
                        )
                        .map_err(|_| "No se pudieron cargar los contratistas".into())
                    });
                self.nuevo_ingreso.completar_busqueda(r);
            }
            AccionNuevoIngreso::Preparar { contratista_id } => {
                let r = core
                    .ok_or_else(|| "No se pudo preparar el ingreso".into())
                    .and_then(|c| c.preparar_ingreso(contratista_id).map_err(mensaje_ingreso));
                self.nuevo_ingreso.completar_preparacion(r);
            }
            AccionNuevoIngreso::Registrar {
                contratista_id,
                medio,
                gafete,
            } => {
                let resultado = match (&self.sesion, core) {
                    (Some(s), _) if s.id == 0 => {
                        Err("La sesión de desarrollo no puede registrar movimientos reales".into())
                    }
                    (Some(s), Some(c)) => c
                        .registrar_ingreso(s, contratista_id, medio, gafete)
                        .map(|r| r.registro_id)
                        .map_err(mensaje_ingreso),
                    _ => Err("No se pudo registrar el ingreso".into()),
                };
                // Se queda en Nuevo Ingreso tras registrar (a diferencia de
                // antes, que saltaba a Ingresos Activos) — con varios
                // contratistas por procesar seguidos, ese salto obligaba a
                // volver a navegar por cada uno. `completar_registro` deja
                // su propio mensaje de confirmación ("✓ Ingreso registrado
                // — X") y pide recargar la misma búsqueda para que la
                // lista no quede en blanco ni pierda lo que el operador
                // ya tenía filtrado.
                let recarga = self.nuevo_ingreso.completar_registro(resultado);
                self.procesar_accion_nuevo_ingreso(recarga, core);
            }
        }
    }

    fn procesar_accion_activos(&mut self, accion: AccionActivos, core: Option<&AppCore>) {
        match accion {
            AccionActivos::Ninguna => {}
            AccionActivos::Volver => self.vista = Vista::MenuPrincipal,
            AccionActivos::Buscar {
                texto,
                seleccionar_id,
                empresa_id,
                tipos,
                gafete,
                medio,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudieron cargar los ingresos activos".into())
                    .and_then(|c| {
                        c.listar_ingresos_activos(
                            &crate::database::queries::ingresos::FiltroIngresosActivos {
                                texto,
                                empresa_id,
                                tipos_incluidos: tipos,
                                gafete_numero: gafete,
                                medio_ingreso: medio,
                                ..Default::default()
                            },
                        )
                        .map_err(|_| "No se pudieron cargar los ingresos activos".into())
                    });
                self.activos.completar_busqueda(resultado, seleccionar_id);
            }
            AccionActivos::RegistrarSalida {
                registro_id,
                nombre,
            } => {
                let resultado = match (&self.sesion, core) {
                    (Some(s), _) if s.id == 0 => {
                        Err("La sesión de desarrollo no puede registrar movimientos reales".into())
                    }
                    (Some(s), Some(c)) => {
                        c.registrar_salida(s, registro_id).map_err(mensaje_salida)
                    }
                    _ => Err("No se pudo registrar la salida".into()),
                };
                let recarga = self
                    .activos
                    .completar_salida(resultado, registro_id, &nombre);
                self.procesar_accion_activos(recarga, core);
            }
        }
    }

    fn procesar_accion_historial(&mut self, accion: AccionHistorial, core: Option<&AppCore>) {
        match accion {
            AccionHistorial::Ninguna => {}
            AccionHistorial::Volver => self.vista = Vista::MenuPrincipal,
            AccionHistorial::Consultar(filtro) => {
                let resultado = core
                    .ok_or_else(|| "No se pudo cargar el historial".into())
                    .and_then(|core| {
                        core.buscar_historial(&filtro)
                            .map_err(|_| "No se pudo cargar el historial".into())
                    });
                self.historial.completar(resultado);
            }
            AccionHistorial::Exportar {
                filtro,
                columnas,
                destino,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo exportar el historial".to_owned())
                    .and_then(|core| {
                        core.exportar_historial(&filtro, &columnas, &destino)
                            .map_err(|error| error.to_string())
                    });
                self.historial.completar_exportacion(resultado, &destino);
            }
        }
    }

    fn iniciar_sesion(&mut self, sesion: UsuarioSesion) {
        self.sesion = Some(sesion);
        self.menu.nueva_sesion();
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
