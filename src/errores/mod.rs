//! Registro de errores propio, sin el SDK de Sentry (fase 4 de
//! `docs/features-futuras/plan-telemetria-y-registro-de-errores.md`). Una
//! sola implementación para el escritorio y el núcleo del móvil.
//!
//! - Siempre encendido: no depende de la orden de telemetría ni se apaga
//!   desde internet.
//! - Cada error se escribe a disco en el momento ([`cola`]) y un hilo propio
//!   lo manda al receptor (`supabase/functions-staging/receptor-errores`)
//!   cada minuto ([`envio`]). Sin destino configurado, quedan en disco.
//! - Lleva la plataforma, la versión, la traza (marcando el código propio),
//!   la pantalla actual y las últimas 20 operaciones ([`rastro`]): nombre,
//!   resultado y hora, nunca datos. Todo el texto pasa por
//!   [`limpiar_texto`], y el receptor limpia otra vez.
//! - Orígenes: pánicos de Rust ([`panico`]), errores inesperados del núcleo
//!   (`mensajes::mensaje_generico`) y lo que mandan las plataformas
//!   (excepciones de Kotlin, ANR, fallos nativos, errores de JavaScript).
//! - Un mismo error no se reporta más de 3 veces por sesión, y una sesión no
//!   reporta más de 100: un error en un bucle no inunda nada.

mod cola;
mod destino;
mod envio;
mod formato;
mod limpieza;
mod panico;
mod rastro;
mod traza;

use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;

pub use destino::Destino;
pub use formato::{ESQUEMA, Marco, Operacion, Origen, Reporte};
pub use limpieza::{limpiar_ruta, limpiar_texto};
pub use traza::{hilo_principal_de_anr, marcos_de_backtrace, marcos_de_texto};

use cola::Cola;
use rastro::Rastro;

const LARGO_MAXIMO_TITULO: usize = 300;
const LARGO_MAXIMO_PANTALLA: usize = 60;
const MAXIMO_POR_TITULO: u32 = 3;
const MAXIMO_POR_SESION: u32 = 100;
/// Antes del primer envío, para no competir con el arranque.
const ESPERA_INICIAL_ENVIO: Duration = Duration::from_secs(15);
const INTERVALO_ENVIO: Duration = Duration::from_secs(60);

/// Lo que cada plataforma dice de sí misma al instalar el registro.
#[derive(Debug, Clone)]
pub struct Configuracion {
    /// Donde vive `errores_pendientes/` (el directorio de datos de la app).
    pub directorio: PathBuf,
    /// `escritorio` o `movil`.
    pub plataforma: String,
    pub version_app: String,
    /// El DSN de siempre (ver [`Destino::desde_dsn`]); vacío = sin envío.
    pub dsn: String,
    /// Prefijos del código propio en las trazas (`control_acceso`,
    /// `com.brisas.controlacceso`…).
    pub propios: Vec<String>,
    /// Sistema operativo, modelo, arquitectura… Sólo datos del equipo.
    pub contexto: BTreeMap<String, String>,
}

/// Cuántas veces se reportó cada error en esta sesión.
#[derive(Debug, Default)]
struct Limitador {
    por_titulo: HashMap<String, u32>,
    total: u32,
}

impl Limitador {
    /// ¿Se reporta este error? Los títulos se comparan sin números, que
    /// cambian de una vez a otra (ids, índices, tiempos).
    fn permite(&mut self, titulo: &str) -> bool {
        if self.total >= MAXIMO_POR_SESION {
            return false;
        }
        let clave: String = titulo
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_digit() { '#' } else { c })
            .collect();
        let veces = self.por_titulo.entry(clave).or_default();
        if *veces >= MAXIMO_POR_TITULO {
            return false;
        }
        *veces += 1;
        self.total += 1;
        true
    }
}

/// El candado, o `None` si no se puede tomar sin esperar (en un pánico:
/// esperar podría colgar el proceso si el mismo hilo ya lo tenía).
fn tomar<T>(candado: &Mutex<T>, esperar: bool) -> Option<MutexGuard<'_, T>> {
    if esperar {
        Some(
            candado
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    } else {
        candado.try_lock().ok()
    }
}

