//! Despachadores de administración: Usuarios, Auditoría y Respaldos.

use crate::application::AppCore;
use crate::mensajes::mensaje_usuario;
use crate::tui::app::{App, Vista};
use crate::tui::auditoria::AccionAuditoria;
use crate::tui::configuracion::{AccionAjustes, AccionRespaldos};
use crate::tui::usuarios::AccionUsuarios;

impl App {
    pub(in crate::tui::app) fn procesar_accion_usuarios(
        &mut self,
        accion: AccionUsuarios,
        core: Option<&AppCore>,
    ) {
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

    pub(in crate::tui::app) fn procesar_recarga_usuarios(
        &mut self,
        accion: AccionUsuarios,
        core: Option<&AppCore>,
    ) {
        if !matches!(accion, AccionUsuarios::Ninguna) {
            self.procesar_accion_usuarios(accion, core);
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

    pub(in crate::tui::app) fn procesar_accion_auditoria(
        &mut self,
        accion: AccionAuditoria,
        core: Option<&AppCore>,
    ) {
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

    pub(in crate::tui::app) fn procesar_accion_configuracion(
        &mut self,
        accion: AccionAjustes,
        core: Option<&AppCore>,
    ) {
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
                        self.salida = crate::tui::app::SalidaApp::Restaurar { candidata: ruta };
                        self.salir = true;
                    }
                    Err(error) => self.configuracion.completar_creacion(Err(error)),
                }
            }
        }
    }
}
