//! Integración con el receptor en la nube — solo compilado con la feature
//! `nube`. Ver `docs/planes-implementados/plan-persistencia-nube.md` para el diseño completo.
//!
//! Cubre: autenticación de dispositivo (secreto → token), el envío
//! (`drenar_cola`) de la bandeja de salida local hacia el receptor, y la
//! recepción (`recibir_ingresos_abiertos`/`cerrar_ingreso_remoto`) de lo
//! que el otro dispositivo del mismo sitio tiene abierto ahora mismo.

pub mod auth_supabase;
pub mod cache_token;
pub mod cliente;
pub mod credenciales;
pub mod sincronizacion;

pub use auth_supabase::{
    AuthSupabaseError, Jwk, SesionSupabase, cambiar_password, login, obtener_jwks, refrescar,
    verificar_token_offline,
};
pub use cache_token::CacheTokenDispositivo;
pub use cliente::{MetadatosDispositivo, NubeError, TokenDispositivo, autenticar_dispositivo};
pub use sincronizacion::{
    ConflictoIngresoActivo, ConflictoIngresoProveedorActivo, ConflictoMovimientoVisitaActivo,
    ContextoSincronizacion, IngresoProveedorRemoto, IngresoRemoto, PrestamoGafeteProvisionalRemoto,
    ResumenCatalogo, ResumenCatalogoRutas, ResumenDrenado, SincronizacionError,
    cerrar_ingreso_proveedor_remoto, cerrar_ingreso_remoto,
    cerrar_prestamo_gafete_provisional_remoto, contar_fallos_permanentes,
    contratista_activo_en_otro_sitio, contratistas_con_conflicto_activo, drenar_cola,
    gafete_de_proveedor_ocupado_en_otro_dispositivo, gafete_de_visita_ocupado_en_otro_dispositivo,
    gafete_ocupado_en_otro_dispositivo, gafete_provisional_ocupado_en_otro_dispositivo,
    proveedor_activo_en_otro_sitio, proveedores_con_conflicto_activo, recibir_catalogo_del_sitio,
    recibir_catalogo_rutas_del_sitio, recibir_cierres_de_ingresos_propios,
    recibir_cierres_de_ingresos_propios_proveedor, recibir_citas_del_sitio,
    recibir_devoluciones_propias_gafete_provisional, recibir_historial_del_sitio,
    recibir_historial_gafetes_provisionales_del_sitio,
    recibir_historial_ingresos_proveedor_del_sitio, recibir_historial_visitas_del_sitio,
    recibir_ingresos_abiertos, recibir_ingresos_proveedor_abiertos,
    recibir_prestamos_gafete_provisional_abiertos, usuario_sigue_activo_remoto,
    visitante_activo_en_otro_sitio, visitantes_con_conflicto_activo,
};

const BASE_URL_ENV: &str = "CONTROL_ACCESO_SUPABASE_URL";
const APIKEY_ENV: &str = "CONTROL_ACCESO_SUPABASE_APIKEY";

/// URL del proyecto Supabase activo -- pública, no un secreto (es la
/// dirección del servidor, no una credencial). Por defecto, producción
/// (`control-acceso-nube`); overrideable en tiempo de ejecución con
/// `CONTROL_ACCESO_SUPABASE_URL` para apuntar a un proyecto de staging
/// (ver `docs/recuperacion-sitio-staging.md`) sin tocar ni recompilar
/// código -- mismo criterio que `database::connection::DATABASE_PATH_ENV`:
/// una variable de entorno con default de producción, no algo que haya
/// que setear siempre. Resuelta una sola vez por proceso (`OnceLock`): un
/// dispositivo no cambia de proyecto a mitad de ejecución.
pub fn base_url() -> &'static str {
    static VALOR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VALOR.get_or_init(|| resolver_base_url(std::env::var(BASE_URL_ENV).ok()))
}

/// Parte pura de [`base_url`], separada para poder testearla sin depender
/// de variables de entorno globales (mutarlas en tests es inherentemente
/// no seguro entre threads) -- mismo criterio que
/// `database::connection::resolver_ruta_base_datos`.
fn resolver_base_url(valor_env: Option<String>) -> String {
    valor_env.unwrap_or_else(|| "https://xidaepyaljzkpbsxrqsm.supabase.co".to_string())
}

/// Clave publicable del proyecto activo (`sb_publishable_...`) -- por
/// diseño de Supabase, esta clave es segura para viajar en cualquier
/// cliente; sola no da acceso a nada, RLS decide todo según el JWT que la
/// acompañe. Mismo mecanismo de override que [`base_url`], variable
/// `CONTROL_ACCESO_SUPABASE_APIKEY` -- las dos viajan juntas, apuntar la
/// URL a staging sin cambiar también la apikey no tendría sentido.
pub fn apikey() -> &'static str {
    static VALOR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VALOR.get_or_init(|| resolver_apikey(std::env::var(APIKEY_ENV).ok()))
}

/// Ver [`resolver_base_url`] -- misma razón de ser.
fn resolver_apikey(valor_env: Option<String>) -> String {
    valor_env.unwrap_or_else(|| "sb_publishable_Sr9DPGMD7MFirLQfG7ViWg_6pJeEpqU".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_sin_override_usa_produccion() {
        assert_eq!(
            resolver_base_url(None),
            "https://xidaepyaljzkpbsxrqsm.supabase.co"
        );
    }

    #[test]
    fn base_url_con_override_lo_respeta() {
        assert_eq!(
            resolver_base_url(Some("https://staging.example.supabase.co".to_string())),
            "https://staging.example.supabase.co"
        );
    }

    #[test]
    fn apikey_sin_override_usa_produccion() {
        assert_eq!(
            resolver_apikey(None),
            "sb_publishable_Sr9DPGMD7MFirLQfG7ViWg_6pJeEpqU"
        );
    }

    #[test]
    fn apikey_con_override_lo_respeta() {
        assert_eq!(
            resolver_apikey(Some("sb_publishable_de_staging".to_string())),
            "sb_publishable_de_staging"
        );
    }
}
