//! Contador de milisegundos desde que arrancó el equipo, que el usuario no
//! puede mover: cambiar la hora de Windows o de Android no lo afecta. Es la
//! base del reloj confiable ([`crate::tiempo::RelojCorregido`]) y de la
//! duración de una sesión (`nube::sesion_unidad`), con el mismo principio
//! que `Kronos` (Lyft) y `TrustedTime` (Google): hora del servidor + tiempo
//! transcurrido en un contador monotónico.
//!
//! Sigue contando mientras el equipo está suspendido (un teléfono duerme
//! casi todo el turno): por eso no sirve `std::time::Instant`, que en
//! Linux/Android se detiene durante la suspensión. Vuelve a cero al
//! reiniciar el equipo.

/// Milisegundos desde el arranque del equipo, contando la suspensión.
pub fn ms_desde_arranque() -> u64 {
    plataforma::ms_desde_arranque()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod plataforma {
    /// `CLOCK_BOOTTIME`: el mismo reloj que `SystemClock.elapsedRealtime()`
    /// de Android; incluye el tiempo suspendido.
    pub(super) fn ms_desde_arranque() -> u64 {
        super::reloj_posix(libc::CLOCK_BOOTTIME)
    }
}

#[cfg(target_vendor = "apple")]
mod plataforma {
    /// En Darwin, `CLOCK_MONOTONIC` sigue avanzando con el equipo dormido
    /// (`CLOCK_UPTIME_RAW` es el que se detiene).
    pub(super) fn ms_desde_arranque() -> u64 {
        super::reloj_posix(libc::CLOCK_MONOTONIC)
    }
}

#[cfg(windows)]
mod plataforma {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        /// Milisegundos desde el arranque, incluida la suspensión y la
        /// hibernación (a diferencia de `QueryUnbiasedInterruptTime`).
        fn GetTickCount64() -> u64;
    }

    pub(super) fn ms_desde_arranque() -> u64 {
        // SAFETY: sin parámetros ni estado; Windows la documenta como
        // segura de llamar desde cualquier hilo.
        unsafe { GetTickCount64() }
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_vendor = "apple",
    windows
)))]
mod plataforma {
    use std::sync::OnceLock;
    use std::time::Instant;

    /// Sin contador del sistema conocido: desde que arrancó este proceso.
    pub(super) fn ms_desde_arranque() -> u64 {
        static INICIO: OnceLock<Instant> = OnceLock::new();
        u64::try_from(INICIO.get_or_init(Instant::now).elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

#[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
fn reloj_posix(reloj: libc::clockid_t) -> u64 {
    let mut lectura = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `lectura` es un `timespec` válido y propio; `clock_gettime`
    // sólo escribe en él.
    let resultado = unsafe { libc::clock_gettime(reloj, &raw mut lectura) };
    if resultado != 0 {
        // No pasa con estos relojes en los sistemas soportados; si pasara,
        // 0 hace que el reloj confiable descarte su ancla (ver
        // `RelojCorregido`) en vez de inventar una hora.
        return 0;
    }
    let segundos = u64::try_from(lectura.tv_sec).unwrap_or(0);
    let nanos = u64::try_from(lectura.tv_nsec).unwrap_or(0);
    segundos
        .saturating_mul(1000)
        .saturating_add(nanos / 1_000_000)
}

#[cfg(test)]
mod tests {
    use super::ms_desde_arranque;

    #[test]
    fn avanza_y_nunca_retrocede() {
        let primero = ms_desde_arranque();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let segundo = ms_desde_arranque();
        assert!(primero > 0, "el equipo arrancó hace algo de tiempo");
        assert!(segundo >= primero + 15, "avanzó {} ms", segundo - primero);
    }
}
