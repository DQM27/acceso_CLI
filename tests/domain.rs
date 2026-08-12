use chrono::NaiveDate;

use control_acceso::domain::acceso::verificar_acceso;
use control_acceso::domain::resultado_acceso::{
    MotivoDenegacion,
    ResultadoAcceso,
};
use control_acceso::models::contratista::Contratista;
use control_acceso::models::tipo_ingreso::TipoIngreso;

fn contratista(
    tipo_ingreso: TipoIngreso,
    fecha_vencimiento_praind: Option<NaiveDate>,
    tiene_acceso: bool,
) -> Contratista {
    Contratista {
        id: 1,
        cedula: "123456789".to_string(),
        nombre: "Juan Pérez".to_string(),
        empresa_id: 1,
        tipo_ingreso,
        fecha_vencimiento_praind,
        tiene_acceso,
    }
}

#[test]
fn debe_denegar_si_no_tiene_acceso() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        false,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(
            MotivoDenegacion::SinAcceso
        )
    );
}

#[test]
fn debe_denegar_si_praind_esta_vencido() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2026, 8, 9).unwrap()),
        true,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(
            MotivoDenegacion::PraindVencido
        )
    );
}

#[test]
fn debe_advertir_si_praind_vence_en_30_dias() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2026, 9, 9).unwrap()),
        true,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::PermitidoConAdvertencia
    );
}

#[test]
fn debe_advertir_si_praind_vence_hoy() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::Praind,
        Some(hoy),
        true,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::PermitidoConAdvertencia
    );
}

#[test]
fn debe_denegar_si_praind_no_tiene_fecha() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::Praind,
        None,
        true,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(
            MotivoDenegacion::PraindVencido
        )
    );
}

#[test]
fn debe_permitir_praind_vigente() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        true,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Permitido
    );
}

#[test]
fn debe_permitir_ingreso_por_correo_sin_praind() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::PorCorreo,
        None,
        true,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Permitido
    );
}

#[test]
fn debe_permitir_swat_sin_praind() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::Swat,
        None,
        true,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Permitido
    );
}

#[test]
fn in_house_con_praind_vencido_debe_ser_denegado() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::InHouse,
        Some(NaiveDate::from_ymd_opt(2026, 8, 9).unwrap()),
        true,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(
            MotivoDenegacion::PraindVencido
        )
    );
}

#[test]
fn in_house_requiere_praind_vigente() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::InHouse,
        Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        true,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Permitido
    );
}

#[test]
fn in_house_sin_fecha_debe_ser_denegado() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::InHouse,
        None,
        true,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(
            MotivoDenegacion::PraindVencido
        )
    );
}

#[test]
fn swat_sin_acceso_debe_ser_denegado() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(
        TipoIngreso::Swat,
        None,
        false,
    );

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(
            MotivoDenegacion::SinAcceso
        )
    );
}