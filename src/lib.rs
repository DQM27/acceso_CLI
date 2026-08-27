pub mod application;
pub mod database;
pub mod domain;
pub mod historial;
pub mod instancia;
pub mod interfaz_preferida;
pub mod mensajes;
pub mod models;
pub mod services;
pub mod texto;
pub mod tiempo;

#[cfg(feature = "terminal-ui")]
pub mod comandos;
#[cfg(feature = "terminal-ui")]
pub mod tui;
