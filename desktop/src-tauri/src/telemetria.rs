//! Telemetría técnica del escritorio: el mismo esquema que la app Android
//! (`Telemetria.kt`, ver `mobile/android/docs/telemetria-diagnostico.md`) y
//! la misma tabla `telemetria_diagnostico` del proyecto de STAGING.
//! Documentada en `desktop/docs/telemetria-diagnostico.md`.
//!
//! El código de medición va en todos los builds, pero APAGADO: `activa()` es
//! `false` y cada punto de medición es un chequeo de un booleano. Se
//! enciende de dos maneras:
//! - el build de diagnóstico (`cargo tauri build --features telemetria`)
//!   está siempre encendido y, además, se redirige a staging con base y
//!   claves propias (`aislar_build_de_diagnostico`);
//! - en un release normal, sólo mientras haya una regla vigente en staging
//!   para esta instalación (`supabase/scripts/telemetria_reglas_staging.sql`).
//!   Sin regla, o sin red, no se mide ni se manda nada.
//!
//! La app nunca manda datos de operación a staging, sólo telemetría: la
//! dirección de staging está fija acá. Producción no recibe telemetría.
//!
//! Qué se manda: sólo números y nombres técnicos (comandos, pantallas,
//! tipos de error, tiempos, memoria). Nunca cédulas, nombres, placas,
//! contenido de formularios ni mensajes de error (pueden traer datos).
//!
//! Cómo: cola en memoria (con tope) que un hilo propio manda en lotes cada
//! minuto; lo que no se pudo mandar se guarda en un archivo y se reintenta
//! en el próximo arranque. Un hilo más toma una muestra del proceso cada
//! 30 s.

use std::cell::Cell;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Proyecto de STAGING (sandbox): la telemetría nunca va a producción. La
/// llave es la publicable (rol `anon`), que en esta tabla sólo puede
/// insertar (ver `supabase/scripts/telemetria_diagnostico_staging.sql`).
const URL_STAGING: &str = "https://pmrytjktlyiuikxuuxpr.supabase.co";
const APIKEY_STAGING: &str = "sb_publishable_29DwMvfyj8Jq--LBcqxtBA_pTwWrDH4";

/// El build de diagnóstico es un build de PRUEBA, igual que el
/// `diagnostico` de Android (paquete `.diag`, base propia, staging): nunca
/// debe tocar la nube de producción ni la base o las claves de la app real
/// si las dos conviven en la misma PC. Apunta a staging y usa carpetas
/// propias, salvo que ya se hayan elegido a mano (`scripts/activar_sandbox.ps1`).
///
/// Se llama como primera instrucción de `run()`, antes de que exista
/// ningún otro hilo (Sentry, Tauri): modificar el entorno es seguro recién
/// ahí.
#[cfg(feature = "telemetria")]
pub fn aislar_build_de_diagnostico() {
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let mut variables: Vec<(&str, std::ffi::OsString)> = vec![
        ("CONTROL_ACCESO_SUPABASE_URL", URL_STAGING.into()),
        ("CONTROL_ACCESO_SUPABASE_APIKEY", APIKEY_STAGING.into()),
    ];
    if let Some(local) = local {
        variables.push((
            "CONTROL_ACCESO_DB",
            local
                .join("ControlAccesoDiagnostico")
                .join("control_acceso.db")
                .into_os_string(),
        ));
        // `%APPDATA%\ControlAcceso` guarda la clave de la base y la del
        // dispositivo (ver `control_acceso::nube::credenciales`).
        variables.push((
            "APPDATA",
            local
                .join("ControlAccesoDiagnosticoRoaming")
                .into_os_string(),
        ));
    }
    let sandbox_manual = std::env::var_os("CONTROL_ACCESO_SUPABASE_URL").is_some();
    for (nombre, valor) in variables {
        if sandbox_manual && std::env::var_os(nombre).is_some() {
            continue;
        }
        // SAFETY: se llama al inicio de `run()`, con un solo hilo vivo
        // (ver el doc-comment); nadie más lee ni escribe el entorno.
        unsafe { std::env::set_var(nombre, valor) };
    }
}

