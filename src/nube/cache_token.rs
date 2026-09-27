//! Caché compartida del `TokenDispositivo` (ver
//! [`crate::nube::autenticar_dispositivo`]) -- reemplaza tres copias
//! prácticamente idénticas que existían antes de este tipo, cada una con
//! su propio `Mutex<Option<_>>` y el mismo margen de expiración de 30s:
//! `AppCore` (`application::nube`), `GuiState` en escritorio
//! (`desktop/src-tauri/src/estado.rs`) y `Nucleo` en móvil
//! (`mobile/rust-core/src/lib.rs`).
//!
//! Vive AFUERA de `AppCore` a propósito, no como parte de su estado
//! interno -- `AppCore` está protegido por un candado compartido
//! (`core_lock()` en móvil, `GuiState::core()` en escritorio) que sostiene
//! CUALQUIER operación de `SQLite` del proceso mientras esté tomado. Si
//! este caché viviera adentro de `AppCore`, autenticar contra la nube (una
//! llamada de red, potencialmente varios cientos de milisegundos) tendría
//! que sostener ese mismo candado, bloqueando cualquier otra consulta
//! `SQLite` mientras tanto. Esto no es hipotético: ya pasó en producción
//! -- "Registrar" con gafete se sentía como que la app se colgaba en cada
//! registro, porque el chequeo de gafete-en-otro-dispositivo retenía el
//! candado compartido durante la autenticación. Escritorio y móvil ya lo
//! resolvieron cada uno por su lado sosteniendo su propio caché en un
//! campo HERMANO del candado, nunca adentro -- este tipo es esa misma
//! solución escrita una sola vez: quien lo use debe seguir sosteniéndolo
//! como campo hermano de su `AppCore`, no envuelto por el mismo candado.
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::cliente::autenticar_dispositivo;
pub use super::cliente::{MetadatosDispositivo, NubeError, TokenDispositivo};

/// Margen antes del vencimiento real del token en el que ya no se
/// considera "vigente" -- no arrancar una operación con un token que
/// puede vencer a mitad de camino.
///
/// Antes eran 30s, y eso rompía el canal de Realtime en sesiones largas:
/// el canal (`desktop/src/nubeRealtime.ts`, `NubeRealtime.kt`) le pide su
/// JWT a ESTA caché al (re)conectarse y programa la renovación a partir de
/// `expires_in`; la caché devolvía el mismo token viejo hasta 30s antes de
/// vencer, así que una renovación que caía en esa ventana se llevaba un
/// token a punto de vencer, Supabase cerraba el canal y la app quedaba
/// sólo con el pulso de 2 minutos. Con este margen, una renovación
/// programada a tiempo siempre recibe un token nuevo. Costo: pedir el
/// token (12h de vida) 15 minutos antes.
const MARGEN_EXPIRACION: Duration = Duration::from_secs(15 * 60);

/// El token cacheado, si todavía sirve pasado `transcurrido` desde que se
/// obtuvo -- con `expires_in` AJUSTADO a lo que de verdad le queda. Antes
/// se devolvía con el `expires_in` original (12h) aunque el token ya
/// tuviera horas encima: quien programaba su propia renovación a partir de
/// ese número (el canal de Realtime, ver arriba) la programaba horas tarde
/// y el canal moría con el token vencido. Separada
/// de `CacheTokenDispositivo` para poder probarla sin red.
fn reutilizable(token: &TokenDispositivo, transcurrido: Duration) -> Option<TokenDispositivo> {
    let vigente_por = Duration::from_secs(token.expires_in).saturating_sub(MARGEN_EXPIRACION);
    if transcurrido >= vigente_por {
        return None;
    }
    let mut vigente = token.clone();
    vigente.expires_in = token.expires_in.saturating_sub(transcurrido.as_secs());
    vigente.desfase_reloj_ms = None;
    Some(vigente)
}

struct EntradaCache {
    secreto: String,
    token: TokenDispositivo,
    obtenido_en: Instant,
}

/// Caché de un único `TokenDispositivo` por instancia. Un `AppCore`, un
/// `GuiState` o un `Nucleo` sostienen exactamente una de estas -- ver el
/// doc-comment del módulo sobre por qué debe ser un campo hermano del
/// candado de `SQLite`, nunca un campo interno protegido por el mismo
/// candado.
pub struct CacheTokenDispositivo {
    entrada: Mutex<Option<EntradaCache>>,
}

