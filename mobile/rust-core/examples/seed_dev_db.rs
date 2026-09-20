//! Herramienta de desarrollo: crea una base `SQLite` con el esquema real
//! (mismas migraciones que TUI/GUI/móvil) y la llena con datos de prueba —
//! los contratistas reales de `contratistas_base_final_limpia_v15.sql`, un
//! usuario ROOT de acceso rápido, y un catálogo de 25 gafetes (el SQL de
//! contratistas es anterior al catálogo de gafetes, no trae ninguno). No se
//! usa desde la app, solo desde la terminal.

fn main() {
    let ruta = std::env::args().nth(1).expect("uso: seed_dev_db <ruta_db>");

    // Igual que AppCore::abrir: aplica el esquema/migraciones reales.
    let conexion = control_acceso::database::connection::open_database(&ruta)
        .expect("no se pudo abrir/crear la base");

    conexion
        .execute_batch(include_str!("seed_usuario_root.sql"))
        .expect("fallo insertando usuario root");

    // Se lee en tiempo de ejecución (no `include_str!`) -- ese SQL tiene
    // datos reales de personas y ya no se trackea en git (ver .gitignore),
    // así que tampoco debe quedar embebido en el binario compilado. Sin ese
    // archivo local (máquina nueva, clon fresco) se sigue de largo sin
    // contratistas en vez de abortar -- el usuario ROOT y los gafetes ya
    // alcanzan para probar login/UI.
    match std::fs::read_to_string("../../contratistas_base_final_limpia_v15.sql") {
        Ok(sql_contratistas) => conexion
            .execute_batch(&sql_contratistas)
            .expect("fallo insertando contratistas"),
        Err(_) => eprintln!(
            "aviso: no se encontró contratistas_base_final_limpia_v15.sql en la raíz del \
             repo -- se sigue sin contratistas de prueba"
        ),
    }

    conexion
        .execute_batch(include_str!("seed_gafetes.sql"))
        .expect("fallo insertando gafetes");

    conexion
        .execute_batch(include_str!("seed_rutas.sql"))
        .expect("fallo insertando catálogo de rutas");

    println!("Semilla de desarrollo cargada en {ruta}");
}