/// Tope de la cola: sin red por mucho tiempo, lo más viejo se descarta
/// (y se cuenta) en vez de crecer sin límite.
const MAXIMO_EN_COLA: usize = 5_000;
const FILAS_POR_LOTE: usize = 200;
const SEGUNDOS_ENVIO: u64 = 60;
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

/// Mínimo entre dos consultas de reglas a staging, aunque se pida revisar
/// varias veces (por ejemplo, varios logins seguidos). No hay sondeo: sólo se
/// consulta al abrir la app y tras un login exitoso.
const SEGUNDOS_MINIMOS_ENTRE_CONSULTAS: u64 = 300;
/// Cuánto espera la consulta de reglas antes de darse por vencida.
const SEGUNDOS_ESPERA_REGLAS: u64 = 10;

/// El build de diagnóstico mide siempre, sin reglas.
const SIEMPRE_ENCENDIDA: bool = cfg!(feature = "telemetria");

/// La regla vigente que está aplicando esta instalación, si hay.
static REGLAS: Mutex<Option<Reglas>> = Mutex::new(None);

/// Hasta cuándo rige la regla actual (segundos Unix); `0` = apagada. Es lo
/// que vigila el vencimiento sin consultar a nadie: cuando pasa la hora,
/// `activa()` da `false` sola.
static VENCE_UNIX: AtomicI64 = AtomicI64::new(0);

/// Dónde vive la base local (y los archivos de telemetría); lo fija
/// `iniciar` para que `revisar_reglas` se pueda llamar desde cualquier lado.
static DIRECTORIO: OnceLock<PathBuf> = OnceLock::new();

/// Cuándo se hizo la última consulta de reglas (haya salido bien o no).
static ULTIMA_CONSULTA: Mutex<Option<Instant>> = Mutex::new(None);

/// ¿Rige una regla que vence en `vence_unix` a las `ahora_unix`?
const fn regla_rige(vence_unix: i64, ahora_unix: i64) -> bool {
    vence_unix > ahora_unix
}

/// ¿Está midiendo ahora mismo? Falso en un release sin regla vigente, o con
/// la regla ya vencida.
pub fn activa() -> bool {
    SIEMPRE_ENCENDIDA
        || regla_rige(
            VENCE_UNIX.load(Ordering::Relaxed),
            chrono::Utc::now().timestamp(),
        )
}

/// ¿Toca consultar las reglas? Sí si nunca se consultó, o si pasó el mínimo
/// desde la última consulta.
fn toca_consultar(ultima: Option<Instant>, ahora: Instant) -> bool {
    ultima.is_none_or(|ultima| {
        ahora.saturating_duration_since(ultima)
            >= Duration::from_secs(SEGUNDOS_MINIMOS_ENTRE_CONSULTAS)
    })
}

/// Una regla tal como la devuelve `reglas_telemetria_vigentes` en staging.
#[derive(Debug, serde::Deserialize)]
struct ReglaRemota {
    /// `None` = todos los tipos de evento.
    eventos: Option<Vec<String>>,
    /// RFC 3339.
    vence_en: String,
}

/// Lo que piden, juntas, todas las reglas vigentes que le tocan a esta
/// instalación. El servidor ya filtró por equipo, plataforma y versión
/// (`reglas_telemetria_vigentes`); acá se combinan y se vigila el
/// vencimiento.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Reglas {
    /// Alguna regla pide todos los tipos de evento.
    todos: bool,
    /// Si ninguna los pide todos, la unión de los tipos pedidos.
    eventos: HashSet<String>,
    /// Cuando vence la última de las reglas.
    vence: chrono::DateTime<chrono::Utc>,
}

impl Reglas {
    /// Combina las reglas que siguen vigentes en `ahora`. `None` si no hay
    /// ninguna: la telemetría queda apagada. Una regla con fecha ilegible se
    /// ignora (nunca se enciende por una regla que no se entiende).
    fn combinar(remotas: &[ReglaRemota], ahora: chrono::DateTime<chrono::Utc>) -> Option<Self> {
        let mut combinadas: Option<Self> = None;
        for remota in remotas {
            let Ok(vence) = chrono::DateTime::parse_from_rfc3339(&remota.vence_en) else {
                continue;
            };
            let vence = vence.with_timezone(&chrono::Utc);
            if vence <= ahora {
                continue;
            }
            let actual = combinadas.get_or_insert_with(|| Self {
                todos: false,
                eventos: HashSet::new(),
                vence,
            });
            match &remota.eventos {
                None => actual.todos = true,
                Some(tipos) => actual.eventos.extend(tipos.iter().cloned()),
            }
            actual.vence = actual.vence.max(vence);
        }
        combinadas
    }

