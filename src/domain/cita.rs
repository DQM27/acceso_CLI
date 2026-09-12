//! Reglas de vigencia de una cita (`docs/planes-implementados/plan-control-visitas.md`): sólo se
//! puede hacer check-in con una cita `Vigente` y dentro de su rango de
//! fechas. Mucho más simple que `domain::acceso::verificar_acceso`
//! (contratistas) a propósito -- decisión explícita del usuario: la carga
//! de datos la hace el anfitrión al agendar, el guardia sólo verifica, no
//! hay PRAIND ni advertencias intermedias que evaluar acá. Binario:
//! permitida o no.

use chrono::NaiveDate;

use crate::models::cita::{Cita, EstadoCita};

/// Versión de la regla de `verificar_cita`, mismo espíritu que
/// `domain::acceso::VERSION_REGLAS_ACCESO` -- todavía sin un lugar donde
/// persistirla (`movimientos_visita` no tiene `reglas_version`, ver el
/// comentario de esa tabla en `database::schema`: a diferencia de
/// contratistas, acá no existe un resultado "permitido con advertencia" que
/// justifique guardar bajo qué versión se decidió cada movimiento -- un
/// movimiento sólo existe si el check-in fue permitido). Se deja la
/// constante de todos modos para que, si el día de mañana esto cambia,
/// quede un solo lugar que subir.
pub const VERSION_REGLAS_VISITA: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoVisita {
    Permitido,
    Denegado(MotivoDenegacionVisita),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotivoDenegacionVisita {
    /// El anfitrión (o un administrador) canceló la cita.
    CitaCancelada,
    /// `hoy` cae afuera de `[fecha_desde, fecha_hasta]` -- incluye tanto
    /// "todavía no empieza" como "ya venció".
    FueraDeVigencia,
}

/// No recibe `sitio_id` a propósito: el dispositivo sólo llega a evaluar
/// esto sobre una `Cita` que ya pasó por el filtro de sincronización
/// (`cita_sitios` en la nube, ver `docs/planes-implementados/plan-control-visitas.md`) -- si la
/// cita está en la base local de este sitio es porque ya se confirmó que
/// aplica acá. Repetir el chequeo de sitio en el dominio sería validar dos
/// veces la misma cosa contra fuentes que podrían desincronizarse.
pub fn verificar_cita(cita: &Cita, hoy: NaiveDate) -> ResultadoVisita {
    if cita.estado == EstadoCita::Cancelada {
        return ResultadoVisita::Denegado(MotivoDenegacionVisita::CitaCancelada);
    }
    if hoy < cita.fecha_desde || hoy > cita.fecha_hasta {
        return ResultadoVisita::Denegado(MotivoDenegacionVisita::FueraDeVigencia);
    }
    ResultadoVisita::Permitido
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cita(estado: EstadoCita, desde: &str, hasta: &str) -> Cita {
        Cita {
            id: 1,
            motivo: None,
            fecha_desde: desde.parse().unwrap(),
            fecha_hasta: hasta.parse().unwrap(),
            anfitrion_nombre: "Anfitrión".to_string(),
            anfitrion_correo: "anfitrion@ejemplo.com".to_string(),
            estado,
            hora_estimada: None,
        }
    }

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    #[test]
    fn vigente_y_dentro_de_rango_permite() {
        let cita = cita(EstadoCita::Vigente, "2026-08-10", "2026-08-15");
        assert_eq!(
            verificar_cita(&cita, fecha("2026-08-12")),
            ResultadoVisita::Permitido
        );
    }

    #[test]
    fn vigente_en_el_primer_y_ultimo_dia_del_rango_permite() {
        let cita = cita(EstadoCita::Vigente, "2026-08-10", "2026-08-15");
        assert_eq!(
            verificar_cita(&cita, fecha("2026-08-10")),
            ResultadoVisita::Permitido
        );
        assert_eq!(
            verificar_cita(&cita, fecha("2026-08-15")),
            ResultadoVisita::Permitido
        );
    }

    #[test]
    fn cancelada_deniega_aunque_este_dentro_del_rango() {
        let cita = cita(EstadoCita::Cancelada, "2026-08-10", "2026-08-15");
        assert_eq!(
            verificar_cita(&cita, fecha("2026-08-12")),
            ResultadoVisita::Denegado(MotivoDenegacionVisita::CitaCancelada)
        );
    }

    #[test]
    fn antes_de_fecha_desde_deniega_por_fuera_de_vigencia() {
        let cita = cita(EstadoCita::Vigente, "2026-08-10", "2026-08-15");
        assert_eq!(
            verificar_cita(&cita, fecha("2026-08-09")),
            ResultadoVisita::Denegado(MotivoDenegacionVisita::FueraDeVigencia)
        );
    }

    #[test]
    fn despues_de_fecha_hasta_deniega_por_fuera_de_vigencia() {
        let cita = cita(EstadoCita::Vigente, "2026-08-10", "2026-08-15");
        assert_eq!(
            verificar_cita(&cita, fecha("2026-08-16")),
            ResultadoVisita::Denegado(MotivoDenegacionVisita::FueraDeVigencia)
        );
    }

    #[test]
    fn cita_de_un_solo_dia_permite_justo_ese_dia() {
        let cita = cita(EstadoCita::Vigente, "2026-08-10", "2026-08-10");
        assert_eq!(
            verificar_cita(&cita, fecha("2026-08-10")),
            ResultadoVisita::Permitido
        );
        assert_eq!(
            verificar_cita(&cita, fecha("2026-08-11")),
            ResultadoVisita::Denegado(MotivoDenegacionVisita::FueraDeVigencia)
        );
    }

    #[test]
    fn cancelada_fuera_de_rango_reporta_cancelada_no_vigencia() {
        // La regla de estado corre primero -- si está cancelada, el motivo
        // es "cancelada", no "fuera de vigencia", aunque también aplicara.
        let cita = cita(EstadoCita::Cancelada, "2026-08-10", "2026-08-15");
        assert_eq!(
            verificar_cita(&cita, fecha("2026-01-01")),
            ResultadoVisita::Denegado(MotivoDenegacionVisita::CitaCancelada)
        );
    }
}
