use chrono::{DateTime, NaiveDateTime, Utc};
use control_acceso::domain::registro_ingreso::salida_es_cronologicamente_valida;

fn fecha(valor: &str) -> DateTime<Utc> {
    control_acceso::tiempo::local_costa_rica_a_utc(
        NaiveDateTime::parse_from_str(valor, "%Y-%m-%d %H:%M:%S").unwrap(),
    )
    .unwrap()
}

#[test]
fn salida_posterior_es_valida() {
    assert!(salida_es_cronologicamente_valida(
        fecha("2026-08-11 08:00:00"),
        fecha("2026-08-11 17:00:00")
    ));
}

#[test]
fn salida_igual_al_ingreso_es_valida() {
    let instante = fecha("2026-08-11 08:00:00");
    assert!(salida_es_cronologicamente_valida(instante, instante));
}

#[test]
fn salida_anterior_es_invalida() {
    assert!(!salida_es_cronologicamente_valida(
        fecha("2026-08-11 08:00:00"),
        fecha("2026-08-11 07:59:59")
    ));
}
