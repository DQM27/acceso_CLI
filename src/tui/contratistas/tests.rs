use super::*;

fn tecla(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn escribir(state: &mut ContratistasState, texto: &str) {
    for c in texto.chars() {
        state.handle_key(tecla(KeyCode::Char(c)));
    }
}
fn abrir_crear_valido(state: &mut ContratistasState) {
    state.handle_key(tecla(KeyCode::Char('N')));
    escribir(state, "999001");
    state.handle_key(tecla(KeyCode::Down));
    escribir(state, "Persona Nueva");
    state.handle_key(tecla(KeyCode::Down));
    state.handle_key(tecla(KeyCode::Down));
    state.handle_key(tecla(KeyCode::Down));
    escribir(state, "20/09/2026");
}

#[test]
fn seleccion_arriba_abajo_respeta_limites() {
    let mut s = ContratistasState::default();
    s.handle_key(tecla(KeyCode::Down));
    assert_eq!(s.seleccion, Some(1));
    s.handle_key(tecla(KeyCode::Up));
    s.handle_key(tecla(KeyCode::Up));
    assert_eq!(s.seleccion, Some(0));
}
#[test]
fn busca_por_cedula() {
    let mut s = ContratistasState::default();
    let q = s.registros[2].cedula.clone();
    s.handle_key(tecla(KeyCode::Char('/')));
    escribir(&mut s, &q);
    assert_eq!(s.indices_filtrados().len(), 1);
}
#[test]
fn busca_por_nombre_sin_importar_mayusculas() {
    let mut s = ContratistasState::default();
    s.handle_key(tecla(KeyCode::Char('/')));
    escribir(&mut s, "MARÍA MORA");
    assert!(!s.indices_filtrados().is_empty());
}
#[test]
fn busca_por_empresa() {
    let mut s = ContratistasState::default();
    s.handle_key(tecla(KeyCode::Char('/')));
    escribir(&mut s, "constructora alfa");
    assert!(!s.indices_filtrados().is_empty());
}
#[test]
fn abre_y_cierra_detalle() {
    let mut s = ContratistasState::default();
    s.handle_key(tecla(KeyCode::Enter));
    assert!(matches!(s.modo, ModoContratistas::Detalle { .. }));
    s.handle_key(tecla(KeyCode::Esc));
    assert_eq!(s.modo, ModoContratistas::Normal);
}
#[test]
fn abre_y_cancela_creacion() {
    let mut s = ContratistasState::default();
    let n = s.registros.len();
    s.handle_key(tecla(KeyCode::Char('N')));
    assert!(matches!(s.modo, ModoContratistas::Formulario(_)));
    s.handle_key(tecla(KeyCode::Esc));
    assert_eq!(s.registros.len(), n);
}
#[test]
fn crea_contratista_valido_y_lo_selecciona() {
    let mut s = ContratistasState::default();
    abrir_crear_valido(&mut s);
    s.handle_key(tecla(KeyCode::Char('G')));
    assert_eq!(s.registros.len(), 41);
    assert_eq!(s.registros.last().unwrap().cedula, "999001");
    assert_eq!(s.seleccion, Some(40));
}
#[test]
fn rechaza_cedula_vacia() {
    let mut s = ContratistasState::default();
    s.handle_key(tecla(KeyCode::Char('N')));
    s.handle_key(tecla(KeyCode::Char('G')));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.error.as_deref(), Some("La cédula es obligatoria"));
}
#[test]
fn rechaza_nombre_vacio() {
    let mut s = ContratistasState::default();
    s.handle_key(tecla(KeyCode::Char('N')));
    escribir(&mut s, "123");
    s.handle_key(tecla(KeyCode::Char('G')));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.error.as_deref(), Some("El nombre es obligatorio"));
}
#[test]
fn praind_requiere_fecha() {
    assert_requiere_fecha(0, false);
}
#[test]
fn in_house_requiere_fecha() {
    assert_requiere_fecha(1, false);
}
fn assert_requiere_fecha(tipo: usize, ruta: bool) {
    let mut f = FormularioContratista::nuevo();
    f.cedula = "1".into();
    f.nombre = "X".into();
    f.tipo = tipo;
    f.personal_ruta = ruta;
    let mut s = ContratistasState::default();
    assert_eq!(s.guardar(&f), Err("Fecha PRAIND requerida".into()));
}
#[test]
fn por_correo_normal_no_requiere_fecha() {
    assert_no_requiere_fecha(2);
}
#[test]
fn swat_normal_no_requiere_fecha() {
    assert_no_requiere_fecha(3);
}
fn assert_no_requiere_fecha(tipo: usize) {
    let mut f = FormularioContratista::nuevo();
    f.cedula = "1".into();
    f.nombre = "X".into();
    f.tipo = tipo;
    let mut s = ContratistasState::default();
    assert!(s.guardar(&f).is_ok());
    assert_eq!(s.registros.last().unwrap().fecha_praind, None);
}
#[test]
fn personal_de_ruta_requiere_fecha() {
    assert_requiere_fecha(3, true);
}
#[test]
fn empresa_es_seleccionable() {
    let mut s = ContratistasState::default();
    s.handle_key(tecla(KeyCode::Char('N')));
    s.handle_key(tecla(KeyCode::Down));
    s.handle_key(tecla(KeyCode::Down));
    s.handle_key(tecla(KeyCode::Enter));
    s.handle_key(tecla(KeyCode::Down));
    s.handle_key(tecla(KeyCode::Enter));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.empresa, 1);
}
#[test]
fn tipo_es_seleccionable() {
    let mut s = ContratistasState::default();
    s.handle_key(tecla(KeyCode::Char('N')));
    for _ in 0..3 {
        s.handle_key(tecla(KeyCode::Down));
    }
    s.handle_key(tecla(KeyCode::Enter));
    s.handle_key(tecla(KeyCode::Down));
    s.handle_key(tecla(KeyCode::Enter));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.tipo, 1);
}
#[test]
fn edicion_precarga_datos() {
    let mut s = ContratistasState::default();
    let original = s.registros[0].clone();
    s.handle_key(tecla(KeyCode::Char('E')));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.cedula, original.cedula);
    assert_eq!(f.nombre, original.nombre);
    assert_eq!(f.modo, ModoFormulario::Editar { id: original.id });
}
#[test]
fn cancelar_edicion_no_modifica_original() {
    let mut s = ContratistasState::default();
    let original = s.registros[0].clone();
    s.handle_key(tecla(KeyCode::Char('E')));
    escribir(&mut s, "9");
    s.handle_key(tecla(KeyCode::Esc));
    assert_eq!(s.registros[0], original);
}
#[test]
fn guardar_edicion_modifica_y_conserva_id() {
    let mut s = ContratistasState::default();
    let id = s.registros[0].id;
    s.handle_key(tecla(KeyCode::Char('E')));
    s.handle_key(tecla(KeyCode::Down));
    escribir(&mut s, " Editado");
    s.handle_key(tecla(KeyCode::Down));
    s.handle_key(tecla(KeyCode::Char('g')));
    assert!(s.registros[0].nombre.ends_with("Editado"));
    assert_eq!(s.registros[0].id, id);
}

