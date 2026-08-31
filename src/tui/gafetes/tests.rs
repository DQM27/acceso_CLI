use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};

fn k(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}
fn escribir(s: &mut GafetesState, texto: &str) {
    for c in texto.chars() {
        s.handle_key(k(KeyCode::Char(c)));
    }
}
fn gafete(id: i64, numero: i64, estado: EstadoGafete) -> GafeteResumen {
    GafeteResumen {
        id,
        numero,
        estado,
        contratista_deudor_id: None,
        contratista_deudor_nombre: None,
        fecha_marcado_perdido: None,
    }
}
fn datos() -> Vec<GafeteResumen> {
    vec![
        gafete(1, 5, EstadoGafete::Disponible),
        gafete(2, 9, EstadoGafete::Perdido),
    ]
}

#[test]
fn inicia_vacio_y_aplica_resultados_reales() {
    let mut s = GafetesState::default();
    assert!(s.gafetes.is_empty() && s.seleccion.is_none());
    s.completar_busqueda(Ok(datos()), None);
    assert_eq!(s.gafetes.len(), 2);
    assert_eq!(s.seleccion, Some(0));
}

#[test]
fn n_abre_alta_individual_por_defecto_y_tab_alterna_a_rango() {
    let mut s = GafetesState::default();
    s.handle_key(k(KeyCode::Char('N')));
    let ModoGafetes::Alta(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.modo, ModoFormularioAlta::Individual);

    s.handle_key(k(KeyCode::Tab));
    let ModoGafetes::Alta(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.modo, ModoFormularioAlta::Rango);
}

#[test]
fn alta_individual_valida_numero_antes_de_despachar() {
    let mut s = GafetesState::default();
    s.handle_key(k(KeyCode::Char('N')));
    // Vacío: no despacha, deja error en el formulario.
    assert_eq!(s.handle_key(k(KeyCode::Enter)), AccionGafetes::Ninguna);
    assert!(matches!(&s.modo, ModoGafetes::Alta(f) if f.error.is_some()));

    escribir(&mut s, "12");
    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionGafetes::CrearUno { numero: 12 }
    );
}

#[test]
fn alta_rango_exige_desde_menor_o_igual_a_hasta() {
    let mut s = GafetesState::default();
    s.handle_key(k(KeyCode::Char('N')));
    s.handle_key(k(KeyCode::Tab)); // Rango
    escribir(&mut s, "9");
    s.handle_key(k(KeyCode::Down)); // siguiente campo: Hasta
    escribir(&mut s, "3");
    assert_eq!(s.handle_key(k(KeyCode::Enter)), AccionGafetes::Ninguna);
    assert!(matches!(&s.modo, ModoGafetes::Alta(f) if f.error.is_some()));
}

#[test]
fn alta_rango_valido_despacha_crear_rango() {
    let mut s = GafetesState::default();
    s.handle_key(k(KeyCode::Char('N')));
    s.handle_key(k(KeyCode::Tab));
    escribir(&mut s, "1");
    s.handle_key(k(KeyCode::Down));
    escribir(&mut s, "5");
    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionGafetes::CrearRango { desde: 1, hasta: 5 }
    );
}

#[test]
fn completar_alta_deja_prefijo_de_exito_solo_si_no_hubo_error() {
    let mut s = GafetesState::default();
    s.handle_key(k(KeyCode::Char('N')));
    escribir(&mut s, "12");
    let recarga = s.completar_alta(Ok(1), 12);
    assert!(matches!(
        recarga,
        AccionGafetes::Buscar {
            seleccionar_id: Some(1),
            ..
        }
    ));
    assert!(s.mensaje.as_deref().unwrap().starts_with('✓'));
    assert!(matches!(s.modo, ModoGafetes::Normal));

    s.handle_key(k(KeyCode::Char('N')));
    escribir(&mut s, "12");
    let recarga = s.completar_alta(Err("Ya existe un gafete con ese número".into()), 12);
    assert_eq!(recarga, AccionGafetes::Ninguna);
    assert!(matches!(&s.modo, ModoGafetes::Alta(f) if f.error.is_some()));
}

#[test]
fn b_solo_ofrece_baja_si_esta_disponible() {
    let mut s = GafetesState::default();
    s.completar_busqueda(Ok(datos()), None); // seleccion en Disponible (id 1)
    s.handle_key(k(KeyCode::Char('B')));
    assert!(matches!(
        s.modo,
        ModoGafetes::ConfirmacionBaja {
            gafete_id: 1,
            numero: 5
        }
    ));

    let mut s2 = GafetesState::default();
    s2.completar_busqueda(Ok(datos()), None);
    s2.handle_key(k(KeyCode::Down)); // selecciona el Perdido (id 2)
    s2.handle_key(k(KeyCode::Char('B')));
    assert!(matches!(s2.modo, ModoGafetes::Normal));
}

