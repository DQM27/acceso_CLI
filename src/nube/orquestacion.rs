//! La sincronización con la nube, escrita UNA sola vez. Antes la misma
//! secuencia de etapas vivía copiada en `AppCore`, en el escritorio
//! (`comandos::nube`) y dos veces en el puente móvil, y las copias ya se
//! habían desincronizado entre sí.
//!
//! Los hosts sólo resuelven credenciales y contexto (secreto, token, qué
//! conexión usar, sin retener el candado del núcleo), llaman a
//! [`sincronizar`] o [`recibir`] y convierten el resumen a su tipo.
//!
//! Diferencias legítimas entre hosts, expresadas acá y no con copias:
//! - Qué etapas correr: [`AlcanceSincronizacion`] (completo para el pulso y
//!   el botón; parcial para un aviso en vivo, el login sin sesión o la
//!   activación inicial).
//! - Qué guarda cada equipo: [`PerfilDispositivo`]. El móvil no guarda
//!   ningún historial del sitio (decisión del dueño: el historial vive en
//!   el escritorio y el panel web; el celular es para operar). El
//!   escritorio guarda los últimos 24 meses (`retencion`).

use rusqlite::Connection;

use super::alcance::AlcanceSincronizacion;
use super::retencion;
use super::sincronizacion::{
    self, ConflictoGafeteActivo, ConflictoIngresoActivo, ConflictoIngresoProveedorActivo,
    ConflictoMovimientoVisitaActivo, ContextoSincronizacion, ResumenCatalogo, ResumenCatalogoRutas,
    SincronizacionError,
};

/// Cuántas filas de la bandeja de salida se mandan por sincronización.
const LOTE_BANDEJA_SALIDA: u32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfilDispositivo {
    /// Guarda todo, incluidos los historiales del sitio.
    Escritorio,
    /// No guarda ningún historial del sitio (contratistas, visitas,
    /// proveedores, gafetes provisionales) ni revisa conflictos de visitas:
    /// el historial se consulta en el escritorio y el panel web.
    Movil,
}

impl PerfilDispositivo {
    const fn guarda_historiales(self) -> bool {
        matches!(self, Self::Escritorio)
    }
}

/// Todo lo que una sincronización puede informar. Las etapas que no
/// corrieron (por alcance o por perfil) quedan en cero o vacías.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResumenSincronizacionNube {
    pub enviados: u32,
    pub fallidos: u32,
    /// Ver `ResumenDrenado::conflictos_gafete`.
    pub conflictos_gafete: Vec<ConflictoGafeteActivo>,
    pub cierres_recibidos: u32,
    pub cierres_recibidos_proveedor: u32,
    /// Ingresos que el otro dispositivo del sitio tiene abiertos.
    pub remotos_abiertos: u32,
    pub catalogo: ResumenCatalogo,
    pub catalogo_rutas: ResumenCatalogoRutas,
    pub movimientos_historial_recibidos: u32,
    pub citas_recibidas: u32,
    pub historial_visitas_recibidos: u32,
    pub historial_ingresos_proveedor_recibidos: u32,
    pub historial_gafetes_provisionales_recibidos: u32,
    /// Los tres `conflictos_*` son de mejor esfuerzo: si la consulta falla
    /// quedan vacíos, nunca tumban una sincronización que por lo demás
    /// anduvo.
    pub conflictos_ingreso: Vec<ConflictoIngresoActivo>,
    pub conflictos_movimiento_visita: Vec<ConflictoMovimientoVisitaActivo>,
    pub conflictos_ingreso_proveedor: Vec<ConflictoIngresoProveedorActivo>,
}

/// Vacía la bandeja de salida (siempre, sea cual sea el alcance: es local
/// y barata si está vacía) y después corre [`recibir`].
pub fn sincronizar(
    conexion: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    alcance: AlcanceSincronizacion,
    perfil: PerfilDispositivo,
) -> Result<ResumenSincronizacionNube, SincronizacionError> {
    let drenado = sincronizacion::drenar_cola(conexion, contexto, LOTE_BANDEJA_SALIDA)?;
    let mut resumen = recibir(conexion, contexto, alcance, perfil)?;
    resumen.enviados = drenado.enviados;
    resumen.fallidos = drenado.fallidos;
    resumen.conflictos_gafete = drenado.conflictos_gafete;
    if alcance.es_completo() {
        mantener_base_local(conexion, perfil);
    }
    Ok(resumen)
}