impl Default for CacheTokenDispositivo {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheTokenDispositivo {
    pub fn new() -> Self {
        Self {
            entrada: Mutex::new(None),
        }
    }

    /// Reusa el último token mientras siga vigente para el MISMO secreto;
    /// si no, autentica de nuevo contra `device-auth` y lo cachea. Un
    /// acierto de caché no vuelve a medir el desfase de reloj
    /// (`TokenDispositivo::desfase_reloj_ms` queda en `None`) -- sólo se
    /// mide cuando de verdad se habla con el receptor.
    ///
    /// Quien llama es responsable de aplicar `desfase_reloj_ms` a su
    /// propio reloj cuando venga `Some` (ver
    /// `AppCore::actualizar_desfase_reloj`) -- este tipo no conoce ningún
    /// reloj, sólo el token; exactamente el mismo reparto de
    /// responsabilidades que ya tenían las tres copias que reemplaza.
    pub fn autenticar_con_cache(&self, secreto: &str) -> Result<TokenDispositivo, NubeError> {
        self.autenticar_y_cachear(secreto, None)
    }

    /// Igual que [`Self::autenticar_con_cache`], pero permite adjuntar
    /// `metadata` cuando hace falta mandarla (activación inicial del
    /// dispositivo, o cualquier renovación si quien llama ya la arma con
    /// la versión de la app -- ver `AppCore::autenticar_y_cachear`, que
    /// hace exactamente eso).
    pub fn autenticar_y_cachear(
        &self,
        secreto: &str,
        metadata: Option<&MetadatosDispositivo>,
    ) -> Result<TokenDispositivo, NubeError> {
        {
            let cache = self
                .entrada
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entrada) = cache.as_ref()
                && entrada.secreto == secreto
                && let Some(token) = reutilizable(&entrada.token, entrada.obtenido_en.elapsed())
            {
                return Ok(token);
            }
        }

        let token = autenticar_dispositivo(super::base_url(), secreto, metadata)?;
        *self
            .entrada
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(EntradaCache {
            secreto: secreto.to_string(),
            token: token.clone(),
            obtenido_en: Instant::now(),
        });
        Ok(token)
    }

