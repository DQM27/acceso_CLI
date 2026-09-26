//! Laboratorio aislado: cliente de Supabase Realtime (Phoenix Channels)
//! escrito desde cero en Rust, sobre `tokio-tungstenite`. Ver `README.md`
//! de este directorio para el criterio de aprobación y el plan por etapas.

pub mod backoff;
pub mod cliente;
pub mod protocolo;
pub mod supervisor;

pub use cliente::{ClienteRealtime, ErrorCliente};
pub use supervisor::{
    ConfigCanalPrivado, EventoSupervisor, EventoSupervisorPrivado, supervisar_canal_privado,
    supervisar_heartbeat,
};
