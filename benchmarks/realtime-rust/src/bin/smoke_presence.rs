//! Presence contra `control-acceso-staging` REAL, con dos dispositivos
//! simultáneos -- el caso real que usa `nubeRealtime.ts` hoy para el panel
//! de "quién está conectado".
//!
//! Dos hallazgos reales que corrigieron la primera versión de este binario
//! (verificados con `ClienteRealtime::diagnostico_mostrar_todo`, no
//! adivinados -- la documentación oficial sugiere que `presence_state`
//! llega automáticamente al unirse, y no fue lo que pasó contra el
//! servidor real):
//! 1. `presence_state`/`presence_diff` NO llegan automáticamente al hacer
//!    `phx_join` -- recién aparecen después de que EL PROPIO cliente hace
//!    su primer `track()`. Unirse sin trackear nunca nada no dispara nada.
//! 2. El `track` (`event: "presence"`) SÍ recibe un `phx_reply` normal --
//!    a diferencia de `access_token`, que la documentación confirma que no
//!    responde nada en éxito.
//!
//! Uso: variables para AMBOS dispositivos de laboratorio (mismo sitio):
//! ```sh
//! REALTIME_WS_URL=... REALTIME_APIKEY=... REALTIME_SITIO_ID=... \
//! REALTIME_DEVICE_JWT_A=... REALTIME_DEVICE_JWT_B=... \
//! cargo run --manifest-path benchmarks/realtime-rust/Cargo.toml --bin smoke_presence
//! ```

use std::time::Duration;

use lattis_realtime_spike::{ClienteRealtime, instalar_crypto_provider_tolerante};

fn variable_requerida(nombre: &str) -> String {
    std::env::var(nombre).unwrap_or_else(|_| {
        eprintln!("Falta {nombre}");
        std::process::exit(2);
    })
}

#[tokio::main]
async fn main() {
    instalar_crypto_provider_tolerante();

    let url_base = variable_requerida("REALTIME_WS_URL");
    let apikey = variable_requerida("REALTIME_APIKEY");
    let sitio_id = variable_requerida("REALTIME_SITIO_ID");
    let jwt_a = variable_requerida("REALTIME_DEVICE_JWT_A");
    let jwt_b = variable_requerida("REALTIME_DEVICE_JWT_B");

    let url = format!("{url_base}?apikey={apikey}&vsn=1.0.0");
    let topic = format!("realtime:sitio:{sitio_id}");

    // --- Dispositivo A se une y publica su presencia ---
    println!("[A] conectando y uniéndose...");
    let mut a = ClienteRealtime::conectar(&url)
        .await
        .expect("A no pudo conectar");
    a.unirse_privado(&topic, &jwt_a)
        .await
        .expect("A no pudo unirse");

    println!("[A] track()...");
    a.trackear_presencia(
        &topic,
        serde_json::json!({ "cedula": "A", "nombre": "Dispositivo A" }),
    )
    .await
    .expect("A no pudo trackear");

    // El track dispara presence_state (el snapshot completo, con A solo) y
    // presence_diff (A entrando) -- en ese orden, ambos como consecuencia
    // del propio track, no del join.
    let estado_inicial = a
        .esperar_mensaje(&topic, "presence_state", Duration::from_secs(10))
        .await
        .expect("A no recibió presence_state tras su propio track");
    println!("[A] presence_state (debería traer sólo a A): {estado_inicial}");

    // El propio track() de A genera TAMBIÉN un presence_diff -- A anunciando
    // su propio join (separado del presence_state, que es el snapshot
    // completo). Hay que consumirlo antes de esperar el de B, o se lo
    // confunde con el de B (bug real encontrado corriendo esto la primera
    // vez contra el servidor real).
    let diff_propio = a
        .esperar_mensaje(&topic, "presence_diff", Duration::from_secs(10))
        .await
        .expect("A no recibió su propio presence_diff tras trackear");
    println!("[A] (descartado, es el propio) presence_diff de A: {diff_propio}");

    // --- Dispositivo B se une después, y trackea ---
    println!("[B] conectando, uniéndose y publicando su presencia...");
    let mut b = ClienteRealtime::conectar(&url)
        .await
        .expect("B no pudo conectar");
    b.unirse_privado(&topic, &jwt_b)
        .await
        .expect("B no pudo unirse");
    b.trackear_presencia(
        &topic,
        serde_json::json!({ "cedula": "B", "nombre": "Dispositivo B" }),
    )
    .await
    .expect("B no pudo trackear");

    // --- A tiene que enterarse de B por presence_diff, en tiempo real ---
    println!("[A] esperando el presence_diff con el join de B...");
    let diff = a
        .esperar_mensaje(&topic, "presence_diff", Duration::from_secs(10))
        .await
        .expect("A no recibió el presence_diff de B");
    println!("[A] presence_diff recibido: {diff}");

    let tiene_a_b = diff["joins"].as_object().is_some_and(|mapa| {
        mapa.values().any(|valor| {
            valor["metas"]
                .as_array()
                .is_some_and(|metas| metas.iter().any(|m| m["cedula"] == "B"))
        })
    });

    if tiene_a_b {
        println!("OK -- A vio en vivo, por Presence real, que B se conectó (cedula=B).");
    } else {
        eprintln!("El presence_diff no trae a B con la forma esperada: {diff}");
        std::process::exit(1);
    }

    let _ = a.cerrar().await;
    let _ = b.cerrar().await;
}
