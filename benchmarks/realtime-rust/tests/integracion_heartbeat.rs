//! Prueba de INTEGRACIÓN (distinta de las unitarias en `src/cliente.rs`):
//! ejercita el crate como lo haría alguien que sólo importa
//! `lattis_realtime_spike` desde afuera -- API pública únicamente, sin
//! acceso a nada `pub(crate)`. Si algo que debería ser público quedó
//! privado sin querer, esto no compila; si el contrato público se rompe
//! (cambia una firma, un tipo), esto lo detecta antes que cualquier
//! consumidor real.

mod comun;

use std::time::Duration;

use lattis_realtime_spike::{ClienteRealtime, EventoSupervisor, supervisar_heartbeat};

#[tokio::test]
async fn el_heartbeat_funciona_de_punta_a_punta_via_api_publica() {
    let url = comun::servidor_heartbeat_simple().await;
    let mut cliente = ClienteRealtime::conectar(&url).await.unwrap();
    assert!(cliente.latido().await.unwrap());
    cliente.cerrar().await.unwrap();
}

#[tokio::test]
async fn el_supervisor_de_heartbeat_es_utilizable_desde_afuera_del_crate() {
    let url = comun::servidor_heartbeat_simple().await;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let tarea = tokio::spawn(supervisar_heartbeat(
        url,
        Duration::from_millis(10),
        Duration::from_millis(50),
        tx,
        || false,
    ));

    let primer_evento = rx.recv().await.unwrap();
    assert_eq!(primer_evento, EventoSupervisor::Conectado);
    let segundo_evento = rx.recv().await.unwrap();
    assert_eq!(segundo_evento, EventoSupervisor::LatidoOk);

    tarea.abort();
}
