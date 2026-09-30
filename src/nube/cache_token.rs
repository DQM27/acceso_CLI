//! Caché compartida del `TokenDispositivo` -- reemplaza tres copias
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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use super::cliente::{MetadatosDispositivo, NubeError, TokenDispositivo};
use super::cliente::{autenticar_con_firmante, vincular_con_codigo};
use super::firmante::FirmanteDispositivo;

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
    /// Dispositivo al que pertenece el token: un token sólo se reusa para
    /// la misma vinculación (re-vincular cambia de clave y lo descarta).
    dispositivo_id: String,
    token: TokenDispositivo,
    obtenido_en: Instant,
}

/// Caché de un único `TokenDispositivo` por instancia. Un `AppCore`, un
/// `GuiState` o un `Nucleo` sostienen exactamente una de estas -- ver el
/// doc-comment del módulo sobre por qué debe ser un campo hermano del
/// candado de `SQLite`, nunca un campo interno protegido por el mismo
/// candado.
///
/// La única credencial del equipo es su clave (ver `nube::firmante`): la
/// caché la recibe una vez con [`Self::establecer_firmante`] y desde ahí se
/// autentica sola.
pub struct CacheTokenDispositivo {
    entrada: Mutex<Option<EntradaCache>>,
    /// Quién firma por este equipo. `None` sólo en procesos que no hablan
    /// con la nube (tests): cualquier autenticación falla con
    /// [`NubeError::SinCredencial`].
    firmante: Mutex<Option<Arc<dyn FirmanteDispositivo>>>,
    /// Desfase en milisegundos medido en segundo plano (ver
    /// [`Self::guardar`]) que todavía nadie aplicó: viaja una sola vez, en
    /// el próximo token que se entregue.
    desfase_pendiente: Arc<Mutex<Option<i64>>>,
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
            firmante: Mutex::new(None),
            desfase_pendiente: Arc::new(Mutex::new(None)),
        }
    }

    /// Configura quién firma por este equipo. Se llama una vez al arrancar
    /// (escritorio con `FirmanteArchivo`, móvil con el de Keystore).
    pub fn establecer_firmante(&self, firmante: Arc<dyn FirmanteDispositivo>) {
        *self
            .firmante
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(firmante);
        self.invalidar();
    }

    fn firmante(&self) -> Option<Arc<dyn FirmanteDispositivo>> {
        self.firmante
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// `dispositivo_id` de la clave vinculada, si la hay.
    pub fn dispositivo_vinculado(&self) -> Option<String> {
        self.firmante()?.dispositivo_vinculado()
    }

    /// `true` si este equipo está vinculado a la nube (tiene clave aceptada
    /// por el servidor). Sin vinculación la nube está "sin configurar": no
    /// hay con quién chocar y ningún chequeo remoto toca la red.
    pub fn vinculado(&self) -> bool {
        self.dispositivo_vinculado().is_some()
    }

    /// Huella RFC 7638 de la clave vinculada, si la hay. El aviso en vivo
    /// `dispositivo_expulsado` trae la huella del equipo que queda fuera: al
    /// re-vincular, el equipo nuevo comparte `dispositivo_id` con el viejo y
    /// sólo comparando la huella sabe que el aviso no es para él.
    pub fn huella_vinculada(&self) -> Option<String> {
        let firmante = self.firmante()?;
        firmante.dispositivo_vinculado()?;
        let jwk = firmante.clave_publica_jwk().ok()?;
        super::firmante::ClavePublicaJwk::desde_json(&jwk)
            .ok()
            .map(|clave| clave.huella())
    }

    /// Canjea un código de vinculación del panel y cachea el primer token.
    ///
    /// Si la clave vigente ya estaba vinculada (re-vincular), se estrena
    /// una nueva antes de canjear: cada vinculación usa su propia clave. Si
    /// todavía no estaba vinculada (primer arranque, o un intento anterior
    /// con un código equivocado), se reutiliza.
    ///
    /// `dispositivo_esperado`: si viene, el código tiene que ser de ese
    /// dispositivo; se usa al re-vincular un equipo que ya tiene datos
    /// locales, para no atarlo por error a otro dispositivo u otro sitio.
    pub fn vincular(
        &self,
        codigo: &str,
        dispositivo_esperado: Option<&str>,
        metadata: Option<&MetadatosDispositivo>,
    ) -> Result<TokenDispositivo, NubeError> {
        let firmante = self.firmante().ok_or(NubeError::SinCredencial)?;
        if firmante.dispositivo_vinculado().is_some() {
            firmante.regenerar()?;
        }
        let jwk = firmante.clave_publica_jwk()?;
        let token = vincular_con_codigo(
            super::base_url(),
            codigo,
            &jwk,
            dispositivo_esperado,
            metadata,
        )?;
        firmante.marcar_vinculada(&token.dispositivo_id)?;
        self.guardar(&token);
        Ok(token)
    }

    /// Reusa el último token mientras siga vigente para la misma
    /// vinculación; si no, se autentica con la clave (desafío + aserción
    /// firmada) y lo cachea. Un acierto de caché no vuelve a medir el
    /// desfase de reloj (`TokenDispositivo::desfase_reloj_ms` queda en
    /// `None`, salvo que traiga la medición en milisegundos que terminó en
    /// segundo plano) -- sólo se mide cuando de verdad se habla con el
    /// receptor.
    ///
    /// Quien llama es responsable de aplicar `desfase_reloj_ms` a su
    /// propio reloj cuando venga `Some` (ver
    /// `AppCore::actualizar_desfase_reloj`) -- este tipo no conoce ningún
    /// reloj, sólo el token.
    pub fn autenticar_con_cache(&self) -> Result<TokenDispositivo, NubeError> {
        self.autenticar_y_cachear(None)
    }

    /// Igual que [`Self::autenticar_con_cache`], pero adjuntando `metadata`
    /// cuando hace falta pedir un token nuevo (versión de la app, para
    /// `VERSION_MINIMA_ACEPTADA`).
    pub fn autenticar_y_cachear(
        &self,
        metadata: Option<&MetadatosDispositivo>,
    ) -> Result<TokenDispositivo, NubeError> {
        let firmante = self.firmante().ok_or(NubeError::SinCredencial)?;
        let dispositivo_id = firmante
            .dispositivo_vinculado()
            .ok_or(NubeError::SinCredencial)?;
        {
            let cache = self
                .entrada
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entrada) = cache.as_ref()
                && entrada.dispositivo_id == dispositivo_id
                && let Some(mut token) = reutilizable(&entrada.token, entrada.obtenido_en.elapsed())
            {
                token.desfase_reloj_ms = tomar(&self.desfase_pendiente);
                return Ok(token);
            }
        }

        let token = autenticar_con_firmante(super::base_url(), firmante.as_ref(), metadata)?;
        self.guardar(&token);
        Ok(token)
    }

    fn guardar(&self, token: &TokenDispositivo) {
        // El token sale ya con el desfase del header `Date` (segundos). La
        // medición en milisegundos (4 consultas, ~0,3-0,6 s en 4G) corre
        // aparte: antes se hacía en línea y la pagaba el login del teléfono
        // (1,3-1,7 s con token nuevo, medido con telemetría). Queda en
        // `desfase_pendiente` y viaja en el próximo token que se entregue
        // (la sincronización que sigue al login, el canal de Realtime).
        // Una medición vieja sin aplicar se descarta: la nueva la reemplaza.
        *self
            .desfase_pendiente
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        medir_en_segundo_plano(&token.access_token, Arc::clone(&self.desfase_pendiente));
        *self
            .entrada
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(EntradaCache {
            dispositivo_id: token.dispositivo_id.clone(),
            token: token.clone(),
            obtenido_en: Instant::now(),
        });
    }

    /// Descarta el token cacheado -- ver
    /// `SincronizacionError::token_dispositivo_vencido`: el receptor lo
    /// rechazó a mitad de una sincronización aunque `autenticar_con_cache`
    /// lo creía vigente (desfase de reloj, o el dispositivo estuvo
    /// inactivo más de lo que el margen de expiración contemplaba). La próxima
    /// llamada pide uno nuevo sin esperar a que el "vigente por" calculado
    /// localmente se cumpla solo. También tras un aviso de expulsión.
    pub fn invalidar(&self) {
        *self
            .entrada
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
}

