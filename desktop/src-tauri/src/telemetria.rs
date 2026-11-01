//! Telemetría técnica del escritorio, con el mismo catálogo que la app
//! Android. Va a PRODUCCIÓN, al esquema `observabilidad`, por la receptora
//! `observabilidad` (ver
//! `docs/features-futuras/plan-observabilidad-en-produccion.md`). Documentada
//! en `desktop/docs/telemetria-diagnostico.md`.
//!
//! Un solo código para todos los builds, sin flags: cada evento pertenece a
//! un grupo (`control_acceso::telemetria::grupo_de_evento`) y sólo se mide si
//! la orden vigente de este equipo lo permite
//! (`control_acceso::telemetria::config`). `base` va siempre, `rendimiento`
//! por defecto y el resto lo enciende el panel, con vencimiento (ver
//! `docs/features-futuras/plan-telemetria-y-registro-de-errores.md`).
//!
//! Identidad: el token del propio equipo (el mismo de la operación). La nube
//! pone el equipo y la unidad desde el token; nada de eso viaja en el lote.
//!
//! Qué se manda: sólo números y nombres técnicos (comandos, pantallas,
//! tipos de error, tiempos, memoria). Nunca cédulas, nombres, placas,
//! contenido de formularios ni mensajes de error (pueden traer datos).
//!
//! Cómo: cola en memoria (con tope) que un hilo propio pasa cada minuto a la
//! cola en disco del núcleo (`control_acceso::telemetria::cola`) y manda con
//! `telemetria::envio`: lote columnar, brotli, HTTP/2 y el token del equipo.
//! Lo que no sale queda en disco, sin pérdidas ni duplicados, y se reintenta.
//! Un hilo más toma una muestra del proceso cada 2 minutos.

use std::cell::Cell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use control_acceso::telemetria::cola::ColaTelemetria;
use control_acceso::telemetria::lote::EventoTelemetria;
use control_acceso::telemetria::{Grupo, config, grupo_de_evento};

/// Proyecto de STAGING: sólo para saber en qué ambiente corre este build
/// (`ambiente` de la sesión y del registro de errores).
pub const URL_STAGING: &str = "https://pmrytjktlyiuikxuuxpr.supabase.co";

/// Tope de la cola en memoria entre dos pasadas a disco (un minuto): lo más
/// viejo se descarta (y se cuenta) en vez de crecer sin límite.
const MAXIMO_EN_COLA: usize = 5_000;
const SEGUNDOS_ENVIO: u64 = 60;
/// La memoria y el CPU de una PC en reposo cambian despacio: con 30 s la muestra
/// era más de la mitad de las filas de un equipo parado.
const SEGUNDOS_MUESTREO: u64 = 120;
/// Cuánto puede demorar el cierre de la app por mandar la telemetría.
const ESPERA_ENVIO_AL_CERRAR: std::time::Duration = std::time::Duration::from_secs(3);
/// Largo máximo de un evento del frontend, en bytes de JSON.
const MAXIMO_BYTES_EVENTO: usize = 16 * 1024;

const ARCHIVO_DISPOSITIVO: &str = "telemetria_dispositivo";
/// La cola del formato anterior (filas para staging). Se borra al arrancar:
/// el formato no sirve para la receptora nueva.
const ARCHIVO_PENDIENTE_ANTERIOR: &str = "telemetria_pendiente.jsonl";
/// Existe mientras la app está abierta; si al arrancar sigue ahí, la vez
/// anterior terminó sin pasar por el cierre normal (cuelgue, crash, "Finalizar
/// tarea", apagón).
const ARCHIVO_SESION_ABIERTA: &str = "telemetria_sesion_abierta";

/// ¿Se mide este tipo de evento ahora? Lo decide la orden vigente de este
/// equipo; un tipo que no está en el catálogo no se mide (se asigna a un grupo
/// en `control_acceso::telemetria::grupos` antes de usarse).
pub fn permite(tipo: &str) -> bool {
    grupo_de_evento(tipo).is_some_and(|grupo| config().permite(grupo, ahora().timestamp()))
}

