pub mod application;
pub mod database;
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
pub mod services;
pub mod texto;
pub mod tiempo;

#[cfg(feature = "terminal-ui")]
pub mod comandos;
#[cfg(feature = "terminal-ui")]
pub mod tui;
