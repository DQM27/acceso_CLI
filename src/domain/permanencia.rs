//! Regla de permanencia: cuánto lleva una persona adentro sin registrar su
//! salida. Más de 12 horas es un **aviso** (casi siempre una salida que nadie
//! registró, lo que falsea el conteo de gente adentro) y más de 14 horas es
//! **crítico**: nadie puede estar tanto tiempo.
//!
//! Esta es la única definición de la regla: las interfaces (escritorio,
//! móvil) sólo muestran el nivel que el núcleo calcula, con el reloj
//! corregido del núcleo (nunca el reloj crudo del equipo, que puede estar
//! desfasado).

use chrono::{DateTime, Duration, Utc};

/// Qué tan grave es lo que lleva una persona adentro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum NivelPermanencia {
    Normal,
    /// Más de 12 horas.
    Aviso,
    /// Más de 14 horas.
    Critico,
}

/// Desde cuándo cada nivel. Los valores por defecto son la regla; existe como
/// dato para que las pruebas (y un sandbox de desarrollo) puedan acortarlos
/// sin esperar horas, pero el dominio no lee nada del entorno.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UmbralesPermanencia {
    pub aviso: Duration,
    pub critico: Duration,
}

impl Default for UmbralesPermanencia {
    fn default() -> Self {
        Self {
            aviso: Duration::hours(12),
            critico: Duration::hours(14),
        }
    }
}

/// Nivel de quien entró en `ingreso`, visto en `ahora`. Es estricto: justo 12
/// horas todavía es normal. Un ingreso "en el futuro" (reloj desfasado que
/// aún no se corrigió) cuenta como normal, nunca como aviso.
pub fn nivel_de_permanencia(
    ingreso: DateTime<Utc>,
    ahora: DateTime<Utc>,
    umbrales: UmbralesPermanencia,
) -> NivelPermanencia {
    let permanencia = ahora - ingreso;
    if permanencia > umbrales.critico {
        NivelPermanencia::Critico
    } else if permanencia > umbrales.aviso {
        NivelPermanencia::Aviso
    } else {
        NivelPermanencia::Normal
    }
}

/// Cuántas personas hay en cada nivel que merece mirarse. `en_aviso` cuenta
/// sólo a quienes están entre 12 y 14 horas (los críticos van aparte en
/// `criticos`), así no se cuenta dos veces a nadie.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ResumenPermanencia {
    pub en_aviso: usize,
    pub criticos: usize,
}

pub fn resumir_permanencia(
    niveles: impl IntoIterator<Item = NivelPermanencia>,
) -> ResumenPermanencia {
    let mut resumen = ResumenPermanencia::default();
    for nivel in niveles {
        match nivel {
            NivelPermanencia::Normal => {}
            NivelPermanencia::Aviso => resumen.en_aviso += 1,
            NivelPermanencia::Critico => resumen.criticos += 1,
        }
    }
    resumen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ahora() -> DateTime<Utc> {
        "2026-10-15T12:00:00Z".parse().unwrap()
    }

    fn hace(horas: i64, minutos: i64) -> DateTime<Utc> {
        ahora() - Duration::hours(horas) - Duration::minutes(minutos)
    }

    fn nivel(ingreso: DateTime<Utc>) -> NivelPermanencia {
        nivel_de_permanencia(ingreso, ahora(), UmbralesPermanencia::default())
    }

    #[test]
    fn menos_de_12_horas_es_normal() {
        assert_eq!(nivel(hace(0, 1)), NivelPermanencia::Normal);
        assert_eq!(nivel(hace(11, 59)), NivelPermanencia::Normal);
    }

    #[test]
    fn justo_12_horas_todavia_es_normal_y_un_minuto_mas_es_aviso() {
        assert_eq!(nivel(hace(12, 0)), NivelPermanencia::Normal);
        assert_eq!(nivel(hace(12, 1)), NivelPermanencia::Aviso);
    }

    #[test]
    fn entre_12_y_14_horas_es_aviso() {
        assert_eq!(nivel(hace(13, 30)), NivelPermanencia::Aviso);
        assert_eq!(nivel(hace(14, 0)), NivelPermanencia::Aviso);
    }

    #[test]
    fn mas_de_14_horas_es_critico() {
        assert_eq!(nivel(hace(14, 1)), NivelPermanencia::Critico);
        assert_eq!(nivel(hace(40, 0)), NivelPermanencia::Critico);
    }

    #[test]
    fn un_ingreso_en_el_futuro_no_alerta() {
        assert_eq!(
            nivel(ahora() + Duration::minutes(11)),
            NivelPermanencia::Normal
        );
    }

    #[test]
    fn los_umbrales_se_pueden_acortar_para_probar() {
        let umbrales = UmbralesPermanencia {
            aviso: Duration::minutes(1),
            critico: Duration::minutes(2),
        };
        assert_eq!(
            nivel_de_permanencia(hace(0, 3), ahora(), umbrales),
            NivelPermanencia::Critico
        );
        assert_eq!(
            nivel_de_permanencia(hace(0, 1) - Duration::seconds(1), ahora(), umbrales),
            NivelPermanencia::Aviso
        );
    }

    #[test]
    fn el_resumen_no_cuenta_dos_veces_a_los_criticos() {
        let resumen = resumir_permanencia([
            NivelPermanencia::Normal,
            NivelPermanencia::Aviso,
            NivelPermanencia::Aviso,
            NivelPermanencia::Critico,
        ]);
        assert_eq!(
            resumen,
            ResumenPermanencia {
                en_aviso: 2,
                criticos: 1
            }
        );
    }

    #[test]
    fn sin_nadie_el_resumen_es_cero() {
        assert_eq!(resumir_permanencia([]), ResumenPermanencia::default());
    }
}