/// La versión de la orden que ya se informó en `orden_telemetria` (`-1` =
/// ninguna todavía).
static VERSION_INFORMADA: AtomicI64 = AtomicI64::new(-1);

/// Informa la versión de la orden aplicada y los grupos vigentes, la primera
/// vez y cada vez que cambia, para que el panel confirme que la orden llegó.
fn informar_orden_si_cambio() {
    let version = config().version();
    if VERSION_INFORMADA.swap(version, Ordering::Relaxed) != version {
        evento(
            "orden_telemetria",
            serde_json::json!({ "version": version, "grupos": grupos_vigentes() }),
        );
    }
}

/// Los nombres de los grupos que rigen ahora, `base` incluido.
fn grupos_vigentes() -> Vec<&'static str> {
    let vigentes = config().grupos_vigentes(ahora().timestamp());
    Grupo::TODOS
        .into_iter()
        .filter(|g| vigentes.contiene(*g))
        .map(Grupo::nombre)
        .collect()
}

/// Eventos medidos desde la última pasada a disco, con tope. Encolar es sólo
/// tomar un candado y empujar: lo caro (disco, red) lo hace el hilo de envío.
#[derive(Debug, Default)]
struct Cola {
    filas: VecDeque<EventoTelemetria>,
    descartadas: u64,
}

impl Cola {
    fn agregar(&mut self, evento: EventoTelemetria) {
        if self.filas.len() >= MAXIMO_EN_COLA {
            self.filas.pop_front();
            self.descartadas += 1;
        }
        self.filas.push_back(evento);
    }

    fn tomar_todo(&mut self) -> Vec<EventoTelemetria> {
        self.filas.drain(..).collect()
    }
}

struct Telemetria {
    /// La cabecera de la sesión que viaja en cada lote (ver [`cabecera_sesion`]).
    sesion: serde_json::Value,
    directorio: PathBuf,
    inicio: Instant,
    cola: Mutex<Cola>,
    disco: ColaTelemetria,
}

/// Token del equipo para mandar: lo da el estado de la app (la misma caché de
/// la operación). `None` = todavía no hay (equipo sin vincular, sin red).
type ProveedorToken = Box<dyn Fn() -> Option<String> + Send + Sync>;

static TOKEN: OnceLock<ProveedorToken> = OnceLock::new();

/// Fija de dónde sale el token. Una sola vez, al arrancar la app.
pub fn usar_token(proveedor: ProveedorToken) {
    let _ = TOKEN.set(proveedor);
}

static TELEMETRIA: OnceLock<Telemetria> = OnceLock::new();

/// El reloj del núcleo (hora del servidor + contador de arranque, ver
/// `control_acceso::tiempo::RelojCorregido`): los eventos se sellan con la
/// misma hora que los registros, no con la de Windows. Sin él (antes de
/// abrir el núcleo), la del sistema.
static RELOJ: OnceLock<std::sync::Arc<dyn control_acceso::tiempo::Reloj>> = OnceLock::new();

/// Fija el reloj con que se sellan los eventos. Una sola vez, al abrir el
/// núcleo.
pub fn usar_reloj(reloj: std::sync::Arc<dyn control_acceso::tiempo::Reloj>) {
    let _ = RELOJ.set(reloj);
}

fn ahora() -> chrono::DateTime<chrono::Utc> {
    RELOJ
        .get()
        .map_or_else(chrono::Utc::now, |reloj| reloj.ahora_utc())
}

/// Un evento con la hora del reloj del núcleo. `esquema` y `plataforma` ya no
/// van en cada evento: viajan una vez, en la cabecera de la sesión. Un `datos`
/// que no es un objeto se guarda como `{"valor": ...}` para no perderlo.
fn evento_de(tipo: &str, datos: serde_json::Value) -> EventoTelemetria {
    let datos = match datos {
        serde_json::Value::Object(mut objeto) => {
            objeto.remove("esquema");
            objeto.remove("plataforma");
            objeto
        }
        otro => serde_json::Map::from_iter([("valor".to_owned(), otro)]),
    };
    EventoTelemetria {
        ocurrido_ms: ahora().timestamp_millis(),
        tipo: tipo.to_owned(),
        datos,
    }
}

