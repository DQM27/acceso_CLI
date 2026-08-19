use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};
fn k(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}
fn empresa() -> Empresa {
    Empresa {
        id: 5,
        nombre: "Empresa Real".into(),
        activo: true,
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
    s.completar_busqueda(
        Ok(PaginaContratistas {
            items: vec![resumen()],
            total: 1,
        }),
        None,
    )
}
fn escribir(s: &mut ContratistasState, t: &str) {
    for c in t.chars() {
        s.handle_key(k(KeyCode::Char(c)));
    }
}

#[test]
fn pagedown_avanza_una_pagina_y_pageup_retrocede() {
    let mut s = ContratistasState::default();
    s.completar_empresas(Ok(vec![empresa()]));
    s.completar_busqueda(
        Ok(PaginaContratistas {
            items: vec![resumen()],
            total: 120,
        }),
        None,
    );

    let AccionContratistas::Buscar { offset, .. } = s.handle_key(k(KeyCode::PageDown)) else {
        panic!("debía pedir la siguiente página")
    };
    assert_eq!(offset, LIMITE_PAGINA);

    // Simula la respuesta de esa página para que el estado interno sepa en
    // qué offset quedó antes de retroceder.
    s.completar_busqueda(
        Ok(PaginaContratistas {
            items: vec![resumen()],
            total: 120,
        }),
        None,
    );
    let AccionContratistas::Buscar { offset, .. } = s.handle_key(k(KeyCode::PageUp)) else {
        panic!("debía pedir la página anterior")
    };
    assert_eq!(offset, 0);
}

#[test]
fn pagedown_no_hace_nada_si_ya_esta_en_la_ultima_pagina() {
    let mut s = ContratistasState::default();
    s.completar_empresas(Ok(vec![empresa()]));
    s.completar_busqueda(
        Ok(PaginaContratistas {
            items: vec![resumen()],
            total: 1,
        }),
        None,
    );
    assert!(matches!(
        s.handle_key(k(KeyCode::PageDown)),
        AccionContratistas::Ninguna
    ));
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
fn busqueda_incremental_emite_consulta_real_tras_el_debounce() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Char('/')));
    assert!(matches!(
        s.handle_key(k(KeyCode::Char('j'))),
        AccionContratistas::Ninguna
    ));
    let accion = s.tick(Instant::now() + DURACION_DEBOUNCE + Duration::from_millis(1));
    assert!(matches!(accion, AccionContratistas::Buscar{texto:Some(t),..} if t=="j"));
    assert_eq!(s.registros.len(), 1);
}
#[test]
fn flecha_izquierda_mueve_el_cursor_en_el_campo_nombre_sin_borrar() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Char('N')));
    // Cédula -> Nombre.
    s.handle_key(k(KeyCode::Down));
    escribir(&mut s, "Jose Perez");
    // Corrige "Jose" -> "José" moviendo el cursor en vez de borrar todo.
    for _ in 0.." Perez".chars().count() {
        s.handle_key(k(KeyCode::Left));
    }
    s.handle_key(k(KeyCode::Backspace));
    s.handle_key(k(KeyCode::Char('é')));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!("debía seguir en el formulario")
    };
    assert_eq!(f.nombre.value(), "José Perez");
}
#[test]
fn busqueda_admite_clave_valor_de_empresa_tipo_y_deja_texto_libre() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Char('/')));
    escribir(&mut s, "empresa:Real tipo:praind Ana");
    let accion = s.handle_key(k(KeyCode::Enter));
    // El último carácter tecleado ('a' de Ana) ya disparó la última consulta.
    let _ = accion;
    let AccionContratistas::Buscar {
        texto,
        empresa_id,
        tipos,
        ..
    } = s.buscar(None)
    else {
        panic!("debía consultar")
    };
    assert_eq!(empresa_id, Some(5));
    assert_eq!(tipos, Some(vec![TipoIngreso::Praind]));
    assert_eq!(texto.as_deref(), Some("Ana"));
}