fn ahora() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[derive(Debug)]
struct Registro {
    plataforma: String,
    version_app: String,
    propios: Vec<String>,
    contexto: BTreeMap<String, String>,
    cola: Cola,
    destino: Option<Destino>,
    rastro: Mutex<Rastro>,
    pantalla: Mutex<Option<String>>,
    limitador: Mutex<Limitador>,
}

impl Registro {
    fn nuevo(configuracion: Configuracion) -> Self {
        Self {
            destino: Destino::desde_dsn(&configuracion.dsn),
            cola: Cola::en(&configuracion.directorio),
            plataforma: configuracion.plataforma,
            version_app: configuracion.version_app,
            propios: configuracion.propios,
            contexto: configuracion
                .contexto
                .into_iter()
                .map(|(clave, valor)| (clave, limpiar_texto(&valor)))
                .collect(),
            rastro: Mutex::default(),
            pantalla: Mutex::default(),
            limitador: Mutex::default(),
        }
    }

    fn propios(&self) -> Vec<&str> {
        self.propios.iter().map(String::as_str).collect()
    }

    fn anotar_operacion(&self, nombre: &str, ok: Option<bool>) {
        if let Some(mut rastro) = tomar(&self.rastro, true) {
            rastro.anotar(nombre, ok, ahora());
        }
    }

    fn fijar_pantalla(&self, nombre: &str) {
        let nombre = limpiar_texto(nombre.trim());
        let nombre =
            (!nombre.is_empty()).then(|| nombre.chars().take(LARGO_MAXIMO_PANTALLA).collect());
        if let Some(mut pantalla) = tomar(&self.pantalla, true) {
            *pantalla = nombre;
        }
    }

    /// Arma el reporte y lo guarda en disco. `traza` se calcula sólo si el
    /// limitador deja pasar el error (capturar una traza cuesta).
    fn reportar_con(
        &self,
        origen: Origen,
        titulo: &str,
        traza: impl FnOnce() -> Vec<Marco>,
        esperar: bool,
    ) -> bool {
        let titulo: String = limpiar_texto(titulo.trim())
            .chars()
            .take(LARGO_MAXIMO_TITULO)
            .collect();
        let titulo = if titulo.is_empty() {
            "(sin mensaje)".to_owned()
        } else {
            titulo
        };
        // Sin poder tomar el limitador (sólo en un pánico) se reporta igual.
        if !tomar(&self.limitador, esperar).is_none_or(|mut limitador| limitador.permite(&titulo)) {
            return false;
        }
        let reporte = Reporte {
            esquema: ESQUEMA,
            id: crate::database::identificador::generar_uuid_v4(),
            ocurrido_en: ahora(),
            plataforma: self.plataforma.clone(),
            version_app: self.version_app.clone(),
            origen: origen.nombre().to_owned(),
            titulo,
            traza: traza(),
            pantalla: tomar(&self.pantalla, esperar).and_then(|p| p.clone()),
            operaciones: tomar(&self.rastro, esperar)
                .map(|r| r.copia())
                .unwrap_or_default(),
            contexto: self.contexto.clone(),
        };
        match self.cola.guardar(&reporte) {
            Ok(()) => true,
            Err(error) => {
                log::warn!("no se pudo guardar un error para reportarlo: {error}");
                false
            }
        }
    }

    fn reportar(&self, origen: Origen, titulo: &str, traza: Vec<Marco>) -> bool {
        self.reportar_con(origen, titulo, || traza, true)
    }

    fn reportar_sin_bloquear(
        &self,
        origen: Origen,
        titulo: &str,
        traza: impl FnOnce() -> Vec<Marco>,
    ) -> bool {
        self.reportar_con(origen, titulo, traza, false)
    }

