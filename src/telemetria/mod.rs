//! Qué mide este equipo: los estudios (grupos de eventos) encendidos por la
//! orden del panel (`docs/features-futuras/plan-telemetria-y-registro-de-errores.md`).
//!
//! Una sola definición para el móvil y el escritorio:
//!
//! - [`grupos`]: los grupos y a cuál pertenece cada tipo de evento.
//! - [`config`]: el estado en memoria que consulta cada punto de medición
//!   (una lectura atómica, sin tocar la base) y la regla para aplicar una orden
//!   nueva.
//!
//! El guardado local de la última orden está en
//! [`crate::database::config_telemetria`].

pub mod config;
pub mod grupos;

pub use config::{ConfigTelemetria, Orden, config};
pub use grupos::{CATALOGO, Grupo, Grupos, grupo_de_evento};
