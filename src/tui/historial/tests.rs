use chrono::NaiveDate;
use crossterm::event::KeyModifiers;
use std::time::Instant;

use super::*;
use crate::database::queries::ingresos::EstadoMovimiento;
use crate::models::{empresa::Empresa, medio_ingreso::MedioIngreso, tipo_ingreso::TipoIngreso};

fn tecla(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn sesion_prueba() -> crate::services::autenticacion_service::UsuarioSesion {
    crate::services::autenticacion_service::UsuarioSesion {
        id: 1,
        cedula: "1-1111-1111".into(),
        nombre: "Ana Quintana".into(),
        rol: crate::models::usuario::RolUsuario::Root,
    }
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
                empresa_activa_snapshot: true,
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
        empresa_id: Some(crate::database::queries::Igualdad::Incluye(8)),
        tipos: Some(vec![TipoIngreso::PorCorreo]),
        gafete: "27".into(),
        estado: EstadoMovimiento::Cerrados,
        ..Default::default()
    };
    let query = construir(&filtro, "", 50, 100, Some(77)).unwrap();
    assert_eq!(query.texto_persona.as_deref(), Some("Ana"));
    assert_eq!(
        query.empresa_id,
        Some(crate::database::queries::Igualdad::Incluye(8))
    );
    assert_eq!(query.tipos_incluidos, Some(vec![TipoIngreso::PorCorreo]));
    assert_eq!(
        query.gafete_numero,
        Some(crate::database::queries::Igualdad::Incluye(27))
    );
    assert_eq!(query.estado, EstadoMovimiento::Cerrados);
    assert_eq!((query.limite, query.offset), (50, 100));
    assert_eq!(query.corte_id, Some(77));
}