    fn reportar_texto(&self, origen: Origen, titulo: &str, traza: &str) -> bool {
        let traza = if origen == Origen::Anr {
            hilo_principal_de_anr(traza)
        } else {
            traza
        };
        self.reportar_con(
            origen,
            titulo,
            || marcos_de_texto(traza, &self.propios()),
            true,
        )
    }

    fn reportar_inesperado(&self, error: &dyn Display) -> bool {
        self.reportar_con(
            Origen::Nucleo,
            &error.to_string(),
            || {
                marcos_de_backtrace(
                    &std::backtrace::Backtrace::force_capture().to_string(),
                    &self.propios(),
                )
            },
            true,
        )
    }

    fn enviar(&self) -> Result<usize, String> {
        self.destino
            .as_ref()
            .map_or(Ok(0), |destino| envio::enviar(destino, &self.cola))
    }
}

static REGISTRO: OnceLock<Registro> = OnceLock::new();

/// Pone en marcha el registro: gancho de pánicos y, si hay destino, el hilo
/// de envío. Una sola vez por proceso (devuelve `false` si ya estaba).
pub fn instalar(configuracion: Configuracion) -> bool {
    if REGISTRO.set(Registro::nuevo(configuracion)).is_err() {
        return false;
    }
    panico::instalar_gancho();
    if REGISTRO
        .get()
        .is_some_and(|registro| registro.destino.is_some())
    {
        let _ = std::thread::Builder::new()
            .name("errores-envio".into())
            .spawn(|| {
                std::thread::sleep(ESPERA_INICIAL_ENVIO);
                loop {
                    if let Err(error) = enviar_pendientes() {
                        log::warn!("no se pudieron mandar los errores pendientes: {error}");
                    }
                    std::thread::sleep(INTERVALO_ENVIO);
                }
            });
    }
    true
}

pub fn instalado() -> bool {
    REGISTRO.get().is_some()
}

/// Anota una operación en el rastro que acompaña al próximo error.
pub fn anotar_operacion(nombre: &str, ok: Option<bool>) {
    if let Some(registro) = REGISTRO.get() {
        registro.anotar_operacion(nombre, ok);
    }
}

/// La pantalla que está viendo quien opera (vacío = ninguna).
pub fn fijar_pantalla(nombre: &str) {
    if let Some(registro) = REGISTRO.get() {
        registro.fijar_pantalla(nombre);
    }
}

/// Reporta un error con su traza ya armada.
pub fn reportar(origen: Origen, titulo: &str, traza: Vec<Marco>) -> bool {
    REGISTRO
        .get()
        .is_some_and(|registro| registro.reportar(origen, titulo, traza))
}

/// Reporta un error cuya traza llega como texto (Kotlin, JavaScript, volcado
/// de ANR; ver [`marcos_de_texto`]).
pub fn reportar_texto(origen: Origen, titulo: &str, traza: &str) -> bool {
    REGISTRO
        .get()
        .is_some_and(|registro| registro.reportar_texto(origen, titulo, traza))
}

/// Un error inesperado del núcleo, con la traza de quien lo produjo. Lo
/// llama `mensajes::mensaje_generico`.
pub fn reportar_inesperado(error: &dyn Display) -> bool {
    REGISTRO
        .get()
        .is_some_and(|registro| registro.reportar_inesperado(error))
}