#[test]
fn p_solo_ofrece_marcar_perdido_si_esta_disponible() {
    let mut s = GafetesState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Char('P')));
    assert!(matches!(s.modo, ModoGafetes::MarcarPerdidoBuscarDeudor(_)));
}

#[test]
fn r_solo_ofrece_resolver_si_esta_perdido() {
    let mut s = GafetesState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Char('R'))); // seleccionado: Disponible, no aplica
    assert!(matches!(s.modo, ModoGafetes::Normal));

    s.handle_key(k(KeyCode::Down)); // selecciona el Perdido
    s.handle_key(k(KeyCode::Char('R')));
    assert!(matches!(
        s.modo,
        ModoGafetes::ConfirmacionResolver {
            gafete_id: 2,
            numero: 9,
            motivo: MotivoResolucionGafete::Pagado,
        }
    ));
    // 2 alterna el motivo a Apareció sin perder el gafete de destino.
    s.handle_key(k(KeyCode::Char('2')));
    assert!(matches!(
        s.modo,
        ModoGafetes::ConfirmacionResolver {
            motivo: MotivoResolucionGafete::Aparecido,
            ..
        }
    ));
}

#[test]
fn buscar_deudor_navega_resultados_y_enter_marca_perdido() {
    let mut s = GafetesState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Char('P')));
    escribir(&mut s, "jose");
    assert_eq!(
        s.tick_deudor(Instant::now() + DURACION_DEBOUNCE + Duration::from_millis(1)),
        AccionGafetes::BuscarDeudor {
            texto: Some("jose".into())
        }
    );
    s.completar_busqueda_deudor(Ok(vec![
        crate::database::queries::contratistas::ContratistaResumen {
            id: 42,
            empresa_id: 1,
            cedula: "1".into(),
            nombre: "José".into(),
            empresa_nombre: "Acme".into(),
            tipo_ingreso: crate::models::tipo_ingreso::TipoIngreso::Praind,
            fecha_vencimiento_praind: None,
            es_personal_ruta: false,
            tiene_acceso: true,
            tiene_ingreso_activo: false,
        },
    ]));
    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionGafetes::MarcarPerdido {
            id: 1,
            numero: 5,
            contratista_id: 42,
        }
    );
}

#[test]
fn h_abre_historial_vacio_y_completar_lo_llena() {
    use crate::models::gafete::TipoIncidenteGafete;

    let mut s = GafetesState::default();
    s.completar_busqueda(Ok(datos()), None); // selecciona id 1, numero 5

    assert_eq!(
        s.handle_key(k(KeyCode::Char('H'))),
        AccionGafetes::VerHistorial { id: 1, numero: 5 }
    );
    assert!(matches!(
        &s.modo,
        ModoGafetes::Historial { numero: 5, incidentes } if incidentes.is_empty()
    ));

    let incidente = crate::database::queries::gafetes_incidentes::IncidenteGafete {
        id: 1,
        tipo: TipoIncidenteGafete::Perdido,
        fecha_hora: chrono::Utc::now(),
        usuario_nombre: "Root".into(),
        contratista_nombre: Some("Juan".into()),
        motivo_resolucion: None,
        gafete_numero: 5,
    };
    s.completar_historial(Ok(vec![incidente.clone()]), 5);
    assert!(matches!(
        &s.modo,
        ModoGafetes::Historial { numero: 5, incidentes } if incidentes == &vec![incidente]
    ));

    assert_eq!(s.handle_key(k(KeyCode::Esc)), AccionGafetes::Ninguna);
    assert!(matches!(s.modo, ModoGafetes::Normal));
}

#[test]
fn completar_historial_con_error_vuelve_a_normal_con_mensaje() {
    let mut s = GafetesState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Char('H')));

    s.completar_historial(Err("No se pudo cargar el historial del gafete".into()), 5);

    assert!(matches!(s.modo, ModoGafetes::Normal));
    assert_eq!(
        s.mensaje.as_deref(),
        Some("No se pudo cargar el historial del gafete")
    );
}

#[test]
fn interpretar_filtro_reconoce_numero_y_estado_negado() {
    assert_eq!(
        interpretar_filtro("9"),
        FiltroGafetes {
            numero: Some(9),
            estado: None
        }
    );
    assert_eq!(
        interpretar_filtro("-estado:de_baja"),
        FiltroGafetes {
            numero: None,
            estado: Some(Igualdad::Excluye(EstadoGafete::DeBaja)),
        }
    );
    assert_eq!(interpretar_filtro(""), FiltroGafetes::default());
}