/// La cabecera de la sesión de esta app, que la nube guarda una vez
/// (`observabilidad.sesiones`). La versión va en semver y el commit aparte;
/// un commit que no es un hash (`sinhash`) no se manda.
fn cabecera_sesion(sesion_id: &str, instalacion: &str) -> serde_json::Value {
    let commit = env!("BUILD_COMMIT_HASH");
    let commit_valido =
        (7..=40).contains(&commit.len()) && commit.chars().all(|c| c.is_ascii_hexdigit());
    let ambiente = if control_acceso::nube::base_url() == URL_STAGING {
        "staging"
    } else {
        "produccion"
    };
    serde_json::json!({
        "id": sesion_id,
        "plataforma": "escritorio",
        "version_app": env!("CARGO_PKG_VERSION"),
        "commit": commit_valido.then_some(commit),
        "ambiente": ambiente,
        "iniciada_en": ahora().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "sistema": std::env::consts::OS,
        "ram_total_mb": sistema::ram_total_mb(),
        "detalle": {
            "arquitectura": std::env::consts::ARCH,
            "nucleos_cpu": std::thread::available_parallelism().map(std::num::NonZero::get).ok(),
            "build": if cfg!(debug_assertions) { "debug" } else { "release" },
            // Una reinstalación cambia este número: sirve para verla.
            "instalacion": instalacion,
        },
    })
}

/// ¿`tipo` es un nombre técnico razonable (minúsculas, dígitos y `_`)?
/// Filtra lo que manda el frontend.
fn tipo_valido(tipo: &str) -> bool {
    (1..=40).contains(&tipo.len())
        && tipo
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Encola un evento. Si la orden vigente no lo permite (o antes de `iniciar`)
/// no hace nada.
pub fn evento(tipo: &str, datos: serde_json::Value) {
    if !permite(tipo) {
        return;
    }
    let Some(t) = TELEMETRIA.get() else {
        return;
    };
    t.cola
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .agregar(evento_de(tipo, datos));
}

/// Toma el candado del núcleo (`tomar`) y suma a `acumulado` lo que esperó
/// para conseguirlo. Sólo lo usan los comandos que se miden por dentro.
pub fn medir_espera<T>(acumulado: &Cell<Duration>, tomar: impl FnOnce() -> T) -> T {
    let inicio = Instant::now();
    let tomado = tomar();
    acumulado.set(acumulado.get() + inicio.elapsed());
    tomado
}

/// Milisegundos con un decimal.
fn ms(duracion: Duration) -> f64 {
    (duracion.as_secs_f64() * 10_000.0).round() / 10.0
}

/// `transicion_ingreso` del escritorio, con los campos del teléfono
/// (`resultado`, `ms_nucleo`, `ms_total`) más lo que el escritorio puede
/// separar: `ms_cola` (esperando un hilo del pool) y `ms_espera_candado`
/// (esperando `state.core()`). `ms_nucleo` es lo que queda de `ms_total`
/// sin la espera del candado: reglas locales y consulta a la nube.
/// `ms_total` no incluye el IPC: ese tiempo sale en `llamada_nucleo`.
pub fn transicion_ingreso(
    resultado: &str,
    cola: Duration,
    espera_candado: Duration,
    total: Duration,
) -> serde_json::Value {
    serde_json::json!({
        "resultado": resultado,
        "ms_cola": ms(cola),
        "ms_espera_candado": ms(espera_candado),
        "ms_nucleo": ms(total.saturating_sub(espera_candado)),
        "ms_total": ms(total),
    })
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

/// Los grupos que rigen ahora. El frontend sólo mide los de esta lista y la
/// vuelve a pedir cada tanto: una orden puede llegar o vencer con la app
/// abierta.
#[tauri::command]
pub fn telemetria_grupos() -> Vec<&'static str> {
    grupos_vigentes()
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

/// Lo que `version_app` dice de este build: `desktop-1.8.12+abc1234`.
pub fn version_app() -> String {
    format!(
        "desktop-{}+{}",
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_COMMIT_HASH")
    )
}

/// Pone en marcha la telemetría. Arranca siempre: lo que se mide lo decide la
/// orden vigente de este equipo (`permite`). `directorio` es donde vive la base
/// de datos local.
pub fn iniciar(directorio: &Path) {
    arrancar(directorio);
}

/// Arranca la telemetría una sola vez: identifica la instalación y la
/// sesión, informa cómo terminó la vez anterior y lanza los hilos de envío
/// y de muestreo.
fn arrancar(directorio: &Path) {
    if TELEMETRIA.get().is_some() {
        return;
    }
    let cierre_anterior_limpio = revisar_cierre_anterior(directorio);
    let instalacion = identificador_de_instalacion(directorio);
    let _ = std::fs::remove_file(directorio.join(ARCHIVO_PENDIENTE_ANTERIOR));
    let t = Telemetria {
        sesion: cabecera_sesion(
            &control_acceso::database::identificador::generar_uuid_v4(),
            &instalacion,
        ),
        directorio: directorio.to_path_buf(),
        inicio: Instant::now(),
        cola: Mutex::new(Cola::default()),
        disco: ColaTelemetria::en(directorio),
    };
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
            "build": if cfg!(debug_assertions) { "debug" } else { "release" },
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
    informar_orden_si_cambio();
    instalar_gancho_de_panic();
    envio::lanzar_hilos();
}

/// Anota el panic (sólo dónde ocurrió, nunca el mensaje) y pasa la cola a
/// disco antes de seguir con el gancho anterior (el del registro de errores,
/// que sí guarda el mensaje y la traza). No manda nada por la red acá: el panic
/// puede ocurrir dentro de un hilo de tokio, donde el cliente HTTP bloqueante
/// entra en pánico a su vez. Lo guardado sale en el próximo envío.
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
            cola.agregar(evento_de(
                "panic",
                serde_json::json!({ "donde": donde, "hilo": hilo }),
            ));
            let todo = cola.tomar_todo();
            drop(cola);
            let _ = t.disco.agregar(&todo);
        }
        anterior(info);
    }));
}

