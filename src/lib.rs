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

#[cfg(feature = "terminal-ui")]
pub mod cli;
#[cfg(feature = "terminal-ui")]
pub mod tui;
