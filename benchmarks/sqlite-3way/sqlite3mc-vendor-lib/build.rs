//! Compila `vendor/sqlite3mc_amalgamation.c` (SQLite3 Multiple Ciphers
//! 2.5.1 sobre SQLite 3.53.4 -- procedencia y checksum en
//! `vendor/README-fuente.md`) como librería ESTÁTICA, sin DLL/import
//! library en tiempo de ejecución. Mismos flags que usa `libsqlite3-sys`
//! para compilar su propio `sqlite3.c` bajo la feature `bundled` (ver ese
//! `build.rs` en el registry de Cargo) -- replicados a propósito para que
//! el benchmark de 3 motores compare sobre la misma superficie de
//! features (FTS5/JSON1/RTREE/etc.), no sólo sobre el cifrado.
//!
//! El resultado se deja en `dist/` (ruta FIJA dentro de este crate, no el
//! `OUT_DIR` con hash que usa Cargo normalmente) a propósito: el crate
//! hermano `sqlite3mc` apunta `SQLITE3_LIB_DIR` ahí vía su
//! `.cargo/config.toml`. Tiene que ser una ruta fija y existir ANTES de
//! que arranque `cargo build/run` de `sqlite3mc` -- `libsqlite3-sys`
//! resuelve su propio `-lstatic=sqlite3` cuando SE COMPILA A SÍ MISMO,
//! antes de que cualquier build script río abajo llegue a ejecutarse, así
//! que no alcanza con que este crate sea sólo una `build-dependency` de
//! `sqlite3mc`: hay que compilarlo aparte, como paso previo (ver
//! `experiments/db-cipher-lab/README.md`).

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=vendor/sqlite3mc_amalgamation.c");
    println!("cargo:rerun-if-changed=vendor/sqlite3mc_amalgamation.h");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dist = manifest_dir.join("dist");
    std::fs::create_dir_all(&dist).expect("no se pudo crear dist/");

    cc::Build::new()
        .file("vendor/sqlite3mc_amalgamation.c")
        .include("vendor")
        .warnings(false)
        .out_dir(&dist)
        // Mismos flags que `libsqlite3-sys` bajo `bundled` (ver su
        // `build.rs`), menos los específicos de SQLCipher -- este
        // amalgamation ya trae su propio codec multi-cipher, no necesita
        // `SQLITE_HAS_CODEC` ni vendorizar OpenSSL.
        .define("SQLITE_CORE", None)
        .define("SQLITE_DEFAULT_FOREIGN_KEYS", "1")
        .define("SQLITE_ENABLE_API_ARMOR", None)
        .define("SQLITE_ENABLE_COLUMN_METADATA", None)
        .define("SQLITE_ENABLE_DBSTAT_VTAB", None)
        .define("SQLITE_ENABLE_FTS3", None)
        .define("SQLITE_ENABLE_FTS3_PARENTHESIS", None)
        .define("SQLITE_ENABLE_FTS5", None)
        .define("SQLITE_ENABLE_JSON1", None)
        .define("SQLITE_ENABLE_LOAD_EXTENSION", "1")
        .define("SQLITE_ENABLE_MEMORY_MANAGEMENT", None)
        .define("SQLITE_ENABLE_RTREE", None)
        .define("SQLITE_ENABLE_STAT4", None)
        .define("SQLITE_SOUNDEX", None)
        .define("SQLITE_THREADSAFE", "1")
        .define("SQLITE_USE_URI", None)
        .compile("sqlite3");

    println!(
        "cargo:warning=SQLite3MC estatico compilado en {}",
        dist.display()
    );
}
