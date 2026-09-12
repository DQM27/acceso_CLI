use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

const CLAVE: &str = "brisas-lab-sqlcipher-2026-09-10";
const CLAVE_INCORRECTA: &str = "esta-clave-no-debe-abrir-la-db";
const CABECERA_SQLITE: &[u8] = b"SQLite format 3\0";

fn limpiar(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(PathBuf::from(format!("{}-wal", path.display())));
    let _ = fs::remove_file(PathBuf::from(format!("{}-shm", path.display())));
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "brisas-sqlcipher-smoke-{}.db",
        std::process::id()
    ));
    limpiar(&path);

    let version: String;
    {
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "key", CLAVE)?;
        version = conn.pragma_query_value(None, "cipher_version", |row| row.get(0))?;
        if version.trim().is_empty() {
            return Err("PRAGMA cipher_version no devolvió una versión".into());
        }
        conn.execute_batch(
            "CREATE TABLE prueba(id INTEGER PRIMARY KEY, valor TEXT NOT NULL);\n             INSERT INTO prueba(valor) VALUES ('cifrado-ok');",
        )?;
    }

    let bytes = fs::read(&path)?;
    if bytes.starts_with(CABECERA_SQLITE) {
        limpiar(&path);
        return Err("la base conserva la cabecera SQLite en claro; no está cifrada".into());
    }

    {
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "key", CLAVE)?;
        let valor: String = conn.query_row(
            "SELECT valor FROM prueba WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        if valor != "cifrado-ok" {
            limpiar(&path);
            return Err("la reapertura con la clave correcta devolvió datos inesperados".into());
        }
    }

    {
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "key", CLAVE_INCORRECTA)?;
        let resultado = conn.query_row(
            "SELECT count(*) FROM sqlite_master",
            [],
            |row| row.get::<_, i64>(0),
        );
        if resultado.is_ok() {
            limpiar(&path);
            return Err("SQLCipher aceptó una clave incorrecta".into());
        }
    }

    println!("SQLCipher smoke test OK; cipher_version={version}");
    limpiar(&path);
    Ok(())
}
