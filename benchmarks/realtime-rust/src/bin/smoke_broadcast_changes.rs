//! Etapa 3.5: la pregunta que originó todo este laboratorio -- ¿se puede
//! mandar la FILA COMPLETA por el broadcast, en vez de un aviso vacío que
//! obliga a un roundtrip REST aparte? Sí: `realtime.broadcast_changes()`
//! (built-in de Supabase, no algo que inventamos) arma el payload con
//! `record`/`old_record` completos -- este binario prueba que el cliente
//! recibe esos datos reales sin hacer ningún GET/SELECT después.
//!
//! A diferencia de `smoke_private_channel.rs` (que sólo prueba el join
//! privado con el aviso vacío real de producción, `cambio_nube`), acá el
//! trigger de laboratorio (`private.lab_lattis_emitir_fila_completa`, ver
//! README.md) usa `broadcast_changes` -- el mecanismo que, si se adoptara,
//! reemplazaría el patrón actual de `nubeRealtime.ts`
//! (aviso vacío → `sincronizarConNube()` completo).
//!
//! Uso: igual que `smoke_private_channel`, con `REALTIME_DEVICE_JWT` y
//! `REALTIME_SITIO_ID` del dispositivo/sitio de laboratorio de esta etapa.

use std::time::Duration;

use lattis_realtime_spike::ClienteRealtime;
use rusqlite::Connection;

const EVENTO: &str = "fila_completa";
const ESPERA_EVENTO: Duration = Duration::from_secs(60);

fn variable_requerida(nombre: &str) -> String {
    std::env::var(nombre).unwrap_or_else(|_| {
        eprintln!("Falta {nombre}");
        std::process::exit(2);
    })
}

fn abrir_sqlite_plano(ruta: &str) -> rusqlite::Result<Connection> {
    let conexion = Connection::open(ruta)?;
    conexion.execute_batch(
        "CREATE TABLE IF NOT EXISTS datos_completos_recibidos (
            id_remoto INTEGER NOT NULL,
            nombre TEXT NOT NULL,
            cedula TEXT NOT NULL,
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

    let url_base = variable_requerida("REALTIME_WS_URL");
    let apikey = variable_requerida("REALTIME_APIKEY");
    let jwt_dispositivo = variable_requerida("REALTIME_DEVICE_JWT");
    let sitio_id = variable_requerida("REALTIME_SITIO_ID");
    let ruta_sqlite = std::env::var("LATTIS_LAB_SQLITE_PATH")
        .unwrap_or_else(|_| "./lattis_lab.sqlite3".to_string());

    let topic = format!("realtime:sitio:{sitio_id}");
    let url = format!("{url_base}?apikey={apikey}&vsn=1.0.0");

    println!("Conectando a {url_base}...");
    let mut cliente = ClienteRealtime::conectar(&url)
        .await
        .unwrap_or_else(|error| {
            eprintln!("FALLÓ la conexión: {error}");
            std::process::exit(1);
        });

    println!("Uniéndose al canal PRIVADO {topic}...");
    if let Err(error) = cliente.unirse_privado(&topic, &jwt_dispositivo).await {
        eprintln!("FALLÓ el phx_join privado: {error}");
        std::process::exit(1);
    }

    println!(
        "Esperando un evento '{EVENTO}' hasta {ESPERA_EVENTO:?} \
         (hacé el INSERT en _lab_lattis_datos_completos con sitio_id = {sitio_id} ahora)..."
    );

    let payload = match cliente.esperar_evento(&topic, EVENTO, ESPERA_EVENTO).await {
        Ok(payload) => payload,
        Err(error) => {
            eprintln!("FALLÓ esperando el evento: {error}");
            std::process::exit(1);
        }
    };

    // A diferencia de cambio_nube (que sólo trae metadata), acá `record` es
    // la fila NUEVA COMPLETA -- si esto tiene los campos reales (nombre,
    // cedula, etc.), quedó demostrado que no hace falta ningún GET/SELECT
    // aparte para saber qué cambió.
    println!("Broadcast recibido (fila completa, sin roundtrip REST): {payload}");
    let fila = &payload["record"];

    let id_remoto = fila["id"].as_i64().expect("record sin 'id'");
    let nombre = fila["nombre"]
        .as_str()
        .expect("record sin 'nombre'")
        .to_string();
    let cedula = fila["cedula"]
        .as_str()
        .expect("record sin 'cedula'")
        .to_string();
    let creado_en = fila["creado_en"]
        .as_str()
        .expect("record sin 'creado_en'")
        .to_string();

    let conexion_sqlite = abrir_sqlite_plano(&ruta_sqlite).unwrap_or_else(|error| {
        eprintln!("FALLÓ abriendo SQLite local ({ruta_sqlite}): {error}");
        std::process::exit(1);
    });
    conexion_sqlite
        .execute(
            "INSERT INTO datos_completos_recibidos (id_remoto, nombre, cedula, creado_en_remoto) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id_remoto, nombre, cedula, creado_en],
        )
        .unwrap_or_else(|error| {
            eprintln!("FALLÓ el INSERT local: {error}");
            std::process::exit(1);
        });

    println!(
        "OK -- fila completa escrita en {ruta_sqlite} SIN ningún GET/SELECT adicional: \
         id={id_remoto}, nombre={nombre}, cedula={cedula}."
    );

    if let Err(error) = cliente.cerrar().await {
        eprintln!("Aviso: no se pudo cerrar limpio: {error}");
    }
}
