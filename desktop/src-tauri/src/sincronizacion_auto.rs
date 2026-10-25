//! Qué fallos de la sincronización automática merecen un aviso a Sentry.
//!
//! La sincronización automática (`iniciar_sincronizacion_automatica` en
//! `lib.rs`) corre cada 5 minutos con la app abierta, haya alguien con sesión
//! o no. Sin sesión, sin red o con el equipo sin vincular falla por razones
//! normales, y mandar cada una a Sentry llenó el proyecto de ruido: 552 avisos
//! "No hay una sesión activa" en 28 días, con picos de 186 en un día (la PC
//! esperando en el login), más del 90 % de todo lo que reportaba el escritorio.
//!
//! Dos reglas, probadas acá sin Tauri ni red:
//! - los fallos **esperados** quedan sólo en el registro local;
//! - un mismo mensaje inesperado no se repite en Sentry antes de una hora.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Comienzos de los mensajes de los fallos que son estados normales de la
/// sincronización automática (los textos salen de `control_acceso::mensajes`
/// y de `GuiState::sesion_activa`; el resto de cada mensaje puede variar).
const FALLOS_ESPERADOS: [&str; 6] = [
    "No hay una sesión activa",
    "Su sesión no está autorizada para usar la nube",
    "Sólo una sesión ROOT activa puede gestionar la nube",
    "Este dispositivo todavía no está vinculado a la nube",
    "Este dispositivo fue suspendido",
    "No se pudo conectar con la nube",
];

/// Lo que un mismo mensaje inesperado espera antes de volver a reportarse.
const ESPERA_ENTRE_REPORTES: Duration = Duration::from_secs(60 * 60);

/// `true` si el fallo es un estado normal y no hay nada que avisar.
///
/// Normal es: sin sesión, sin vincular, suspendido o sin red. Lo que no está
/// en la lista se reporta, porque es justo lo que interesa ver: no poder leer
/// la base local, que la nube no reconozca al equipo o una versión que ya no
/// es compatible.
#[must_use]
pub fn es_fallo_esperado(mensaje: &str) -> bool {
    FALLOS_ESPERADOS
        .iter()
        .any(|esperado| mensaje.starts_with(esperado))
}

/// Recuerda cuándo se reportó cada mensaje, para no repetirlo en cada pulso.
#[derive(Default)]
pub struct ReportesLimitados {
    ultimos: HashMap<String, Instant>,
}

impl ReportesLimitados {
    #[must_use]
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// `true` si `mensaje` hay que reportarlo ahora; lo da por reportado.
    /// Un mismo mensaje no vuelve a dar `true` hasta pasada una hora.
    pub fn debe_reportar(&mut self, mensaje: &str, ahora: Instant) -> bool {
        if let Some(anterior) = self.ultimos.get(mensaje)
            && ahora.saturating_duration_since(*anterior) < ESPERA_ENTRE_REPORTES
        {
            return false;
        }
        self.ultimos.insert(mensaje.to_owned(), ahora);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_estados_normales_no_se_reportan() {
        for mensaje in [
            "No hay una sesión activa",
            "Su sesión no está autorizada para usar la nube",
            "Sólo una sesión ROOT activa puede gestionar la nube",
            "Este dispositivo todavía no está vinculado a la nube",
            "Este dispositivo fue suspendido -- contacte a un administrador",
            "No se pudo conectar con la nube, intente de nuevo",
        ] {
            assert!(es_fallo_esperado(mensaje), "{mensaje}");
        }
    }

    #[test]
    fn lo_inesperado_si_se_reporta() {
        for mensaje in [
            "No se pudo leer la base de datos local",
            "La nube ya no reconoce a este dispositivo (fue retirado en el panel) -- pida a un \
             administrador un código de reconexión",
            "Esta versión de la app ya no es compatible -- actualice la app para seguir sincronizando",
            "El receptor rechazó el pedido, intente de nuevo más tarde",
            "",
        ] {
            assert!(!es_fallo_esperado(mensaje), "{mensaje}");
        }
    }

    #[test]
    fn un_mensaje_no_se_repite_dentro_de_la_hora() {
        let mut reportes = ReportesLimitados::nuevo();
        let inicio = Instant::now();
        assert!(reportes.debe_reportar("falla", inicio));
        assert!(!reportes.debe_reportar("falla", inicio + Duration::from_secs(5 * 60)));
        assert!(!reportes.debe_reportar("falla", inicio + Duration::from_secs(59 * 60)));
    }

    #[test]
    fn pasada_la_hora_vuelve_a_reportarse() {
        let mut reportes = ReportesLimitados::nuevo();
        let inicio = Instant::now();
        assert!(reportes.debe_reportar("falla", inicio));
        let despues = inicio + Duration::from_secs(61 * 60);
        assert!(reportes.debe_reportar("falla", despues));
        assert!(
            !reportes.debe_reportar("falla", despues + Duration::from_secs(60)),
            "la hora vuelve a contar desde el último aviso"
        );
    }

    #[test]
    fn cada_mensaje_cuenta_aparte() {
        let mut reportes = ReportesLimitados::nuevo();
        let inicio = Instant::now();
        assert!(reportes.debe_reportar("una", inicio));
        assert!(reportes.debe_reportar("otra", inicio));
        assert!(!reportes.debe_reportar("una", inicio));
    }
}
