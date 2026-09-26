//! Punta a punta #1 -- el patrón que usa HOY producción
//! (`private.emitir_cambio_nube_sitio`): un aviso vacío obliga a un
//! SEGUNDO viaje para tener el dato real.
//!
//! Contra un mock local (sin red externa) en vez de Supabase real -- este
//! archivo corre en cada `cargo test`, no depende de credenciales ni de
//! que `control-acceso-staging` esté arriba. Los `smoke_*` binarios en
//! `src/bin/` siguen siendo la prueba manual contra el servidor real; esto
//! es la versión automatizada, permanente, que no se puede romper sin que
//! CI se entere.

mod comun;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use lattis_realtime_spike::ClienteRealtime;
use rusqlite::Connection;

#[tokio::test]
async fn el_patron_actual_necesita_dos_round_trips_para_tener_el_dato() {
    let contador = Arc::new(AtomicUsize::new(0));
    let url = comun::servidor_aviso_vacio_con_resync(Arc::clone(&contador)).await;

    let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
    cliente.unirse_publico("realtime:lab:e2e").await.unwrap();

    // 1er round-trip: el aviso -- SIN datos reales, sólo metadata.
    let aviso = cliente
        .esperar_evento("realtime:lab:e2e", "cambio_nube", Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(aviso["operation"], "INSERT");
    assert!(
        aviso.get("nombre").is_none(),
        "el aviso vacío NO debe traer datos reales -- si los trae, este mock \
         dejó de representar el patrón actual de producción: {aviso:?}"
    );

    // 2do round-trip: el "resync" -- acá recién aparece el dato real.
    let datos = cliente
        .solicitar("realtime:lab:e2e", "resync_fetch", serde_json::json!({}))
        .await
        .unwrap();

    let conexion = Connection::open_in_memory().unwrap();
    conexion
        .execute_batch("CREATE TABLE datos (id INTEGER, nombre TEXT, cedula TEXT);")
        .unwrap();
    conexion
        .execute(
            "INSERT INTO datos (id, nombre, cedula) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                datos["id"].as_i64().unwrap(),
                datos["nombre"].as_str().unwrap(),
                datos["cedula"].as_str().unwrap(),
            ],
        )
        .unwrap();

    let nombre: String = conexion
        .query_row("SELECT nombre FROM datos WHERE id = 1", [], |f| f.get(0))
        .unwrap();
    assert_eq!(nombre, "dato via resync");

    // La prueba dura: hizo falta exactamente UN mensaje cliente→servidor
    // DESPUÉS del join (el `resync_fetch`) para tener el dato -- ésa es la
    // "distancia" (en round-trips, no en reloj de pared, que en loopback es
    // ruido) que separa "avisar" de "avisar Y traer el dato".
    assert_eq!(
        contador.load(Ordering::SeqCst),
        1,
        "el patrón actual necesitó 1 mensaje extra tras el aviso para tener el dato"
    );
}
