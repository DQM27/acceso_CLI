pub mod application;
pub mod database;
pub mod domain;
pub mod historial;
pub mod instancia;
// Sin feature gate a propósito: parser+resolver+ContextState no dependían de
// terminal -- huérfano desde que se retiraron CLI/TUI (2026-09-12, código en
// la rama `archive/cli-tui-2026-09-12`), se deja por si se retoman en vez de
// borrarlo junto con ellas.
pub mod lenguaje_comandos;
pub mod mensajes;
pub mod models;
#[cfg(feature = "nube")]
pub mod nube;
pub mod services;
pub mod texto;
pub mod tiempo;

// `cifrado-sqlcipher`, `sqlite-plano` y `cifrado-sqlite3mc` compilan
// versiones incompatibles de `libsqlite3-sys` (vendorizado con OpenSSL, sin
// cifrar, o -- a futuro -- con SQLite3 Multiple Ciphers) -- dos o más a la
// vez no tiene sentido y probablemente ni compile limpio. Ninguna de las
// tres tampoco es un error silencioso más grave: sin alguna, `rusqlite` no
// tiene ningún motor SQLite vendorizado para enlazar.
#[cfg(any(
    all(feature = "cifrado-sqlcipher", feature = "sqlite-plano"),
    all(feature = "cifrado-sqlcipher", feature = "cifrado-sqlite3mc"),
    all(feature = "sqlite-plano", feature = "cifrado-sqlite3mc"),
))]
compile_error!(
    "cifrado-sqlcipher, sqlite-plano y cifrado-sqlite3mc son mutuamente excluyentes -- \
     elegí una sola (ver Cargo.toml)"
);
#[cfg(not(any(
    feature = "cifrado-sqlcipher",
    feature = "sqlite-plano",
    feature = "cifrado-sqlite3mc"
)))]
compile_error!(
    "falta elegir un motor SQLite: activá la feature cifrado-sqlcipher (real, default), \
     sqlite-plano (rápido para iterar local, sin cifrar) o cifrado-sqlite3mc (en \
     evaluación, todavía sin motor real enlazado -- ver Cargo.toml)"
);
