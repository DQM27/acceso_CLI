use super::*;
use crate::{domain::resultado_acceso::ResultadoAcceso, models::tipo_ingreso::TipoIngreso};
use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
fn k(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}
fn resumen() -> ContratistaResumen {
    ContratistaResumen {
        id: 7,
        empresa_id: 2,
        cedula: "001".into(),
        nombre: "José".into(),
        empresa_nombre: "Álvarez".into(),
        tipo_ingreso: TipoIngreso::PorCorreo,
        fecha_vencimiento_praind: None,
        es_personal_ruta: false,
        tiene_acceso: true,
    }
}
fn preparar(requiere: bool) -> PreparacionIngreso {
    PreparacionIngreso {
        contratista_id: 7,
        cedula: "001".into(),
        nombre: "José".into(),
        empresa_nombre: "Álvarez".into(),
        tipo_ingreso: TipoIngreso::PorCorreo,
        resultado_acceso: ResultadoAcceso::Permitido,
        requiere_gafete: requiere,
        tiene_ingreso_activo: false,
    }
}
#[test]
fn inicia_vacio_y_busqueda_emite_acciones() {
    let mut s = NuevoIngresoState::default();
    assert!(s.contratistas.is_empty());
    assert!(
        matches!(s.handle_key(k(KeyCode::Char('j'))),AccionNuevoIngreso::Buscar{texto:Some(t)}if t=="j")
    );
    s.completar_busqueda(Ok(vec![resumen()]));
    assert_eq!(s.seleccion, Some(0));
    assert!(matches!(
        s.handle_key(k(KeyCode::Enter)),
        AccionNuevoIngreso::Preparar { contratista_id: 7 }
    ));
}
#[test]
fn preparacion_real_controla_flujo_y_gafete() {
    let mut s = NuevoIngresoState::default();
    s.completar_busqueda(Ok(vec![resumen()]));
    s.completar_preparacion(Ok(preparar(true)));
    assert_eq!(s.etapa, EtapaNuevoIngreso::Preparacion);
    s.handle_key(k(KeyCode::Enter));
    s.handle_key(k(KeyCode::Enter));
    assert_eq!(s.etapa, EtapaNuevoIngreso::Gafete);
    for c in "26".chars() {
        s.handle_key(k(KeyCode::Char(c)));
    }
    assert!(matches!(
        s.handle_key(k(KeyCode::Enter)),
        AccionNuevoIngreso::ConsultarGafete { numero: 26 }
    ));
    s.completar_gafete(Ok(false));
    assert_eq!(s.etapa, EtapaNuevoIngreso::Confirmar);
    assert!(matches!(
        s.handle_key(k(KeyCode::Char('Y'))),
        AccionNuevoIngreso::Registrar {
            contratista_id: 7,
            gafete: Some(26),
            ..
        }
    ));
}
#[test]
fn sin_gafete_confirma_none_y_cancelar_no_registra() {
    let mut s = NuevoIngresoState::default();
    s.completar_busqueda(Ok(vec![resumen()]));
    s.completar_preparacion(Ok(preparar(false)));
    s.handle_key(k(KeyCode::Enter));
    s.handle_key(k(KeyCode::Enter));
    assert_eq!(s.etapa, EtapaNuevoIngreso::Confirmar);
    assert!(matches!(
        s.handle_key(k(KeyCode::Esc)),
        AccionNuevoIngreso::Ninguna
    ));
    assert!(matches!(s.etapa, EtapaNuevoIngreso::Medio { .. }));
}
#[test]
fn denegado_o_activo_no_continua() {
    for p in [
        {
            let mut p = preparar(false);
            p.resultado_acceso = ResultadoAcceso::Denegado(
                crate::domain::resultado_acceso::MotivoDenegacion::SinAcceso,
            );
            p
        },
        {
            let mut p = preparar(false);
            p.tiene_ingreso_activo = true;
            p
        },
    ] {
        let mut s = NuevoIngresoState::default();
        s.completar_preparacion(Ok(p.clone()));
        s.handle_key(k(KeyCode::Enter));
        assert_eq!(s.etapa, EtapaNuevoIngreso::Preparacion);
    }
}
#[test]
fn gafete_invalido_y_ocupado_son_presentables() {
    let mut s = NuevoIngresoState {
        etapa: EtapaNuevoIngreso::Gafete,
        ..Default::default()
    };
    s.handle_key(k(KeyCode::Enter));
    assert_eq!(s.error.as_deref(), Some("El gafete es requerido"));
    s.gafete_texto = "10".into();
    s.completar_gafete(Ok(true));
    assert_eq!(s.error.as_deref(), Some("El gafete ya está en uso"));
}
#[test]
fn fecha_determinista_pertenece_al_core_no_al_state() {
    let fecha = NaiveDate::from_ymd_opt(2026, 8, 12).unwrap();
    assert_eq!(fecha.to_string(), "2026-08-12");
}