#[test]
fn busqueda_admite_praind_ruta_y_acceso() {
    let mut s = ContratistasState::default();
    s.set_hoy(NaiveDate::from_ymd_opt(2026, 8, 17).unwrap());
    s.filtro = "praind:vencido ruta:si acceso:no".into();
    let AccionContratistas::Buscar {
        praind,
        personal_ruta,
        tiene_acceso,
        texto,
        ..
    } = s.buscar(None)
    else {
        panic!("debía consultar")
    };
    assert_eq!(
        praind,
        Some(crate::database::queries::contratistas::FiltroPraind::Vencido {
            hoy: NaiveDate::from_ymd_opt(2026, 8, 17).unwrap()
        })
    );
    assert_eq!(personal_ruta, Some(true));
    assert_eq!(tiene_acceso, Some(false));
    assert_eq!(texto, None);
}

#[test]
fn clave_no_reconocida_o_lista_de_praind_cae_a_texto_libre() {
    let s = ContratistasState {
        filtro: "praind:vence,vencido tipo:invalido".into(),
        ..Default::default()
    };
    let AccionContratistas::Buscar { texto, praind, tipos, .. } = s.buscar(None) else {
        panic!("debía consultar")
    };
    assert_eq!(praind, None);
    assert_eq!(tipos, None);
    assert_eq!(texto.as_deref(), Some("praind:vence,vencido tipo:invalido"));
}

#[test]
fn enter_edita_directamente_con_ids_y_datos_reales() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Enter));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert!(matches!(f.modo, ModoFormulario::Editar { id: 7 }));
    assert_eq!(f.campo, 1);
    assert_eq!(f.cedula.value(), "001-2");
    assert_eq!(f.empresa, 0);
    assert_eq!(f.tipo, TipoIngreso::Praind);
}
#[test]
fn panel_refleja_la_seleccion_resaltada_sin_pasos_extra() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    assert_eq!(s.seleccionado().map(|c| c.id), Some(7));
}
#[test]
fn edicion_no_permite_enfocar_ni_modificar_cedula() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Enter));
    // Nombre es el primer campo habilitado en modo Editar (Cédula queda
    // excluida); subir desde ahí da la vuelta al último campo (Acceso) en
    // vez de quedarse pegado, así que nunca aterriza en Cédula.
    s.handle_key(k(KeyCode::Up));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.campo, 6);
    assert_ne!(f.campo, 0, "no debe poder enfocar Cédula en modo Editar");

    s.handle_key(k(KeyCode::Down));
    s.handle_key(k(KeyCode::Char('9')));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.campo, 1);
    assert_eq!(f.cedula.value(), "001-2");
    assert_eq!(f.nombre.value(), "José Hernández9");
}

#[test]
fn tab_en_el_ultimo_campo_da_la_vuelta_al_primero_y_viceversa() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Char('N')));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.campo, 0, "Crear empieza en Cédula");

    // TAB siete veces (uno por campo) debe volver exactamente a Cédula.
    for _ in 0..7 {
        s.handle_key(k(KeyCode::Tab));
    }
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.campo, 0);

    // Shift+Tab (BackTab) desde Cédula da la vuelta al último campo.
    s.handle_key(k(KeyCode::BackTab));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.campo, 6);
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
    let a = s.handle_key(k(KeyCode::Enter));
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
fn espacio_abre_el_desplegable_y_alterna_los_campos_booleanos() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Enter));

    // Empresa: SPACE abre el desplegable en vez de guardar.
    s.handle_key(k(KeyCode::Tab));
    s.handle_key(k(KeyCode::Char(' ')));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert!(f.desplegable.is_some());
    s.handle_key(k(KeyCode::Esc));

    // Avanza hasta el campo Ruta y confirma que SPACE lo alterna.
    for _ in 0..20 {
        let ModoContratistas::Formulario(f) = &s.modo else {
            panic!()
        };
        if CampoFormulario::TODOS[f.campo] == CampoFormulario::Ruta {
            break;
        }
        s.handle_key(k(KeyCode::Tab));
    }
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(CampoFormulario::TODOS[f.campo], CampoFormulario::Ruta);
    let ruta_inicial = f.personal_ruta;

    s.handle_key(k(KeyCode::Char(' ')));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert_eq!(f.personal_ruta, !ruta_inicial);
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
    f.cedula = TextInput::new("1");
    f.nombre = TextInput::new("A");
    assert_eq!(
        construir(&f, Some(5)).unwrap().fecha_vencimiento_praind,
        None
    );
}
#[test]
fn cancelar_no_muta_y_callback_recarga() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Enter));
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
