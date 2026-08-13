use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn k(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}
fn escribir(s: &mut EmpresasState, texto: &str) {
    for c in texto.chars() {
        s.handle_key(k(KeyCode::Char(c)));
    }
}
fn datos() -> Vec<EmpresaResumen> {
    vec![
        EmpresaResumen {
            id: 7,
            nombre: "Constructora Álvarez".into(),
            contratistas: 3,
        },
        EmpresaResumen {
            id: 9,
            nombre: "Brisas".into(),
            contratistas: 0,
        },
    ]
}

#[test]
fn inicia_vacia_y_aplica_resultados_reales() {
    let mut s = EmpresasState::default();
    assert!(s.empresas.is_empty() && s.seleccion.is_none());
    s.completar_busqueda(Ok(datos()), None);
    assert_eq!(s.empresas.len(), 2);
    assert_eq!(s.seleccion, Some(0));
}

#[test]
fn busqueda_incremental_emite_consulta_real_sin_filtrar_vec() {
    let mut s = EmpresasState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Char('/')));
    assert_eq!(
        s.handle_key(k(KeyCode::Char('a'))),
        AccionEmpresas::Buscar {
            texto: Some("a".into()),
            seleccionar_id: None
        }
    );
    assert_eq!(s.empresas.len(), 2);
    s.completar_busqueda(Ok(vec![]), None);
    assert!(s.empresas.is_empty() && s.seleccion.is_none());
}

#[test]
fn detalle_y_edicion_usan_id_nombre_y_conteo_reales() {
    let mut s = EmpresasState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Enter));
    assert!(matches!(s.modo, ModoEmpresas::Detalle { id: 7 }));
    s.handle_key(k(KeyCode::Char('E')));
    let ModoEmpresas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.nombre, "Constructora Álvarez");
    assert!(matches!(f.modo, ModoFormularioEmpresa::Editar { id: 7 }));
}

#[test]
fn crear_y_actualizar_emiten_intenciones_sin_mutar_datos() {
    let mut s = EmpresasState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Char('N')));
    escribir(&mut s, "Nueva");
    assert_eq!(
        s.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT)),
        AccionEmpresas::Crear {
            nombre: "Nueva".into()
        }
    );
    assert_eq!(s.empresas.len(), 2);
    s.handle_key(k(KeyCode::Esc));
    s.handle_key(k(KeyCode::Char('E')));
    assert!(matches!(
        s.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT)),
        AccionEmpresas::Actualizar { id: 7, .. }
    ));
}

#[test]
fn callbacks_exito_recargan_y_error_permanece_en_formulario() {
    let mut s = EmpresasState::default();
    s.handle_key(k(KeyCode::Char('N')));
    escribir(&mut s, " Nueva ");
    assert_eq!(
        s.completar_creacion(Ok(12), " Nueva "),
        AccionEmpresas::Buscar {
            texto: None,
            seleccionar_id: Some(12)
        }
    );
    s.completar_busqueda(
        Ok(vec![EmpresaResumen {
            id: 12,
            nombre: "Nueva".into(),
            contratistas: 0,
        }]),
        Some(12),
    );
    assert_eq!(s.seleccion, Some(0));
    s.handle_key(k(KeyCode::Char('E')));
    s.completar_actualizacion(
        Err("Ya existe una empresa con ese nombre".into()),
        12,
        "Duplicada",
    );
    let ModoEmpresas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert!(f.error.is_some());
}

#[test]
fn cancelar_no_emite_escritura_y_escape_raiz_vuelve() {
    let mut s = EmpresasState::default();
    s.handle_key(k(KeyCode::Char('N')));
    escribir(&mut s, "No guardar");
    assert_eq!(s.handle_key(k(KeyCode::Esc)), AccionEmpresas::Ninguna);
    assert_eq!(s.handle_key(k(KeyCode::Esc)), AccionEmpresas::Volver);
}

#[test]
fn error_de_carga_es_presentable_y_movimiento_respeta_limites() {
    let mut s = EmpresasState::default();
    s.completar_busqueda(Err("No se pudo cargar la base de empresas".into()), None);
    assert_eq!(
        s.error_carga.as_deref(),
        Some("No se pudo cargar la base de empresas")
    );
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Up));
    assert_eq!(s.seleccion, Some(0));
    for _ in 0..5 {
        s.handle_key(k(KeyCode::Down));
    }
    assert_eq!(s.seleccion, Some(1));
}