#[test]
fn busqueda_rapida_se_combina_con_filtros_aplicados_tras_el_debounce() {
    let mut state = HistorialState::default();
    state.filtro_aplicado.empresa_id = Some(crate::database::queries::Igualdad::Incluye(3));
    let accion = state.handle_key(tecla(KeyCode::Char('a')));
    assert_eq!(accion, AccionHistorial::Ninguna);
    let accion =
        state.tick(Instant::now() + DURACION_DEBOUNCE + std::time::Duration::from_millis(1));
    let AccionHistorial::Consultar(filtro) = accion else {
        panic!("debía consultar")
    };
    assert_eq!(filtro.texto_persona.as_deref(), Some("a"));
    assert_eq!(
        filtro.empresa_id,
        Some(crate::database::queries::Igualdad::Incluye(3))
    );
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
    assert_eq!(
        state.handle_key(tecla(KeyCode::Char('a'))),
        AccionHistorial::Ninguna
    );
    let accion =
        state.tick(Instant::now() + DURACION_DEBOUNCE + std::time::Duration::from_millis(1));
    let AccionHistorial::Consultar(filtro) = accion else {
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
}

#[test]
fn panel_refleja_la_seleccion_resaltada_sin_pasos_extra() {
    let mut state = HistorialState::default();
    state.completar(Ok(pagina(2, 2)));
    assert_eq!(state.seleccionado().map(|r| r.registro_id), Some(1));
    state.handle_key(tecla(KeyCode::Down));
    assert_eq!(state.seleccionado().map(|r| r.registro_id), Some(2));
}

#[test]
fn f3_alterna_entre_linea_de_tiempo_y_vista_clasica() {
    let mut state = HistorialState::default();
    assert_eq!(state.vista, ViewMode::Timeline);
    state.handle_key(tecla(KeyCode::F(3)));
    assert_eq!(state.vista, ViewMode::Classic);
    state.handle_key(tecla(KeyCode::F(3)));
    assert_eq!(state.vista, ViewMode::Timeline);
}

#[test]
fn f4_no_hace_nada_fuera_de_la_vista_clasica_y_letras_sueltas_van_a_la_busqueda() {
    let mut state = HistorialState::default();
    state.handle_key(tecla(KeyCode::F(4)));
    assert_eq!(state.modo, ModoHistorial::Normal);

    state.handle_key(tecla(KeyCode::Char('f')));
    assert_eq!(state.modo, ModoHistorial::Normal);
    assert_eq!(state.busqueda.value(), "f");
}

#[test]
fn f4_abre_el_editor_de_columnas_solo_en_la_vista_clasica() {
    let mut state = HistorialState::default();
    state.handle_key(tecla(KeyCode::F(3)));
    assert_eq!(state.vista, ViewMode::Classic);
    state.handle_key(tecla(KeyCode::F(4)));
    assert!(matches!(
        state.modo,
        ModoHistorial::Columnas {
            seleccion: 0,
            proposito: PropositoColumnas::Vista,
        }
    ));

    state.handle_key(tecla(KeyCode::Char(' ')));
    assert!(!state.columnas_clasica[0].1);

    state.handle_key(tecla(KeyCode::Esc));
    assert_eq!(state.modo, ModoHistorial::Normal);
}

#[test]
fn f5_pide_columnas_y_ruta_antes_de_exportar_todo_el_filtro() {
    let mut state = HistorialState::default();
    state.completar(Ok(pagina(2, 73)));

    assert_eq!(
        state.handle_key(tecla(KeyCode::F(5))),
        AccionHistorial::Ninguna
    );
    assert!(matches!(
        state.modo,
        ModoHistorial::Columnas {
            seleccion: 0,
            proposito: PropositoColumnas::Exportacion,
        }
    ));

    state.handle_key(tecla(KeyCode::Char(' ')));
    assert!(!state.columnas_clasica[0].1, "FECHA debía quedar omitida");
    state.handle_key(tecla(KeyCode::Enter));
    assert!(matches!(state.modo, ModoHistorial::RutaExportacion { .. }));

    let AccionHistorial::Exportar {
        filtro,
        columnas,
        destino,
    } = state.handle_key(tecla(KeyCode::Enter))
    else {
        panic!("debía confirmar la exportación")
    };
    assert_eq!(filtro.offset, 0);
    assert_eq!(filtro.corte_id, Some(100));
    assert_eq!(columnas.len(), ColumnaHistorial::ALL.len() - 1);
    assert!(!columnas.contains(&ColumnaHistorial::Fecha));
    assert_eq!(destino.extension().and_then(|e| e.to_str()), Some("xlsx"));
}

#[test]
fn f5_sin_resultados_no_abre_el_flujo_de_exportacion() {
    let mut state = HistorialState::default();
    state.handle_key(tecla(KeyCode::F(5)));
    assert_eq!(state.modo, ModoHistorial::Normal);
    assert_eq!(
        state.mensaje.as_deref(),
        Some("No hay movimientos para exportar")
    );
}

#[test]
fn parsear_consulta_resuelve_empresa_por_nombre_parcial_y_deja_texto_libre() {
    let empresas = vec![
        Empresa {
            id: 5,
            nombre: "Brisas del Oeste".into(),
            activo: true,
        },
        Empresa {
            id: 9,
            nombre: "Aldama Servicios".into(),
            activo: true,
        },
    ];
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(&base, "empresa:\"Brisas del Oeste\" Ana", &empresas);
    assert_eq!(
        filtros.empresa_id,
        Some(crate::database::queries::Igualdad::Incluye(5))
    );
    assert_eq!(libre, "Ana");
}

#[test]
fn parsear_consulta_empresa_ignora_tildes_en_ambos_lados() {
    let empresas = vec![Empresa {
        id: 9,
        nombre: "Álvarez Ingeniería".into(),
        activo: true,
    }];
    let base = FiltrosHistorial::default();
    let (filtros, _) = parsear_consulta(&base, "empresa:alvarez", &empresas);
    assert_eq!(
        filtros.empresa_id,
        Some(crate::database::queries::Igualdad::Incluye(9))
    );
}

#[test]
fn parsear_consulta_niega_empresa_gafete_ingreso_y_salida() {
    let empresas = vec![Empresa {
        id: 9,
        nombre: "Álvarez Ingeniería".into(),
        activo: true,
    }];
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(
        &base,
        "-empresa:alvarez -gafete:26 -ingreso:ana -salida:ana",
        &empresas,
    );
    assert_eq!(
        filtros.empresa_id,
        Some(crate::database::queries::Igualdad::Excluye(9))
    );
    assert_eq!(filtros.gafete, "26");
    assert!(filtros.gafete_negado);
    assert_eq!(filtros.usuario_ingreso, "ana");
    assert!(filtros.usuario_ingreso_negado);
    assert_eq!(filtros.usuario_salida, "ana");
    assert!(filtros.usuario_salida_negado);
    assert!(libre.is_empty());
}

#[test]
fn parsear_consulta_reconoce_tipo_estado_y_gafete() {
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(&base, "tipo:praind estado:activos gafete:27", &[]);
    assert_eq!(filtros.tipos, Some(vec![TipoIngreso::Praind]));
    assert_eq!(filtros.estado, EstadoMovimiento::Activos);
    assert_eq!(filtros.gafete, "27");
    assert!(libre.is_empty());
}

/// Regresión de "`gafete:abc` (no numérico) se comporta distinto en Activos
/// e Historial" (`docs/hallazgos-buscador.md`): un valor no numérico debe
/// caer a texto libre en silencio, igual que Activos — no debe escribirse
/// en `f.gafete` (eso haría que `construir()` rechace toda la búsqueda).
#[test]
fn parsear_consulta_gafete_no_numerico_cae_a_texto_libre() {
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(&base, "gafete:abc", &[]);
    assert_eq!(filtros.gafete, "");
    assert_eq!(libre, "gafete:abc");
}

#[test]
fn parsear_consulta_reconoce_quien_dio_ingreso_y_quien_dio_salida() {
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(&base, "ingreso:quintana salida:\"Ana Solano\"", &[]);
    assert_eq!(filtros.usuario_ingreso, "quintana");
    assert_eq!(filtros.usuario_salida, "Ana Solano");
    assert!(libre.is_empty());
}

#[test]
fn ingreso_y_salida_no_admiten_lista_pero_si_negacion() {
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(&base, "ingreso:a,b -salida:c", &[]);
    assert_eq!(filtros.usuario_ingreso, "");
    assert_eq!(filtros.usuario_salida, "c");
    assert!(filtros.usuario_salida_negado);
    assert_eq!(libre, "ingreso:a,b");
}

#[test]
fn parsear_consulta_admite_listas_de_tipo() {
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(&base, "tipo:praind,swat", &[]);
    assert_eq!(
        filtros.tipos,
        Some(vec![TipoIngreso::Praind, TipoIngreso::Swat])
    );
    assert!(libre.is_empty());
}

#[test]
fn parsear_consulta_niega_tipo_incluyendo_los_demas() {
    let base = FiltrosHistorial::default();
    let (filtros, _) = parsear_consulta(&base, "-tipo:swat", &[]);
    let tipos = filtros.tipos.expect("debía filtrar por tipos");
    assert_eq!(tipos.len(), 3);
    assert!(!tipos.contains(&TipoIngreso::Swat));
    assert!(tipos.contains(&TipoIngreso::Praind));
}

#[test]
fn parsear_consulta_niega_estado_invirtiendo_activos_y_cerrados() {
    let base = FiltrosHistorial::default();
    let (activos, _) = parsear_consulta(&base, "-estado:cerrados", &[]);
    assert_eq!(activos.estado, EstadoMovimiento::Activos);
    let (cerrados, _) = parsear_consulta(&base, "-estado:activos", &[]);
    assert_eq!(cerrados.estado, EstadoMovimiento::Cerrados);
}

#[test]
fn parsear_consulta_deja_como_texto_libre_las_listas_pero_admite_negacion_de_gafete() {
    let base = FiltrosHistorial::default();
    // gafete no admite listas (cae a texto libre), pero sí negación de un
    // único valor.
    let (filtros, libre) = parsear_consulta(&base, "gafete:1,2 -gafete:3", &[]);
    assert_eq!(filtros.gafete, "3");
    assert!(filtros.gafete_negado);
    assert_eq!(libre, "gafete:1,2");
}

#[test]
fn parsear_consulta_conserva_lo_no_reconocido_como_texto_libre() {
    let base = FiltrosHistorial::default();
    let (filtros, libre) = parsear_consulta(&base, "empresa:Inexistente tipo:invalido Juan", &[]);
    assert_eq!(filtros.empresa_id, None);
    assert_eq!(filtros.tipos, None);
    assert_eq!(libre, "empresa:Inexistente tipo:invalido Juan");
}

#[test]
fn parsear_consulta_no_pisa_los_campos_no_mencionados_del_filtro_base() {
    let base = FiltrosHistorial {
        desde: "01/08/2026".into(),
        empresa_id: Some(crate::database::queries::Igualdad::Incluye(3)),
        ..Default::default()
    };
    let (filtros, _) = parsear_consulta(&base, "tipo:swat", &[]);
    assert_eq!(filtros.desde, "01/08/2026");
    assert_eq!(
        filtros.empresa_id,
        Some(crate::database::queries::Igualdad::Incluye(3))
    );
    assert_eq!(filtros.tipos, Some(vec![TipoIngreso::Swat]));
}

#[test]
fn busqueda_rapida_admite_clave_valor_y_se_combina_con_filtros_del_panel() {
    let mut state = HistorialState::default();
    state.completar_empresas(Ok(vec![Empresa {
        id: 3,
        nombre: "Expenic Industrial".into(),
        activo: true,
    }]));
    state.filtro_aplicado.tipos = Some(vec![TipoIngreso::Praind]);
    for c in "empresa:Expenic".chars() {
        state.handle_key(tecla(KeyCode::Char(c)));
    }
    let accion = state.handle_key(tecla(KeyCode::Enter));
    assert_eq!(accion, AccionHistorial::Ninguna);
    let AccionHistorial::Consultar(filtro) = state.solicitud_carga() else {
        panic!("debía consultar")
    };
    assert_eq!(
        filtro.empresa_id,
        Some(crate::database::queries::Igualdad::Incluye(3))
    );
    assert_eq!(filtro.tipos_incluidos, Some(vec![TipoIngreso::Praind]));
    assert_eq!(filtro.texto_persona, None);
}

#[test]
fn la_linea_de_tiempo_agrupa_por_dia_y_muestra_el_glifo_de_actividad() {
    use ratatui::{Terminal, backend::TestBackend};

    let mut state = HistorialState::default();
    state.completar(Ok(pagina(2, 2)));

    let backend = TestBackend::new(140, 30);
    let mut terminal = Terminal::new(backend).expect("backend de prueba");
    terminal
        .draw(|frame| {
            render::render(
                frame,
                frame.area(),
                &state,
                &sesion_prueba(),
                crate::tui::ui_kit::ThemePreset::Brisas.theme(),
            )
        })
        .expect("debe renderizar");
    let texto: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|celda| celda.symbol())
        .collect();

    assert!(texto.contains("movimientos"));
    assert!(texto.contains("Ana Solano"));
    assert!(texto.contains("Brisas"));
    assert!(texto.contains("●"));
    // El panel de detalle sigue la convención "Etiqueta: valor" del resto de
    // la app, sin "Evaluación"/"Reglas", con duración y trazabilidad de
    // quién registró entrada y salida.
    assert!(texto.contains("Empresa: Brisas"));
    assert!(texto.contains("Entrada: 08:30"));
    assert!(!texto.contains("Evaluación"));
    assert!(!texto.contains("Reglas"));
    assert!(texto.contains("Ingreso registrado por: Quintana"));
}

