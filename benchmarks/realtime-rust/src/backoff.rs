//! Backoff exponencial con tope -- misma fórmula que ya usa
//! `desktop/src/nubeRealtime.ts` (`REINTENTO_BASE_MS * 2 ** intentos`,
//! capado en `REINTENTO_TOPE_MS`). Función pura, sin estado -- quien la usa
//! (`supervisor.rs`) es responsable de resetear el contador de intentos a 0
//! apenas una conexión llega a "conectado" de verdad, para que una falla
//! puntual después de andar mucho tiempo bien no arranque desde el tope.

use std::time::Duration;

pub fn proxima_espera(intentos_seguidos: u32, base: Duration, tope: Duration) -> Duration {
    // `saturating_pow`/`saturating_mul`: con `intentos_seguidos` grande (una
    // racha larga de fallas) `2^intentos` desborda un u64 mucho antes de
    // llegar a nada razonable -- saturar en el máximo y dejar que el
    // `.min(tope)` de abajo lo recorte es más simple que acotar el
    // exponente a mano, y da el mismo resultado.
    let factor = 2u64.saturating_pow(intentos_seguidos);
    let espera_ms = (base.as_millis() as u64).saturating_mul(factor);
    let tope_ms = tope.as_millis() as u64;
    Duration::from_millis(espera_ms.min(tope_ms))
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
}
