//! Un valor escalar que se interpola en el tiempo — nunca por número de
//! frames (DEC-004). Si se pierde un frame, la animación sigue exactamente
//! donde debería estar según el reloj, no según cuántos `draw()` corrieron.

use std::time::{Duration, Instant};

use super::easing::Easing;

#[derive(Debug, Clone, Copy)]
pub struct Animacion {
    inicio: Instant,
    duracion: Duration,
    desde: f32,
    hasta: f32,
    easing: Easing,
}

impl Animacion {
    pub fn nueva(desde: f32, hasta: f32, duracion: Duration, easing: Easing) -> Self {
        Self {
            inicio: Instant::now(),
            duracion,
            desde,
            hasta,
            easing,
        }
    }

    /// Ya resuelta en `valor`, sin transcurrir — la usa `VisualQuality::Off`
    /// y cualquier elemento que arranca directamente en su estado estable.
    pub fn resuelta(valor: f32) -> Self {
        Self::nueva(valor, valor, Duration::ZERO, Easing::Linear)
    }

    fn progreso(&self) -> f32 {
        if self.duracion.is_zero() {
            return 1.0;
        }
        let transcurrido = self.inicio.elapsed().as_secs_f32();
        (transcurrido / self.duracion.as_secs_f32()).clamp(0.0, 1.0)
    }

    pub fn valor(&self) -> f32 {
        let t = self.easing.aplicar(self.progreso());
        self.desde + (self.hasta - self.desde) * t
    }

    pub fn termino(&self) -> bool {
        self.progreso() >= 1.0
    }
}