/// Pasa lo medido a la cola en disco del núcleo (de ahí sale, y ahí sobrevive
/// a un cierre). Informa lo que se descartó por los topes.
fn pasar_a_disco(t: &Telemetria) {
    let (eventos, descartadas_memoria) = {
        let mut cola = t
            .cola
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (cola.tomar_todo(), std::mem::take(&mut cola.descartadas))
    };
    let descartadas_disco = match t.disco.agregar(&eventos) {
        Ok(descartados) => descartados as u64,
        Err(error) => {
            log::warn!("telemetría: no se pudo guardar la cola en disco: {error}");
            eventos.len() as u64
        }
    };
    let descartadas = descartadas_memoria + descartadas_disco;
    if descartadas > 0 {
        // Va a la cola en memoria: sale en la próxima pasada.
        evento(
            "telemetria_descartada",
            serde_json::json!({ "filas": descartadas }),
        );
    }
}

/// ¿No hay nada esperando para mandarse (ni en memoria ni en disco)?
fn cola_vacia() -> bool {
    TELEMETRIA.get().is_none_or(|t| {
        t.cola
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .filas
            .is_empty()
            && !t.disco.hay_pendientes()
    })
}

/// Cierre normal de la app: borra la marca de sesión abierta, pasa la cola a
/// disco y la intenta mandar (desde un hilo propio, nunca desde el de quien
/// llama, y esperando como mucho `ESPERA_ENVIO_AL_CERRAR`). Lo que no salga
/// queda en disco para el próximo arranque.
pub fn cerrar() {
    let Some(t) = TELEMETRIA.get() else {
        return;
    };
    evento(
        "cierre",
        serde_json::json!({ "minutos_abierta": t.inicio.elapsed().as_secs() / 60 }),
    );
    let _ = std::fs::remove_file(t.directorio.join(ARCHIVO_SESION_ABIERTA));
    pasar_a_disco(t);
    // Sin nada pendiente: ni petición.
    if cola_vacia() {
        return;
    }
    let (listo, esperar) = std::sync::mpsc::channel();
    let lanzado = std::thread::Builder::new()
        .name("telemetria-cierre".into())
        .spawn(move || {
            envio::enviar_todo();
            let _ = listo.send(());
        });
    if lanzado.is_ok() {
        let _ = esperar.recv_timeout(ESPERA_ENVIO_AL_CERRAR);
    }
}

