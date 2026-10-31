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
//! - [`lote`]: el lote que viaja a la nube (formato columnar + brotli).
//! - [`cola`]: los eventos esperando en disco, sin pérdidas ni duplicados.
//! - [`envio`]: el envío a la receptora (HTTP/2, brotli, token del equipo).
//! - [`disparador`]: cuándo mandar (escritorio cada 60 s, móvil "a caballo"
//!   de otro tráfico).
//!
//! El guardado local de la última orden está en
//! [`crate::database::config_telemetria`].

#[cfg(feature = "nube")]
pub mod cola;
pub mod config;
pub mod disparador;
#[cfg(feature = "nube")]
pub mod envio;
pub mod grupos;
#[cfg(feature = "nube")]
pub mod lote;

pub use config::{ConfigTelemetria, Orden, config};
pub use grupos::{CATALOGO, Grupo, Grupos, grupo_de_evento};