/// Saca (y deja vacío) el desfase pendiente.
fn tomar(pendiente: &Mutex<Option<i64>>) -> Option<i64> {
    pendiente
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
}

/// Mide el desfase en milisegundos en un hilo propio y lo deja en
/// `destino`. Si no se puede medir (sin la función en el servidor, red
/// lenta) no deja nada y queda el del header `Date`.
fn medir_en_segundo_plano(access_token: &str, destino: Arc<Mutex<Option<i64>>>) {
    let access_token = access_token.to_string();
    let lanzado = std::thread::Builder::new()
        .name("reloj-preciso".to_string())
        .spawn(move || {
            if let Some(preciso) = super::reloj_preciso::medir_desfase_ms(
                super::base_url(),
                super::apikey(),
                &access_token,
            ) {
                *destino
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(preciso);
            }
        });
    if let Err(error) = lanzado {
        log::warn!("no se pudo lanzar la medición del reloj: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use std::sync::Mutex;

    use super::{MARGEN_EXPIRACION, TokenDispositivo, reutilizable, tomar};

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

    #[test]
    fn el_desfase_medido_en_segundo_plano_viaja_una_sola_vez() {
        let pendiente = Mutex::new(Some(-240));
        assert_eq!(tomar(&pendiente), Some(-240));
        assert_eq!(
            tomar(&pendiente),
            None,
            "ya aplicado: los siguientes aciertos no lo repiten"
        );
    }
}
