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

/// Resultado de [`verificar_acceso`] para mostrar en una lista, con los mismos
/// nombres que la vista `panel_contratistas_estado` del panel web
/// (`PERMITIDO`, `PRAIND_VENCIDO`...): escritorio y panel dicen lo mismo de
/// cada contratista.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum EstadoAccesoLista {
    Permitido,
    PermitidoConAdvertencia,
    PraindVencido,
    PraindNoRegistrado,
    SinAcceso,
    EmpresaInactiva,
}

impl From<ResultadoAcceso> for EstadoAccesoLista {
    fn from(resultado: ResultadoAcceso) -> Self {
        match resultado {
            ResultadoAcceso::Permitido => Self::Permitido,
            ResultadoAcceso::PermitidoConAdvertencia => Self::PermitidoConAdvertencia,
            ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido) => Self::PraindVencido,
            ResultadoAcceso::Denegado(MotivoDenegacion::PraindNoRegistrado) => {
                Self::PraindNoRegistrado
            }
            ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso) => Self::SinAcceso,
            ResultadoAcceso::Denegado(MotivoDenegacion::EmpresaInactiva) => Self::EmpresaInactiva,
        }
    }
}

/// Días que faltan para que venza la PRAIND (negativo: ya venció). `None`
/// si no la requiere o no tiene fecha registrada.
pub fn dias_para_vencer_praind(contratista: &Contratista, hoy: NaiveDate) -> Option<i64> {
    if !super::contratista::requiere_praind(contratista) {
        return None;
    }
    contratista
        .fecha_vencimiento_praind
        .map(|vence| (vence - hoy).num_days())
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

#[cfg(test)]
mod tests_estado_lista {
    use super::*;
    use crate::models::tipo_ingreso::TipoIngreso;

    fn fecha(dia: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, dia).unwrap()
    }

    fn contratista(
        tipo: TipoIngreso,
        vence: Option<NaiveDate>,
        tiene_acceso: bool,
        empresa_activa: bool,
    ) -> Contratista {
        Contratista::reconstruir(
            1,
            "108470293".into(),
            "PERSONA".into(),
            1,
            tipo,
            vence,
            false,
            tiene_acceso,
            empresa_activa,
        )
    }

    #[test]
    fn cada_resultado_de_las_reglas_tiene_su_estado() {
        let hoy = fecha(5);
        let estado = |c: &Contratista| EstadoAccesoLista::from(verificar_acceso(c, hoy));
        assert_eq!(
            estado(&contratista(
                TipoIngreso::Praind,
                Some(fecha(30)),
                true,
                true
            )),
            EstadoAccesoLista::PermitidoConAdvertencia
        );
        assert_eq!(
            estado(&contratista(
                TipoIngreso::Praind,
                Some(fecha(4)),
                true,
                true
            )),
            EstadoAccesoLista::PraindVencido
        );
        assert_eq!(
            estado(&contratista(TipoIngreso::Praind, None, true, true)),
            EstadoAccesoLista::PraindNoRegistrado
        );
        assert_eq!(
            estado(&contratista(
                TipoIngreso::Praind,
                Some(fecha(4)),
                false,
                true
            )),
            EstadoAccesoLista::SinAcceso
        );
        assert_eq!(
            estado(&contratista(
                TipoIngreso::Praind,
                Some(fecha(30)),
                true,
                false
            )),
            EstadoAccesoLista::EmpresaInactiva
        );
    }

    #[test]
    fn dias_para_vencer_solo_si_requiere_praind_y_tiene_fecha() {
        let hoy = fecha(5);
        assert_eq!(
            dias_para_vencer_praind(
                &contratista(TipoIngreso::Praind, Some(fecha(8)), true, true),
                hoy
            ),
            Some(3)
        );
        assert_eq!(
            dias_para_vencer_praind(
                &contratista(TipoIngreso::Praind, Some(fecha(2)), true, true),
                hoy
            ),
            Some(-3)
        );
        assert_eq!(
            dias_para_vencer_praind(&contratista(TipoIngreso::Praind, None, true, true), hoy),
            None
        );
    }
}
