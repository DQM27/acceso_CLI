//! Telemetría técnica del build de diagnóstico del escritorio: el mismo
//! esquema que la app Android (`Telemetria.kt`, ver
//! `mobile/android/docs/telemetria-diagnostico.md`) y la misma tabla
//! `telemetria_diagnostico` del proyecto de STAGING. Documentada en
//! `desktop/docs/telemetria-diagnostico.md`.
//!
//! Sólo existe con la feature `telemetria` (`cargo tauri build --features
//! telemetria`): sin ella `activa()` es `false`, `iniciar` no hace nada y
//! cada punto de medición es un chequeo de un booleano. Producción nunca
//! recibe telemetría: la dirección de staging está fija acá.
//!
//! Qué se manda: sólo números y nombres técnicos (comandos, pantallas,
//! tipos de error, tiempos, memoria). Nunca cédulas, nombres, placas,
//! contenido de formularios ni mensajes de error (pueden traer datos).
//!
//! Cómo: cola en memoria (con tope) que un hilo propio manda en lotes cada
//! minuto; lo que no se pudo mandar se guarda en un archivo y se reintenta
//! en el próximo arranque. Un hilo más toma una muestra del proceso cada
//! 30 s.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// Proyecto de STAGING (sandbox): la telemetría nunca va a producción. La
/// llave es la publicable (rol `anon`), que en esta tabla sólo puede
/// insertar (ver `supabase/scripts/telemetria_diagnostico_staging.sql`).
#[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
const URL_STAGING: &str = "https://pmrytjktlyiuikxuuxpr.supabase.co";
#[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
const APIKEY_STAGING: &str = "sb_publishable_29DwMvfyj8Jq--LBcqxtBA_pTwWrDH4";

/// Tope de la cola: sin red por mucho tiempo, lo más viejo se descarta
/// (y se cuenta) en vez de crecer sin límite.
const MAXIMO_EN_COLA: usize = 5_000;
#[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
const FILAS_POR_LOTE: usize = 200;
#[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
const SEGUNDOS_ENVIO: u64 = 60;
#[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
const SEGUNDOS_MUESTREO: u64 = 30;
/// Cuánto puede demorar el cierre de la app por mandar la telemetría.
const ESPERA_ENVIO_AL_CERRAR: std::time::Duration = std::time::Duration::from_secs(3);
/// Largo máximo de un evento del frontend, en bytes de JSON.
const MAXIMO_BYTES_EVENTO: usize = 16 * 1024;

const ARCHIVO_DISPOSITIVO: &str = "telemetria_dispositivo";
const ARCHIVO_PENDIENTE: &str = "telemetria_pendiente.jsonl";
/// Existe mientras la app está abierta; si al arrancar sigue ahí, la vez
/// anterior terminó sin pasar por el cierre normal (cuelgue, crash, "Finalizar
/// tarea", apagón).
const ARCHIVO_SESION_ABIERTA: &str = "telemetria_sesion_abierta";

/// ¿Esta compilación tiene telemetría?
pub const fn activa() -> bool {
    cfg!(feature = "telemetria")
}

/// Cola de filas JSON ya armadas, con tope.
#[derive(Debug, Default)]
struct Cola {
    filas: VecDeque<String>,
    descartadas: u64,
}

impl Cola {
    fn agregar(&mut self, fila: String) {
        if self.filas.len() >= MAXIMO_EN_COLA {
            self.filas.pop_front();
            self.descartadas += 1;
        }
        self.filas.push_back(fila);
    }

    #[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
    fn tomar(&mut self, cuantas: usize) -> Vec<String> {
        let cuantas = cuantas.min(self.filas.len());
        self.filas.drain(..cuantas).collect()
    }

    /// Devuelve un lote que no se pudo mandar, adelante y en orden.
    #[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
    fn devolver(&mut self, lote: Vec<String>) {
        for fila in lote.into_iter().rev() {
            self.filas.push_front(fila);
        }
        while self.filas.len() > MAXIMO_EN_COLA {
            self.filas.pop_front();
            self.descartadas += 1;
        }
    }
}

struct Telemetria {
    dispositivo: String,
    sesion: String,
    version: String,
    directorio: PathBuf,
    inicio: Instant,
    cola: Mutex<Cola>,
}

static TELEMETRIA: OnceLock<Telemetria> = OnceLock::new();

/// Una fila de `telemetria_diagnostico`, como la arma la app Android.
fn fila(t: &Telemetria, tipo: &str, datos: serde_json::Value) -> String {
    serde_json::json!({
        "ocurrido_en": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "dispositivo": t.dispositivo,
        "sesion": t.sesion,
        "version_app": t.version,
        "tipo": tipo,
        "datos": datos,
    })
    .to_string()
}

