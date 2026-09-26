//! Etapa 2: `phx_join` a un canal PRIVADO real (`realtime:sitio:<uuid>`),
//! con un JWT de dispositivo real emitido por la Edge Function `device-auth`
//! de `control-acceso-staging` -- esta vez SÍ pasa por la política de
//! `realtime.messages` (`dispositivos reciben broadcast de su sitio`, ver
//! `docs/arquitectura/arquitectura-supabase.md` sección 4.2), a diferencia
//! del canal público de la Etapa 1.5.
//!
//! Requiere un dispositivo y un sitio descartables ya registrados en
//! staging (ver README.md, "Etapa 2") y el `access_token` que devuelve
//! `device-auth` para ese dispositivo.
//!
//! Uso:
//! ```sh
//! REALTIME_WS_URL="wss://pmrytjktlyiuikxuuxpr.supabase.co/realtime/v1/websocket" \
//! REALTIME_APIKEY="<anon key>" \
//! REALTIME_DEVICE_JWT="<access_token de device-auth>" \
//! REALTIME_SITIO_ID="<sitio_id del dispositivo de laboratorio>" \
//! cargo run --manifest-path benchmarks/realtime-rust/Cargo.toml --bin smoke_private_channel
//! ```

use std::time::Duration;

use lattis_realtime_spike::ClienteRealtime;

const EVENTO: &str = "cambio_nube";
const ESPERA_EVENTO: Duration = Duration::from_secs(60);

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

    let topic = format!("realtime:sitio:{sitio_id}");
    let url = format!("{url_base}?apikey={apikey}&vsn=1.0.0");

    println!("Conectando a {url_base}...");
    let mut cliente = ClienteRealtime::conectar(&url)
        .await
        .unwrap_or_else(|error| {
            eprintln!("FALLÓ la conexión: {error}");
            std::process::exit(1);
        });

    println!("Uniéndose al canal PRIVADO {topic} con el JWT del dispositivo...");
    if let Err(error) = cliente.unirse_privado(&topic, &jwt_dispositivo).await {
        eprintln!("FALLÓ el phx_join privado: {error}");
        std::process::exit(1);
    }
    println!("Join privado aceptado -- la política de realtime.messages validó el JWT.");

    println!(
        "Esperando un evento '{EVENTO}' hasta {ESPERA_EVENTO:?} \
         (hacé un INSERT en contratistas/empresas/gafetes/ingresos/usuarios/movimientos_visita \
         con sitio_id = {sitio_id} ahora)..."
    );

    match cliente.esperar_evento(&topic, EVENTO, ESPERA_EVENTO).await {
        Ok(payload) => println!("OK -- broadcast privado recibido: {payload}"),
        Err(error) => {
            eprintln!("FALLÓ esperando el evento: {error}");
            std::process::exit(1);
        }
    }

    if let Err(error) = cliente.cerrar().await {
        eprintln!("Aviso: no se pudo cerrar limpio: {error}");
    }
}
