//! Curvas de easing sobre un progreso temporal ya normalizado a [0.0, 1.0].
//!
//! Cuatro curvas alcanzan lo que necesita una TUI operativa (ver
//! `docs/lenguaje-visual-mutaciones.md` §12) — no hace falta una colección
//! más grande.

// `EaseIn`/`EaseInOut` son parte del vocabulario mínimo documentado
// (docs/lenguaje-visual-mutaciones.md §12: aparición → EaseOut, desaparición
// → EaseIn) aunque todavía sólo la aparición (§14.1) esté cableada — se usan
// en cuanto se agregue la desaparición animada del aviso o cualquier
// transición reversible.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Easing {
    /// `t` se recibe y se devuelve ya acotado a [0.0, 1.0].
    pub fn aplicar(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
        }
    }
}

#[cfg(test)]
// `assert_eq!` contra 0.0/1.0 exactos abajo: son los extremos de un clamp
// (`Linear.aplicar(-1.0/2.0)`) o del inicio/fin garantizado de una curva, no
// el resultado de una interpolación acumulando error de punto flotante — la
// comparación exacta es la aserción correcta acá, no una aproximación floja.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn extremos_de_cada_curva_coinciden_con_los_limites() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
        ] {
            assert_eq!(easing.aplicar(0.0), 0.0);
            assert!((easing.aplicar(1.0) - 1.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn valores_fuera_de_rango_se_acotan() {
        assert_eq!(Easing::Linear.aplicar(-1.0), 0.0);
        assert_eq!(Easing::Linear.aplicar(2.0), 1.0);
    }
}