/// ¿`tipo` es un nombre técnico razonable (minúsculas, dígitos y `_`)?
/// Filtra lo que manda el frontend.
fn tipo_valido(tipo: &str) -> bool {
    (1..=40).contains(&tipo.len())
        && tipo
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Encola un evento. Sin telemetría (o antes de `iniciar`) no hace nada.
pub fn evento(tipo: &str, datos: serde_json::Value) {
    let Some(t) = TELEMETRIA.get() else {
        return;
    };
    let fila = fila(t, tipo, datos);
    t.cola
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .agregar(fila);
}

/// Milisegundos desde que arrancó el proceso (desde `iniciar`).
pub fn ms_desde_inicio() -> Option<u128> {
    TELEMETRIA.get().map(|t| t.inicio.elapsed().as_millis())
}

/// Evento que manda el frontend (ver `desktop/src/telemetria.ts`).
#[derive(Debug, serde::Deserialize)]
pub struct EventoFrontend {
    tipo: String,
    datos: serde_json::Value,
}

/// ¿Esta compilación manda telemetría? El frontend sólo mide si es así.
#[tauri::command]
pub const fn telemetria_activa() -> bool {
    activa()
}

/// Encola eventos armados por el frontend (comandos, Realtime, pantallas,
/// errores). Descarta en silencio los que no tienen forma válida: es sólo
/// diagnóstico, nunca debe fallar una operación por esto.
#[tauri::command]
pub fn telemetria_eventos(eventos: Vec<EventoFrontend>) {
    if TELEMETRIA.get().is_none() {
        return;
    }
    for EventoFrontend { tipo, datos } in eventos {
        if tipo_valido(&tipo) && datos.to_string().len() <= MAXIMO_BYTES_EVENTO {
            evento(&tipo, datos);
        }
    }
}

/// Lee (o crea) el identificador aleatorio de esta instalación: no es el
/// de la máquina ni el del dispositivo en la nube.
fn identificador_de_instalacion(directorio: &Path) -> String {
    let ruta = directorio.join(ARCHIVO_DISPOSITIVO);
    if let Ok(texto) = std::fs::read_to_string(&ruta) {
        let texto = texto.trim();
        if !texto.is_empty() {
            return texto.to_owned();
        }
    }
    let nuevo = control_acceso::database::identificador::generar_uuid_v4();
    let _ = std::fs::write(&ruta, &nuevo);
    nuevo
}

/// ¿La vez anterior se cerró bien? `None` si es el primer arranque con
/// telemetría. Deja la marca de sesión abierta para la próxima.
fn revisar_cierre_anterior(directorio: &Path) -> Option<bool> {
    let marca = directorio.join(ARCHIVO_SESION_ABIERTA);
    let ya_habia_arrancado = directorio.join(ARCHIVO_DISPOSITIVO).exists();
    let quedo_abierta = marca.exists();
    let _ = std::fs::write(&marca, chrono::Utc::now().to_rfc3339());
    ya_habia_arrancado.then_some(!quedo_abierta)
}

fn cargar_pendientes(t: &Telemetria) {
    let ruta = t.directorio.join(ARCHIVO_PENDIENTE);
    if let Ok(texto) = std::fs::read_to_string(&ruta) {
        let mut cola = t
            .cola
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        texto
            .lines()
            .filter(|linea| !linea.trim().is_empty())
            .for_each(|linea| cola.agregar(linea.to_owned()));
    }
    let _ = std::fs::remove_file(ruta);
}

/// Arranca la telemetría (sólo con la feature `telemetria`): identifica la
/// instalación y la sesión, informa cómo terminó la vez anterior y lanza
/// los hilos de envío y de muestreo. `directorio` es donde vive la base de
/// datos local.
pub fn iniciar(directorio: &Path) {
    if !activa() || TELEMETRIA.get().is_some() {
        return;
    }
    let cierre_anterior_limpio = revisar_cierre_anterior(directorio);
    let t = Telemetria {
        dispositivo: identificador_de_instalacion(directorio),
        sesion: control_acceso::database::identificador::generar_uuid_v4(),
        version: format!(
            "desktop-{}-diag+{}",
            env!("CARGO_PKG_VERSION"),
            env!("BUILD_COMMIT_HASH")
        ),
        directorio: directorio.to_path_buf(),
        inicio: Instant::now(),
        cola: Mutex::new(Cola::default()),
    };
    cargar_pendientes(&t);
    if TELEMETRIA.set(t).is_err() {
        return;
    }
    evento(
        "sesion_inicio",
        serde_json::json!({
            "plataforma": "escritorio",
            "so": std::env::consts::OS,
            "arquitectura": std::env::consts::ARCH,
            "nucleos_cpu": std::thread::available_parallelism().map(std::num::NonZero::get).ok(),
            "ram_total_mb": sistema::ram_total_mb(),
            "build": if cfg!(debug_assertions) { "debug" } else { "diagnostico" },
        }),
    );
    if let Some(limpio) = cierre_anterior_limpio {
        evento(
            "salida_anterior",
            serde_json::json!({
                "motivo_nombre": if limpio { "CIERRE_NORMAL" } else { "SIN_CIERRE_NORMAL" },
            }),
        );
    }
    instalar_gancho_de_panic();
    envio::lanzar_hilos();
}

/// Anota el panic (sólo dónde ocurrió, nunca el mensaje) y guarda la cola
/// en el archivo de pendientes antes de seguir con el gancho anterior
/// (Sentry). No manda nada por la red acá: el panic puede ocurrir dentro de
/// un hilo de tokio, donde el cliente HTTP bloqueante entra en pánico a su
/// vez. Lo guardado sale en el próximo arranque.
fn instalar_gancho_de_panic() {
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let donde = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()));
        let hilo = std::thread::current().name().map(str::to_owned);
        // `try_lock`: si el panic ocurrió con la cola tomada, esperarla
        // colgaría la app en vez de dejarla terminar.
        if let Some(t) = TELEMETRIA.get()
            && let Ok(mut cola) = t.cola.try_lock()
        {
            cola.agregar(fila(
                t,
                "panic",
                serde_json::json!({ "donde": donde, "hilo": hilo }),
            ));
            let todo = cola.filas.iter().cloned().collect::<Vec<_>>().join("\n");
            drop(cola);
            let _ = std::fs::write(t.directorio.join(ARCHIVO_PENDIENTE), todo);
        }
        anterior(info);
    }));
}