mod envio {
    use std::time::Duration;

    use super::{
        SEGUNDOS_ENVIO, SEGUNDOS_MUESTREO, TELEMETRIA, TOKEN, cola_vacia, evento,
        informar_orden_si_cambio, pasar_a_disco, permite, sistema,
    };

    pub fn lanzar_hilos() {
        let _ = std::thread::Builder::new()
            .name("telemetria-envio".into())
            .spawn(|| {
                loop {
                    std::thread::sleep(Duration::from_secs(SEGUNDOS_ENVIO));
                    // Una orden nueva pudo llegar por el aviso en vivo o la
                    // sincronización.
                    informar_orden_si_cambio();
                    // Sin cola no se arma ni el cliente HTTP.
                    if !cola_vacia() {
                        enviar_todo();
                    }
                }
            });
        let _ = std::thread::Builder::new()
            .name("telemetria-muestreo".into())
            .spawn(|| {
                let mut muestreo = sistema::Muestreo::default();
                loop {
                    std::thread::sleep(Duration::from_secs(SEGUNDOS_MUESTREO));
                    if permite("muestra_sistema") {
                        evento("muestra_sistema", muestreo.tomar());
                    }
                }
            });
    }

    /// Pasa lo medido a disco y manda la cola del núcleo hasta vaciarla o hasta
    /// que la nube no pueda recibir más (lo que quede se reintenta en la
    /// próxima vuelta, con el mismo `lote_id`). Sólo desde hilos propios
    /// (fuera de tokio): usa el cliente HTTP bloqueante del núcleo, el mismo
    /// de la operación (HTTP/2, conexión ya abierta).
    pub fn enviar_todo() {
        let Some(t) = TELEMETRIA.get() else {
            return;
        };
        pasar_a_disco(t);
        if !t.disco.hay_pendientes() {
            return;
        }
        // Sin token (equipo sin vincular, sin red) queda todo en disco.
        let Some(token) = TOKEN.get().and_then(|proveedor| proveedor()) else {
            return;
        };
        let resumen = control_acceso::telemetria::envio::vaciar_cola(
            &t.disco,
            &t.sesion,
            &control_acceso::nube::cliente_http(),
            control_acceso::nube::base_url(),
            control_acceso::nube::apikey(),
            &token,
            control_acceso::database::identificador::generar_uuid_v4,
        );
        if resumen.descartadas > 0 {
            evento(
                "telemetria_descartada",
                serde_json::json!({ "filas": resumen.descartadas }),
            );
        }
        if let Some(motivo) = resumen.pendiente {
            log::debug!("telemetría: quedó pendiente ({motivo})");
        }
    }
}

/// Datos del proceso y del equipo.
mod sistema {
    /// Muestra periódica: memoria y CPU del proceso (sin contar los procesos
    /// de `WebView2`, que son aparte) y memoria del equipo.
    #[derive(Default)]
    pub struct Muestreo {
        anterior: Option<(std::time::Instant, u64)>,
    }

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
    pub const fn memoria_proceso() -> Option<(u64, u64, u64)> {
        None
    }
    #[cfg(not(windows))]
    pub const fn cpu_proceso_ms() -> Option<u64> {
        None
    }
    #[cfg(not(windows))]
    pub const fn ram_total_mb() -> Option<u64> {
        None
    }
    #[cfg(not(windows))]
    pub const fn ram_libre_mb() -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_cola_descarta_lo_mas_viejo_y_entrega_en_orden() {
        let mut cola = Cola::default();
        for i in 0..MAXIMO_EN_COLA + 2 {
            cola.agregar(EventoTelemetria {
                ocurrido_ms: i64::try_from(i).unwrap(),
                tipo: "reloj".to_owned(),
                datos: serde_json::Map::new(),
            });
        }
        assert_eq!(cola.descartadas, 2);

        let todo = cola.tomar_todo();
        assert_eq!(todo.len(), MAXIMO_EN_COLA);
        let horas: Vec<i64> = todo.iter().take(3).map(|e| e.ocurrido_ms).collect();
        assert_eq!(horas, [2, 3, 4], "sin los dos más viejos, en orden");
        assert!(cola.tomar_todo().is_empty(), "queda vacía");
    }