#[test]
fn g_minuscula_guarda_fuera_del_nombre() {
    let mut s = ContratistasState::default();
    abrir_crear_valido(&mut s);
    s.handle_key(tecla(KeyCode::Char('g')));
    assert_eq!(s.registros.len(), 41);
    assert_eq!(s.modo, ModoContratistas::Normal);
}

#[test]
fn fecha_acepta_ocho_digitos_y_agrega_separadores() {
    let mut s = ContratistasState::default();
    s.handle_key(tecla(KeyCode::Char('N')));
    for _ in 0..4 {
        s.handle_key(tecla(KeyCode::Down));
    }
    escribir(&mut s, "20092026abc99");
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.fecha_praind, "20/09/2026");
}
#[test]
fn columnas_se_configuran_sin_ocultar_todas() {
    let mut s = ContratistasState::default();
    s.handle_key(tecla(KeyCode::Char('C')));
    s.handle_key(tecla(KeyCode::Char(' ')));
    assert!(!s.columnas[0].1);
    for i in 1..s.columnas.len() {
        s.modo = ModoContratistas::Columnas { seleccion: i };
        s.handle_key(tecla(KeyCode::Char(' ')));
    }
    assert_eq!(s.columnas.iter().filter(|(_, v)| *v).count(), 1);
}
#[test]
fn scroll_sigue_la_seleccion() {
    let mut s = ContratistasState::default();
    for _ in 0..20 {
        s.handle_key(tecla(KeyCode::Down));
    }
    assert_eq!(s.seleccion, Some(20));
    assert_eq!(s.inicio_visible(8), 13);
}
#[test]
fn fecha_invalida_se_rechaza() {
    let mut f = FormularioContratista::nuevo();
    f.cedula = "1".into();
    f.nombre = "X".into();
    f.fecha_praind = "99/99/2026".into();
    let mut s = ContratistasState::default();
    assert_eq!(s.guardar(&f), Err("Fecha inválida. Use DD/MM/YYYY".into()));
}
