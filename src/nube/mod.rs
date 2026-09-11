//! Integración con el receptor en la nube — solo compilado con la feature
//! `nube`. Ver `docs/plan-persistencia-nube.md` para el diseño completo.
//!
//! Cubre: autenticación de dispositivo (secreto → token), el envío
//! (`drenar_cola`) de la bandeja de salida local hacia el receptor, y la
//! recepción (`recibir_ingresos_abiertos`/`cerrar_ingreso_remoto`) de lo
//! que el otro dispositivo del mismo sitio tiene abierto ahora mismo.

pub mod cliente;
pub mod credenciales;
pub mod sincronizacion;

pub use cliente::{MetadatosDispositivo, NubeError, TokenDispositivo, autenticar_dispositivo};
pub use sincronizacion::{
    ConflictoIngresoActivo, ContextoSincronizacion, IngresoRemoto, ResumenCatalogo,
    ResumenDrenado, SincronizacionError, cerrar_ingreso_remoto, contar_fallos_permanentes,
    contratista_activo_en_otro_sitio, contratistas_con_conflicto_activo, drenar_cola,
    gafete_ocupado_en_otro_dispositivo, recibir_catalogo_del_sitio,
    recibir_cierres_de_ingresos_propios, recibir_citas_del_sitio, recibir_historial_del_sitio,
    recibir_historial_visitas_del_sitio, recibir_ingresos_abiertos, usuario_sigue_activo_remoto,
};

/// URL del proyecto Supabase (`control-acceso-nube`) -- pública, no un
/// secreto (es la dirección del servidor, no una credencial).
pub const BASE_URL: &str = "https://xidaepyaljzkpbsxrqsm.supabase.co";

/// Clave publicable del proyecto (`sb_publishable_...`) -- por diseño de
/// Supabase, esta clave es segura para viajar en cualquier cliente; sola no
/// da acceso a nada, RLS decide todo según el JWT que la acompañe.
pub const APIKEY: &str = "sb_publishable_Sr9DPGMD7MFirLQfG7ViWg_6pJeEpqU";
