//! Smoke test contra el servidor REAL de Realtime de `control-acceso-staging`
//! (nunca `control-acceso-nube`, el proyecto de producción -- ver README.md).
//! Sólo prueba el heartbeat del protocolo (topic fijo `"phoenix"`, no exige
//! ningún JWT de dispositivo) -- primer peldaño antes de intentar `phx_join`
//! a un canal privado real.
//!
//! Uso:
//! ```sh
//! REALTIME_WS_URL="wss://pmrytjktlyiuikxuuxpr.supabase.co/realtime/v1/websocket" \
//! REALTIME_APIKEY="<anon key de control-acceso-staging>" \
//! cargo run --manifest-path benchmarks/realtime-rust/Cargo.toml --bin smoke_heartbeat
//! ```

use brisas_realtime_spike::ClienteRealtime;

#[tokio::main]
async fn main() {
    // Ver el comentario de la dependencia `rustls` en Cargo.toml -- sin esto,
    // el primer `connect_async` contra `wss://` entra en pánico.
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("instalar el CryptoProvider de rustls una sola vez, al arrancar");

    let url_base = std::env::var("REALTIME_WS_URL").unwrap_or_else(|_| {
        eprintln!(
            "Falta REALTIME_WS_URL -- ej. wss://pmrytjktlyiuikxuuxpr.supabase.co/realtime/v1/websocket \
             (el ref de control-acceso-staging, NUNCA el de producción)"
        );
        std::process::exit(2);
    });
    let apikey = std::env::var("REALTIME_APIKEY").unwrap_or_else(|_| {
        eprintln!("Falta REALTIME_APIKEY -- la anon key de control-acceso-staging");
        std::process::exit(2);
    });

    let url = format!("{url_base}?apikey={apikey}&vsn=1.0.0");

    println!("Conectando a {url_base}...");
    let mut cliente = match ClienteRealtime::conectar(&url).await {
        Ok(cliente) => cliente,
        Err(error) => {
            eprintln!("FALLÓ la conexión: {error}");
            std::process::exit(1);
        }
    };
    println!("Conectado. Mandando heartbeat...");

    match cliente.latido().await {
        Ok(true) => println!("OK -- el servidor respondió phx_reply status=ok al heartbeat."),
        Ok(false) => {
            eprintln!("El servidor respondió, pero no con status=ok -- ver protocolo.rs");
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("FALLÓ el heartbeat: {error}");
            std::process::exit(1);
        }
    }

    if let Err(error) = cliente.cerrar().await {
        eprintln!("Aviso: no se pudo cerrar limpio: {error}");
    }
}
