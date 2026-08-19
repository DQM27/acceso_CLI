use chrono::NaiveDate;

use control_acceso::domain::acceso::verificar_acceso;
use control_acceso::domain::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};
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
        es_personal_ruta: false,
        tiene_acceso,
        empresa_activa: true,
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
        ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso)
    );
}

#[test]
fn debe_denegar_si_la_empresa_esta_inactiva() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let mut contratista = contratista(
        TipoIngreso::PorCorreo,
        None,
        true,
    );
    contratista.empresa_activa = false;

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(MotivoDenegacion::EmpresaInactiva)
    );
}

#[test]
fn empresa_inactiva_bloquea_incluso_con_acceso_individual_y_praind_vigente() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let mut contratista = contratista(
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        true,
    );
    contratista.empresa_activa = false;

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(MotivoDenegacion::EmpresaInactiva)
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
        ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido)
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

    assert_eq!(resultado, ResultadoAcceso::PermitidoConAdvertencia);
}

#[test]
fn debe_advertir_si_praind_vence_hoy() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(TipoIngreso::Praind, Some(hoy), true);

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(resultado, ResultadoAcceso::PermitidoConAdvertencia);
}

#[test]
fn debe_denegar_si_praind_no_tiene_fecha() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(TipoIngreso::Praind, None, true);

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido)
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

    assert_eq!(resultado, ResultadoAcceso::Permitido);
}

#[test]
fn debe_permitir_ingreso_por_correo_sin_praind() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(TipoIngreso::PorCorreo, None, true);

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(resultado, ResultadoAcceso::Permitido);
}

#[test]
fn debe_permitir_swat_sin_praind() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(TipoIngreso::Swat, None, true);

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(resultado, ResultadoAcceso::Permitido);
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
        ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido)
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

    assert_eq!(resultado, ResultadoAcceso::Permitido);
}

#[test]
fn in_house_sin_fecha_debe_ser_denegado() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(TipoIngreso::InHouse, None, true);

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido)
    );
}

#[test]
fn swat_sin_acceso_debe_ser_denegado() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();

    let contratista = contratista(TipoIngreso::Swat, None, false);

    let resultado = verificar_acceso(&contratista, hoy);

    assert_eq!(
        resultado,
        ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso)
    );
}

#[test]
fn reglas_definitivas_de_praind_por_tipo() {
    let casos = [
        (TipoIngreso::Praind, true),
        (TipoIngreso::InHouse, true),
        (TipoIngreso::PorCorreo, false),
        (TipoIngreso::Swat, false),
    ];

    for (tipo_ingreso, esperado) in casos {
        let contratista = contratista(tipo_ingreso, None, true);
        assert_eq!(contratista.requiere_praind(), esperado);
    }
}

#[test]
fn personal_de_ruta_requiere_praind_sin_importar_el_tipo() {
    for tipo_ingreso in [
        TipoIngreso::Praind,
        TipoIngreso::InHouse,
        TipoIngreso::PorCorreo,
        TipoIngreso::Swat,
    ] {
        let mut contratista = contratista(tipo_ingreso, None, true);
        contratista.es_personal_ruta = true;
        assert!(contratista.requiere_praind());
    }
}

#[test]
fn reglas_definitivas_de_gafete_por_tipo() {
    let casos = [
        (TipoIngreso::Praind, true),
        (TipoIngreso::InHouse, false),
        (TipoIngreso::PorCorreo, true),
        (TipoIngreso::Swat, false),
    ];

    for (tipo_ingreso, esperado) in casos {
        let contratista = contratista(tipo_ingreso, None, true);
        assert_eq!(contratista.requiere_gafete(), esperado);
    }
}

#[test]
fn personal_de_ruta_no_requiere_gafete_sin_importar_el_tipo() {
    for tipo_ingreso in [
        TipoIngreso::Praind,
        TipoIngreso::InHouse,
        TipoIngreso::PorCorreo,
        TipoIngreso::Swat,
    ] {
        let mut contratista = contratista(tipo_ingreso, None, true);
        contratista.es_personal_ruta = true;
        assert!(!contratista.requiere_gafete());
    }
}

#[test]
fn personal_de_ruta_sin_praind_debe_ser_denegado() {
    let hoy = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();
    let mut contratista = contratista(TipoIngreso::PorCorreo, None, true);
    contratista.es_personal_ruta = true;

    assert_eq!(
        verificar_acceso(&contratista, hoy),
        ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido)
    );
}
