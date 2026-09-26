//! Backoff exponencial con tope -- misma fórmula que ya usa
//! `desktop/src/nubeRealtime.ts` (`REINTENTO_BASE_MS * 2 ** intentos`,
//! capado en `REINTENTO_TOPE_MS`). Función pura, sin estado -- quien la usa
//! (`supervisor.rs`) es responsable de resetear el contador de intentos a 0
//! apenas una conexión llega a "conectado" de verdad, para que una falla
//! puntual después de andar mucho tiempo bien no arranque desde el tope.

use std::time::Duration;

/// `Duration::as_millis()` devuelve `u128` (puede representar milenios) --
/// para cualquier `base`/`tope` que alguien pase con sentido (segundos,
/// como mucho minutos) nunca se acerca a desbordar un `u64`, pero
/// `as u64` a secas igual dispara `cast_possible_truncation`. `try_from`
/// con `unwrap_or(u64::MAX)` documenta la garantía en vez de silenciar el
/// lint a ciegas: si alguna vez SÍ desbordara, satura al máximo en lugar
/// de envolver a un número chico (que sería peor: un backoff casi nulo).
fn duracion_a_ms_saturado(duracion: Duration) -> u64 {
    u64::try_from(duracion.as_millis()).unwrap_or(u64::MAX)
}

pub fn proxima_espera(intentos_seguidos: u32, base: Duration, tope: Duration) -> Duration {
    // `saturating_pow`/`saturating_mul`: con `intentos_seguidos` grande (una
    // racha larga de fallas) `2^intentos` desborda un u64 mucho antes de
    // llegar a nada razonable -- saturar en el máximo y dejar que el
    // `.min(tope)` de abajo lo recorte es más simple que acotar el
    // exponente a mano, y da el mismo resultado.
    let factor = 2u64.saturating_pow(intentos_seguidos);
    let espera_ms = duracion_a_ms_saturado(base).saturating_mul(factor);
    let tope_ms = duracion_a_ms_saturado(tope);
    Duration::from_millis(espera_ms.min(tope_ms))
}

