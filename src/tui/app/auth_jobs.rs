//! Trabajos asincrónicos de autenticación y contraseñas: los 4 flujos que
//! calculan un hash de Argon2 en un hilo aparte para no bloquear el bucle de
//! la TUI (login, crear usuario, cambiar contraseña administrativa/propia,
//! ROOT inicial), y su recepción no bloqueante en cada vuelta del bucle.

use std::sync::mpsc;

use crate::application::AppCore;
use crate::models::usuario::RolUsuario;
use crate::services::autenticacion_service::UsuarioSesion;
use crate::services::error::{AutenticacionError, PasswordError, UsuarioServiceError};
use crate::services::usuario_service::CrearRootInicialInput;
use crate::tui::configuracion_inicial::SolicitudRoot;

use super::{App, Vista};
use crate::mensajes::{mensaje_autenticacion, mensaje_usuario};

/// Datos ya validados de un usuario nuevo, a la espera del hash de Argon2 —
/// no incluye `password` en texto plano, que ya se movió al hilo que calcula
/// el hash y no hace falta después.
#[derive(Debug)]
pub(super) enum HiloUsuarioPendiente {
    Creacion(ReceptorHash, UsuarioSesion, DatosUsuarioPendiente, String),
    CambioPassword(ReceptorHash, UsuarioSesion, i64, String),
}

#[derive(Debug, Clone)]
pub(super) struct DatosUsuarioPendiente {
    pub(super) cedula: String,
    pub(super) nombre: String,
    pub(super) rol: RolUsuario,
    pub(super) activo: bool,
}

/// Receptor del hilo aparte que sólo calcula un hash de Argon2 — nunca del resultado
/// final de escribir en `SQLite`, que ocurre después, en el hilo principal.
pub(super) type ReceptorHash = mpsc::Receiver<Result<String, PasswordError>>;
pub(super) type ReceptorCambioPropio = mpsc::Receiver<Result<(String, String), String>>;
pub(super) type ReceptorAutenticacion = mpsc::Receiver<Result<UsuarioSesion, AutenticacionError>>;

impl App {
    /// Revisa sin bloquear si el hilo de verificación de contraseña (Argon2) ya terminó.
    ///
    /// La contraseña se verificó contra un `UsuarioSesion`/hash resueltos
    /// *antes* de que corriera Argon2 (potencialmente varios cientos de ms
    /// atrás, ver `iniciar_autenticacion`) — si la cuenta fue desactivada,
    /// degradada o editada mientras tanto, ese snapshot ya está vencido.
    /// Antes de aceptar la sesión se vuelve a resolver el candidato contra
    /// `SQLite` (rápido, sin Argon2) y se usa ese estado fresco, no el que
    /// llegó por el canal — `buscar_candidato` ya rechaza cuentas inactivas
    /// (`docs/auditoria-dominio-2026-08-20.md`, hallazgo #5).
    pub(super) fn recibir_autenticacion_si_lista(&mut self, core: Option<&AppCore>) {
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
                self.iniciar_sesion(sesion, core);
            }
            Err(error) => self
                .login
                .completar_validacion(Some(mensaje_autenticacion(error))),
        }
    }

    /// Resuelve la cédula de inmediato (rápido, sólo `SQLite`) y, si existe y está activo,
    /// verifica la contraseña en un hilo aparte para no congelar la UI mientras Argon2 calcula.
    pub(super) fn iniciar_autenticacion(
        &mut self,
        cedula: String,
        password: String,
        core: Option<&AppCore>,
    ) {
        let Some(core) = core else {
            self.login.completar_validacion(None);
            self.iniciar_sesion(
                UsuarioSesion {
                    id: 0,
                    cedula: cedula.clone(),
                    nombre: cedula,
                    rol: RolUsuario::Operador,
                },
                None,
            );
            return;
        };
        match core.buscar_candidato_autenticacion(&cedula) {
            Ok(candidato) => {
                let (emisor, receptor) = mpsc::channel();
                std::thread::spawn(move || {
                    let resultado = crate::services::autenticacion_service::verificar_candidato(
                        candidato, &password,
                    );
                    let _ = emisor.send(resultado);
                });
                self.autenticacion_pendiente = Some(receptor);
            }
            Err(error) => self
                .login
                .completar_validacion(Some(mensaje_autenticacion(error))),
        }
    }

    /// Vuelve a resolver el candidato contra `SQLite` justo antes de aceptar la
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
    pub(super) fn generar_hash_en_hilo(password: String) -> ReceptorHash {
        let (emisor, receptor) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = emisor.send(crate::services::password::generar_hash(&password));
        });
        receptor
    }

    /// Valida rápido (sólo `SQLite`) y, si pasa, calcula el hash de Argon2 en un hilo
    /// aparte — la escritura real ocurre después, en el hilo principal, cuando llega.
    pub(super) fn iniciar_creacion_usuario(
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
    pub(super) fn iniciar_cambio_password(
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
    pub(super) fn recibir_hilo_usuario_si_lista(&mut self, core: Option<&AppCore>) {
        let Some(
            HiloUsuarioPendiente::Creacion(receptor, ..)
            | HiloUsuarioPendiente::CambioPassword(receptor, ..),
        ) = &self.hilo_usuario_pendiente
        else {
            return;
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

    pub(super) fn iniciar_cambio_password_propio(
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
            let (emisor, receptor) = mpsc::channel();
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

    pub(super) fn recibir_cambio_password_propio(&mut self, core: Option<&AppCore>) {
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
    pub(super) fn iniciar_root_inicial(&mut self, solicitud: SolicitudRoot, core: &AppCore) {
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

    pub(super) fn recibir_root_inicial_si_lista(&mut self, core: &AppCore) {
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

    /// Espera (bloqueando, con reintentos cortos) cualquier hilo de Argon2 en vuelo
    /// antes de la salida de emergencia — sin esto, la escritura ya validada se
    /// pierde en silencio porque el bucle principal termina sin volver a sondear el
    /// canal. El login no escribe nada y se abandona sin esperar.
    pub(super) fn finalizar_hilos_pendientes(&mut self, core: Option<&AppCore>) {
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
}
