//! Prueba de punta a punta real: Postgres → trigger de Broadcast → WebSocket
//! → este cliente Rust → fila en SQLite local (motor "plano", sin cifrar --
//! sólo para este laboratorio, ver README.md).
//!
//! Requiere los objetos descartables `_lab_lattis_avisos` /
//! `private.lab_lattis_emitir_aviso` creados en `control-acceso-staging`
//! (ver README.md, "Etapa 1.5"). Este binario sólo ESCUCHA -- el INSERT que
//! dispara el aviso se hace aparte (por SQL directo contra staging), para no
//! meter en este laboratorio credenciales de base de datos además de las de
//! Realtime.
//!
//! Uso: igual que `smoke_heartbeat`, más `LATTIS_LAB_SQLITE_PATH` opcional
//! (por defecto `./lattis_lab.sqlite3` en el directorio de trabajo actual).

use std::time::Duration;

use lattis_realtime_spike::ClienteRealtime;
use rusqlite::Connection;

// Supabase antepone "realtime:" al topic que uno le pasa a `.channel(...)`
// del lado del cliente JS -- el nombre "de negocio" (el que compara
// `realtime.topic()` en Postgres, ver `private.lab_lattis_emitir_aviso` y
// la política de `arquitectura-supabase.md` 4.2) NO lo lleva. Sin este
// prefijo, `phx_join` devuelve `{"reason":"unmatched topic"}`.
const TOPIC_LAB: &str = "realtime:lab:lattis";
const EVENTO_LAB: &str = "lab_aviso";
const ESPERA_EVENTO: Duration = Duration::from_secs(60);

fn abrir_sqlite_plano(ruta: &str) -> rusqlite::Result<Connection> {
    let conexion = Connection::open(ruta)?;
    conexion.execute_batch(
        "CREATE TABLE IF NOT EXISTS avisos_recibidos (
            id_remoto INTEGER NOT NULL,
            mensaje TEXT NOT NULL,
            creado_en_remoto TEXT NOT NULL,
            recibido_en TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;
    Ok(conexion)
}

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("instalar el CryptoProvider de rustls una sola vez, al arrancar");

    let url_base = std::env::var("REALTIME_WS_URL").unwrap_or_else(|_| {
        eprintln!("Falta REALTIME_WS_URL (el de control-acceso-staging, NUNCA producción)");
        std::process::exit(2);
    });
    let apikey = std::env::var("REALTIME_APIKEY").unwrap_or_else(|_| {
        eprintln!("Falta REALTIME_APIKEY -- la anon key de control-acceso-staging");
        std::process::exit(2);
    });
    let ruta_sqlite = std::env::var("LATTIS_LAB_SQLITE_PATH")
        .unwrap_or_else(|_| "./lattis_lab.sqlite3".to_string());

    let url = format!("{url_base}?apikey={apikey}&vsn=1.0.0");

    println!("Conectando a {url_base}...");
    let mut cliente = ClienteRealtime::conectar(&url)
        .await
        .unwrap_or_else(|error| {
            eprintln!("FALLÓ la conexión: {error}");
            std::process::exit(1);
        });

    println!("Uniéndose al canal público {TOPIC_LAB}...");
    if let Err(error) = cliente.unirse_publico(TOPIC_LAB).await {
        eprintln!("FALLÓ el phx_join: {error}");
        std::process::exit(1);
    }

    println!(
        "Unido. Esperando un evento '{EVENTO_LAB}' hasta {ESPERA_EVENTO:?} \
         (hacé el INSERT en _lab_lattis_avisos de control-acceso-staging ahora)..."
    );

    let payload = match cliente
        .esperar_evento(TOPIC_LAB, EVENTO_LAB, ESPERA_EVENTO)
        .await
    {
        Ok(payload) => payload,
        Err(error) => {
            eprintln!("FALLÓ esperando el evento: {error}");
            std::process::exit(1);
        }
    };
    println!("Broadcast recibido: {payload}");

    let id_remoto = payload["id"].as_i64().expect("payload sin 'id'");
    let mensaje = payload["mensaje"]
        .as_str()
        .expect("payload sin 'mensaje'")
        .to_string();
    let creado_en = payload["creado_en"]
        .as_str()
        .expect("payload sin 'creado_en'")
        .to_string();

    let conexion_sqlite = abrir_sqlite_plano(&ruta_sqlite).unwrap_or_else(|error| {
        eprintln!("FALLÓ abriendo SQLite local ({ruta_sqlite}): {error}");
        std::process::exit(1);
    });
    conexion_sqlite
        .execute(
            "INSERT INTO avisos_recibidos (id_remoto, mensaje, creado_en_remoto) VALUES (?1, ?2, ?3)",
            rusqlite::params![id_remoto, mensaje, creado_en],
        )
        .unwrap_or_else(|error| {
            eprintln!("FALLÓ el INSERT local: {error}");
            std::process::exit(1);
        });

    let total: i64 = conexion_sqlite
        .query_row("SELECT COUNT(*) FROM avisos_recibidos", [], |fila| {
            fila.get(0)
        })
        .unwrap();
    println!(
        "OK -- escrito en {ruta_sqlite} (motor plano). Total de avisos acumulados ahí: {total}."
    );

    if let Err(error) = cliente.cerrar().await {
        eprintln!("Aviso: no se pudo cerrar limpio: {error}");
    }
}
