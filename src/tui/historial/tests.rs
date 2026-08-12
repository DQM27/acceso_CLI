#![allow(clippy::field_reassign_with_default)]

use super::*;

fn tecla(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn aplicar(state: &mut HistorialState, filtro: FiltrosHistorial) {
    state.filtro_aplicado = filtro;
    state.ajustar_seleccion();
}

#[test]
fn filtra_por_rango_de_fechas() {
    let mut state = HistorialState::default();
    let mut f = FiltrosHistorial::default();
    f.desde = "05/08/2026".into();
    f.hasta = "05/08/2026".into();
    aplicar(&mut state, f);
    let indices = state.indices_filtrados();
    assert_eq!(indices.len(), 12);
    assert!(
        indices
            .iter()
            .all(|i| state.registros[*i].fecha == fecha("05/08/2026").unwrap())
    );
}

#[test]
fn filtra_por_nombre_y_cedula() {
    for consulta in ["carlos", "310220488"] {
        let mut state = HistorialState::default();
        let mut f = FiltrosHistorial::default();
        f.nombre_cedula = consulta.into();
        aplicar(&mut state, f);
        assert!(!state.indices_filtrados().is_empty());
        assert!(
            state
                .indices_filtrados()
                .iter()
                .all(|i| state.registros[*i].nombre == "Carlos Rojas")
        );
    }
}

#[test]
fn filtra_por_empresa_tipo_y_estado() {
    let mut state = HistorialState::default();
    let mut f = FiltrosHistorial::default();
    f.empresa = "Brisas".into();
    f.tipo = "IN HOUSE".into();
    f.estado = EstadoFiltro::Cerrados;
    aplicar(&mut state, f);
    assert!(!state.indices_filtrados().is_empty());
    assert!(state.indices_filtrados().iter().all(|i| {
        let r = &state.registros[*i];
        r.empresa == "Brisas" && r.tipo == "IN HOUSE" && r.salida.is_some()
    }));
}

#[test]
fn filtra_por_gafete() {
    let mut state = HistorialState::default();
    let gafete = state.registros.iter().find_map(|r| r.gafete).unwrap();
    let mut f = FiltrosHistorial::default();
    f.gafete = gafete.to_string();
    aplicar(&mut state, f);
    assert!(!state.indices_filtrados().is_empty());
    assert!(state.indices_filtrados().iter().all(|i| {
        state.registros[*i]
            .gafete
            .is_some_and(|g| g.to_string().contains(&gafete.to_string()))
    }));
}

#[test]
fn combina_tipo_y_estado_activo_con_and() {
    let mut state = HistorialState::default();
    let mut f = FiltrosHistorial::default();
    f.tipo = "PRAIND".into();
    f.estado = EstadoFiltro::Activos;
    aplicar(&mut state, f);
    assert!(!state.indices_filtrados().is_empty());
    assert!(state.indices_filtrados().iter().all(|i| {
        let r = &state.registros[*i];
        r.tipo == "PRAIND" && r.salida.is_none()
    }));
}

#[test]
fn limpiar_filtros_restaura_predeterminados() {
    let mut state = HistorialState::default();
    state.filtro_aplicado.empresa = "Brisas".into();
    state.handle_key(tecla(KeyCode::Char('f')));
    state.handle_key(tecla(KeyCode::Char('l')));
    state.handle_key(tecla(KeyCode::Char('a')));
    assert_eq!(state.filtro_aplicado, FiltrosHistorial::default());
}

#[test]
fn busqueda_rapida_funciona_y_escape_limpia() {
    let mut state = HistorialState::default();
    state.handle_key(tecla(KeyCode::Char('/')));
    for c in "carlos".chars() {
        state.handle_key(tecla(KeyCode::Char(c)));
    }
    assert!(!state.indices_filtrados().is_empty());
    assert!(
        state
            .indices_filtrados()
            .iter()
            .all(|i| state.registros[*i].nombre == "Carlos Rojas")
    );
    state.handle_key(tecla(KeyCode::Esc));
    assert!(state.busqueda.is_empty());
}

#[test]
fn resultado_vacio_deja_seleccion_none() {
    let mut state = HistorialState::default();
    let mut f = FiltrosHistorial::default();
    f.nombre_cedula = "nadie".into();
    aplicar(&mut state, f);
    assert!(state.indices_filtrados().is_empty());
    assert_eq!(state.seleccion, None);
}

#[test]
fn seleccion_es_valida_despues_de_filtrar() {
    let mut state = HistorialState::default();
    state.seleccion = Some(100);
    let mut f = FiltrosHistorial::default();
    f.empresa = "Brisas".into();
    aplicar(&mut state, f);
    assert!(
        state
            .seleccion
            .is_some_and(|i| i < state.indices_filtrados().len())
    );
}

#[test]
fn activos_muestran_salida_y_usuario_ausentes() {
    let r = historial_mock::movimientos_historial()
        .into_iter()
        .find(|r| r.salida.is_none())
        .unwrap();
    assert_eq!(valor(&r, ColumnaHistorial::Salida), "--");
    assert_eq!(valor(&r, ColumnaHistorial::UsuarioSalida), "--");
}

#[test]
fn detalle_abre_y_cierra() {
    let mut state = HistorialState::default();
    state.handle_key(tecla(KeyCode::Enter));
    assert!(matches!(state.modo, ModoHistorial::Detalle { .. }));
    state.handle_key(tecla(KeyCode::Esc));
    assert_eq!(state.modo, ModoHistorial::Normal);
}

#[test]
fn columnas_alternan_y_no_permiten_ocultar_todas() {
    let mut state = HistorialState::default();
    state.handle_key(tecla(KeyCode::Char('c')));
    state.handle_key(tecla(KeyCode::Char(' ')));
    assert!(!state.columnas[0].1);
    for (_, visible) in &mut state.columnas {
        *visible = false;
    }
    state.columnas[0].1 = true;
    state.modo = ModoHistorial::Columnas { seleccion: 0 };
    state.handle_key(tecla(KeyCode::Char(' ')));
    assert!(state.columnas[0].1);
    assert!(state.mensaje.is_some());
}

#[test]
fn fecha_acepta_solo_ocho_digitos_y_aplica_formato() {
    let mut state = HistorialState::default();
    state.filtro_edicion.desde.clear();
    for caracter in "12082026abc99".chars() {
        state.agregar_caracter_filtro(CampoFiltro::Desde, caracter);
    }
    assert_eq!(state.filtro_edicion.desde, "12/08/2026");
}

#[test]
fn textos_y_gafete_tienen_limites_y_tipo_correcto() {
    let mut state = HistorialState::default();
    state.filtro_edicion.nombre_cedula.clear();
    for caracter in "x".repeat(60).chars() {
        state.agregar_caracter_filtro(CampoFiltro::NombreCedula, caracter);
    }
    for caracter in "12a345".chars() {
        state.agregar_caracter_filtro(CampoFiltro::Gafete, caracter);
    }
    assert_eq!(state.filtro_edicion.nombre_cedula.chars().count(), 40);
    assert_eq!(state.filtro_edicion.gafete, "123");
}

#[test]
fn desplegable_selecciona_empresa_y_escape_cancela() {
    let mut state = HistorialState::default();
    state.abrir_desplegable(CampoFiltro::Empresa, 3);
    state.handle_key(tecla(KeyCode::Down));
    state.handle_key(tecla(KeyCode::Enter));
    assert_eq!(state.filtro_edicion.empresa, "Brisas");
    assert_eq!(
        state.modo,
        ModoHistorial::Filtros {
            seleccion: 3,
            editando: false
        }
    );

    state.abrir_desplegable(CampoFiltro::Tipo, 4);
    state.handle_key(tecla(KeyCode::Down));
    state.handle_key(tecla(KeyCode::Esc));
    assert_eq!(state.filtro_edicion.tipo, "Todos");
}

#[test]
fn no_aplica_fecha_invalida_o_rango_invertido() {
    let mut state = HistorialState::default();
    state.filtro_edicion.desde = "31/02/2026".into();
    state.modo = ModoHistorial::Filtros {
        seleccion: 0,
        editando: false,
    };
    state.handle_key(tecla(KeyCode::Char('a')));
    assert!(matches!(state.modo, ModoHistorial::Filtros { .. }));
    assert!(state.mensaje.is_some());

    state.filtro_edicion.desde = "12/08/2026".into();
    state.filtro_edicion.hasta = "01/08/2026".into();
    state.handle_key(tecla(KeyCode::Char('a')));
    assert!(matches!(state.modo, ModoHistorial::Filtros { .. }));
}
