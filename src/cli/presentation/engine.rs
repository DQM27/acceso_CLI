//! Motor de presentación mínimo: sabe qué animaciones están vivas, calcula
//! su valor actual y le dice al loop si hace falta seguir despertando para
//! pintar el próximo frame. No sabe nada de reglas de negocio, SQLite ni
//! permisos — sólo tiempo, valores e identificadores de elemento.

use std::collections::HashMap;
use std::time::Duration;

use super::animation::Animacion;
use super::easing::Easing;
use super::quality::VisualQuality;

/// Duración de una aparición (fade-in) — en el extremo alto del rango
/// "transición grande" documentado (200–350 ms): perceptible sin sentirse
/// lenta, nunca cerca de un segundo.
pub const DURACION_APARICION: Duration = Duration::from_millis(320);

#[derive(Default)]
pub struct Engine {
    animaciones: HashMap<&'static str, Animacion>,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Arranca (o reinicia desde 0) una aparición para `id`. Con
    /// `VisualQuality::Off` queda resuelta al instante en el valor final —
    /// cero dependencia funcional de la animación (DEC-012).
    pub fn aparecer(&mut self, id: &'static str, calidad: VisualQuality) {
        self.aparecer_con_retraso(id, calidad, Duration::ZERO);
    }

    /// Como `aparecer`, con una pausa de `retraso` antes de arrancar el fade
    /// — para apariciones que se sienten "instantáneas" cuando el fade solo
    /// dura `DURACION_APARICION` a secas (ver la paleta de comandos).
    pub fn aparecer_con_retraso(
        &mut self,
        id: &'static str,
        calidad: VisualQuality,
        retraso: Duration,
    ) {
        let animacion = match calidad {
            VisualQuality::Off => Animacion::resuelta(1.0),
            VisualQuality::Normal => {
                Animacion::con_retraso(0.0, 1.0, retraso, DURACION_APARICION, Easing::EaseOut)
            }
        };
        self.animaciones.insert(id, animacion);
    }

    /// Opacidad actual de `id` en [0.0, 1.0]. Un elemento nunca registrado
    /// se considera ya resuelto en 1.0 — evita destellos en el primer frame,
    /// antes de que nada haya "aparecido" todavía.
    pub fn opacidad(&self, id: &'static str) -> f32 {
        self.animaciones.get(id).map_or(1.0, Animacion::valor)
    }

    /// ¿Sigue habiendo alguna animación en curso? El loop lo usa para saber
    /// si tiene que seguir despertando a ritmo de frame o puede volver a
    /// dormir esperando el próximo evento (ver `proxima_espera` en `mod.rs`).
    pub fn activo(&self) -> bool {
        self.animaciones
            .values()
            .any(|animacion| !animacion.termino())
    }
}

#[cfg(test)]
// `assert_eq!` contra 1.0 exacto abajo: un elemento nunca registrado o con
// `VisualQuality::Off` resuelve la opacidad al instante (valor inicial
// garantizado, sin interpolación de por medio) — no hay error de punto
// flotante acumulado que una comparación exacta pueda esconder acá.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn elemento_nunca_registrado_esta_resuelto() {
        let engine = Engine::new();
        assert_eq!(engine.opacidad("titulo"), 1.0);
        assert!(!engine.activo());
    }

    #[test]
    fn calidad_off_resuelve_al_instante() {
        let mut engine = Engine::new();
        engine.aparecer("titulo", VisualQuality::Off);
        assert_eq!(engine.opacidad("titulo"), 1.0);
        assert!(!engine.activo());
    }

    #[test]
    fn calidad_normal_arranca_cerca_de_cero_y_queda_activa() {
        let mut engine = Engine::new();
        engine.aparecer("titulo", VisualQuality::Normal);
        // No se compara contra 0.0 exacto: entre `nueva()` y esta lectura
        // pasan nanosegundos reales (la animación es de tiempo, no de
        // frames), así que el progreso ya es un valor positivo minúsculo.
        assert!(engine.opacidad("titulo") < 0.01);
        assert!(engine.activo());
    }
}
