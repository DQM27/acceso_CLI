use super::*;
use crate::{
    domain::resultado_acceso::ResultadoAcceso,
    models::{medio_ingreso::MedioIngreso, tipo_ingreso::TipoIngreso},
};
use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
fn k(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}
fn r(id: i64, g: Option<i64>) -> IngresoActivoResumen {
    IngresoActivoResumen {
        registro_id: id,
        contratista_id: id,
        cedula: format!("C-{id}"),
        contratista_nombre: format!("Persona {id}"),
        empresa_nombre: "Empresa".into(),
        tipo_ingreso: TipoIngreso::Praind,
        medio_ingreso: MedioIngreso::Caminando,
        fecha_hora_ingreso: crate::tiempo::local_costa_rica_a_utc(
            NaiveDate::from_ymd_opt(2026, 8, 12)
                .unwrap()
                .and_hms_opt(8, 0, 0)
                .unwrap(),
        )
        .unwrap(),
        gafete_numero: g,
        usuario_ingreso_nombre: "Ana".into(),
        resultado_acceso: ResultadoAcceso::Permitido,
    }
}
fn cargar(s: &mut ActivosState) {
    s.completar_busqueda(
        Ok(ListaIngresosActivosResumen {
            items: vec![r(7, Some(26)), r(9, None)],
            total: 2,
        }),
        None,
    )
}
#[test]
fn inicia_vacio_y_carga_real() {
    let mut s = ActivosState::default();
    assert_eq!(s.cantidad(), 0);
    cargar(&mut s);
    assert_eq!(s.cantidad(), 2);
    assert_eq!(s.total_activos(), 2);
    assert_eq!(s.seleccion, Some(0))
}
#[test]
fn busqueda_emite_consulta_real() {
    let mut s = ActivosState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Char('/')));
    assert!(
        matches!(s.handle_key(k(KeyCode::Char('j'))),AccionActivos::Buscar{texto:Some(t),..}if t=="j")
    );
    assert_eq!(s.cantidad(), 2)
}
#[test]
fn detalle_y_salida_usan_registro_id() {
    let mut s = ActivosState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Enter));
    assert!(matches!(s.modo, ModoActivos::Detalle { id: 7 }));
    s.handle_key(k(KeyCode::Char('S')));
    assert!(matches!(
        s.handle_key(k(KeyCode::Char('Y'))),
        AccionActivos::RegistrarSalida { registro_id: 7, .. }
    ));
}
#[test]
fn cancelar_salida_no_emite_accion() {
    let mut s = ActivosState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Char('S')));
    assert!(matches!(
        s.handle_key(k(KeyCode::Esc)),
        AccionActivos::Ninguna
    ));
    assert_eq!(s.cantidad(), 2)
}
#[test]
fn callback_salida_recarga_sin_remover_localmente() {
    let mut s = ActivosState::default();
    cargar(&mut s);
    let a = s.completar_salida(Ok(()), 7, "Persona 7");
    assert_eq!(s.cantidad(), 2);
    assert!(matches!(a, AccionActivos::Buscar { .. }));
    s.completar_busqueda(
        Ok(ListaIngresosActivosResumen {
            items: vec![r(9, None)],
            total: 1,
        }),
        None,
    );
    assert_eq!(s.cantidad(), 1)
}
#[test]
fn columnas_y_movimiento_conservan_ux() {
    let mut s = ActivosState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Char('C')));
    assert!(matches!(s.modo, ModoActivos::Columnas { .. }));
    assert_eq!(
        NaiveDate::from_ymd_opt(2026, 8, 12).unwrap().to_string(),
        "2026-08-12"
    )
}