/// Escribe TODA la cola en el archivo de pendientes (sin vaciarla): lo que
/// no llegue a mandarse sale en el próximo arranque.
fn guardar_pendientes(t: &Telemetria) {
    let todo = t
        .cola
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .filas
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(t.directorio.join(ARCHIVO_PENDIENTE), todo);
}

/// Cierre normal de la app: borra la marca de sesión abierta, guarda la
/// cola y la intenta mandar (desde un hilo propio, nunca desde el de
/// quien llama, y esperando como mucho `ESPERA_ENVIO_AL_CERRAR`).
pub fn cerrar() {
    let Some(t) = TELEMETRIA.get() else {
        return;
    };
    evento(
        "cierre",
        serde_json::json!({ "minutos_abierta": t.inicio.elapsed().as_secs() / 60 }),
    );
    let _ = std::fs::remove_file(t.directorio.join(ARCHIVO_SESION_ABIERTA));
    guardar_pendientes(t);
    let (listo, esperar) = std::sync::mpsc::channel();
    let lanzado = std::thread::Builder::new()
        .name("telemetria-cierre".into())
        .spawn(move || {
            envio::enviar_todo();
            let _ = listo.send(());
        });
    let termino = lanzado.is_ok() && esperar.recv_timeout(ESPERA_ENVIO_AL_CERRAR).is_ok();
    let vacia = t
        .cola
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .filas
        .is_empty();
    if termino && vacia {
        let _ = std::fs::remove_file(t.directorio.join(ARCHIVO_PENDIENTE));
    }
}

#[cfg(feature = "telemetria")]
mod envio {
    use std::time::Duration;

    use super::{
        APIKEY_STAGING, FILAS_POR_LOTE, SEGUNDOS_ENVIO, SEGUNDOS_MUESTREO, TELEMETRIA, URL_STAGING,
        evento, guardar_pendientes, sistema,
    };

    pub fn lanzar_hilos() {
        let _ = std::thread::Builder::new()
            .name("telemetria-envio".into())
            .spawn(|| {
                loop {
                    std::thread::sleep(Duration::from_secs(SEGUNDOS_ENVIO));
                    enviar_todo();
                }
            });
        let _ = std::thread::Builder::new()
            .name("telemetria-muestreo".into())
            .spawn(|| {
                let mut muestreo = sistema::Muestreo::default();
                loop {
                    std::thread::sleep(Duration::from_secs(SEGUNDOS_MUESTREO));
                    evento("muestra_sistema", muestreo.tomar());
                }
            });
    }