/// Manda ya lo pendiente (bloqueante). Lo hace solo el hilo de envío; las
/// plataformas lo pueden pedir antes de cerrar.
pub fn enviar_pendientes() -> Result<usize, String> {
    REGISTRO.get().map_or(Ok(0), Registro::enviar)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fmt;

    use super::{Configuracion, Limitador, MAXIMO_POR_SESION, Origen, Registro, Reporte};

    fn registro(dir: &std::path::Path) -> Registro {
        Registro::nuevo(Configuracion {
            directorio: dir.to_path_buf(),
            plataforma: "movil".into(),
            version_app: "1.8.12 (57)".into(),
            dsn: String::new(),
            propios: vec!["com.brisas.controlacceso".into(), "control_acceso".into()],
            contexto: BTreeMap::from([("modelo".into(), "SM-A256E".into())]),
        })
    }

    fn guardados(registro: &Registro) -> Vec<Reporte> {
        registro
            .cola
            .pendientes()
            .iter()
            .map(|ruta| serde_json::from_slice(&std::fs::read(ruta).unwrap()).unwrap())
            .collect()
    }

    #[test]
    fn el_reporte_lleva_traza_pantalla_operaciones_y_contexto_limpios() {
        let dir = tempfile::tempdir().unwrap();
        let registro = registro(dir.path());
        registro.fijar_pantalla("Ingreso");
        registro.anotar_operacion("buscar_contratista", Some(true));
        registro.anotar_operacion("registrar_ingreso", Some(false));

        assert!(registro.reportar_texto(
            Origen::Excepcion,
            "java.lang.IllegalStateException: cédula 1-0123-0456 inválida",
            "java.lang.IllegalStateException\n\tat com.brisas.controlacceso.Ingreso.guardar(Ingreso.kt:88)",
        ));

        let [reporte] = guardados(&registro).try_into().unwrap();
        assert_eq!(reporte.origen, "excepcion");
        assert_eq!(reporte.plataforma, "movil");
        assert_eq!(reporte.version_app, "1.8.12 (57)");
        assert_eq!(
            reporte.titulo,
            "java.lang.IllegalStateException: cédula <num> inválida"
        );
        assert_eq!(reporte.traza.len(), 1);
        assert!(reporte.traza[0].propio);
        assert_eq!(reporte.pantalla.as_deref(), Some("Ingreso"));
        let operaciones: Vec<_> = reporte
            .operaciones
            .iter()
            .map(|o| (o.nombre.as_str(), o.ok))
            .collect();
        assert_eq!(
            operaciones,
            [
                ("buscar_contratista", Some(true)),
                ("registrar_ingreso", Some(false))
            ]
        );
        assert_eq!(reporte.contexto["modelo"], "SM-A256E");
        assert_eq!(reporte.id.len(), 36);
    }

    #[test]
    fn el_mismo_error_se_reporta_tres_veces_por_sesion() {
        let dir = tempfile::tempdir().unwrap();
        let registro = registro(dir.path());
        for i in 0..5 {
            registro.reportar(Origen::Js, &format!("TypeError en la fila {i}"), Vec::new());
        }
        registro.reportar(Origen::Js, "Otro error", Vec::new());
        assert_eq!(guardados(&registro).len(), 4, "3 del repetido y 1 del otro");
    }

    #[test]
    fn una_sesion_no_reporta_mas_de_cien() {
        let mut limitador = Limitador::default();
        let permitidos = (0..150)
            .filter(|i| limitador.permite(&format!("error {}", "x".repeat(*i))))
            .count();
        assert_eq!(permitidos, usize::try_from(MAXIMO_POR_SESION).unwrap());
    }

    #[test]
    fn un_error_inesperado_del_nucleo_lleva_su_titulo() {
        struct Fallo;
        impl fmt::Display for Fallo {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("database is locked")
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let registro = registro(dir.path());
        assert!(registro.reportar_inesperado(&Fallo));
        let [reporte] = guardados(&registro).try_into().unwrap();
        assert_eq!(reporte.origen, "nucleo");
        assert_eq!(reporte.titulo, "database is locked");
    }

    #[test]
    fn sin_titulo_se_dice_que_no_hubo_mensaje() {
        let dir = tempfile::tempdir().unwrap();
        let registro = registro(dir.path());
        assert!(registro.reportar(Origen::Nativo, "   ", Vec::new()));
        assert_eq!(guardados(&registro)[0].titulo, "(sin mensaje)");
    }

    #[test]
    fn sin_destino_no_se_manda_nada() {
        let dir = tempfile::tempdir().unwrap();
        let registro = registro(dir.path());
        registro.reportar(Origen::Js, "x", Vec::new());
        assert_eq!(registro.enviar(), Ok(0));
        assert_eq!(guardados(&registro).len(), 1, "queda en disco");
    }
}
