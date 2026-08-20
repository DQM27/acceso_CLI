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
fn avisa_cuando_quedan_resultados_fuera_de_la_lista() {
    let mut s = NuevoIngresoState::default();
    assert_eq!(s.resultados_ocultos(), None);

    s.completar_busqueda(Ok(PaginaContratistas {
        items: vec![resumen()],
        total: 120,
    }));
    assert_eq!(s.resultados_ocultos(), Some(120));

    s.completar_busqueda(Ok(PaginaContratistas {
        items: vec![resumen()],
        total: 1,
    }));
    assert_eq!(s.resultados_ocultos(), None);
}

#[test]
fn inicia_vacio_y_busqueda_emite_acciones_tras_el_debounce() {
    let mut s = NuevoIngresoState::default();
    assert!(s.contratistas.is_empty());
    assert_eq!(
        s.handle_key(k(KeyCode::Char('j'))),
        AccionNuevoIngreso::Ninguna
    );
    let futuro =
        std::time::Instant::now() + DURACION_DEBOUNCE + std::time::Duration::from_millis(1);
    assert!(matches!(
        s.tick(futuro),
        AccionNuevoIngreso::Buscar{texto:Some(t)} if t=="j"
    ));
    s.completar_busqueda(Ok(PaginaContratistas {
        items: vec![resumen()],
        total: 1,
    }));
    assert_eq!(s.seleccion, Some(0));
    assert!(matches!(
        s.handle_key(k(KeyCode::Enter)),
        AccionNuevoIngreso::Preparar { contratista_id: 7 }
    ));
}
#[test]
fn preparacion_con_gafete_pasa_directo_al_formulario_unico() {
    let mut s = NuevoIngresoState::default();
    s.completar_busqueda(Ok(PaginaContratistas {
        items: vec![resumen()],
        total: 1,
    }));
    s.completar_preparacion(Ok(preparar(true)));
    assert_eq!(s.etapa, EtapaNuevoIngreso::Formulario);
    assert!(!s.campo_es_gafete()); // arranca en Medio

    // TAB mueve el foco a Gafete cuando se requiere.
    s.handle_key(k(KeyCode::Tab));
    assert!(s.campo_es_gafete());
    for c in "26".chars() {
        s.handle_key(k(KeyCode::Char(c)));
    }
    assert!(matches!(
        s.handle_key(k(KeyCode::Enter)),
        AccionNuevoIngreso::Registrar {
            contratista_id: 7,
            gafete: Some(26),
            ..
        }
    ));
}
#[test]
fn sin_gafete_enter_registra_directo_desde_medio() {
    let mut s = NuevoIngresoState::default();
    s.completar_busqueda(Ok(PaginaContratistas {
        items: vec![resumen()],
        total: 1,
    }));
    s.completar_preparacion(Ok(preparar(false)));
    assert_eq!(s.etapa, EtapaNuevoIngreso::Formulario);
    assert!(matches!(
        s.handle_key(k(KeyCode::Enter)),
        AccionNuevoIngreso::Registrar {
            contratista_id: 7,
            gafete: None,
            ..
        }
    ));
}
#[test]
fn flechas_cambian_el_medio_y_esc_vuelve_a_buscar() {
    let mut s = NuevoIngresoState::default();
    s.completar_busqueda(Ok(PaginaContratistas {
        items: vec![resumen()],
        total: 1,
    }));
    s.completar_preparacion(Ok(preparar(false)));
    let inicial = s.medio_actual();
    s.handle_key(k(KeyCode::Right));
    assert_ne!(s.medio_actual(), inicial);

    s.handle_key(k(KeyCode::Esc));
    assert_eq!(s.etapa, EtapaNuevoIngreso::Buscar);
}
#[test]
fn denegado_o_activo_no_continua_y_deja_mensaje_en_buscar() {
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
        assert_eq!(s.etapa, EtapaNuevoIngreso::Buscar);
        assert!(s.error.is_some());
    }
}
#[test]
fn gafete_vacio_es_presentable_y_no_registra() {
    let mut s = NuevoIngresoState::default();
    s.completar_busqueda(Ok(PaginaContratistas {
        items: vec![resumen()],
        total: 1,
    }));
    s.completar_preparacion(Ok(preparar(true)));
    s.handle_key(k(KeyCode::Tab));
    assert!(matches!(
        s.handle_key(k(KeyCode::Enter)),
        AccionNuevoIngreso::Ninguna
    ));
    assert_eq!(s.error.as_deref(), Some("El gafete es requerido"));
}
#[test]
fn gafete_ocupado_llega_como_error_del_backend_al_registrar() {
    let mut s = NuevoIngresoState::default();
    s.completar_busqueda(Ok(PaginaContratistas {
        items: vec![resumen()],
        total: 1,
    }));
    s.completar_preparacion(Ok(preparar(true)));
    assert!(!s.completar_registro(Err("El gafete ya está en uso".into())));
    assert_eq!(s.error.as_deref(), Some("El gafete ya está en uso"));
    // el formulario sigue abierto para corregir, no vuelve a Buscar.
    assert_eq!(s.etapa, EtapaNuevoIngreso::Formulario);
}
#[test]
fn fecha_determinista_pertenece_al_core_no_al_state() {
    let fecha = NaiveDate::from_ymd_opt(2026, 8, 12).unwrap();
    assert_eq!(fecha.to_string(), "2026-08-12");
}