    /// ¿Esta regla deja pasar este tipo de evento?
    fn permite(&self, tipo: &str) -> bool {
        self.todos || self.eventos.contains(tipo)
    }

    fn vigente(&self, ahora: chrono::DateTime<chrono::Utc>) -> bool {
        self.vence > ahora
    }

    /// Lo que rige después de una consulta. `respuesta` es `None` si no hubo
    /// respuesta (sin red, staging caído): queda como estaba, mientras no
    /// haya vencido. Una respuesta sin reglas vigentes apaga de inmediato.
    fn siguiente(
        actual: Option<&Self>,
        respuesta: Option<&[ReglaRemota]>,
        ahora: chrono::DateTime<chrono::Utc>,
    ) -> Option<Self> {
        respuesta.map_or_else(
            || actual.filter(|regla| regla.vigente(ahora)).cloned(),
            |remotas| Self::combinar(remotas, ahora),
        )
    }
}

/// ¿Este tipo de evento está permitido? El build de diagnóstico deja pasar
/// todo; en un release, sólo lo que pide la regla vigente.
fn tipo_permitido(tipo: &str) -> bool {
    SIEMPRE_ENCENDIDA
        || REGLAS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .is_some_and(|reglas| reglas.permite(tipo))
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

    fn tomar(&mut self, cuantas: usize) -> Vec<String> {
        let cuantas = cuantas.min(self.filas.len());
        self.filas.drain(..cuantas).collect()
    }

    /// Devuelve un lote que no se pudo mandar, adelante y en orden.
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

/// Versión del formato de las filas (`datos.esquema`), la misma que
/// `ESQUEMA_TELEMETRIA` del teléfono. Ver `docs/telemetria-contrato.md`.
const ESQUEMA_TELEMETRIA: u32 = 2;

/// Agrega a `datos` la versión del formato y la plataforma, sin pisar lo que
/// el evento ya traiga. Un `datos` que no es un objeto se deja como está.
fn con_esquema_y_plataforma(mut datos: serde_json::Value) -> serde_json::Value {
    if let Some(objeto) = datos.as_object_mut() {
        objeto
            .entry("esquema")
            .or_insert_with(|| serde_json::json!(ESQUEMA_TELEMETRIA));
        objeto
            .entry("plataforma")
            .or_insert_with(|| serde_json::json!("escritorio"));
    }
    datos
}

/// Una fila de `telemetria_diagnostico`, como la arma la app Android.
fn fila(t: &Telemetria, tipo: &str, datos: serde_json::Value) -> String {
    serde_json::json!({
        "ocurrido_en": ahora().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "dispositivo": t.dispositivo,
        "sesion": t.sesion,
        "version_app": t.version,
        "tipo": tipo,
        "datos": con_esquema_y_plataforma(datos),
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
    if !activa() || !tipo_permitido(tipo) {
        return;
    }
    let Some(t) = TELEMETRIA.get() else {
        return;
    };
    let fila = fila(t, tipo, datos);
    t.cola
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .agregar(fila);
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

/// ¿Está midiendo ahora? El frontend sólo mide mientras sea así y lo vuelve
/// a preguntar cada tanto (una regla puede encenderla o apagarla).
#[tauri::command]
pub fn telemetria_activa() -> bool {
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

/// Lo que `version_app` dice de este build: `desktop-1.8.12+abc1234`, y
/// `desktop-1.8.12-diag+abc1234` en el de diagnóstico.
fn version_app() -> String {
    format!(
        "desktop-{}{}+{}",
        env!("CARGO_PKG_VERSION"),
        if SIEMPRE_ENCENDIDA { "-diag" } else { "" },
        env!("BUILD_COMMIT_HASH")
    )
}

/// Pone en marcha la telemetría. En el build de diagnóstico arranca ya. En
/// un release hace una consulta puntual a staging para saber si hay una regla
/// para esta instalación (`revisar_reglas`); sin regla vigente, o sin red,
/// no se mide ni se manda nada. `directorio` es donde vive la base de datos
/// local.
pub fn iniciar(directorio: &Path) {
    if SIEMPRE_ENCENDIDA {
        arrancar(directorio);
        return;
    }
    let _ = DIRECTORIO.set(directorio.to_path_buf());
    revisar_reglas();
}

/// Pregunta a staging si hay una regla para esta instalación, en segundo
/// plano: no bloquea quien llama (la ventana, el login) y el hilo termina
/// con la respuesta, o a los `SEGUNDOS_ESPERA_REGLAS`. Se llama al abrir la
/// app y tras cada login exitoso; no hay sondeo. Como mucho una consulta
/// cada `SEGUNDOS_MINIMOS_ENTRE_CONSULTAS`, aunque se llame más seguido. El
/// build de diagnóstico no consulta (mide siempre).
pub fn revisar_reglas() {
    if SIEMPRE_ENCENDIDA {
        return;
    }
    let Some(directorio) = DIRECTORIO.get() else {
        return;
    };
    {
        let mut ultima = ULTIMA_CONSULTA
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let ahora = Instant::now();
        if !toca_consultar(*ultima, ahora) {
            return;
        }
        *ultima = Some(ahora);
    }
    let directorio = directorio.clone();
    let _ = std::thread::Builder::new()
        .name("telemetria-consulta".into())
        .spawn(move || {
            let dispositivo = identificador_de_instalacion(&directorio);
            aplicar_reglas(&directorio, consultar_reglas(&dispositivo).ok().as_deref());
        });
}

/// Pregunta a staging qué reglas le tocan a esta instalación. Desde un hilo
/// propio (fuera de tokio): usa el cliente HTTP bloqueante.
fn consultar_reglas(dispositivo: &str) -> Result<Vec<ReglaRemota>, ()> {
    let cliente = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(SEGUNDOS_ESPERA_REGLAS))
        .build()
        .map_err(|_| ())?;
    let respuesta = cliente
        .post(format!(
            "{URL_STAGING}/rest/v1/rpc/reglas_telemetria_vigentes"
        ))
        .header("apikey", APIKEY_STAGING)
        .json(&serde_json::json!({
            "p_dispositivo": dispositivo,
            "p_plataforma": "escritorio",
            "p_version": version_app(),
        }))
        .send()
        .map_err(|_| ())?;
    if !respuesta.status().is_success() {
        return Err(());
    }
    respuesta.json::<Vec<ReglaRemota>>().map_err(|_| ())
}

/// Aplica lo que dicen las reglas (ver `Reglas::siguiente`): enciende
/// (arrancando la telemetría la primera vez), cambia lo que se permite o
/// apaga.
fn aplicar_reglas(directorio: &Path, respuesta: Option<&[ReglaRemota]>) {
    let mut vigentes = REGLAS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let nuevas = Reglas::siguiente(vigentes.as_ref(), respuesta, chrono::Utc::now());
    VENCE_UNIX.store(
        nuevas.as_ref().map_or(0, |regla| regla.vence.timestamp()),
        Ordering::Relaxed,
    );
    vigentes.clone_from(&nuevas);
    drop(vigentes);
    if nuevas.is_some() {
        arrancar(directorio);
    }
}

/// Arranca la telemetría una sola vez: identifica la instalación y la
/// sesión, informa cómo terminó la vez anterior y lanza los hilos de envío
/// y de muestreo.
fn arrancar(directorio: &Path) {
    if TELEMETRIA.get().is_some() {
        return;
    }
    let cierre_anterior_limpio = revisar_cierre_anterior(directorio);
    let t = Telemetria {
        dispositivo: identificador_de_instalacion(directorio),
        sesion: control_acceso::database::identificador::generar_uuid_v4(),
        version: version_app(),
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
            "build": if cfg!(debug_assertions) {
                "debug"
            } else if SIEMPRE_ENCENDIDA {
                "diagnostico"
            } else {
                "release"
            },
        }),
    );
    // En un release la telemetría no deja marca cuando está apagada, así que
    // "no había marca" no prueba un cierre normal: sólo se informa el caso
    // en que la marca sí quedó.
    if let Some(limpio) = cierre_anterior_limpio
        && (SIEMPRE_ENCENDIDA || !limpio)
    {
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

/// ¿No hay filas esperando para mandarse?
fn cola_vacia() -> bool {
    TELEMETRIA.get().is_none_or(|t| {
        t.cola
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .filas
            .is_empty()
    })
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
    // Apagada y sin nada pendiente: ni archivo ni petición.
    if !activa() && cola_vacia() {
        return;
    }
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

mod envio {
    use std::time::Duration;

    use super::{
        APIKEY_STAGING, FILAS_POR_LOTE, SEGUNDOS_ENVIO, SEGUNDOS_MUESTREO, TELEMETRIA, URL_STAGING,
        activa, cola_vacia, evento, guardar_pendientes, sistema,
    };

    pub fn lanzar_hilos() {
        let _ = std::thread::Builder::new()
            .name("telemetria-envio".into())
            .spawn(|| {
                loop {
                    std::thread::sleep(Duration::from_secs(SEGUNDOS_ENVIO));
                    // Apagada, sólo se vacía lo que quedó de antes: sin cola
                    // no se arma ni el cliente HTTP.
                    if activa() || !cola_vacia() {
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
                    if activa() {
                        evento("muestra_sistema", muestreo.tomar());
                    }
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

    fn regla(eventos: Option<&[&str]>, vence_en: &str) -> ReglaRemota {
        ReglaRemota {
            eventos: eventos.map(|tipos| tipos.iter().map(|t| (*t).to_owned()).collect()),
            vence_en: vence_en.to_owned(),
        }
    }

    fn hora(texto: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(texto)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    const AHORA: &str = "2026-10-17T12:00:00Z";

    #[test]
    fn sin_reglas_no_se_mide() {
        assert_eq!(Reglas::combinar(&[], hora(AHORA)), None);
    }

    #[test]
    fn una_regla_vencida_o_ilegible_no_enciende() {
        let remotas = [
            regla(None, "2026-10-17T11:59:59Z"),
            regla(None, "2026-10-17T12:00:00Z"),
            regla(None, "no es una fecha"),
        ];
        assert_eq!(Reglas::combinar(&remotas, hora(AHORA)), None);
    }

    #[test]
    fn una_regla_sin_lista_de_eventos_deja_pasar_todo() {
        let reglas =
            Reglas::combinar(&[regla(None, "2026-10-18T12:00:00+00:00")], hora(AHORA)).unwrap();
        assert!(reglas.permite("llamada_nucleo"));
        assert!(reglas.permite("cualquier_otro"));
    }

    #[test]
    fn una_regla_con_lista_solo_deja_pasar_esos_tipos() {
        let reglas = Reglas::combinar(
            &[regla(
                Some(&["transicion_ingreso", "llamada_nucleo"]),
                "2026-10-18T12:00:00Z",
            )],
            hora(AHORA),
        )
        .unwrap();
        assert!(reglas.permite("transicion_ingreso"));
        assert!(reglas.permite("llamada_nucleo"));
        assert!(!reglas.permite("realtime"));
    }

    #[test]
    fn varias_reglas_se_suman_y_vence_la_ultima() {
        let remotas = [
            regla(Some(&["a"]), "2026-10-17T13:00:00Z"),
            regla(Some(&["b"]), "2026-10-17T15:00:00Z"),
            regla(Some(&["c"]), "2026-10-17T11:00:00Z"),
        ];
        let reglas = Reglas::combinar(&remotas, hora(AHORA)).unwrap();
        assert!(reglas.permite("a") && reglas.permite("b"));
        assert!(!reglas.permite("c"), "la vencida no suma");
        assert!(reglas.vigente(hora("2026-10-17T14:59:59Z")));
        assert!(!reglas.vigente(hora("2026-10-17T15:00:00Z")));
    }

    #[test]
    fn si_una_regla_pide_todo_las_demas_no_lo_limitan() {
        let remotas = [
            regla(Some(&["a"]), "2026-10-17T13:00:00Z"),
            regla(None, "2026-10-17T14:00:00Z"),
        ];
        assert!(
            Reglas::combinar(&remotas, hora(AHORA))
                .unwrap()
                .permite("x")
        );
    }

    #[test]
    fn la_respuesta_del_servidor_se_lee_como_regla() {
        let remotas: Vec<ReglaRemota> = serde_json::from_str(
            r#"[{"id":7,"nivel":"ligero","eventos":null,"vence_en":"2026-10-18T12:00:00.123456+00:00"},
                {"id":8,"nivel":"diagnostico","eventos":["realtime"],"vence_en":"2026-10-18T12:00:00+00:00"}]"#,
        )
        .unwrap();
        let reglas = Reglas::combinar(&remotas, hora(AHORA)).unwrap();
        assert!(reglas.todos);
        assert!(reglas.eventos.contains("realtime"));
    }

    fn vigente_hasta(vence_en: &str) -> Reglas {
        Reglas::combinar(&[regla(None, vence_en)], hora(AHORA)).unwrap()
    }

    #[test]
    fn sin_respuesta_y_apagada_sigue_apagada() {
        assert_eq!(Reglas::siguiente(None, None, hora(AHORA)), None);
    }

    #[test]
    fn sin_respuesta_la_regla_que_rige_sigue_hasta_vencer() {
        let actual = vigente_hasta("2026-10-17T13:00:00Z");
        assert_eq!(
            Reglas::siguiente(Some(&actual), None, hora("2026-10-17T12:59:59Z")),
            Some(actual.clone()),
        );
        assert_eq!(
            Reglas::siguiente(Some(&actual), None, hora("2026-10-17T13:00:00Z")),
            None,
            "vencida, se apaga sola sin consultar"
        );
    }

    #[test]
    fn una_respuesta_sin_reglas_apaga_de_inmediato() {
        let actual = vigente_hasta("2026-10-18T12:00:00Z");
        assert_eq!(
            Reglas::siguiente(Some(&actual), Some(&[]), hora(AHORA)),
            None
        );
    }

    #[test]
    fn una_respuesta_con_reglas_enciende_o_reemplaza() {
        let nueva = [regla(Some(&["realtime"]), "2026-10-18T12:00:00Z")];
        let encendida = Reglas::siguiente(None, Some(&nueva), hora(AHORA)).unwrap();
        assert!(encendida.permite("realtime") && !encendida.permite("pantalla"));
        let actual = vigente_hasta("2026-10-17T13:00:00Z");
        let reemplazada = Reglas::siguiente(Some(&actual), Some(&nueva), hora(AHORA)).unwrap();
        assert!(
            !reemplazada.todos,
            "manda la respuesta nueva, no la anterior"
        );
    }

    #[test]
    fn la_regla_rige_solo_antes_de_vencer() {
        assert!(!regla_rige(0, 1_000), "0 = apagada");
        assert!(regla_rige(1_001, 1_000));
        assert!(!regla_rige(1_000, 1_000));
        assert!(!regla_rige(999, 1_000));
    }

    #[test]
    fn como_mucho_una_consulta_cada_cinco_minutos() {
        let base = Instant::now();
        let despues = |segundos| base + Duration::from_secs(segundos);
        assert!(toca_consultar(None, base), "nunca se consultó");
        assert!(!toca_consultar(Some(base), base));
        assert!(!toca_consultar(Some(base), despues(299)));
        assert!(toca_consultar(Some(base), despues(300)));
        assert!(
            !toca_consultar(Some(despues(10)), base),
            "un reloj que retrocede no habilita otra consulta"
        );
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
    fn todo_evento_lleva_el_esquema_y_la_plataforma() {
        let datos = con_esquema_y_plataforma(serde_json::json!({ "nombre": "login" }));
        assert_eq!(datos["esquema"], ESQUEMA_TELEMETRIA);
        assert_eq!(datos["plataforma"], "escritorio");
        assert_eq!(datos["nombre"], "login");
    }

    #[test]
    fn lo_que_ya_trae_el_evento_manda_sobre_el_esquema_y_la_plataforma() {
        let datos =
            con_esquema_y_plataforma(serde_json::json!({ "plataforma": "otra", "esquema": 99 }));
        assert_eq!(datos["plataforma"], "otra");
        assert_eq!(datos["esquema"], 99);
    }

    #[test]
    fn un_dato_que_no_es_objeto_se_deja_como_esta() {
        assert_eq!(
            con_esquema_y_plataforma(serde_json::json!(5)),
            serde_json::json!(5)
        );
        assert_eq!(
            con_esquema_y_plataforma(serde_json::json!(null)),
            serde_json::json!(null)
        );
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
