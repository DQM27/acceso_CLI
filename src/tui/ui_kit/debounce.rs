use std::time::{Duration, Instant};

/// Retrasa disparar una acción hasta que pasa un tiempo sin nueva actividad
/// — pensado para búsquedas: cada tecla llama a `marcar`, y sólo cuando
/// pasan `duracion` sin una tecla nueva, la siguiente llamada a `listo`
/// devuelve `true` (una sola vez, hasta el próximo `marcar`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Debounce {
    pendiente_desde: Option<Instant>,
}

impl Debounce {
    pub fn marcar(&mut self, ahora: Instant) {
        self.pendiente_desde = Some(ahora);
    }

    /// `true` la primera vez que se consulta después de que pasó `duracion`
    /// desde el último `marcar`. Consume lo pendiente: una llamada
    /// inmediatamente posterior devuelve `false` hasta el próximo `marcar`.
    pub fn listo(&mut self, ahora: Instant, duracion: Duration) -> bool {
        match self.pendiente_desde {
            Some(desde) if ahora.saturating_duration_since(desde) >= duracion => {
                self.pendiente_desde = None;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_esta_listo_antes_de_que_pase_la_duracion() {
        let inicio = Instant::now();
        let mut debounce = Debounce::default();
        debounce.marcar(inicio);
        assert!(!debounce.listo(inicio + Duration::from_millis(100), Duration::from_millis(250)));
    }

    #[test]
    fn queda_listo_una_vez_que_pasa_la_duracion() {
        let inicio = Instant::now();
        let mut debounce = Debounce::default();
        debounce.marcar(inicio);
        assert!(debounce.listo(inicio + Duration::from_millis(250), Duration::from_millis(250)));
    }

    #[test]
    fn listo_solo_se_dispara_una_vez_por_marca() {
        let inicio = Instant::now();
        let mut debounce = Debounce::default();
        debounce.marcar(inicio);
        let despues = inicio + Duration::from_millis(300);
        assert!(debounce.listo(despues, Duration::from_millis(250)));
        assert!(!debounce.listo(despues, Duration::from_millis(250)));
    }

    #[test]
    fn una_marca_nueva_reinicia_la_espera() {
        let inicio = Instant::now();
        let mut debounce = Debounce::default();
        debounce.marcar(inicio);
        debounce.marcar(inicio + Duration::from_millis(200));
        // Sin la segunda marca, esto ya habría estado listo (400ms desde la
        // primera); con ella, sólo pasaron 200ms desde la marca vigente.
        assert!(!debounce.listo(inicio + Duration::from_millis(400), Duration::from_millis(250)));
    }

    #[test]
    fn sin_marcar_nunca_esta_listo() {
        let mut debounce = Debounce::default();
        assert!(!debounce.listo(Instant::now(), Duration::from_millis(250)));
    }
}
