use super::*;

fn tecla(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn escribir(state: &mut ActivosState, texto: &str) {
    for caracter in texto.chars() {
        state.handle_key(tecla(KeyCode::Char(caracter)));
    }
}

fn buscar(texto: &str) -> ActivosState {
    let mut state = ActivosState::default();
    state.handle_key(tecla(KeyCode::Char('/')));
    escribir(&mut state, texto);
    state
}

#[test]
fn seleccion_se_mueve_y_respeta_limites() {
    let mut state = ActivosState::default();
    state.handle_key(tecla(KeyCode::Down));
    assert_eq!(state.seleccion, Some(1));
    state.handle_key(tecla(KeyCode::Up));
    state.handle_key(tecla(KeyCode::Up));
    assert_eq!(state.seleccion, Some(0));
    for _ in 0..100 {
        state.handle_key(tecla(KeyCode::Down));
    }
    assert_eq!(state.seleccion, Some(79));
}

#[test]
fn scroll_logico_sigue_a_la_seleccion() {
    let mut state = ActivosState::default();
    for _ in 0..8 {
        state.handle_key(tecla(KeyCode::Down));
    }
    assert_eq!(state.inicio_visible(5), 4);
}

#[test]
fn busca_por_nombre_cedula_empresa_y_gafete() {
    for (consulta, nombre) in [
        ("carlos", "Carlos Rojas"),
        ("310220488", "Carlos Rojas"),
        ("electromecánicos", "Marco Antonio Hernández"),
        ("47", "Laura Villalobos"),
    ] {
        let state = buscar(consulta);
        let indices = state.indices_filtrados();
        assert!(
            indices
                .iter()
                .any(|indice| state.registros[*indice].nombre == nombre)
        );
    }
}

#[test]
fn busqueda_sin_resultados_y_escape_limpia_filtro() {
    let mut state = buscar("nadie-existe");
    assert!(state.indices_filtrados().is_empty());
    assert_eq!(state.seleccion, None);
    state.handle_key(tecla(KeyCode::Esc));
    assert_eq!(state.indices_filtrados().len(), 80);
    assert_eq!(state.seleccion, Some(0));
}

#[test]
fn enter_conserva_filtro_y_devuelve_foco_a_tabla() {
    let mut state = buscar("carlos");
    state.handle_key(tecla(KeyCode::Enter));
    assert_eq!(state.modo, ModoActivos::Normal);
    assert_eq!(state.indices_filtrados().len(), 4);
}

#[test]
fn escape_limpia_primero_el_filtro_y_despues_vuelve() {
    let mut state = buscar("carlos");
    state.handle_key(tecla(KeyCode::Enter));

    assert_eq!(
        state.handle_key(tecla(KeyCode::Esc)),
        AccionActivos::Ninguna
    );
    assert!(state.filtro.is_empty());
    assert_eq!(state.indices_filtrados().len(), 80);

    assert_eq!(state.handle_key(tecla(KeyCode::Esc)), AccionActivos::Volver);
}

#[test]
fn confirmacion_se_abre_y_n_o_escape_cancelan() {
    for cancelar in [KeyCode::Char('n'), KeyCode::Esc] {
        let mut state = ActivosState::default();
        state.handle_key(tecla(KeyCode::Char('s')));
        assert!(matches!(state.modo, ModoActivos::ConfirmarSalida { .. }));
        state.handle_key(tecla(cancelar));
        assert_eq!(state.modo, ModoActivos::Normal);
        assert_eq!(state.cantidad(), 80);
    }
}

#[test]
fn confirmar_salida_elimina_disminuye_contador_y_conserva_posicion() {
    let mut state = ActivosState::default();
    for _ in 0..7 {
        state.handle_key(tecla(KeyCode::Down));
    }
    let id = state.id_seleccionado().unwrap();
    state.handle_key(tecla(KeyCode::Char('s')));
    state.handle_key(tecla(KeyCode::Char('y')));
    assert_eq!(state.cantidad(), 79);
    assert!(!state.registros.iter().any(|registro| registro.id == id));
    assert_eq!(state.seleccion, Some(7));
}

#[test]
fn salida_sin_gafete_no_inventa_liberacion() {
    let mut state = ActivosState::default();
    state.handle_key(tecla(KeyCode::Down));
    state.handle_key(tecla(KeyCode::Char('s')));
    state.handle_key(tecla(KeyCode::Char('y')));
    assert!(!state.mensaje.as_deref().unwrap().contains("liberado"));
}

#[test]
fn f2_encuentra_gafete_y_salida_elimina_registro_correcto() {
    let mut state = ActivosState::default();
    state.handle_key(tecla(KeyCode::F(2)));
    escribir(&mut state, "8");
    state.handle_key(tecla(KeyCode::Enter));
    assert_eq!(
        state.modo,
        ModoActivos::SalidaPorGafete(SalidaGafete::Encontrado { id: 3 })
    );
    state.handle_key(tecla(KeyCode::Char('y')));
    assert_eq!(state.cantidad(), 79);
    assert!(
        !state
            .registros
            .iter()
            .any(|registro| registro.gafete == Some(8))
    );
    state.handle_key(tecla(KeyCode::F(2)));
    escribir(&mut state, "8");
    state.handle_key(tecla(KeyCode::Enter));
    assert!(matches!(
        &state.modo,
        ModoActivos::SalidaPorGafete(SalidaGafete::Capturando { error: Some(_), .. })
    ));
}

#[test]
fn f2_reporta_gafete_inexistente() {
    let mut state = ActivosState::default();
    state.handle_key(tecla(KeyCode::F(2)));
    escribir(&mut state, "999");
    state.handle_key(tecla(KeyCode::Enter));
    assert!(matches!(
        &state.modo,
        ModoActivos::SalidaPorGafete(SalidaGafete::Capturando { error: Some(_), .. })
    ));
}

#[test]
fn detalle_se_abre_cierra_y_puede_iniciar_salida() {
    let mut state = ActivosState::default();
    state.handle_key(tecla(KeyCode::Enter));
    assert!(matches!(state.modo, ModoActivos::Detalle { id: 1 }));
    state.handle_key(tecla(KeyCode::Esc));
    assert_eq!(state.modo, ModoActivos::Normal);
    state.handle_key(tecla(KeyCode::Enter));
    state.handle_key(tecla(KeyCode::Char('s')));
    assert!(matches!(state.modo, ModoActivos::ConfirmarSalida { id: 1 }));
}

#[test]
fn columnas_se_abren_cambian_y_no_permiten_ocultar_todas() {
    let mut state = ActivosState::default();
    state.handle_key(tecla(KeyCode::Char('c')));
    assert!(matches!(state.modo, ModoActivos::Columnas { .. }));
    state.handle_key(tecla(KeyCode::Char(' ')));
    assert!(!state.columnas[0].1);
    for indice in 1..state.columnas.len() {
        state.columnas[indice].1 = false;
    }
    state.columnas[0].1 = true;
    state.modo = ModoActivos::Columnas { seleccion: 0 };
    state.handle_key(tecla(KeyCode::Char(' ')));
    assert!(state.columnas[0].1);
    assert!(state.mensaje.is_some());
    state.handle_key(tecla(KeyCode::Esc));
    assert_eq!(state.modo, ModoActivos::Normal);
}

#[test]
fn advertencia_no_altera_seleccion() {
    let mut state = ActivosState::default();
    for _ in 0..3 {
        state.handle_key(tecla(KeyCode::Down));
    }
    assert!(state.registros[3].advertencia.is_some());
    assert_eq!(state.id_seleccionado(), Some(4));
}
