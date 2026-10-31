//! La telemetría del teléfono sale por el núcleo
//! (`docs/features-futuras/plan-red-movil-por-el-nucleo.md`): Kotlin sigue
//! midiendo y le entrega los eventos ya armados; el núcleo los guarda en su
//! cola en disco y los manda con su cliente HTTP/2, brotli y el token del
//! equipo, "a caballo" de otro tráfico (`control_acceso::telemetria`). Antes,
//! Kotlin los mandaba por su cuenta a staging con `HttpURLConnection`
//! (HTTP/1.1, una conexión nueva cada minuto).

use std::sync::OnceLock;

use control_acceso::telemetria::cola::ColaTelemetria;
use control_acceso::telemetria::disparador::{Perfil, ultima_actividad_red};
use control_acceso::telemetria::envio::{Destino, RitmoEnvio, enviar_si_conviene};
use control_acceso::telemetria::lote::EventoTelemetria;
use serde_json::{Map, Value};

use crate::Nucleo;

struct EnvioTelemetria {
    cola: ColaTelemetria,
    sesion: Value,
    ritmo: RitmoEnvio,
}

/// Uno por proceso: la telemetría es de la app, no de una base abierta.
static ENVIO: OnceLock<EnvioTelemetria> = OnceLock::new();

fn ahora_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// La cabecera que manda Kotlin, con lo que la nube exige asegurado: la
/// plataforma es `movil` y la versión, sólo semver (`1.8.13`, aunque el build
/// diga `1.8.13-algo`). `None` si no es un objeto JSON o no trae `id`.
fn cabecera_saneada(json: &str) -> Option<Value> {
    let mut cabecera: Map<String, Value> = serde_json::from_str::<Value>(json)
        .ok()?
        .as_object()?
        .clone();
    cabecera.get("id")?.as_str()?;
    cabecera.insert("plataforma".into(), Value::from("movil"));
    let version = cabecera
        .get("version_app")
        .and_then(Value::as_str)
        .and_then(semver_de)
        .unwrap_or_else(|| "0.0.0".to_owned());
    cabecera.insert("version_app".into(), Value::from(version));
    Some(Value::Object(cabecera))
}

/// Los tres primeros números de una versión (`1.8.13-debug` → `1.8.13`).
fn semver_de(version: &str) -> Option<String> {
    let numeros: Vec<&str> = version
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?
        .split('.')
        .take(3)
        .collect();
    (numeros.len() == 3 && numeros.iter().all(|n| !n.is_empty() && n.len() <= 4))
        .then(|| numeros.join("."))
}

#[uniffi::export]
impl Nucleo {
    /// Prepara el envío de la telemetría: la cola en disco en `directorio` (el
    /// de la app) y la cabecera de esta sesión (JSON). Una vez por proceso;
    /// `false` si la cabecera no sirve o ya estaba iniciada.
    pub fn telemetria_iniciar(&self, directorio: String, cabecera_json: String) -> bool {
        let Some(sesion) = cabecera_saneada(&cabecera_json) else {
            return false;
        };
        ENVIO
            .set(EnvioTelemetria {
                cola: ColaTelemetria::en(std::path::Path::new(&directorio)),
                sesion,
                ritmo: RitmoEnvio::desde(ahora_ms()),
            })
            .is_ok()
    }

    /// Guarda en la cola en disco los eventos que midió Kotlin, cada uno como
    /// una línea `{"t": ms, "tipo": ..., "d": {...}}`. Devuelve cuántos se
    /// perdieron (ilegibles o por el tope de la cola). Sin `telemetria_iniciar`
    /// se pierden todos: Kotlin no llama antes.
    pub fn telemetria_agregar(&self, lineas: Vec<String>) -> u32 {
        let total = u32::try_from(lineas.len()).unwrap_or(u32::MAX);
        let Some(envio) = ENVIO.get() else {
            return total;
        };
        let eventos: Vec<EventoTelemetria> = lineas
            .iter()
            .filter_map(|linea| EventoTelemetria::desde_linea(linea))
            .collect();
        let ilegibles = lineas.len() - eventos.len();
        let por_tope = envio.cola.agregar(&eventos).unwrap_or(eventos.len());
        u32::try_from(ilegibles + por_tope).unwrap_or(u32::MAX)
    }

    /// Una vuelta del hilo de telemetría de Kotlin: manda sólo si conviene
    /// (la radio ya está despierta por otra petición, pasó mucho sin mandar o
    /// se juntó mucho; ver `telemetria::disparador`). Barato si no conviene:
    /// no toca la red ni pide el token. `true` si mandó algo.
    pub fn telemetria_enviar_si_conviene(&self) -> bool {
        let Some(envio) = ENVIO.get() else {
            return false;
        };
        let cliente = control_acceso::nube::cliente_http();
        let destino = Destino {
            cliente: &cliente,
            base_url: control_acceso::nube::base_url(),
            apikey: control_acceso::nube::apikey(),
        };
        enviar_si_conviene(
            Perfil::Movil,
            &envio.ritmo,
            ahora_ms(),
            ultima_actividad_red(),
            &envio.cola,
            &envio.sesion,
            || {
                self.autenticar_con_cache()
                    .ok()
                    .map(|token| token.access_token)
            },
            &destino,
            control_acceso::database::identificador::generar_uuid_v4,
        )
        .is_some_and(|resumen| resumen.lotes > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_version_queda_en_semver() {
        assert_eq!(semver_de("1.8.13").as_deref(), Some("1.8.13"));
        assert_eq!(semver_de("1.8.13-debug").as_deref(), Some("1.8.13"));
        assert_eq!(semver_de("1.8.13.4").as_deref(), Some("1.8.13"));
        assert_eq!(semver_de("1.8"), None);
        assert_eq!(semver_de("desktop-1.10.7"), None);
        assert_eq!(semver_de(""), None);
    }

    #[test]
    fn la_cabecera_asegura_plataforma_y_version() {
        let cabecera = cabecera_saneada(
            r#"{"id":"dddddddd-0000-0000-0000-000000000001","plataforma":"escritorio","version_app":"1.8.13-debug","version_codigo":58}"#,
        )
        .unwrap();
        assert_eq!(cabecera["plataforma"], "movil");
        assert_eq!(cabecera["version_app"], "1.8.13");
        assert_eq!(cabecera["version_codigo"], 58);
        assert_eq!(cabecera_saneada("[]"), None);
        assert_eq!(
            cabecera_saneada(r#"{"version_app":"1.8.13"}"#),
            None,
            "sin id"
        );
        assert_eq!(cabecera_saneada("no es json"), None);
    }
}
