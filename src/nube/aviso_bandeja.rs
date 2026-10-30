//! Aviso a quien opera cuando algo registrado en este equipo no llegó a la
//! nube. Lo decide el núcleo; escritorio y móvil sólo lo dibujan.
//!
//! Incidente del 2026-10-29: la salida de un contratista quedó 17 minutos
//! sin subir y ningún equipo lo avisó; el operador lo descubrió porque el
//! teléfono la tenía afuera y el escritorio adentro. Con este aviso, el
//! equipo que tiene algo pendiente lo dice.

use chrono::{DateTime, Utc};

use super::sincronizacion::EstadoBandejaSalida;

/// Un registro recién hecho tarda unos segundos en subir: avisar antes de
/// esto sólo haría parpadear el aviso en cada registro normal.
pub const ESPERA_ANTES_DE_AVISAR_SEGUNDOS: i64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvisoBandejaSalida {
    pub texto: String,
    /// `true`: hay registros que ya no se reintentan solos (alguien tiene
    /// que mirarlos). `false`: siguen intentándose.
    pub critico: bool,
}

/// `None` si no hay nada que avisar: la bandeja está vacía, o lo pendiente es
/// tan reciente que está subiendo en este momento.
pub fn aviso_bandeja_salida(
    estado: &EstadoBandejaSalida,
    ahora: DateTime<Utc>,
) -> Option<AvisoBandejaSalida> {
    if estado.fallidos > 0 {
        let texto = if estado.fallidos == 1 {
            "1 registro no se pudo subir a la nube: avise al administrador".to_string()
        } else {
            format!(
                "{} registros no se pudieron subir a la nube: avise al administrador",
                estado.fallidos
            )
        };
        return Some(AvisoBandejaSalida {
            texto,
            critico: true,
        });
    }

    let espera = estado
        .pendiente_mas_antiguo
        .as_deref()
        .and_then(|creado| DateTime::parse_from_rfc3339(creado).ok())
        .map(|creado| ahora.signed_duration_since(creado.with_timezone(&Utc)))?;
    if estado.pendientes == 0 || espera.num_seconds() < ESPERA_ANTES_DE_AVISAR_SEGUNDOS {
        return None;
    }

    let cuantos = if estado.pendientes == 1 {
        "1 registro sin subir a la nube".to_string()
    } else {
        format!("{} registros sin subir a la nube", estado.pendientes)
    };
    Some(AvisoBandejaSalida {
        texto: format!("{cuantos} (hace {})", hace(espera.num_minutes())),
        critico: false,
    })
}

fn hace(minutos: i64) -> String {
    match minutos {
        ..=1 => "1 min".to_string(),
        2..=59 => format!("{minutos} min"),
        _ => format!("{} h", minutos / 60),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn ahora() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 30, 4, 17, 0).unwrap()
    }

    fn pendientes(cuantos: i64, desde: &str) -> EstadoBandejaSalida {
        EstadoBandejaSalida {
            pendientes: cuantos,
            pendiente_mas_antiguo: Some(desde.to_string()),
            fallidos: 0,
        }
    }

    #[test]
    fn sin_nada_pendiente_no_avisa() {
        assert_eq!(
            aviso_bandeja_salida(&EstadoBandejaSalida::default(), ahora()),
            None
        );
    }

    #[test]
    fn un_registro_que_esta_subiendo_no_avisa() {
        // Hecho hace 20 s: está subiendo, avisar sólo haría parpadear.
        assert_eq!(
            aviso_bandeja_salida(&pendientes(1, "2026-10-30T04:16:40Z"), ahora()),
            None
        );
    }

    /// El incidente: la salida de las 22:00:38 (04:00:38 UTC) seguía sin
    /// subir a las 22:17.
    #[test]
    fn la_salida_trabada_del_incidente_se_avisa_con_su_antiguedad() {
        let aviso = aviso_bandeja_salida(&pendientes(1, "2026-10-30T04:00:38Z"), ahora()).unwrap();

        assert_eq!(aviso.texto, "1 registro sin subir a la nube (hace 16 min)");
        assert!(!aviso.critico, "todavía se reintenta solo");
    }

    #[test]
    fn varios_pendientes_usan_el_mas_viejo_y_el_plural() {
        let aviso = aviso_bandeja_salida(&pendientes(3, "2026-10-30T02:05:00Z"), ahora()).unwrap();

        assert_eq!(aviso.texto, "3 registros sin subir a la nube (hace 2 h)");
    }

    #[test]
    fn justo_al_minuto_ya_avisa() {
        let aviso = aviso_bandeja_salida(&pendientes(1, "2026-10-30T04:16:00Z"), ahora()).unwrap();

        assert_eq!(aviso.texto, "1 registro sin subir a la nube (hace 1 min)");
    }

    #[test]
    fn los_fallos_permanentes_ganan_y_son_criticos() {
        let estado = EstadoBandejaSalida {
            pendientes: 2,
            pendiente_mas_antiguo: Some("2026-10-30T04:00:00Z".to_string()),
            fallidos: 2,
        };

        let aviso = aviso_bandeja_salida(&estado, ahora()).unwrap();

        assert!(aviso.critico);
        assert_eq!(
            aviso.texto,
            "2 registros no se pudieron subir a la nube: avise al administrador"
        );
    }
}
