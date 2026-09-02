//! Corre un script SQL de carga masiva contra la base real, con los mismos
//! pasos de seguridad que usa la app (bloqueo de instancia, respaldo
//! pre-migración, migraciones al día). Uso:
//!
//! ```text
//! cargo run --example importar_contratistas -- <archivo.sql> [ruta_db]
//! ```
//!
//! Sin `ruta_db`, usa la misma resolución que la app real
//! (`CONTROL_ACCESO_DB` o `%LOCALAPPDATA%\ControlAcceso\control_acceso.db`).

use std::{env, fs, path::PathBuf};

use control_acceso::database::connection::{open_database, ruta_base_datos};
use control_acceso::instancia::InstanciaGuard;

fn main() {
    let mut args = env::args().skip(1);
    let sql_path = PathBuf::from(
        args.next()
            .expect("uso: importar_contratistas <archivo.sql> [ruta_db]"),
    );
    let db_path = args.next().map_or_else(
        || ruta_base_datos().expect("no se pudo resolver la ruta de la base de datos"),
        PathBuf::from,
    );

    println!("Base de datos: {}", db_path.display());
    println!("Script SQL:    {}", sql_path.display());

    let _instancia = InstanciaGuard::adquirir(&db_path)
        .expect("no se pudo adquirir el bloqueo de instancia (¿la app está abierta?)");

    let connection =
        open_database(&db_path).expect("no se pudo abrir/migrar la base de datos");

    let sql = fs::read_to_string(&sql_path).expect("no se pudo leer el archivo SQL");

    let contratistas_antes: i64 = connection
        .query_row("SELECT COUNT(*) FROM contratistas", [], |r| r.get(0))
        .unwrap();
    let empresas_antes: i64 = connection
        .query_row("SELECT COUNT(*) FROM empresas", [], |r| r.get(0))
        .unwrap();

    connection
        .execute_batch(&sql)
        .expect("fallo al ejecutar el script de import");

    let contratistas_despues: i64 = connection
        .query_row("SELECT COUNT(*) FROM contratistas", [], |r| r.get(0))
        .unwrap();
    let empresas_despues: i64 = connection
        .query_row("SELECT COUNT(*) FROM empresas", [], |r| r.get(0))
        .unwrap();

    println!(
        "empresas: {empresas_antes} -> {empresas_despues} ({:+})",
        empresas_despues - empresas_antes
    );
    println!(
        "contratistas: {contratistas_antes} -> {contratistas_despues} ({:+})",
        contratistas_despues - contratistas_antes
    );
}
