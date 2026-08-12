use chrono::{Duration, NaiveDate};

use crate::models::contratista::Contratista;
use crate::models::tipo_ingreso::TipoIngreso;

use super::resultado_acceso::{
    MotivoDenegacion,
    ResultadoAcceso,
};

const DIAS_ADVERTENCIA_PRAIND: i64 = 30;

pub fn verificar_acceso(
    contratista: &Contratista,
    hoy: NaiveDate,
) -> ResultadoAcceso {
    // Regla 1:
    // Si no tiene autorización de acceso,
    // no puede ingresar bajo ninguna circunstancia.
    if !contratista.tiene_acceso {
        return ResultadoAcceso::Denegado(
            MotivoDenegacion::SinAcceso,
        );
    }

    // Regla 2:
    // POR CORREO no requiere PRAIND.
    if contratista.tipo_ingreso == TipoIngreso::PorCorreo {
        return ResultadoAcceso::Permitido;
    }

    // Regla 3:
    // SWAT no requiere PRAIND.
    if contratista.tipo_ingreso == TipoIngreso::Swat {
        return ResultadoAcceso::Permitido;
    }

    // PRAIND e IN HOUSE requieren fecha de vencimiento.
    let Some(fecha_vencimiento) =
        contratista.fecha_vencimiento_praind
    else {
        return ResultadoAcceso::Denegado(
            MotivoDenegacion::PraindVencido,
        );
    };

    // Regla 4:
    // PRAIND vencido = acceso denegado.
    if fecha_vencimiento < hoy {
        return ResultadoAcceso::Denegado(
            MotivoDenegacion::PraindVencido,
        );
    }

    // Regla 5:
    // Si vence dentro de los próximos 30 días,
    // puede ingresar pero recibe advertencia.
    let limite_advertencia =
        hoy + Duration::days(DIAS_ADVERTENCIA_PRAIND);

    if fecha_vencimiento <= limite_advertencia {
        return ResultadoAcceso::PermitidoConAdvertencia;
    }

    // Regla 6:
    // PRAIND vigente y con más de 30 días.
    ResultadoAcceso::Permitido
}