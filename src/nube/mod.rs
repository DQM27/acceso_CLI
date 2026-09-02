//! Integración con el receptor en la nube — solo compilado con la feature
//! `nube`. Ver `docs/plan-persistencia-nube.md` para el diseño completo.
//!
//! Por ahora sólo cubre la autenticación de dispositivo (secreto → token).
//! La bandeja de salida (outbox) que sube/baja datos reales todavía no
//! existe — es el siguiente paso, pendiente de diseño de esquema.

pub mod cliente;
pub mod credenciales;
pub mod sincronizacion;

pub use cliente::{NubeError, TokenDispositivo, autenticar_dispositivo};
pub use sincronizacion::{ContextoSincronizacion, ResumenDrenado, SincronizacionError, drenar_cola};

/// URL del proyecto Supabase (`control-acceso-nube`) -- pública, no un
/// secreto (es la dirección del servidor, no una credencial).
pub const BASE_URL: &str = "https://xidaepyaljzkpbsxrqsm.supabase.co";

/// Clave publicable del proyecto (`sb_publishable_...`) -- por diseño de
/// Supabase, esta clave es segura para viajar en cualquier cliente; sola no
/// da acceso a nada, RLS decide todo según el JWT que la acompañe.
pub const APIKEY: &str = "sb_publishable_Sr9DPGMD7MFirLQfG7ViWg_6pJeEpqU";
