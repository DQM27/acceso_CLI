use chrono::{Duration, NaiveDate};

use super::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};
use crate::models::contratista::Contratista;

pub const DIAS_ADVERTENCIA_PRAIND: i64 = 30;

/// Versión de las 6 reglas de `verificar_acceso`, grabada en cada movimiento
/// (`registro_ingresos.reglas_version`) para poder distinguir en el
/// histórico bajo qué versión de la lógica se decidió cada entrada. Vive
/// aquí, junto a las reglas que versiona, para que cambiarlas sin subir este
/// número salte a la vista — antes vivía en `models::registro_ingreso`, una
/// struct de persistencia sin relación visible con `verificar_acceso`.
/// Re-exportada desde allá (`models::registro_ingreso::VERSION_REGLAS_ACCESO`)
/// para no romper a quien ya la importa de ese camino.
pub const VERSION_REGLAS_ACCESO: i64 = 2;

pub fn verificar_acceso(contratista: &Contratista, hoy: NaiveDate) -> ResultadoAcceso {
    // Regla 0:
    // Si la empresa del contratista está inactiva, no puede ingresar bajo
    // ninguna circunstancia — sin importar su acceso individual.
    if !contratista.empresa_activa() {
        return ResultadoAcceso::Denegado(MotivoDenegacion::EmpresaInactiva);
    }

    // Regla 1:
    // Si no tiene autorización de acceso,
    // no puede ingresar bajo ninguna circunstancia.
    if !contratista.tiene_acceso {
        return ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso);
    }

    // Regla 2:
    // Si el contratista no requiere PRAIND, puede ingresar.
    if !super::contratista::requiere_praind(contratista) {
        return ResultadoAcceso::Permitido;
    }

    // Todo contratista que requiere PRAIND debe tener
    // una fecha de vencimiento registrada.
    let Some(fecha_vencimiento) = contratista.fecha_vencimiento_praind else {
        return ResultadoAcceso::Denegado(MotivoDenegacion::PraindNoRegistrado);
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

/// Aviso de una fila en las listas de contratistas (escritorio y móvil).
/// Sólo informa: si puede entrar lo decide `verificar_acceso` al preparar
/// el ingreso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvisoAccesoLista {
    AccesoDenegado,
    PraindVencido,
}

/// Sin acceso pesa más que el PRAIND vencido: se muestra un solo aviso.
pub fn aviso_acceso_en_lista(
    tiene_acceso: bool,
    fecha_vencimiento_praind: Option<NaiveDate>,
    hoy: NaiveDate,
) -> Option<AvisoAccesoLista> {
    if !tiene_acceso {
        return Some(AvisoAccesoLista::AccesoDenegado);
    }
    super::contratista::praind_vencido(fecha_vencimiento_praind, hoy)
        .then_some(AvisoAccesoLista::PraindVencido)
}

#[cfg(test)]
mod tests_aviso_lista {
    use super::*;

    fn fecha(dia: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, dia).unwrap()
    }

    #[test]
    fn sin_acceso_pesa_mas_que_el_praind_vencido() {
        assert_eq!(
            aviso_acceso_en_lista(false, Some(fecha(1)), fecha(27)),
            Some(AvisoAccesoLista::AccesoDenegado)
        );
    }

    #[test]
    fn praind_vencido_solo_si_la_fecha_ya_paso() {
        assert_eq!(
            aviso_acceso_en_lista(true, Some(fecha(26)), fecha(27)),
            Some(AvisoAccesoLista::PraindVencido)
        );
        assert_eq!(
            aviso_acceso_en_lista(true, Some(fecha(27)), fecha(27)),
            None
        );
        assert_eq!(aviso_acceso_en_lista(true, None, fecha(27)), None);
    }
}