    /// Descarta el token cacheado -- ver
    /// `SincronizacionError::token_dispositivo_vencido`: el receptor lo
    /// rechazó a mitad de una sincronización aunque `autenticar_con_cache`
    /// lo creía vigente (desfase de reloj, o el dispositivo estuvo
    /// inactivo más de lo que el margen de expiración contemplaba). La próxima
    /// llamada pide uno nuevo sin esperar a que el "vigente por" calculado
    /// localmente se cumpla solo.
    pub fn invalidar(&self) {
        *self
            .entrada
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    /// Chequeo remoto de "¿esta cédula ya tiene un ingreso activo en OTRO
    /// sitio ahora mismo?" -- `None` sin secreto (nada con qué consultar),
    /// y también `None` ante cualquier falla (sin red, token rechazado,
    /// lo que sea): este chequeo es puramente informativo, mejor esfuerzo
    /// a propósito (`docs/pendientes.md`, "chequeo cruzado de ingresos
    /// abiertos entre sitios") -- sin conexión, el registro sigue local y
    /// el conflicto, si lo hay, se detecta después al sincronizar
    /// (`ConflictoIngresoActivo`). NUNCA debe frenar un registro por sí
    /// solo -- por eso `Option`, no `Result`: quien llama no tiene forma
    /// de distinguir "no hay conflicto" de "no se pudo verificar", y no
    /// debería poder hacerlo, para no tentarse a tratarlos distinto.
    pub fn contratista_activo_en_otro_sitio(&self, secreto: &str, cedula: &str) -> Option<String> {
        if secreto.trim().is_empty() {
            return None;
        }
        let token = self.autenticar_con_cache(secreto).ok()?;
        let contexto = super::ContextoSincronizacion {
            base_url: super::base_url(),
            apikey: super::apikey(),
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        super::contratista_activo_en_otro_sitio(&contexto, cedula)
            .ok()
            .flatten()
    }

    /// Chequeo remoto de "¿este número de gafete ya está activo en este
    /// MISMO sitio, del lado de OTRO dispositivo, ahora mismo?" --
    /// deliberadamente asimétrico respecto de
    /// [`Self::contratista_activo_en_otro_sitio`]: acá SÍ se propaga el
    /// error (`Result`, no `Option`) en vez de asumir "libre" cuando la
    /// consulta falla. No es un descuido ni una inconsistencia a limpiar
    /// -- son dos políticas distintas a propósito:
    ///
    /// - `contratista_activo_en_otro_sitio` es una ADVERTENCIA informativa
    ///   (el registro local ya es válido igual, esto sólo avisa de un
    ///   posible duplicado entre sitios que se puede corregir después).
    /// - Este chequeo es lo único que evita que DOS dispositivos del mismo
    ///   sitio le entreguen el mismo número físico de gafete a dos
    ///   personas distintas a la vez -- si no se puede verificar que está
    ///   libre, no hay forma segura de decir que sí lo está. Sin secreto
    ///   guardado (dispositivo sin nube configurada) no hay con quién
    ///   chocar, por eso el único camino que no toca la red es `Ok(true)`
    ///   explícito -- nunca "silenciar el error y asumir libre".
    ///
    /// Si algún día alguien consolida este método con el de arriba (misma
    /// forma superficial: autenticar, armar contexto, llamar), que la
    /// firma de retorno siga siendo la señal de que hay dos políticas
    /// distintas debajo, no una.
    pub fn gafete_ocupado_en_otro_dispositivo(
        &self,
        secreto: &str,
        numero: i64,
    ) -> Result<bool, super::SincronizacionError> {
        // `?` convierte `NubeError` -> `SincronizacionError` sola
        // (`#[from]` en `SincronizacionError::Red`) -- autenticar es la
        // única parte de este chequeo que puede fallar con el tipo más
        // angosto, `gafete_ocupado_en_otro_dispositivo` ya devuelve el
        // más ancho.
        let token = self.autenticar_con_cache(secreto)?;
        let contexto = super::ContextoSincronizacion {
            base_url: super::base_url(),
            apikey: super::apikey(),
            token: &token.access_token,
            dispositivo_id: &token.dispositivo_id,
            sitio_id: &token.sitio_id,
        };
        super::gafete_ocupado_en_otro_dispositivo(&contexto, numero)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{MARGEN_EXPIRACION, TokenDispositivo, reutilizable};

    fn token_de_12_horas() -> TokenDispositivo {
        TokenDispositivo {
            access_token: "jwt".to_string(),
            expires_in: 12 * 60 * 60,
            sitio_id: "s1".to_string(),
            dispositivo_id: "d1".to_string(),
            tipo: "pc".to_string(),
            desfase_reloj_ms: Some(42),
        }
    }

    #[test]
    fn un_acierto_de_cache_informa_lo_que_de_verdad_le_queda_al_token() {
        let token = reutilizable(&token_de_12_horas(), Duration::from_secs(11 * 60 * 60))
            .expect("a 11h todavía está vigente");
        assert_eq!(
            token.expires_in,
            60 * 60,
            "a las 11h de obtenido le queda 1h, no las 12h originales"
        );
        assert_eq!(
            token.desfase_reloj_ms, None,
            "un acierto no vuelve a medir el reloj"
        );
    }

    #[test]
    fn deja_de_reusarlo_con_margen_suficiente_para_la_renovacion_del_canal() {
        // El canal de Realtime renueva cada 10 minutos: en cualquier
        // instante de los últimos 10 minutos del token, la caché ya tiene
        // que estar entregando uno NUEVO.
        let limite = Duration::from_secs(12 * 60 * 60)
            .checked_sub(MARGEN_EXPIRACION)
            .unwrap();
        let justo_antes = limite.checked_sub(Duration::from_secs(1)).unwrap();
        assert!(MARGEN_EXPIRACION > Duration::from_secs(10 * 60));
        assert!(reutilizable(&token_de_12_horas(), justo_antes).is_some());
        assert!(reutilizable(&token_de_12_horas(), limite).is_none());
        assert!(
            reutilizable(
                &token_de_12_horas(),
                Duration::from_secs(12 * 60 * 60 - 10 * 60)
            )
            .is_none()
        );
    }

    #[test]
    fn un_token_de_vida_menor_al_margen_nunca_se_reusa() {
        let mut corto = token_de_12_horas();
        corto.expires_in = 60;
        assert!(reutilizable(&corto, Duration::ZERO).is_none());
    }
}
