//! Etapa 4 -- estrés real contra `control-acceso-staging`: corre
//! `supervisar_canal_privado` completo (heartbeat propio + renovación
//! proactiva de JWT, sin reconectar) contra el servidor de Supabase de
//! verdad, no un mock. Deja correr varios ciclos de renovación reales para
//! confirmar que el `access_token` push in-band (investigado, no probado
//! nunca contra un servidor real hasta este binario) funciona como dice la
//! documentación.
//!
//! Uso:
//! ```sh
//! REALTIME_WS_URL="wss://pmrytjktlyiuikxuuxpr.supabase.co/realtime/v1/websocket" \
//! REALTIME_APIKEY="<anon key>" \
//! REALTIME_DEVICE_JWT="<access_token de device-auth>" \
//! REALTIME_SITIO_ID="<sitio_id del dispositivo de laboratorio>" \
//! cargo run --manifest-path benchmarks/realtime-rust/Cargo.toml --bin smoke_supervisor_privado
//! ```

use std::time::Duration;

use lattis_realtime_spike::{
    ConfigCanalPrivado, EventoSupervisorPrivado, supervisar_canal_privado,
};

fn variable_requerida(nombre: &str) -> String {
    std::env::var(nombre).unwrap_or_else(|_| {
        eprintln!("Falta {nombre}");
        std::process::exit(2);
    })
}

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("instalar el CryptoProvider de rustls una sola vez, al arrancar");

    let url_base = variable_requerida("REALTIME_WS_URL");
    let apikey = variable_requerida("REALTIME_APIKEY");
    let jwt_dispositivo = variable_requerida("REALTIME_DEVICE_JWT");
    let sitio_id = variable_requerida("REALTIME_SITIO_ID");
    let url = format!("{url_base}?apikey={apikey}&vsn=1.0.0");
    let topic = format!("realtime:sitio:{sitio_id}");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let jwt_para_closure = jwt_dispositivo.clone();
    let tarea = tokio::spawn(supervisar_canal_privado(
        ConfigCanalPrivado {
            url,
            topic,
            evento_esperado: "cambio_nube".to_string(),
            backoff_base: Duration::from_secs(2),
            backoff_tope: Duration::from_secs(30),
            intervalo_heartbeat: Duration::from_secs(15),
            // Cada 5s en vez de las horas reales que duraría un JWT --
            // este binario corre un par de minutos, no 12h; el objetivo es
            // ver VARIOS ciclos de renovación reales en poco tiempo, no
            // esperar a que el JWT real esté por expirar.
            renovar_token_cada: Some(Duration::from_secs(5)),
        },
        // Mismo JWT real en cada llamada -- alcanza para probar que el
        // MECANISMO de push in-band funciona contra el servidor real; no
        // hace falta un token "distinto" cada vez para eso (esa lógica de
        // no-cachear-el-viejo ya está probada contra el mock en
        // supervisor.rs).
        move || jwt_para_closure.clone(),
        tx,
        || false,
    ));

    println!("Corriendo 90s contra el servidor real -- Ctrl+C para cortar antes...");
    let mut renovaciones = 0u32;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);

    loop {
        tokio::select! {
            () = tokio::time::sleep_until(deadline) => break,
            evento = rx.recv() => {
                match evento {
                    Some(EventoSupervisorPrivado::UnidoAlCanal) => println!("[{:>5.1}s] Unido al canal privado real.", tiempo_transcurrido()),
                    Some(EventoSupervisorPrivado::TokenRenovado) => {
                        renovaciones += 1;
                        println!("[{:>5.1}s] Token renovado SIN reconectar (van {renovaciones}).", tiempo_transcurrido());
                    }
                    Some(EventoSupervisorPrivado::EventoRecibido(payload)) => {
                        println!("[{:>5.1}s] Broadcast real recibido: {payload}", tiempo_transcurrido());
                    }
                    Some(EventoSupervisorPrivado::Desconectado { motivo }) => {
                        println!("[{:>5.1}s] Desconectado: {motivo}", tiempo_transcurrido());
                    }
                    Some(EventoSupervisorPrivado::Reintentando { intento, espera }) => {
                        println!("[{:>5.1}s] Reintentando (intento {intento}, espera {espera:?})", tiempo_transcurrido());
                    }
                    None => break,
                }
            }
        }
    }

    tarea.abort();

    if renovaciones >= 2 {
        println!(
            "OK -- {renovaciones} renovaciones de JWT sin reconectar contra el servidor REAL de \
             Supabase. Si el canal hubiera muerto por inactividad (sin heartbeat propio), esto \
             nunca habría llegado a la segunda renovación."
        );
    } else {
        eprintln!(
            "Esperaba al menos 2 renovaciones en 90s (una cada 5s) -- sólo hubo {renovaciones}."
        );
        std::process::exit(1);
    }
}

fn tiempo_transcurrido() -> f64 {
    use std::sync::OnceLock;
    static INICIO: OnceLock<tokio::time::Instant> = OnceLock::new();
    let inicio = *INICIO.get_or_init(tokio::time::Instant::now);
    inicio.elapsed().as_secs_f64()
}
