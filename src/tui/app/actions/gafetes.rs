//! Despachador de `GestionGafetes` — archivo propio en vez de sumarlo a
//! `catalogos.rs` (que ya es "Contratistas y Empresas"), mismo patrón que el
//! resto de `procesar_accion_*`.

use crate::application::AppCore;
use crate::database::queries::contratistas::FiltroContratistas;
use crate::mensajes::mensaje_gafete;
use crate::tui::app::{App, Vista};
use crate::tui::gafetes::AccionGafetes;

impl App {
    pub(in crate::tui::app) fn procesar_accion_gafetes(
        &mut self,
        accion: AccionGafetes,
        core: Option<&AppCore>,
    ) {
        let actor = self.sesion.clone();
        match accion {
            AccionGafetes::Ninguna => {}
            AccionGafetes::Volver => self.vista = Vista::MenuPrincipal,
            AccionGafetes::Buscar {
                filtro,
                seleccionar_id,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo cargar el catálogo de gafetes".to_owned())
                    .and_then(|core| {
                        core.buscar_gafetes(&filtro)
                            .map_err(|_| "No se pudo cargar el catálogo de gafetes".to_owned())
                    });
                self.gafetes.completar_busqueda(resultado, seleccionar_id);
            }
            AccionGafetes::CrearUno { numero } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar el gafete".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.crear_gafete(actor, numero).map_err(mensaje_gafete)
                    });
                let recarga = self.gafetes.completar_alta(resultado, numero);
                if !matches!(recarga, AccionGafetes::Ninguna) {
                    self.procesar_accion_gafetes(recarga, core);
                }
            }
            AccionGafetes::CrearRango { desde, hasta } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo guardar el rango de gafetes".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.crear_gafetes_rango(actor, desde, hasta)
                            .map_err(mensaje_gafete)
                    });
                let recarga = self.gafetes.completar_alta_rango(resultado, desde, hasta);
                if !matches!(recarga, AccionGafetes::Ninguna) {
                    self.procesar_accion_gafetes(recarga, core);
                }
            }
            AccionGafetes::DarDeBaja { id, numero } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo dar de baja el gafete".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.dar_de_baja_gafete(actor, id).map_err(mensaje_gafete)
                    });
                let recarga = self.gafetes.completar_baja(resultado, id, numero);
                if !matches!(recarga, AccionGafetes::Ninguna) {
                    self.procesar_accion_gafetes(recarga, core);
                }
            }
            AccionGafetes::BuscarDeudor { texto } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo buscar contratistas".to_owned())
                    .and_then(|core| {
                        core.buscar_contratistas(&FiltroContratistas {
                            texto,
                            ..Default::default()
                        })
                        .map(|pagina| pagina.items)
                        .map_err(|_| "No se pudo buscar contratistas".to_owned())
                    });
                self.gafetes.completar_busqueda_deudor(resultado);
            }
            AccionGafetes::MarcarPerdido {
                id,
                numero,
                contratista_id,
            } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo marcar el gafete como perdido".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.marcar_gafete_perdido(actor, id, contratista_id)
                            .map_err(mensaje_gafete)
                    });
                let recarga = self.gafetes.completar_marcar_perdido(resultado, id, numero);
                if !matches!(recarga, AccionGafetes::Ninguna) {
                    self.procesar_accion_gafetes(recarga, core);
                }
            }
            AccionGafetes::Resolver { id, numero, motivo } => {
                let resultado = core
                    .ok_or_else(|| "No se pudo resolver la deuda del gafete".to_owned())
                    .and_then(|core| {
                        let actor = actor
                            .as_ref()
                            .ok_or_else(|| "No hay una sesión activa".to_owned())?;
                        core.resolver_gafete(actor, id, motivo)
                            .map_err(mensaje_gafete)
                    });
                let recarga = self.gafetes.completar_resolver(resultado, id, numero);
                if !matches!(recarga, AccionGafetes::Ninguna) {
                    self.procesar_accion_gafetes(recarga, core);
                }
            }
        }
    }
}