    /// Manda la cola en lotes; si un lote falla, lo devuelve y guarda todo
    /// lo pendiente en el archivo para el próximo arranque. Sólo desde
    /// hilos propios (fuera de tokio): usa el cliente HTTP bloqueante.
    pub fn enviar_todo() {
        let Some(t) = TELEMETRIA.get() else {
            return;
        };
        let cliente = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
        {
            Ok(cliente) => cliente,
            Err(error) => {
                log::warn!("telemetría: no se pudo crear el cliente HTTP: {error}");
                return;
            }
        };
        let descartadas = std::mem::take(
            &mut t
                .cola
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .descartadas,
        );
        if descartadas > 0 {
            evento(
                "telemetria_descartada",
                serde_json::json!({ "filas": descartadas }),
            );
        }
        loop {
            let lote = t
                .cola
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .tomar(FILAS_POR_LOTE);
            if lote.is_empty() {
                return;
            }
            let cuerpo = format!("[{}]", lote.join(","));
            let enviado = cliente
                .post(format!("{URL_STAGING}/rest/v1/telemetria_diagnostico"))
                .header("apikey", APIKEY_STAGING)
                .header("Content-Type", "application/json")
                .header("Prefer", "return=minimal")
                .body(cuerpo)
                .send()
                .is_ok_and(|respuesta| respuesta.status().is_success());
            if !enviado {
                t.cola
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .devolver(lote);
                guardar_pendientes(t);
                return;
            }
        }
    }
}

#[cfg(not(feature = "telemetria"))]
mod envio {
    pub const fn lanzar_hilos() {}
    pub const fn enviar_todo() {}
}

/// Datos del proceso y del equipo.
mod sistema {
    /// Muestra periódica: memoria y CPU del proceso (sin contar los procesos
    /// de `WebView2`, que son aparte) y memoria del equipo.
    #[derive(Default)]
    #[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
    pub struct Muestreo {
        anterior: Option<(std::time::Instant, u64)>,
    }

    #[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
    impl Muestreo {
        pub fn tomar(&mut self) -> serde_json::Value {
            let ahora = std::time::Instant::now();
            let cpu_ms = cpu_proceso_ms();
            // Mismo criterio que el teléfono: `un_nucleo` puede pasar de 100
            // si el proceso usa varios núcleos; `equipo` es sobre todos.
            let un_nucleo = match (self.anterior, cpu_ms) {
                (Some((antes, cpu_antes)), Some(cpu)) => {
                    let pared_ms = ahora.duration_since(antes).as_millis();
                    let usado = u128::from(cpu.saturating_sub(cpu_antes));
                    (pared_ms > 0).then(|| usado * 100 / pared_ms)
                }
                _ => None,
            };
            let nucleos = std::thread::available_parallelism().map_or(1, |n| {
                u128::from(u32::try_from(n.get()).unwrap_or(u32::MAX))
            });
            if let Some(cpu) = cpu_ms {
                self.anterior = Some((ahora, cpu));
            }
            let memoria = memoria_proceso();
            let cola = super::TELEMETRIA.get().map(|t| {
                t.cola
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .filas
                    .len()
            });
            serde_json::json!({
                "memoria_proceso_mb": memoria.map(|m| m.0),
                "memoria_privada_mb": memoria.map(|m| m.1),
                "pico_memoria_mb": memoria.map(|m| m.2),
                "cpu_un_nucleo_pct": un_nucleo,
                "cpu_dispositivo_pct": un_nucleo.map(|pct| pct / nucleos),
                "sistema_disponible_mb": ram_libre_mb(),
                "cola_telemetria": cola,
            })
        }
    }

    #[cfg(windows)]
    mod windows_api {
        use windows::Win32::Foundation::{FILETIME, HANDLE};
        use windows::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
        };
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

        const MB: u64 = 1024 * 1024;

        fn proceso_actual() -> HANDLE {
            // SAFETY: sólo devuelve el pseudo-handle constante del proceso
            // actual; no hay nada que cerrar ni puntero que validar.
            unsafe { GetCurrentProcess() }
        }

        /// (en uso, privada, pico) en MB.
        pub fn memoria_proceso() -> Option<(u64, u64, u64)> {
            let mut contadores = PROCESS_MEMORY_COUNTERS_EX {
                cb: u32::try_from(std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>()).ok()?,
                ..Default::default()
            };
            let puntero = (&raw mut contadores).cast::<PROCESS_MEMORY_COUNTERS>();
            let proceso = proceso_actual();
            // SAFETY: `proceso` es el pseudo-handle del proceso actual;
            // `puntero` apunta a una estructura EX propia, viva durante la
            // llamada, con `cb` fijado a su tamaño real: la API sólo la llena.
            unsafe { GetProcessMemoryInfo(proceso, puntero, contadores.cb) }.ok()?;
            Some((
                contadores.WorkingSetSize as u64 / MB,
                contadores.PrivateUsage as u64 / MB,
                contadores.PeakWorkingSetSize as u64 / MB,
            ))
        }

