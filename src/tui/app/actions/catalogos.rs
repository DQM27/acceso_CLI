//! Despachadores de catálogos: Contratistas y Empresas.

use crate::application::AppCore;
use crate::mensajes::{mensaje_contratista, mensaje_empresa};
use crate::tui::app::{App, Vista};
use crate::tui::contratistas::AccionContratistas;
use crate::tui::empresas::AccionEmpresas;

impl App {
    pub(in crate::tui::app) fn procesar_accion_empresas(
        &mut self,
        accion: AccionEmpresas,
        core: Option<&AppCore>,
    ) {
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

    pub(in crate::tui::app) fn procesar_accion_contratistas(
        &mut self,
        accion: AccionContratistas,
        core: Option<&AppCore>,
    ) {
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
                            .map(|()| None)
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
}
