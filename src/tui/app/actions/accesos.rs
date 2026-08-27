//! Despachadores de acceso: Ingresos Activos, Historial, Nuevo Ingreso y el
//! overlay global de Salida Rápida (F2).

use crate::application::AppCore;
use crate::mensajes::{mensaje_ingreso, mensaje_salida};
use crate::tui::activos::AccionActivos;
use crate::tui::app::{App, Vista};
use crate::tui::historial::AccionHistorial;
use crate::tui::nuevo_ingreso::AccionNuevoIngreso;
use crate::tui::salida_rapida::AccionSalidaRapida;

impl App {
    pub(in crate::tui::app) fn procesar_accion_salida_rapida(
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

    pub(in crate::tui::app) fn procesar_accion_nuevo_ingreso(
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

    pub(in crate::tui::app) fn procesar_accion_activos(
        &mut self,
        accion: AccionActivos,
        core: Option<&AppCore>,
    ) {
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

    pub(in crate::tui::app) fn procesar_accion_historial(
        &mut self,
        accion: AccionHistorial,
        core: Option<&AppCore>,
    ) {
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
}