/// Mantenimiento al final de cada sincronización COMPLETA (el pulso y el
/// botón; no cada aviso en vivo): retención del historial
/// (sólo quien lo guarda, ver `retencion`) y `PRAGMA optimize`. Mejor
/// esfuerzo: si falla, la sincronización ya anduvo y no se tumba por esto.
fn mantener_base_local(conexion: &Connection, perfil: PerfilDispositivo) {
    if perfil.guarda_historiales() {
        let limite =
            retencion::limite_retencion(chrono::Utc::now(), retencion::MESES_HISTORIAL_ESCRITORIO);
        if let Err(error) = retencion::purgar_historiales_del_sitio(conexion, limite) {
            log::warn!("retención del historial: {error}");
        }
    }
    if let Err(error) = conexion.execute_batch("PRAGMA optimize") {
        log::warn!("PRAGMA optimize: {error}");
    }
}

/// Sólo la parte de recepción, sin mandar nada: para cuando todavía no hay
/// una sesión que autorice enviar (login sin sesión, activación inicial).
/// El orden de las etapas es el de siempre.
pub fn recibir(
    conexion: &Connection,
    contexto: &ContextoSincronizacion<'_>,
    alcance: AlcanceSincronizacion,
    perfil: PerfilDispositivo,
) -> Result<ResumenSincronizacionNube, SincronizacionError> {
    let historiales = perfil.guarda_historiales();
    let mut resumen = ResumenSincronizacionNube::default();

    if alcance.ingresos {
        resumen.cierres_recibidos =
            sincronizacion::recibir_cierres_de_ingresos_propios(conexion, contexto)?;
    }
    if alcance.ingresos_proveedor {
        resumen.cierres_recibidos_proveedor =
            sincronizacion::recibir_cierres_de_ingresos_propios_proveedor(conexion, contexto)?;
    }
    if alcance.ingresos {
        let remotos = sincronizacion::recibir_ingresos_abiertos(conexion, contexto)?;
        resumen.remotos_abiertos = u32::try_from(remotos.len()).unwrap_or(u32::MAX);
    }
    if alcance.ingresos_proveedor {
        sincronizacion::recibir_ingresos_proveedor_abiertos(conexion, contexto)?;
    }
    if alcance.gafetes_provisionales {
        sincronizacion::recibir_prestamos_gafete_provisional_abiertos(conexion, contexto)?;
        sincronizacion::recibir_devoluciones_propias_gafete_provisional(conexion, contexto)?;
    }
    if alcance.catalogo {
        resumen.catalogo = sincronizacion::recibir_catalogo_del_sitio(conexion, contexto)?;
        // Veto por persona: viaja con el catálogo (misma etapa) pero con su
        // propia marca de agua.
        sincronizacion::recibir_personas_vetadas(conexion, contexto)?;
    }
    if alcance.catalogo_rutas {
        resumen.catalogo_rutas =
            sincronizacion::recibir_catalogo_rutas_del_sitio(conexion, contexto)?;
    }
    if alcance.ingresos && historiales {
        resumen.movimientos_historial_recibidos =
            sincronizacion::recibir_historial_del_sitio(conexion, contexto)?;
    }
    if alcance.citas {
        resumen.citas_recibidas = sincronizacion::recibir_citas_del_sitio(conexion, contexto)?;
    }
    if alcance.visitas && historiales {
        resumen.historial_visitas_recibidos =
            sincronizacion::recibir_historial_visitas_del_sitio(conexion, contexto)?;
    }
    if alcance.ingresos_proveedor && historiales {
        resumen.historial_ingresos_proveedor_recibidos =
            sincronizacion::recibir_historial_ingresos_proveedor_del_sitio(conexion, contexto)?;
    }
    if alcance.gafetes_provisionales && historiales {
        resumen.historial_gafetes_provisionales_recibidos =
            sincronizacion::recibir_historial_gafetes_provisionales_del_sitio(conexion, contexto)?;
    }

    if alcance.ingresos {
        resumen.conflictos_ingreso =
            sincronizacion::contratistas_con_conflicto_activo(conexion, contexto)
                .unwrap_or_default();
    }
    if alcance.visitas && historiales {
        resumen.conflictos_movimiento_visita =
            sincronizacion::visitantes_con_conflicto_activo(conexion, contexto).unwrap_or_default();
    }
    if alcance.ingresos_proveedor {
        resumen.conflictos_ingreso_proveedor =
            sincronizacion::proveedores_con_conflicto_activo(conexion, contexto)
                .unwrap_or_default();
    }

    Ok(resumen)
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use super::*;
    use crate::database::schema::initialize_database;

    /// Servidor HTTP falso: contesta `[]` a todo y anota la línea de cada
    /// pedido ("GET /rest/v1/tabla?..."), para saber qué etapas corrieron.
    fn servidor_que_anota() -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind en localhost");
        let direccion = listener.local_addr().expect("dirección local");
        let pedidos = Arc::new(Mutex::new(Vec::new()));
        let anotados = Arc::clone(&pedidos);
        thread::spawn(move || {
            for conexion in listener.incoming() {
                let Ok(mut conexion) = conexion else { return };
                let mut lector = BufReader::new(conexion.try_clone().expect("clonar socket"));
                let mut primera = String::new();
                if lector.read_line(&mut primera).is_err() {
                    continue;
                }
                let mut largo_cuerpo = 0;
                loop {
                    let mut linea = String::new();
                    if lector.read_line(&mut linea).is_err() || linea == "\r\n" || linea.is_empty()
                    {
                        break;
                    }
                    if let Some((nombre, valor)) = linea.split_once(':')
                        && nombre.eq_ignore_ascii_case("content-length")
                    {
                        largo_cuerpo = valor.trim().parse().unwrap_or(0);
                    }
                }
                let mut cuerpo = vec![0; largo_cuerpo];
                let _ = lector.read_exact(&mut cuerpo);
                anotados.lock().unwrap().push(primera.trim().to_string());
                let _ = conexion.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                      Content-Length: 2\r\nConnection: close\r\n\r\n[]",
                );
            }
        });
        (format!("http://{direccion}"), pedidos)
    }

    fn correr(
        alcance: AlcanceSincronizacion,
        perfil: PerfilDispositivo,
    ) -> (ResumenSincronizacionNube, Vec<String>) {
        let conexion = Connection::open_in_memory().unwrap();
        initialize_database(&conexion).unwrap();
        let (base_url, pedidos) = servidor_que_anota();
        let contexto = ContextoSincronizacion {
            base_url: &base_url,
            apikey: "apikey",
            token: "token",
            dispositivo_id: "11111111-1111-1111-1111-111111111111",
            sitio_id: "22222222-2222-2222-2222-222222222222",
        };
        let resumen = sincronizar(&conexion, &contexto, alcance, perfil).unwrap();
        let pedidos = pedidos.lock().unwrap().clone();
        (resumen, pedidos)
    }

    fn pidio(pedidos: &[String], recurso: &str) -> bool {
        pedidos.iter().any(|pedido| pedido.contains(recurso))
    }

    #[test]
    fn el_alcance_completo_en_escritorio_corre_todas_las_etapas() {
        let (resumen, pedidos) = correr(
            AlcanceSincronizacion::completo(),
            PerfilDispositivo::Escritorio,
        );

        assert_eq!(resumen, ResumenSincronizacionNube::default());
        for recurso in [
            "/ingresos",
            "/ingresos_proveedor",
            "/prestamos_gafete_provisional",
            "/empresas",
            "/contratistas",
            "/vehiculos_ruta",
            "/encargados_ruta",
            "/citas",
            "/movimientos_visita",
        ] {
            assert!(pidio(&pedidos, recurso), "no pidió {recurso}: {pedidos:#?}");
        }
    }

    #[test]
    fn el_movil_no_trae_ningun_historial() {
        let (_, escritorio) = correr(
            AlcanceSincronizacion::completo(),
            PerfilDispositivo::Escritorio,
        );
        let (_, movil) = correr(AlcanceSincronizacion::completo(), PerfilDispositivo::Movil);

        assert!(movil.len() < escritorio.len(), "{movil:#?}");
        assert!(!pidio(&movil, "/movimientos_visita"), "{movil:#?}");
        // La consulta del historial de ingresos es la única que pide
        // `resultado_acceso`.
        assert!(pidio(&escritorio, "resultado_acceso"), "{escritorio:#?}");
        assert!(!pidio(&movil, "resultado_acceso"), "{movil:#?}");
        assert!(pidio(&movil, "/ingresos") && pidio(&movil, "/citas"));
    }

    #[test]
    fn solo_catalogo_no_toca_ingresos_ni_citas() {
        let (_, pedidos) = correr(
            AlcanceSincronizacion::solo_catalogo(),
            PerfilDispositivo::Escritorio,
        );

        assert!(pidio(&pedidos, "/empresas") && pidio(&pedidos, "/contratistas"));
        for recurso in [
            "/ingresos",
            "/citas",
            "/vehiculos_ruta",
            "/movimientos_visita",
        ] {
            assert!(!pidio(&pedidos, recurso), "pidió {recurso}: {pedidos:#?}");
        }
    }

    #[test]
    fn catalogo_y_rutas_suma_solo_las_rutas() {
        let (_, pedidos) = correr(
            AlcanceSincronizacion::catalogo_y_rutas(),
            PerfilDispositivo::Movil,
        );

        assert!(pidio(&pedidos, "/vehiculos_ruta") && pidio(&pedidos, "/encargados_ruta"));
        assert!(!pidio(&pedidos, "/ingresos") && !pidio(&pedidos, "/citas"));
    }
}