        /// CPU usada por el proceso (usuario + núcleo), en ms.
        pub fn cpu_proceso_ms() -> Option<u64> {
            let mut creacion = FILETIME::default();
            let mut salida = FILETIME::default();
            let mut nucleo = FILETIME::default();
            let mut usuario = FILETIME::default();
            let proceso = proceso_actual();
            // SAFETY: pseudo-handle del proceso actual y cuatro FILETIME
            // propios, vivos durante la llamada, que la API sólo escribe.
            unsafe {
                GetProcessTimes(
                    proceso,
                    &raw mut creacion,
                    &raw mut salida,
                    &raw mut nucleo,
                    &raw mut usuario,
                )
            }
            .ok()?;
            let a_ms = |f: FILETIME| {
                ((u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime)) / 10_000
            };
            Some(a_ms(nucleo) + a_ms(usuario))
        }

        fn estado_memoria() -> Option<MEMORYSTATUSEX> {
            let mut estado = MEMORYSTATUSEX {
                dwLength: u32::try_from(std::mem::size_of::<MEMORYSTATUSEX>()).ok()?,
                ..Default::default()
            };
            // SAFETY: estructura propia con `dwLength` fijado a su tamaño.
            unsafe { GlobalMemoryStatusEx(&raw mut estado) }.ok()?;
            Some(estado)
        }

        pub fn ram_total_mb() -> Option<u64> {
            estado_memoria().map(|e| e.ullTotalPhys / MB)
        }

        pub fn ram_libre_mb() -> Option<u64> {
            estado_memoria().map(|e| e.ullAvailPhys / MB)
        }
    }

    #[cfg(windows)]
    pub use windows_api::{cpu_proceso_ms, memoria_proceso, ram_libre_mb, ram_total_mb};

    // Fuera de Windows (desarrollo en Linux/macOS) no hay estas mediciones.
    #[cfg(not(windows))]
    #[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
    pub const fn memoria_proceso() -> Option<(u64, u64, u64)> {
        None
    }
    #[cfg(not(windows))]
    #[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
    pub const fn cpu_proceso_ms() -> Option<u64> {
        None
    }
    #[cfg(not(windows))]
    pub const fn ram_total_mb() -> Option<u64> {
        None
    }
    #[cfg(not(windows))]
    #[cfg_attr(not(feature = "telemetria"), allow(dead_code))]
    pub const fn ram_libre_mb() -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_cola_descarta_lo_mas_viejo_y_devuelve_en_orden() {
        let mut cola = Cola::default();
        for i in 0..MAXIMO_EN_COLA + 2 {
            cola.agregar(i.to_string());
        }
        assert_eq!(cola.descartadas, 2);
        assert_eq!(cola.filas.front().map(String::as_str), Some("2"));

        let lote = cola.tomar(3);
        assert_eq!(lote, ["2", "3", "4"]);
        cola.devolver(lote);
        assert_eq!(cola.tomar(3), ["2", "3", "4"], "vuelve adelante y en orden");
    }

    #[test]
    fn solo_se_aceptan_tipos_tecnicos() {
        assert!(tipo_valido("llamada_comando"));
        assert!(tipo_valido("realtime"));
        assert!(!tipo_valido(""));
        assert!(!tipo_valido("Cedula 1-2345"));
        assert!(!tipo_valido(&"a".repeat(41)));
    }

    #[test]
    fn detecta_si_la_vez_anterior_no_se_cerro_bien() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(revisar_cierre_anterior(dir.path()), None, "primer arranque");
        identificador_de_instalacion(dir.path());
        assert_eq!(
            revisar_cierre_anterior(dir.path()),
            Some(false),
            "la marca de la vez anterior seguía ahí"
        );
        std::fs::remove_file(dir.path().join(ARCHIVO_SESION_ABIERTA)).unwrap();
        assert_eq!(revisar_cierre_anterior(dir.path()), Some(true));
    }

    #[test]
    fn el_identificador_de_instalacion_se_conserva() {
        let dir = tempfile::tempdir().unwrap();
        let primero = identificador_de_instalacion(dir.path());
        assert_eq!(identificador_de_instalacion(dir.path()), primero);
        assert_eq!(primero.len(), 36);
    }
}
