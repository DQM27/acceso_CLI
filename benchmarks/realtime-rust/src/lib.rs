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

/// Instala el `CryptoProvider` de `rustls` que necesita `connect_async` para
/// hablar `wss://` -- TOLERANTE a que ya haya uno instalado, a propósito.
///
/// Hallazgo real al evaluar la integración con `desktop/src-tauri` (ver
/// README.md, "Etapa 4 -- integración real"): ese árbol de dependencias ya
/// trae `reqwest` con la feature `rustls-tls` (para `nube`, las llamadas
/// REST a Supabase) -- `reqwest`/`hyper-rustls` instalan su propio
/// `CryptoProvider` por su cuenta la primera vez que arman un cliente
/// HTTPS, sin que el código de la app lo pida explícito en ningún lado
/// (confirmado: no hay ningún `install_default()` fuera de este
/// laboratorio). En un proceso donde ambos coexisten, quien llegue
/// primero (reqwest o este cliente) instala el proveedor real -- lo único
/// que importa es que sea EL MISMO backend (`ring`, ver `Cargo.toml`),
/// nunca cuál de los dos ganó la carrera. Los binarios `smoke_*` de este
/// laboratorio, al correr solos (nunca junto a `reqwest`), sí podían
/// darse el lujo de `.expect()` -- una integración real no puede.
pub fn instalar_crypto_provider_tolerante() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
