use chrono::NaiveDate;

/// Mismo molde que `domain::resultado_acceso::ResultadoAcceso` -- no hace
/// falta una variante "Denegado" real (a diferencia de ese otro dominio):
/// sin correo de autorización el flujo ni siquiera llega a intentar el
/// registro, queda bloqueado en la UI antes de llamar al servicio (ver
/// `docs/planes-implementados/plan-control-rutas.md`, "Bloqueo transitorio
/// por documento vencido").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ResultadoSalidaRuta {
    Permitido,
    PermitidoConAutorizacion,
}

/// Regla de negocio central del módulo: la fecha del documento de carga
/// debe coincidir con hoy -- si no coincide (antes o después, no sólo
/// "vencido"), se exige que el guardia confirme que tiene el correo de
/// autorización. `None` = bloqueado, la UI no debe dejar continuar.
///
/// `tiene_correo_autorizacion` es una declaración del guardia (checkbox en
/// pantalla, ver `PantallaRutas.kt`), no algo que este dominio pueda
/// verificar por sí mismo -- mismo nivel de confianza que ya usa
/// `RegistroIngresoService` para otros datos declarados en el momento.
pub fn verificar_fecha_documento(
    fecha_documento: NaiveDate,
    hoy: NaiveDate,
    tiene_correo_autorizacion: bool,
) -> Option<ResultadoSalidaRuta> {
    if fecha_documento == hoy {
        Some(ResultadoSalidaRuta::Permitido)
    } else if tiene_correo_autorizacion {
        Some(ResultadoSalidaRuta::PermitidoConAutorizacion)
    } else {
        None
    }
}

pub fn salida_es_cronologicamente_valida(
    fecha_hora_salida: chrono::DateTime<chrono::Utc>,
    fecha_hora_retorno: chrono::DateTime<chrono::Utc>,
) -> bool {
    fecha_hora_retorno >= fecha_hora_salida
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fecha(texto: &str) -> NaiveDate {
        texto.parse().unwrap()
    }

    #[test]
    fn fecha_documento_igual_a_hoy_permite_sin_autorizacion() {
        let hoy = fecha("2026-09-15");
        assert_eq!(
            verificar_fecha_documento(hoy, hoy, false),
            Some(ResultadoSalidaRuta::Permitido)
        );
    }

    #[test]
    fn fecha_documento_distinta_sin_correo_bloquea() {
        assert_eq!(
            verificar_fecha_documento(fecha("2026-09-14"), fecha("2026-09-15"), false),
            None
        );
    }

    #[test]
    fn fecha_documento_distinta_con_correo_permite_con_autorizacion() {
        assert_eq!(
            verificar_fecha_documento(fecha("2026-09-14"), fecha("2026-09-15"), true),
            Some(ResultadoSalidaRuta::PermitidoConAutorizacion)
        );
    }

    #[test]
    fn fecha_documento_futura_tambien_requiere_autorizacion() {
        // "No coincide con la fecha actual" cubre ambos lados, no sólo
        // documentos atrasados -- ver doc-comment de la función.
        assert_eq!(
            verificar_fecha_documento(fecha("2026-09-16"), fecha("2026-09-15"), false),
            None
        );
        assert_eq!(
            verificar_fecha_documento(fecha("2026-09-16"), fecha("2026-09-15"), true),
            Some(ResultadoSalidaRuta::PermitidoConAutorizacion)
        );
    }

    #[test]
    fn retorno_no_puede_ser_anterior_a_la_salida() {
        let salida = chrono::Utc::now();
        assert!(salida_es_cronologicamente_valida(salida, salida));
        assert!(!salida_es_cronologicamente_valida(
            salida,
            salida - chrono::Duration::seconds(1)
        ));
    }
}