    #[test]
    fn la_transicion_de_ingreso_separa_el_candado_del_resto() {
        let datos = transicion_ingreso(
            "formulario",
            Duration::from_millis(3),
            Duration::from_millis(40),
            Duration::from_millis(1500),
        );
        assert_eq!(datos["resultado"], "formulario");
        assert_eq!(datos["ms_cola"], 3.0);
        assert_eq!(datos["ms_espera_candado"], 40.0);
        assert_eq!(datos["ms_nucleo"], 1460.0);
        assert_eq!(datos["ms_total"], 1500.0);
    }

    #[test]
    fn medir_espera_acumula_lo_que_tardo_en_tomar_el_candado() {
        let acumulado = Cell::new(Duration::ZERO);
        let valor = medir_espera(&acumulado, || {
            std::thread::sleep(Duration::from_millis(20));
            7
        });
        assert_eq!(valor, 7);
        assert!(acumulado.get() >= Duration::from_millis(20));
        medir_espera(&acumulado, || ());
        assert!(
            acumulado.get() >= Duration::from_millis(20),
            "acumula, no reemplaza"
        );
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
    fn el_evento_no_repite_esquema_ni_plataforma() {
        let evento = evento_de(
            "llamada_nucleo",
            serde_json::json!({ "nombre": "login", "esquema": 2, "plataforma": "escritorio" }),
        );
        assert_eq!(evento.tipo, "llamada_nucleo");
        assert_eq!(
            serde_json::Value::Object(evento.datos),
            serde_json::json!({ "nombre": "login" })
        );
        assert!(
            evento.ocurrido_ms > 1_700_000_000_000,
            "hora en ms desde la época"
        );
    }

    #[test]
    fn un_dato_que_no_es_objeto_no_se_pierde() {
        let evento = evento_de("reloj", serde_json::json!(5));
        assert_eq!(
            serde_json::Value::Object(evento.datos),
            serde_json::json!({ "valor": 5 })
        );
    }

    #[test]
    fn la_cabecera_de_la_sesion_va_en_semver_y_sin_hash_falso() {
        let cabecera = cabecera_sesion("dddddddd-0000-0000-0000-000000000001", "instalacion-1");
        assert_eq!(cabecera["id"], "dddddddd-0000-0000-0000-000000000001");
        assert_eq!(cabecera["plataforma"], "escritorio");
        let version = cabecera["version_app"].as_str().unwrap();
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
        assert_eq!(version.split('.').count(), 3, "semver, como exige la nube");
        assert!(!version.contains("desktop") && !version.contains('+'));
        match cabecera["commit"].as_str() {
            Some(commit) => {
                assert!(commit.chars().all(|c| c.is_ascii_hexdigit()) && commit.len() >= 7);
            }
            None => assert_eq!(env!("BUILD_COMMIT_HASH"), "sinhash"),
        }
        assert!(["produccion", "staging"].contains(&cabecera["ambiente"].as_str().unwrap()));
        assert_eq!(cabecera["detalle"]["instalacion"], "instalacion-1");
    }

    #[test]
    fn la_cola_en_memoria_respeta_el_tope_y_cuenta_lo_descartado() {
        let mut cola = Cola::default();
        for _ in 0..MAXIMO_EN_COLA + 3 {
            cola.agregar(evento_de("reloj", serde_json::json!({})));
        }
        assert_eq!(cola.filas.len(), MAXIMO_EN_COLA);
        assert_eq!(cola.descartadas, 3);
        assert_eq!(cola.tomar_todo().len(), MAXIMO_EN_COLA);
        assert!(cola.filas.is_empty());
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
