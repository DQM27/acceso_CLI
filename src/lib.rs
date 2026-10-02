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
pub mod reloj_arranque;
pub mod services;
pub mod texto;
pub mod tiempo;
