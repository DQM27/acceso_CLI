//! El estado en memoria de la orden de telemetría y la regla para aplicar una
//! orden nueva.
//!
//! Cada punto de medición pregunta [`ConfigTelemetria::permite`]: una lectura
//! atómica y una comparación de hora, sin tocar la base ni tomar un candado. La
//! base (`database::config_telemetria`) sólo se lee al abrir el núcleo y se
//! escribe cuando llega una orden nueva.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use chrono::{DateTime, Utc};

use super::grupos::{Grupo, Grupos};

/// Una orden del panel para este equipo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orden {
    /// Crece en cada cambio de la orden en la nube; el equipo sólo aplica una
    /// mayor a la que ya tiene.
    pub version: i64,
    /// Los grupos además de `base`.
    pub grupos: Grupos,
    /// Pasado este instante el equipo vuelve al estado por defecto.
    pub vence_en: Option<DateTime<Utc>>,
}

/// En el estado empaquetado: "sin vencimiento".
const SIN_VENCIMIENTO: u64 = (1 << 56) - 1;

/// El estado vigente del equipo.
#[derive(Debug)]
pub struct ConfigTelemetria {
    /// Los grupos en el byte bajo y el vencimiento (segundos Unix) en los 56
    /// bits altos, en un solo valor para que un lector nunca vea una mezcla.
    estado: AtomicU64,
    version: AtomicI64,
    /// Serializa a quienes aplican una orden (aviso en vivo y sincronización).
    escritura: Mutex<()>,
}

impl ConfigTelemetria {
    /// Sin orden: `base` y `rendimiento`, sin vencimiento.
    pub const fn nueva() -> Self {
        Self {
            estado: AtomicU64::new(empaquetar(Grupos::POR_DEFECTO, SIN_VENCIMIENTO)),
            version: AtomicI64::new(0),
            escritura: Mutex::new(()),
        }
    }

    /// ¿Se mide este grupo ahora? `base` siempre. `ahora_unix` son segundos
    /// Unix de la hora confiable del equipo.
    pub fn permite(&self, grupo: Grupo, ahora_unix: i64) -> bool {
        grupo == Grupo::Base || self.grupos_vigentes(ahora_unix).contiene(grupo)
    }

    /// Los grupos que rigen ahora: los de la orden, o los por defecto si ya
    /// venció.
    pub fn grupos_vigentes(&self, ahora_unix: i64) -> Grupos {
        let (grupos, vence) = desempaquetar(self.estado.load(Ordering::Relaxed));
        let ahora = u64::try_from(ahora_unix).unwrap_or(0);
        if vence != SIN_VENCIMIENTO && ahora >= vence {
            Grupos::POR_DEFECTO
        } else {
            grupos.con(Grupo::Base)
        }
    }

    /// La versión de la orden aplicada (`0` = ninguna).
    pub fn version(&self) -> i64 {
        self.version.load(Ordering::Relaxed)
    }

    /// Vuelve al estado sin orden (`base` y `rendimiento`, versión `0`). Se usa
    /// cuando el equipo se vinculó de nuevo con otra identidad y la orden en
    /// memoria era de la anterior.
    pub fn reiniciar(&self) {
        let _escribiendo = self
            .escritura
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.estado.store(
            empaquetar(Grupos::POR_DEFECTO, SIN_VENCIMIENTO),
            Ordering::Relaxed,
        );
        self.version.store(0, Ordering::Relaxed);
    }

    /// Aplica la orden si es más nueva que la que ya rige. Devuelve si cambió
    /// algo; una orden igual o más vieja (un aviso repetido o fuera de orden) no
    /// hace nada.
    pub fn aplicar(&self, orden: &Orden) -> bool {
        let _escribiendo = self
            .escritura
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if orden.version <= self.version.load(Ordering::Relaxed) {
            return false;
        }
        let vence = orden
            .vence_en
            .map_or(SIN_VENCIMIENTO, |v| segundos_empaquetables(v.timestamp()));
        // Primero el estado y después la versión: quien lea la versión ya ve la
        // orden aplicada.
        self.estado
            .store(empaquetar(orden.grupos, vence), Ordering::Relaxed);
        self.version.store(orden.version, Ordering::Relaxed);
        true
    }
}

