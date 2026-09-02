//! Integración con el receptor en la nube — solo compilado con la feature
//! `nube`. Ver `docs/plan-persistencia-nube.md` para el diseño completo.
//!
//! Por ahora sólo cubre la autenticación de dispositivo (secreto → token).
//! La bandeja de salida (outbox) que sube/baja datos reales todavía no
//! existe — es el siguiente paso, pendiente de diseño de esquema.

pub mod cliente;
pub mod credenciales;

pub use cliente::{NubeError, TokenDispositivo, autenticar_dispositivo};