#[test]
fn la_vista_clasica_muestra_tabla_completa_y_el_editor_de_columnas_oculta_una() {
    use ratatui::{Terminal, backend::TestBackend};

    let mut state = HistorialState::default();
    state.completar(Ok(pagina(2, 2)));
    state.handle_key(tecla(KeyCode::F(3)));
    assert_eq!(state.vista, ViewMode::Classic);

    let backend = TestBackend::new(140, 30);
    let mut terminal = Terminal::new(backend).expect("backend de prueba");
    terminal
        .draw(|frame| {
            render::render(
                frame,
                frame.area(),
                &state,
                &sesion_prueba(),
                crate::tui::ui_kit::ThemePreset::Brisas.theme(),
            )
        })
        .expect("debe renderizar");
    let texto: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|celda| celda.symbol())
        .collect();
    assert!(texto.contains("Ana Solano"));
    assert!(texto.contains("CÉDULA"));

    state.handle_key(tecla(KeyCode::F(4)));
    state.handle_key(tecla(KeyCode::Char(' ')));
    let backend = TestBackend::new(140, 30);
    let mut terminal = Terminal::new(backend).expect("backend de prueba");
    terminal
        .draw(|frame| {
            render::render(
                frame,
                frame.area(),
                &state,
                &sesion_prueba(),
                crate::tui::ui_kit::ThemePreset::Brisas.theme(),
            )
        })
        .expect("debe renderizar");
    let texto: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|celda| celda.symbol())
        .collect();
    assert!(texto.contains("COLUMNAS VISIBLES"));
}

#[test]
fn activo_no_tiene_fecha_de_salida_ni_usuario_de_salida() {
    let registro = pagina(1, 1).items.remove(0);
    assert_eq!(registro.fecha_hora_salida, None);
    assert_eq!(registro.usuario_salida_nombre, None);
}