impl Default for ConfigTelemetria {
    fn default() -> Self {
        Self::nueva()
    }
}

/// Los segundos Unix de un vencimiento, acotados a lo que cabe en 56 bits. Uno
/// anterior a 1970 queda en `0` (ya vencido); uno tan lejano que no cabe queda
/// justo antes de "sin vencimiento".
fn segundos_empaquetables(segundos: i64) -> u64 {
    u64::try_from(segundos)
        .unwrap_or(0)
        .min(SIN_VENCIMIENTO - 1)
}

const fn empaquetar(grupos: Grupos, vence: u64) -> u64 {
    (vence << 8) | grupos.bits() as u64
}

fn desempaquetar(estado: u64) -> (Grupos, u64) {
    (Grupos::desde_bits(estado.to_le_bytes()[0]), estado >> 8)
}

/// El estado del proceso. El móvil y el escritorio abren un solo núcleo por
/// proceso, así que es uno solo.
pub fn config() -> &'static ConfigTelemetria {
    static CONFIG: ConfigTelemetria = ConfigTelemetria::nueva();
    &CONFIG
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    const AHORA: i64 = 1_800_000_000;

    fn orden(version: i64, nombres: &[&str], vence_en: Option<i64>) -> Orden {
        Orden {
            version,
            grupos: Grupos::desde_nombres(nombres.iter().copied()),
            vence_en: vence_en.map(|s| Utc.timestamp_opt(s, 0).unwrap()),
        }
    }

    #[test]
    fn sin_orden_se_mide_base_y_rendimiento() {
        let c = ConfigTelemetria::nueva();
        assert!(c.permite(Grupo::Base, AHORA));
        assert!(c.permite(Grupo::Rendimiento, AHORA));
        assert!(!c.permite(Grupo::Fluidez, AHORA));
        assert!(!c.permite(Grupo::Depuracion, AHORA));
        assert_eq!(c.version(), 0);
    }

    #[test]
    fn una_orden_enciende_sus_grupos_y_deja_base_siempre() {
        let c = ConfigTelemetria::nueva();
        assert!(c.aplicar(&orden(1, &["fluidez", "recursos"], None)));
        assert!(c.permite(Grupo::Base, AHORA));
        assert!(c.permite(Grupo::Fluidez, AHORA));
        assert!(c.permite(Grupo::Recursos, AHORA));
        assert!(!c.permite(Grupo::Sincronizacion, AHORA));
        assert_eq!(c.version(), 1);
    }

    #[test]
    fn la_orden_manda_aunque_apague_rendimiento() {
        let c = ConfigTelemetria::nueva();
        c.aplicar(&orden(1, &[], None));
        assert!(!c.permite(Grupo::Rendimiento, AHORA));
        assert!(c.permite(Grupo::Base, AHORA));
    }

    #[test]
    fn una_orden_igual_o_mas_vieja_no_cambia_nada() {
        let c = ConfigTelemetria::nueva();
        assert!(c.aplicar(&orden(5, &["fluidez"], None)));
        assert!(!c.aplicar(&orden(5, &["recursos"], None)));
        assert!(!c.aplicar(&orden(4, &["recursos"], None)));
        assert!(c.permite(Grupo::Fluidez, AHORA));
        assert!(!c.permite(Grupo::Recursos, AHORA));
        assert_eq!(c.version(), 5);
    }

    #[test]
    fn una_orden_mas_nueva_reemplaza_a_la_anterior() {
        let c = ConfigTelemetria::nueva();
        c.aplicar(&orden(1, &["fluidez"], None));
        assert!(c.aplicar(&orden(2, &["escaneo"], None)));
        assert!(!c.permite(Grupo::Fluidez, AHORA));
        assert!(c.permite(Grupo::Escaneo, AHORA));
    }

    #[test]
    fn al_vencer_vuelve_al_estado_por_defecto() {
        let c = ConfigTelemetria::nueva();
        c.aplicar(&orden(1, &["fluidez"], Some(AHORA + 100)));
        assert!(c.permite(Grupo::Fluidez, AHORA + 99));
        // En el instante del vencimiento ya no rige.
        assert!(!c.permite(Grupo::Fluidez, AHORA + 100));
        assert!(!c.permite(Grupo::Fluidez, AHORA + 10_000));
        // Vuelve a lo de siempre: base y rendimiento.
        assert!(c.permite(Grupo::Base, AHORA + 100));
        assert!(c.permite(Grupo::Rendimiento, AHORA + 100));
        // Y conserva la versión: una orden vieja no revive.
        assert_eq!(c.version(), 1);
        assert!(!c.aplicar(&orden(1, &["fluidez"], None)));
    }

    #[test]
    fn una_orden_sin_rendimiento_que_vence_devuelve_el_rendimiento() {
        let c = ConfigTelemetria::nueva();
        c.aplicar(&orden(1, &["recursos"], Some(AHORA + 10)));
        assert!(!c.permite(Grupo::Rendimiento, AHORA));
        assert!(c.permite(Grupo::Rendimiento, AHORA + 10));
    }

    #[test]
    fn una_orden_ya_vencida_al_llegar_no_enciende_nada() {
        let c = ConfigTelemetria::nueva();
        assert!(c.aplicar(&orden(1, &["fluidez"], Some(AHORA - 1))));
        assert!(!c.permite(Grupo::Fluidez, AHORA));
        assert_eq!(c.version(), 1);
    }

    #[test]
    fn un_vencimiento_antes_de_1970_cuenta_como_vencido() {
        let c = ConfigTelemetria::nueva();
        c.aplicar(&orden(1, &["fluidez"], Some(-5)));
        assert!(!c.permite(Grupo::Fluidez, AHORA));
    }

    #[test]
    fn un_vencimiento_lejanisimo_no_se_confunde_con_sin_vencimiento() {
        let c = ConfigTelemetria::nueva();
        // Más allá de lo que cabe en 56 bits: queda acotado, no se desborda.
        c.aplicar(&orden(1, &["fluidez"], Some(i64::MAX / 1_000_000_000)));
        assert!(c.permite(Grupo::Fluidez, AHORA));
    }

    #[test]
    fn la_hora_negativa_del_equipo_no_hace_vencer_una_orden() {
        let c = ConfigTelemetria::nueva();
        c.aplicar(&orden(1, &["fluidez"], Some(AHORA)));
        assert!(c.permite(Grupo::Fluidez, -1));
    }

    #[test]
    fn dos_hilos_leen_mientras_se_aplican_ordenes_sin_ver_mezclas() {
        use std::sync::Arc;
        let c = Arc::new(ConfigTelemetria::nueva());
        let lector = {
            let c = Arc::clone(&c);
            std::thread::spawn(move || {
                for _ in 0..50_000 {
                    // Cada orden enciende `fluidez` y `recursos` juntas o ninguna
                    // de las dos: un lector no puede ver sólo una.
                    let g = c.grupos_vigentes(AHORA);
                    assert_eq!(g.contiene(Grupo::Fluidez), g.contiene(Grupo::Recursos));
                }
            })
        };
        for version in 1..=2_000 {
            let nombres: &[&str] = if version % 2 == 0 {
                &["fluidez", "recursos"]
            } else {
                &[]
            };
            c.aplicar(&orden(version, nombres, None));
        }
        lector.join().unwrap();
        assert_eq!(c.version(), 2_000);
    }
}
