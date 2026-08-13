use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
fn k(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}
fn empresa() -> Empresa {
    Empresa {
        id: 5,
        nombre: "Empresa Real".into(),
    }
}
fn resumen() -> ContratistaResumen {
    ContratistaResumen {
        id: 7,
        empresa_id: 5,
        cedula: "001-2".into(),
        nombre: "José Hernández".into(),
        empresa_nombre: "Empresa Real".into(),
        tipo_ingreso: TipoIngreso::Praind,
        fecha_vencimiento_praind: NaiveDate::from_ymd_opt(2026, 12, 31),
        es_personal_ruta: false,
        tiene_acceso: true,
    }
}
fn cargar(s: &mut ContratistasState) {
    s.completar_empresas(Ok(vec![empresa()]));
    s.completar_busqueda(Ok(vec![resumen()]), None)
}
fn escribir(s: &mut ContratistasState, t: &str) {
    for c in t.chars() {
        s.handle_key(k(KeyCode::Char(c)));
    }
}

#[test]
fn inicia_vacio_y_carga_resultados_reales() {
    let mut s = ContratistasState::default();
    assert!(s.registros.is_empty());
    cargar(&mut s);
    assert_eq!(s.registros[0].empresa_id, 5);
    assert_eq!(s.seleccion, Some(0));
}
#[test]
fn sin_empresas_bloquea_creacion() {
    let mut s = ContratistasState::default();
    s.handle_key(k(KeyCode::Char('N')));
    assert_eq!(
        s.mensaje.as_deref(),
        Some("Debe registrar al menos una empresa antes de crear contratistas")
    );
    assert_eq!(s.modo, ModoContratistas::Normal);
}
#[test]
fn busqueda_incremental_emite_consulta_real() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Char('/')));
    assert!(
        matches!(s.handle_key(k(KeyCode::Char('j'))),AccionContratistas::Buscar{texto:Some(t),..} if t=="j")
    );
    assert_eq!(s.registros.len(), 1);
}
#[test]
fn detalle_edicion_precargan_ids_y_datos_reales() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Enter));
    assert!(matches!(s.modo, ModoContratistas::Detalle { id: 7 }));
    s.handle_key(k(KeyCode::Char('E')));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert!(matches!(f.modo, ModoFormulario::Editar { id: 7 }));
    assert_eq!(f.empresa, 0);
    assert_eq!(f.tipo, TipoIngreso::Praind);
}
#[test]
fn formulario_valida_y_emite_creacion_tipificada() {
    let mut s = ContratistasState::default();
    s.completar_empresas(Ok(vec![empresa()]));
    s.handle_key(k(KeyCode::Char('N')));
    escribir(&mut s, "001");
    s.handle_key(k(KeyCode::Tab));
    escribir(&mut s, "Ana");
    for _ in 0..3 {
        s.handle_key(k(KeyCode::Tab));
    }
    escribir(&mut s, "31122026");
    s.handle_key(k(KeyCode::Tab));
    let a = s.handle_key(k(KeyCode::Char('G')));
    assert!(matches!(
        a,
        AccionContratistas::Crear {
            datos: DatosContratista {
                empresa_id: 5,
                tipo_ingreso: TipoIngreso::Praind,
                fecha_vencimiento_praind: Some(_),
                ..
            },
            ..
        }
    ));
}
#[test]
fn praind_dinamico_usa_regla_de_dominio_y_none_si_no_requerido() {
    let mut f = FormularioContratista::nuevo();
    f.tipo = TipoIngreso::PorCorreo;
    assert!(!f.requiere_praind());
    f.personal_ruta = true;
    assert!(f.requiere_praind());
    f.personal_ruta = false;
    f.fecha_praind = "31/12/2026".into();
    let d = match construir(&f, Some(5)) {
        Err(e) => e,
        Ok(_) => panic!(),
    };
    assert_eq!(d, "La cédula es obligatoria");
    f.cedula = "1".into();
    f.nombre = "A".into();
    assert_eq!(
        construir(&f, Some(5)).unwrap().fecha_vencimiento_praind,
        None
    );
}
#[test]
fn cancelar_no_muta_y_callback_recarga() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Char('E')));
    escribir(&mut s, "x");
    s.handle_key(k(KeyCode::Esc));
    assert_eq!(s.registros[0].nombre, "José Hernández");
    assert!(matches!(
        s.completar_guardado(Ok(None), Some(7), "José"),
        AccionContratistas::Buscar {
            seleccionar_id: Some(7),
            ..
        }
    ));
}
#[test]
fn columnas_limites_fecha_y_escape_raiz_se_conservan() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.set_hoy(NaiveDate::from_ymd_opt(2026, 8, 12).unwrap());
    s.handle_key(k(KeyCode::Up));
    assert_eq!(s.seleccion, Some(0));
    s.handle_key(k(KeyCode::Char('C')));
    s.handle_key(k(KeyCode::Esc));
    assert_eq!(s.modo, ModoContratistas::Normal);
    assert!(matches!(
        s.handle_key(k(KeyCode::Esc)),
        AccionContratistas::Volver
    ));
}
