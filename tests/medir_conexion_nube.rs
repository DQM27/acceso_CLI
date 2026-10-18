//! Medición manual (no corre con `cargo test` normal): ¿cuánto tarda una
//! ronda de consultas en paralelo contra la nube cuando las conexiones están
//! frías, calientes, o llevan un rato sin usarse?
//!
//! Reproduce lo que hace la verificación de un ingreso: varias consultas a la
//! vez con el mismo cliente (mismo timeout y misma configuración que
//! `nube::cliente::cliente_http_verificacion`), cada una sobre su propia
//! conexión porque `reqwest` está compilado sin HTTP/2. Mide la espera de la
//! ronda completa, que es lo que ve quien opera.
//!
//! Necesita las variables del proyecto contra el que se prueba (staging, no
//! producción) y compilar en release:
//!
//! ```text
//! CONTROL_ACCESO_SUPABASE_URL=https://... CONTROL_ACCESO_SUPABASE_APIKEY=sb_publishable_... \
//!   cargo test --release --features nube --test medir_conexion_nube -- --ignored --nocapture
//! ```

#![cfg(feature = "nube")]

use std::fmt::Write;
use std::time::{Duration, Instant};

const EN_PARALELO: usize = 3;

fn variable(nombre: &str) -> String {
    std::env::var(nombre).unwrap_or_else(|_| panic!("falta la variable {nombre}"))
}

/// Una consulta barata (no toca la base: contesta 401 sin tocar nada) cuyo
/// costo es el de la red: conexión nueva si hace falta, TLS y ida y vuelta.
fn pedir(cliente: &reqwest::blocking::Client, url: &str, clave: &str) -> Result<(), String> {
    cliente
        .get(url)
        .header("apikey", clave)
        .send()
        .map(|_| ())
        .map_err(|error| {
            // La cadena completa de causas: es lo que distingue "conexión
            // cerrada por el servidor" de "tiempo agotado".
            let mut texto = error.to_string();
            let mut causa = std::error::Error::source(&error);
            while let Some(interna) = causa {
                let _ = write!(texto, " <- {interna}");
                causa = interna.source();
            }
            texto
        })
}

/// Cuánto tarda la ronda completa de `EN_PARALELO` consultas a la vez, y los
/// errores de las que fallaron.
fn ronda(cliente: &reqwest::blocking::Client, url: &str, clave: &str) -> (Duration, Vec<String>) {
    let inicio = Instant::now();
    let errores = std::thread::scope(|hilos| {
        // El `collect` hace falta: sin él, cada hilo se espera antes de lanzar
        // el siguiente y las consultas saldrían una tras otra, no a la vez.
        #[allow(clippy::needless_collect)]
        let consultas: Vec<_> = (0..EN_PARALELO)
            .map(|_| hilos.spawn(|| pedir(cliente, url, clave)))
            .collect();
        consultas
            .into_iter()
            .filter_map(|consulta| consulta.join().unwrap().err())
            .collect::<Vec<_>>()
    });
    (inicio.elapsed(), errores)
}

fn informar(titulo: &str, (duracion, errores): (Duration, Vec<String>)) {
    println!(
        "{titulo:<44} {:>7.0} ms   fallaron {}/{EN_PARALELO}",
        duracion.as_secs_f64() * 1_000.0,
        errores.len()
    );
    if let Some(primero) = errores.first() {
        println!("    primer error: {primero}");
    }
}

#[test]
#[ignore = "medición manual contra la nube: ver el comentario del archivo"]
fn mide_la_ronda_en_frio_en_caliente_y_tras_esperar() {
    let url = format!("{}/rest/v1/", variable("CONTROL_ACCESO_SUPABASE_URL"));
    let clave = variable("CONTROL_ACCESO_SUPABASE_APIKEY");
    // Misma configuración que `cliente_http_verificacion`.
    let cliente = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    println!("\nRonda = {EN_PARALELO} consultas a la vez, espera de la ronda completa");
    informar(
        "conexiones nuevas (primera ronda)",
        ronda(&cliente, &url, &clave),
    );
    informar(
        "enseguida, conexiones calientes",
        ronda(&cliente, &url, &clave),
    );
    informar(
        "otra vez, conexiones calientes",
        ronda(&cliente, &url, &clave),
    );

    // Cada ronda vuelve a calentar el grupo, así que cada espera se cuenta
    // desde la ronda anterior.
    for segundos in [30_u64, 60, 100, 180] {
        std::thread::sleep(Duration::from_secs(segundos));
        informar(
            &format!("tras {segundos} s sin usarse"),
            ronda(&cliente, &url, &clave),
        );
    }
}
