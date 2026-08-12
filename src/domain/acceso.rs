use chrono::{Duration, NaiveDate};

use super::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};
use crate::models::contratista::Contratista;

const DIAS_ADVERTENCIA_PRAIND: i64 = 30;

pub fn verificar_acceso(contratista: &Contratista, hoy: NaiveDate) -> ResultadoAcceso {
    // Regla 1:
    // Si no tiene autorización de acceso,
    // no puede ingresar bajo ninguna circunstancia.
    if !contratista.tiene_acceso {
        return ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso);
    }

    // Regla 2:
    // Si el contratista no requiere PRAIND, puede ingresar.
    if !contratista.requiere_praind() {
        return ResultadoAcceso::Permitido;
    }

    // Todo contratista que requiere PRAIND debe tener
    // una fecha de vencimiento registrada.
    let Some(fecha_vencimiento) = contratista.fecha_vencimiento_praind else {
        return ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido);
    };

    // Regla 3:
    // PRAIND vencido = acceso denegado.
    if fecha_vencimiento < hoy {
        return ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido);
    }

    // Regla 4:
    // Si vence dentro de los próximos 30 días,
    // puede ingresar pero recibe advertencia.
    let limite_advertencia = hoy + Duration::days(DIAS_ADVERTENCIA_PRAIND);

    if fecha_vencimiento <= limite_advertencia {
        return ResultadoAcceso::PermitidoConAdvertencia;
    }

    // Regla 5:
    // PRAIND vigente y con más de 30 días.
    ResultadoAcceso::Permitido
}
