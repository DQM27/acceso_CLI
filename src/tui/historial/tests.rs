use chrono::NaiveDate;

use super::render::valor;
use super::*;
use crate::models::{empresa::Empresa, medio_ingreso::MedioIngreso};

fn tecla(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn pagina(cantidad: usize, total: usize) -> PaginaHistorial {
    PaginaHistorial {
        items: (0..cantidad)
            .map(|i| MovimientoIngresoResumen {
                registro_id: i as i64 + 1,
                contratista_id: 10,
                cedula: "101010101".into(),
                contratista_nombre: "Ana Solano".into(),
                empresa_nombre: "Brisas".into(),
                tipo_ingreso: TipoIngreso::Praind,
                medio_ingreso: MedioIngreso::Caminando,
                fecha_hora_ingreso: NaiveDate::from_ymd_opt(2026, 8, 12)
                    .unwrap()
                    .and_hms_opt(8, 30, 0)
                    .unwrap(),
                fecha_hora_salida: None,
                gafete_numero: Some(7),
                usuario_ingreso_nombre: "Quintana".into(),
                usuario_salida_nombre: None,
            })
            .collect(),
        total,
    }
}

#[test]
fn fechas_visuales_generan_rango_backend_inclusivo_exclusivo() {
    let filtro = FiltrosHistorial {
        desde: "01/08/2026".into(),
        hasta: "12/08/2026".into(),
        ..Default::default()
    };
    let query = construir(&filtro, "", 50, 0).unwrap();
    assert_eq!(
        query.desde,
        NaiveDate::from_ymd_opt(2026, 8, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    );
    assert_eq!(
        query.hasta,
        NaiveDate::from_ymd_opt(2026, 8, 13)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    );
}

#[test]
fn filtros_mapean_ids_enums_gafete_exacto_y_texto() {
    let filtro = FiltrosHistorial {
        nombre_cedula: "  Ana  ".into(),
        empresa_id: Some(8),
        tipo: Some(TipoIngreso::PorCorreo),
        gafete: "27".into(),
        estado: EstadoMovimiento::Cerrados,
        ..Default::default()
    };
    let query = construir(&filtro, "", 50, 100).unwrap();
    assert_eq!(query.texto_persona.as_deref(), Some("Ana"));
    assert_eq!(query.empresa_id, Some(8));
    assert_eq!(query.tipo_ingreso, Some(TipoIngreso::PorCorreo));
    assert_eq!(query.gafete_numero, Some(27));
    assert_eq!(query.estado, EstadoMovimiento::Cerrados);
    assert_eq!((query.limite, query.offset), (50, 100));
}

#[test]
fn busqueda_rapida_se_combina_con_filtros_aplicados() {
    let mut state = HistorialState::default();
    state.filtro_aplicado.empresa_id = Some(3);
    state.handle_key(tecla(KeyCode::Char('/')));
    let accion = state.handle_key(tecla(KeyCode::Char('a')));
    let AccionHistorial::Consultar(filtro) = accion else {
        panic!("debía consultar")
    };
    assert_eq!(filtro.texto_persona.as_deref(), Some("a"));
    assert_eq!(filtro.empresa_id, Some(3));
}

#[test]
fn selector_de_empresa_conserva_id_real() {
    let mut state = HistorialState::default();
    state.completar_empresas(Ok(vec![Empresa {
        id: 42,
        nombre: "Brisas".into(),
    }]));
    state.modo = ModoHistorial::Desplegable {
        campo: CampoFiltro::Empresa,
        seleccion_filtro: 3,
        opcion: 1,
    };
    state.handle_key(tecla(KeyCode::Enter));
    assert_eq!(state.filtro_edicion.empresa_id, Some(42));
}

#[test]
fn completar_pagina_usa_total_real_y_seleccion_segura() {
    let mut state = HistorialState::default();
    state.completar(Ok(pagina(2, 73)));
    assert_eq!(state.total, 73);
    assert_eq!(state.registros.len(), 2);
    assert_eq!(state.seleccion, Some(0));
    state.completar(Ok(pagina(0, 0)));
    assert_eq!(state.seleccion, None);
}

#[test]
fn page_down_y_page_up_emiten_offsets_de_cincuenta() {
    let mut state = HistorialState::default();
    state.completar(Ok(pagina(50, 120)));
    let AccionHistorial::Consultar(siguiente) = state.handle_key(tecla(KeyCode::PageDown)) else {
        panic!("debía consultar")
    };
    assert_eq!(siguiente.offset, 50);
    let AccionHistorial::Consultar(anterior) = state.handle_key(tecla(KeyCode::PageUp)) else {
        panic!("debía consultar")
    };
    assert_eq!(anterior.offset, 0);
}

#[test]
fn fechas_y_campos_rechazan_formato_o_caracteres_invalidos() {
    assert!(
        construir(
            &FiltrosHistorial {
                desde: "31/02/2026".into(),
                ..Default::default()
            },
            "",
            50,
            0
        )
        .is_err()
    );
    let mut state = HistorialState::default();
    state.filtro_edicion.desde.clear();
    for c in "12082026abc99".chars() {
        state.agregar(CampoFiltro::Desde, c);
    }
    for c in "12a3".chars() {
        state.agregar(CampoFiltro::Gafete, c);
    }
    assert_eq!(state.filtro_edicion.desde, "12/08/2026");
    assert_eq!(state.filtro_edicion.gafete, "123");
}

#[test]
fn detalle_y_columnas_siguen_siendo_solo_presentacion() {
    let mut state = HistorialState::default();
    state.completar(Ok(pagina(1, 1)));
    assert_eq!(
        state.handle_key(tecla(KeyCode::Enter)),
        AccionHistorial::Ninguna
    );
    assert!(matches!(state.modo, ModoHistorial::Detalle { .. }));
    state.handle_key(tecla(KeyCode::Esc));
    state.handle_key(tecla(KeyCode::Char('c')));
    state.handle_key(tecla(KeyCode::Char(' ')));
    assert!(!state.columnas[0].1);
}

#[test]
fn activo_muestra_salida_y_usuario_salida_ausentes() {
    let registro = pagina(1, 1).items.remove(0);
    assert_eq!(valor(&registro, ColumnaHistorial::Salida), "--");
    assert_eq!(valor(&registro, ColumnaHistorial::UsuarioSalida), "--");
}
