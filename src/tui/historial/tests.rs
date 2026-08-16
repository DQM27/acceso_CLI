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
                fecha_hora_ingreso: crate::tiempo::local_costa_rica_a_utc(
                    NaiveDate::from_ymd_opt(2026, 8, 12)
                        .unwrap()
                        .and_hms_opt(8, 30, 0)
                        .unwrap(),
                )
                .unwrap(),
                fecha_hora_salida: None,
                gafete_numero: Some(7),
                usuario_ingreso_nombre: "Quintana".into(),
                usuario_salida_nombre: None,
                resultado_acceso:
                    crate::models::registro_ingreso::ResultadoIngresoRegistrado::Permitido,
                motivo_resultado: None,
                reglas_version: crate::models::registro_ingreso::VERSION_REGLAS_ACCESO,
            })
            .collect(),
        total,
        corte_id: 100,
    }
}

#[test]
fn fechas_visuales_generan_rango_backend_inclusivo_exclusivo() {
    let filtro = FiltrosHistorial {
        desde: "01/08/2026".into(),
        hasta: "12/08/2026".into(),
        ..Default::default()
    };
    let query = construir(&filtro, "", 50, 0, None).unwrap();
    assert_eq!(
        query.desde,
        crate::tiempo::inicio_dia_costa_rica_utc(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap())
            .unwrap()
    );
    assert_eq!(
        query.hasta,
        crate::tiempo::inicio_dia_costa_rica_utc(NaiveDate::from_ymd_opt(2026, 8, 13).unwrap())
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
    let query = construir(&filtro, "", 50, 100, Some(77)).unwrap();
    assert_eq!(query.texto_persona.as_deref(), Some("Ana"));
    assert_eq!(query.empresa_id, Some(8));
    assert_eq!(query.tipo_ingreso, Some(TipoIngreso::PorCorreo));
    assert_eq!(query.gafete_numero, Some(27));
    assert_eq!(query.estado, EstadoMovimiento::Cerrados);
    assert_eq!((query.limite, query.offset), (50, 100));
    assert_eq!(query.corte_id, Some(77));
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
    assert_eq!(siguiente.corte_id, Some(100));
    let AccionHistorial::Consultar(anterior) = state.handle_key(tecla(KeyCode::PageUp)) else {
        panic!("debía consultar")
    };
    assert_eq!(anterior.offset, 0);
    assert_eq!(anterior.corte_id, Some(100));
}

#[test]
fn una_busqueda_nueva_descarta_el_corte_de_la_navegacion_anterior() {
    let mut state = HistorialState::default();
    state.completar(Ok(pagina(50, 120)));
    state.handle_key(tecla(KeyCode::Char('/')));
    let AccionHistorial::Consultar(filtro) = state.handle_key(tecla(KeyCode::Char('a'))) else {
        panic!("debía iniciar una consulta nueva")
    };
    assert_eq!(filtro.offset, 0);
    assert_eq!(filtro.corte_id, None);
}

#[test]
fn volver_a_abrir_historial_inicia_una_fotografia_nueva() {
    let mut state = HistorialState::default();
    state.completar(Ok(pagina(50, 120)));
    state.handle_key(tecla(KeyCode::PageDown));

    let AccionHistorial::Consultar(filtro) = state.solicitud_carga() else {
        panic!("debía recargar el historial")
    };
    assert_eq!(filtro.offset, 0);
    assert_eq!(filtro.corte_id, None);
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
            0,
            None
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
fn parsear_consulta_resuelve_empresa_por_nombre_parcial_y_deja_texto_libre() {
    let empresas = vec![
        Empresa {
            id: 5,
            nombre: "Brisas del Oeste".into(),
        },
        Empresa {
            id: 9,
            nombre: "Aldama Servicios".into(),
        },
    ];
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(&base, "empresa:\"Brisas del Oeste\" Ana", &empresas);
    assert_eq!(filtros.empresa_id, Some(5));
    assert_eq!(libre, "Ana");
}

#[test]
fn parsear_consulta_reconoce_tipo_estado_y_gafete() {
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(&base, "tipo:praind estado:activos gafete:27", &[]);
    assert_eq!(filtros.tipo, Some(TipoIngreso::Praind));
    assert_eq!(filtros.estado, EstadoMovimiento::Activos);
    assert_eq!(filtros.gafete, "27");
    assert!(libre.is_empty());
}

#[test]
fn parsear_consulta_conserva_lo_no_reconocido_como_texto_libre() {
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(&base, "empresa:Inexistente tipo:invalido Juan", &[]);
    assert_eq!(filtros.empresa_id, None);
    assert_eq!(filtros.tipo, None);
    assert_eq!(libre, "empresa:Inexistente tipo:invalido Juan");
}

#[test]
fn parsear_consulta_no_pisa_los_campos_no_mencionados_del_filtro_base() {
    let base = FiltrosHistorial {
        desde: "01/08/2026".into(),
        empresa_id: Some(3),
        ..Default::default()
    };
    let (filtros, _) = parsear_consulta(&base, "tipo:swat", &[]);
    assert_eq!(filtros.desde, "01/08/2026");
    assert_eq!(filtros.empresa_id, Some(3));
    assert_eq!(filtros.tipo, Some(TipoIngreso::Swat));
}

#[test]
fn busqueda_rapida_admite_clave_valor_y_se_combina_con_filtros_del_panel() {
    let mut state = HistorialState::default();
    state.completar_empresas(Ok(vec![Empresa {
        id: 3,
        nombre: "Expenic Industrial".into(),
    }]));
    state.filtro_aplicado.tipo = Some(TipoIngreso::Praind);
    state.handle_key(tecla(KeyCode::Char('/')));
    for c in "empresa:Expenic".chars() {
        state.handle_key(tecla(KeyCode::Char(c)));
    }
    let accion = state.handle_key(tecla(KeyCode::Enter));
    assert_eq!(accion, AccionHistorial::Ninguna);
    let AccionHistorial::Consultar(filtro) = state.solicitud_carga() else {
        panic!("debía consultar")
    };
    assert_eq!(filtro.empresa_id, Some(3));
    assert_eq!(filtro.tipo_ingreso, Some(TipoIngreso::Praind));
    assert_eq!(filtro.texto_persona, None);
}

#[test]
fn activo_muestra_salida_y_usuario_salida_ausentes() {
    let registro = pagina(1, 1).items.remove(0);
    assert_eq!(valor(&registro, ColumnaHistorial::Salida), "--");
    assert_eq!(valor(&registro, ColumnaHistorial::UsuarioSalida), "--");
}
