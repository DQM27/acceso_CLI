//! Cuándo mandar la telemetría
//! (`docs/features-futuras/plan-red-movil-por-el-nucleo.md`).
//!
//! - **Escritorio:** red fija y conexión siempre caliente: cada 60 s si hay
//!   algo.
//! - **Móvil:** "a caballo" de otro tráfico. El módem gasta mucho al
//!   despertarse y queda despierto unos segundos después de cada envío; la
//!   telemetría sale justo después de que algo útil (sincronización,
//!   verificación de un ingreso, aviso push) ya lo despertó, por la misma
//!   conexión HTTP/2. Sólo sale sola si pasó mucho sin otra actividad, o si
//!   se juntó tanto que conviene no esperar más.
//!
//! Sin estado propio salvo la hora de la última actividad de red, que marcan
//! las llamadas del núcleo con [`registrar_actividad_red`]. La decisión es una
//! función pura ([`debe_enviar`]).

use std::sync::atomic::{AtomicI64, Ordering};

/// Qué equipo decide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perfil {
    Escritorio,
    Movil,
}

/// Escritorio: cada cuánto.
pub const INTERVALO_ESCRITORIO_MS: i64 = 60_000;
/// Móvil: la radio sigue despierta este tiempo después de una petición útil.
pub const VENTANA_RADIO_DESPIERTA_MS: i64 = 5_000;
/// Móvil: como mucho una vez cada esto aunque haya actividad (no mandar un
/// lote chiquito detrás de cada verificación).
pub const MINIMO_ENTRE_ENVIOS_MOVIL_MS: i64 = 30_000;
/// Móvil: sin otra actividad, sale solo pasado este tiempo.
pub const MAXIMO_SIN_ENVIAR_MOVIL_MS: i64 = 15 * 60_000;
/// Móvil: con esta cantidad juntada no se espera más (es casi un lote).
pub const PENDIENTES_PARA_NO_ESPERAR: usize = 400;

/// Lo que hace falta saber para decidir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Situacion {
    pub ahora_ms: i64,
    /// Eventos esperando (aproximado basta).
    pub pendientes: usize,
    /// Último envío de telemetría, o el arranque si todavía no hubo.
    pub ultimo_envio_ms: i64,
    /// Última petición útil del núcleo a la nube, si hubo.
    pub ultima_actividad_red_ms: Option<i64>,
}

/// ¿Conviene mandar ahora?
pub fn debe_enviar(perfil: Perfil, situacion: Situacion) -> bool {
    if situacion.pendientes == 0 {
        return false;
    }
    let desde_envio = situacion.ahora_ms - situacion.ultimo_envio_ms;
    match perfil {
        Perfil::Escritorio => desde_envio >= INTERVALO_ESCRITORIO_MS,
        Perfil::Movil => {
            let radio_despierta = situacion.ultima_actividad_red_ms.is_some_and(|actividad| {
                let desde_actividad = situacion.ahora_ms - actividad;
                (0..=VENTANA_RADIO_DESPIERTA_MS).contains(&desde_actividad)
            });
            (radio_despierta && desde_envio >= MINIMO_ENTRE_ENVIOS_MOVIL_MS)
                || desde_envio >= MAXIMO_SIN_ENVIAR_MOVIL_MS
                || situacion.pendientes >= PENDIENTES_PARA_NO_ESPERAR
        }
    }
}

/// Hora (ms) de la última petición útil a la nube; 0 = ninguna todavía.
static ULTIMA_ACTIVIDAD_RED_MS: AtomicI64 = AtomicI64::new(0);

/// La marcan las llamadas del núcleo que de todos modos despertaron la radio.
/// Es una escritura atómica: barata en cualquier hilo.
pub fn registrar_actividad_red(ahora_ms: i64) {
    ULTIMA_ACTIVIDAD_RED_MS.fetch_max(ahora_ms, Ordering::Relaxed);
}

/// La última actividad de red registrada, si hubo.
pub fn ultima_actividad_red() -> Option<i64> {
    match ULTIMA_ACTIVIDAD_RED_MS.load(Ordering::Relaxed) {
        0 => None,
        ms => Some(ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AHORA: i64 = 1_793_470_000_000;

    fn situacion(
        pendientes: usize,
        desde_envio_ms: i64,
        desde_actividad_ms: Option<i64>,
    ) -> Situacion {
        Situacion {
            ahora_ms: AHORA,
            pendientes,
            ultimo_envio_ms: AHORA - desde_envio_ms,
            ultima_actividad_red_ms: desde_actividad_ms.map(|ms| AHORA - ms),
        }
    }

    #[test]
    fn sin_pendientes_nunca() {
        assert!(!debe_enviar(
            Perfil::Escritorio,
            situacion(0, 3_600_000, Some(0))
        ));
        assert!(!debe_enviar(
            Perfil::Movil,
            situacion(0, 3_600_000, Some(0))
        ));
    }

    #[test]
    fn escritorio_cada_minuto() {
        assert!(!debe_enviar(Perfil::Escritorio, situacion(5, 59_999, None)));
        assert!(debe_enviar(Perfil::Escritorio, situacion(5, 60_000, None)));
    }

    #[test]
    fn movil_aprovecha_la_radio_despierta() {
        // Hubo una verificación hace 2 s y el último envío fue hace 40 s.
        assert!(debe_enviar(
            Perfil::Movil,
            situacion(20, 40_000, Some(2_000))
        ));
        // La radio ya se durmió (actividad hace 20 s): espera.
        assert!(!debe_enviar(
            Perfil::Movil,
            situacion(20, 40_000, Some(20_000))
        ));
        // Radio despierta pero se mandó hace 10 s: no un lote por verificación.
        assert!(!debe_enviar(
            Perfil::Movil,
            situacion(20, 10_000, Some(1_000))
        ));
    }

    #[test]
    fn movil_sale_solo_si_pasa_mucho_o_se_junta_mucho() {
        assert!(!debe_enviar(
            Perfil::Movil,
            situacion(20, 14 * 60_000, None)
        ));
        assert!(debe_enviar(Perfil::Movil, situacion(20, 15 * 60_000, None)));
        assert!(debe_enviar(
            Perfil::Movil,
            situacion(PENDIENTES_PARA_NO_ESPERAR, 1_000, None)
        ));
    }

    #[test]
    fn una_actividad_en_el_futuro_no_cuenta_como_radio_despierta() {
        // Un reloj corregido hacia atrás no debe dejar la radio "despierta".
        assert!(!debe_enviar(
            Perfil::Movil,
            situacion(20, 40_000, Some(-60_000))
        ));
    }

    #[test]
    fn la_actividad_registrada_solo_avanza() {
        registrar_actividad_red(AHORA);
        registrar_actividad_red(AHORA - 10_000);
        assert!(ultima_actividad_red().is_some_and(|ms| ms >= AHORA));
    }
}
