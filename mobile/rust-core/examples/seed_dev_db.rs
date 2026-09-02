//! Herramienta de desarrollo: crea una base `SQLite` con el esquema real
//! (mismas migraciones que TUI/GUI/móvil) y la llena con datos de prueba —
//! los contratistas reales de `importar_contratistas_db_browser.sql`, un
//! usuario ROOT de acceso rápido, y un catálogo de 25 gafetes (el SQL de
//! contratistas es anterior al catálogo de gafetes, no trae ninguno). No se
//! usa desde la app, solo desde la terminal.

fn main() {
    let ruta = std::env::args()
        .nth(1)
        .expect("uso: seed_dev_db <ruta_db>");

    // Igual que AppCore::abrir: aplica el esquema/migraciones reales.
    let conexion =
        control_acceso::database::connection::open_database(&ruta).expect("no se pudo abrir/crear la base");

    conexion
        .execute_batch(include_str!("seed_usuario_root.sql"))
        .expect("fallo insertando usuario root");

    conexion
        .execute_batch(include_str!("../../../importar_contratistas_db_browser.sql"))
        .expect("fallo insertando contratistas");

    conexion
        .execute_batch(include_str!("seed_gafetes.sql"))
        .expect("fallo insertando gafetes");

    println!("Semilla de desarrollo cargada en {ruta}");
}
