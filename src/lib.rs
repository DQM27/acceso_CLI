pub mod application;
pub mod database;
#[cfg(feature = "terminal-ui")]
mod diseno_generado;
pub mod domain;
pub mod historial;
pub mod instancia;
pub mod interfaz_preferida;
// Sin feature gate a propósito: parser+resolver+ContextState no dependen de
// terminal (ver su doc-comment) — cualquier interfaz puede reusar el mismo
// lenguaje de comandos sin arrastrar ratatui/crossterm/tui-input.
pub mod lenguaje_comandos;
pub mod mensajes;
pub mod models;
#[cfg(feature = "nube")]
pub mod nube;
pub mod services;
pub mod texto;
pub mod tiempo;

// `cifrado-sqlcipher` y `sqlite-plano` compilan versiones incompatibles de
// `libsqlite3-sys` (vendorizado con o sin OpenSSL) -- las dos a la vez no
// tiene sentido y probablemente ni compile limpio. Ninguna de las dos
// tampoco es un error silencioso más grave: sin alguna, `rusqlite` no tiene
// ningún motor SQLite vendorizado para enlazar.
#[cfg(all(feature = "cifrado-sqlcipher", feature = "sqlite-plano"))]
compile_error!(
    "cifrado-sqlcipher y sqlite-plano son mutuamente excluyentes -- elegí una sola \
     (ver Cargo.toml)"
);
#[cfg(not(any(feature = "cifrado-sqlcipher", feature = "sqlite-plano")))]
compile_error!(
    "falta elegir un motor SQLite: activá la feature cifrado-sqlcipher (real, default) o \
     sqlite-plano (rápido para iterar local, sin cifrar -- ver Cargo.toml)"
);

#[cfg(feature = "terminal-ui")]
pub mod cli;
#[cfg(feature = "terminal-ui")]
pub mod tui;
