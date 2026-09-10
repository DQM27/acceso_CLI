//! Despachadores de administración: Usuarios y Auditoría.

use crate::application::AppCore;
use crate::mensajes::mensaje_usuario;
use crate::tui::app::{App, Vista};
use crate::tui::auditoria::AccionAuditoria;
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
                self.iniciar_creacion_usuario(input, nombre, core);
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
                    .map(|()| None);
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
            sesion.cedula.clone_from(&usuario.cedula);
            sesion.nombre.clone_from(&usuario.nombre);
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
                    core.buscar_auditoria(
                        actor,
                        &crate::database::queries::auditoria::FiltroAuditoria {
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

}
