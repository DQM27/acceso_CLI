//! Punta a punta #2 -- el patrón alternativo (`realtime.broadcast_changes`,
//! Etapa 3.5 del README): la fila completa viaja en el mismo broadcast,
//! CERO round-trips extra. Contraparte directa de
//! `e2e_aviso_vacio_y_resync.rs` -- mismo dato final, mismo destino
//! (`SQLite`), la única diferencia es cuántos mensajes hicieron falta.

mod comun;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use lattis_realtime_spike::ClienteRealtime;
use rusqlite::Connection;

#[tokio::test]
async fn broadcast_changes_entrega_el_dato_sin_ningun_round_trip_extra() {
    let contador = Arc::new(AtomicUsize::new(0));
    let url = comun::servidor_broadcast_changes(Arc::clone(&contador)).await;

    let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
    cliente.unirse_publico("realtime:lab:e2e").await.unwrap();

    // Un único round-trip -- el broadcast YA trae la fila completa.
    let evento = cliente
        .esperar_evento("realtime:lab:e2e", "fila_completa", Duration::from_secs(5))
        .await
        .unwrap();
    let fila = &evento["record"];

    let conexion = Connection::open_in_memory().unwrap();
    conexion
        .execute_batch("CREATE TABLE datos (id INTEGER, nombre TEXT, cedula TEXT);")
        .unwrap();
    conexion
        .execute(
            "INSERT INTO datos (id, nombre, cedula) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                fila["id"].as_i64().unwrap(),
                fila["nombre"].as_str().unwrap(),
                fila["cedula"].as_str().unwrap(),
            ],
        )
        .unwrap();

    let nombre: String = conexion
        .query_row("SELECT nombre FROM datos WHERE id = 1", [], |f| f.get(0))
        .unwrap();
    assert_eq!(nombre, "dato directo");

    // La prueba dura: CERO mensajes cliente→servidor después del join --
    // todo lo que necesitábamos ya venía en el broadcast. Comparado con el
    // "1" de `e2e_aviso_vacio_y_resync.rs`, esto es la evidencia
    // reproducible de que el patrón `broadcast_changes` es estructuralmente
    // más rápido: un round-trip menos, siempre, no "a veces" -- y en una
    // red real (no este loopback) cada round-trip cuesta al menos un RTT
    // completo cliente↔Supabase, no unos microsegundos de localhost.
    assert_eq!(
        contador.load(Ordering::SeqCst),
        0,
        "broadcast_changes no debería necesitar NINGÚN mensaje extra tras el join"
    );
}
