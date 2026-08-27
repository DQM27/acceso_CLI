//! Comandos Tauri agrupados por dominio — un archivo por área de negocio
//! (misma idea que `src/application/*.rs` en el núcleo). Cada uno es una
//! función fina que llama a `AppCore`; la lógica real nunca vive acá.

pub mod autenticacion;
pub mod contratistas;
pub mod empresas;
pub mod usuarios;