/// "Full jitter" (el algoritmo que recomienda el paper de AWS sobre
/// exponential backoff, no algo inventado acá) -- en vez de que todos los
/// clientes que se cayeron a la vez esperen EXACTAMENTE lo mismo y
/// reintenten todos juntos (un "reconnect storm": cuando el servidor
/// reinicia, todos los clientes conectados reciben el cierre y reintentan
/// al mismo intervalo fijo, tumbando al servidor apenas vuelve --
/// documentado en la investigación que motivó esta función), cada cliente
/// espera un tiempo al azar entre 0 y `proxima_espera(...)`.
///
/// `semilla` la aporta quien llama en vez de usar un generador global de
/// azar -- así esto sigue siendo una función PURA (mismo semilla, mismo
/// resultado, siempre), fácil de testear sin mockear nada. Un LCG simple
/// alcanza (no hace falta que sea criptográficamente fuerte, sólo esparcir
/// reintentos simultáneos) y evita sumar el crate `rand` como dependencia
/// nueva sólo para esto.
pub fn proxima_espera_con_jitter(
    intentos_seguidos: u32,
    base: Duration,
    tope: Duration,
    semilla: u64,
) -> Duration {
    let techo_ms = duracion_a_ms_saturado(proxima_espera(intentos_seguidos, base, tope));
    if techo_ms == 0 {
        return Duration::ZERO;
    }
    // Constantes del generador congruencial lineal de Knuth (MMIX) -- sólo
    // para esparcir la semilla, no para nada sensible a la seguridad.
    let mezclada = semilla
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    // La pérdida de precisión de `u64`/`u32::MAX` a `f64` (mantisa de 52
    // bits) es aceptable a propósito: esto sólo necesita una proporción
    // APROXIMADA para esparcir reintentos, no un valor exacto -- perder
    // algunos bits bajos de la semilla no cambia el propósito del jitter.
    #[allow(clippy::cast_precision_loss)]
    let proporcion = (mezclada >> 32) as f64 / f64::from(u32::MAX);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let espera_ms = (techo_ms as f64 * proporcion) as u64;
    Duration::from_millis(espera_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: Duration = Duration::from_secs(2);
    const TOPE: Duration = Duration::from_secs(60);

    #[test]
    fn el_primer_intento_espera_la_base() {
        assert_eq!(proxima_espera(0, BASE, TOPE), BASE);
    }

    #[test]
    fn crece_exponencial_hasta_el_tope() {
        assert_eq!(proxima_espera(1, BASE, TOPE), Duration::from_secs(4));
        assert_eq!(proxima_espera(2, BASE, TOPE), Duration::from_secs(8));
        assert_eq!(proxima_espera(3, BASE, TOPE), Duration::from_secs(16));
        assert_eq!(proxima_espera(4, BASE, TOPE), Duration::from_secs(32));
        assert_eq!(proxima_espera(5, BASE, TOPE), TOPE); // 64s -> capa en 60s
    }

    #[test]
    fn nunca_supera_el_tope_ni_con_muchisimos_intentos_seguidos() {
        assert_eq!(proxima_espera(1_000, BASE, TOPE), TOPE);
        assert_eq!(proxima_espera(u32::MAX, BASE, TOPE), TOPE);
    }

    #[test]
    fn el_jitter_nunca_supera_el_techo_sin_jitter() {
        for semilla in 0..50u64 {
            let con_jitter = proxima_espera_con_jitter(3, BASE, TOPE, semilla);
            let sin_jitter = proxima_espera(3, BASE, TOPE);
            assert!(
                con_jitter <= sin_jitter,
                "semilla {semilla}: {con_jitter:?} > {sin_jitter:?}"
            );
        }
    }

    #[test]
    fn semillas_distintas_dan_esperas_distintas() {
        // No es una garantía matemática (dos semillas podrían coincidir por
        // azar), pero con 20 semillas consecutivas alcanza para confirmar
        // que el jitter realmente varía -- si esto fallara, `semilla` no
        // estaría influyendo en nada y el jitter sería un placebo.
        let valores: std::collections::HashSet<_> = (0..20u64)
            .map(|s| proxima_espera_con_jitter(3, BASE, TOPE, s))
            .collect();
        assert!(
            valores.len() > 1,
            "todas las semillas dieron la misma espera: {valores:?}"
        );
    }
}

#[cfg(test)]
mod propiedades {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Para CUALQUIER combinación de intentos/semilla, el jitter nunca
        /// puede superar la espera sin jitter -- si esto fallara, un
        /// cliente podría esperar MÁS de lo que el backoff exponencial
        /// permite, lo cual invierte el propósito de tener un tope.
        #[test]
        fn jitter_nunca_supera_el_techo(
            intentos in 0u32..200,
            semilla in any::<u64>(),
        ) {
            let base = Duration::from_millis(100);
            let tope = Duration::from_secs(30);
            let con_jitter = proxima_espera_con_jitter(intentos, base, tope, semilla);
            let sin_jitter = proxima_espera(intentos, base, tope);
            prop_assert!(con_jitter <= sin_jitter);
        }

        /// `proxima_espera` en sí, sin jitter: nunca negativa (trivial por
        /// tipo `Duration`, pero documenta la propiedad) y nunca supera el
        /// tope, para cualquier base/tope/intentos razonables.
        #[test]
        fn proxima_espera_respeta_el_tope_siempre(
            intentos in 0u32..u32::MAX,
            base_ms in 1u64..10_000,
            tope_ms in 1u64..120_000,
        ) {
            let base = Duration::from_millis(base_ms);
            let tope = Duration::from_millis(tope_ms);
            let espera = proxima_espera(intentos, base, tope);
            prop_assert!(espera <= tope);
        }
    }
}
