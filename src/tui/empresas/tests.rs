use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};

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
            activo: true,
        },
        EmpresaResumen {
            id: 9,
            nombre: "Brisas".into(),
            contratistas: 0,
            activo: true,
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
    assert_eq!(s.handle_key(k(KeyCode::Char('a'))), AccionEmpresas::Ninguna);
    assert_eq!(
        s.tick(Instant::now() + DURACION_DEBOUNCE + Duration::from_millis(1)),
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
fn enter_edita_directamente_con_id_y_nombre_reales() {
    let mut s = EmpresasState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Enter));
    let ModoEmpresas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.nombre.value(), "Constructora Álvarez");
    assert!(matches!(f.modo, ModoFormularioEmpresa::Editar { id: 7 }));
}

#[test]
fn a_pide_confirmar_el_cambio_de_estado_y_enter_emite_la_intencion() {
    let mut s = EmpresasState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Char('A')));
    let ModoEmpresas::ConfirmacionEstado(c) = &s.modo else {
        panic!("debía pedir confirmación")
    };
    assert_eq!(c.id, 7);
    assert!(!c.activar);
    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionEmpresas::EstablecerActivo {
            id: 7,
            activar: false,
            nombre: "Constructora Álvarez".into(),
        }
    );
}

#[test]
fn esc_en_confirmacion_de_estado_cancela_sin_emitir_nada() {
    let mut s = EmpresasState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Char('A')));
    assert_eq!(s.handle_key(k(KeyCode::Esc)), AccionEmpresas::Ninguna);
    assert_eq!(s.modo, ModoEmpresas::Normal);
}

#[test]
fn panel_refleja_la_seleccion_resaltada_sin_pasos_extra() {
    let mut s = EmpresasState::default();
    s.completar_busqueda(Ok(datos()), None);
    assert_eq!(s.empresa_seleccionada().map(|e| e.id), Some(7));
    s.handle_key(k(KeyCode::Down));
    assert_eq!(s.empresa_seleccionada().map(|e| e.id), Some(9));
}

#[test]
fn crear_y_actualizar_emiten_intenciones_sin_mutar_datos() {
    let mut s = EmpresasState::default();
    s.completar_busqueda(Ok(datos()), None);
    s.handle_key(k(KeyCode::Char('N')));
    escribir(&mut s, "Nueva");
    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionEmpresas::Crear {
            nombre: "Nueva".into()
        }
    );
    assert_eq!(s.empresas.len(), 2);
    s.handle_key(k(KeyCode::Esc));
    s.handle_key(k(KeyCode::Enter));
    assert!(matches!(
        s.handle_key(k(KeyCode::Enter)),
        AccionEmpresas::Actualizar { id: 7, .. }
    ));
}

#[test]
fn nombre_vacio_o_solo_espacios_no_se_despacha_y_muestra_error() {
    let mut s = EmpresasState::default();
    s.handle_key(k(KeyCode::Char('N')));
    escribir(&mut s, "   ");
    assert_eq!(s.handle_key(k(KeyCode::Enter)), AccionEmpresas::Ninguna);
    let ModoEmpresas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.error.as_deref(), Some("El nombre es obligatorio"));
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
            activo: true,
        }]),
        Some(12),
    );
    assert_eq!(s.seleccion, Some(0));
    s.handle_key(k(KeyCode::Enter));
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
        s.mensaje.as_deref(),
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
