use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

const CLAVE: &str = "brisas-lab-sqlite3mc-2026-09-10";
const CLAVE_INCORRECTA: &str = "esta-clave-no-debe-abrir-la-db";
const CABECERA_SQLITE: &[u8] = b"SQLite format 3\0";

fn limpiar(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(PathBuf::from(format!("{}-wal", path.display())));
    let _ = fs::remove_file(PathBuf::from(format!("{}-shm", path.display())));
}

fn configurar(conn: &Connection, clave: &str) -> rusqlite::Result<()> {
    // SQLite3MC recomienda configurar el cipher antes de la clave. ChaCha20-
    // Poly1305 es su esquema autenticado recomendado/default actual.
    conn.pragma_update(None, "cipher", "chacha20")?;
    conn.pragma_update(None, "key", clave)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "brisas-sqlite3mc-smoke-{}.db",
        std::process::id()
    ));
    limpiar(&path);

    {
        let conn = Connection::open(&path)?;
        configurar(&conn, CLAVE)?;
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
        configurar(&conn, CLAVE)?;
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
        configurar(&conn, CLAVE_INCORRECTA)?;
        let resultado = conn.query_row(
            "SELECT count(*) FROM sqlite_master",
            [],
            |row| row.get::<_, i64>(0),
        );
        if resultado.is_ok() {
            limpiar(&path);
            return Err("SQLite3MC aceptó una clave incorrecta".into());
        }
    }

    println!("SQLite3MC smoke test OK; cipher=chacha20-poly1305");
    limpiar(&path);
    Ok(())
}
