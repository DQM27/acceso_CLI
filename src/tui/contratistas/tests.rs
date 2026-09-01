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
        tiene_ingreso_activo: false,
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
fn fecha_praind_permite_mover_el_cursor_y_corregir_un_digito() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key(k(KeyCode::Enter));
    // Editar inicia en Nombre; avanza por Empresa y Tipo hasta Fecha PRAIND.
    for _ in 0..3 {
        s.handle_key(k(KeyCode::Tab));
    }
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!("debía estar editando")
    };
    assert_eq!(
        CampoFormulario::TODOS[f.campo],
        CampoFormulario::FechaPraind
    );
    assert_eq!(f.fecha_praind.value(), "31/12/2026");

    // Vuelve al primer dígito, cambia 31 por 21 y conserva el resto.
    for _ in 0..10 {
        s.handle_key(k(KeyCode::Left));
    }
    s.handle_key(k(KeyCode::Delete));
    s.handle_key(k(KeyCode::Char('2')));

    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!("debía seguir editando")
    };
    assert_eq!(f.fecha_praind.value(), "21/12/2026");
    let AccionContratistas::Actualizar { datos, .. } = s.handle_key(k(KeyCode::Enter)) else {
        panic!("debía emitir la actualización")
    };
    assert_eq!(
        datos.fecha_vencimiento_praind,
        NaiveDate::from_ymd_opt(2026, 12, 21)
    );
}

#[test]
fn formulario_alinea_nombre_y_fecha_a_la_derecha_de_sus_etiquetas() {
    use ratatui::{Terminal, backend::TestBackend};

    let mut state = ContratistasState::default();
    cargar(&mut state);
    state.handle_key_con_rol(k(KeyCode::Enter), RolUsuario::Root);
    let sesion = crate::services::autenticacion_service::UsuarioSesion {
        id: 1,
        cedula: "ROOT".into(),
        nombre: "Root".into(),
        rol: crate::models::usuario::RolUsuario::Root,
    };
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| {
            render::render(
                frame,
                frame.area(),
                &state,
                &sesion,
                crate::tui::ui_kit::ThemePreset::Brisas.theme(),
            )
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let lineas = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>();

    assert!(
        lineas
            .iter()
            .any(|linea| linea.contains("NOMBRE") && linea.contains("José Hernández"))
    );
    assert!(
        lineas
            .iter()
            .any(|linea| linea.contains("FECHA PRAIND") && linea.contains("31/12/2026"))
    );
    assert!(
        lineas
            .iter()
            .any(|linea| linea.contains("CÉDULA") && linea.contains("001-2"))
    );
    assert!(!lineas.iter().any(|linea| linea.contains("no editable")));
}

#[test]
fn buscador_separa_etiqueta_e_input_y_posiciona_el_cursor_al_final() {
    use ratatui::{Terminal, backend::TestBackend};

    let mut state = ContratistasState::default();
    cargar(&mut state);
    state.filtro = "filtro anterior".into();
    state.handle_key(k(KeyCode::Char('/')));

    let sesion = crate::services::autenticacion_service::UsuarioSesion {
        id: 1,
        cedula: "ROOT".into(),
        nombre: "Root".into(),
        rol: crate::models::usuario::RolUsuario::Root,
    };
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    let dibujar = |terminal: &mut Terminal<TestBackend>, state: &ContratistasState| {
        terminal
            .draw(|frame| {
                render::render(
                    frame,
                    frame.area(),
                    state,
                    &sesion,
                    crate::tui::ui_kit::ThemePreset::Brisas.theme(),
                )
            })
            .unwrap();
    };

    dibujar(&mut terminal, &state);
    let lineas = (0..terminal.backend().buffer().area.height)
        .map(|y| {
            (0..terminal.backend().buffer().area.width)
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let fila_etiqueta = lineas
        .iter()
        .position(|linea| linea.contains("BUSCAR CONTRATISTA"))
        .expect("debía renderizar la etiqueta del buscador");
    assert!(
        lineas[fila_etiqueta + 1].trim().is_empty(),
        "al pulsar / el input debe verse vacío aunque exista un filtro anterior"
    );
    let inicio_campo = lineas[fila_etiqueta + 2]
        .find('─')
        .expect("debía subrayar el input");
    let cursor = terminal.backend().cursor_position();
    assert_eq!(
        (cursor.x, cursor.y),
        (
            u16::try_from(inicio_campo).unwrap_or(u16::MAX),
            u16::try_from(fila_etiqueta + 1).unwrap_or(u16::MAX)
        )
    );

    escribir(&mut state, "hfjj");
    dibujar(&mut terminal, &state);
    let lineas = (0..terminal.backend().buffer().area.height)
        .map(|y| {
            (0..terminal.backend().buffer().area.width)
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let fila_etiqueta = lineas
        .iter()
        .position(|linea| linea.contains("BUSCAR CONTRATISTA"))
        .expect("debía mantener la etiqueta del buscador");
    let fila_input = fila_etiqueta + 1;
    let inicio_input = lineas[fila_input]
        .find("hfjj")
        .expect("el texto debe aparecer debajo de la etiqueta");
    let cursor = terminal.backend().cursor_position();

    assert!(lineas[fila_input + 1].contains('─'));
    assert_eq!(cursor.y, u16::try_from(fila_input).unwrap_or(u16::MAX));
    assert_eq!(
        cursor.x,
        u16::try_from(inicio_input).unwrap_or(u16::MAX)
            + u16::try_from("hfjj".len()).unwrap_or(u16::MAX)
    );
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
    assert_eq!(
        empresa_id,
        Some(crate::database::queries::Igualdad::Incluye(5))
    );
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
        Some(
            crate::database::queries::contratistas::FiltroPraind::Vencido {
                hoy: NaiveDate::from_ymd_opt(2026, 8, 17).unwrap()
            }
        )
    );
    assert_eq!(personal_ruta, Some(true));
    assert_eq!(tiene_acceso, Some(false));
    assert_eq!(texto, None);
}

#[test]
fn negar_ruta_y_acceso_invierte_el_booleano() {
    let s = ContratistasState {
        filtro: "-ruta:si -acceso:no".into(),
        ..Default::default()
    };
    let AccionContratistas::Buscar {
        personal_ruta,
        tiene_acceso,
        texto,
        ..
    } = s.buscar(None)
    else {
        panic!("debía consultar")
    };
    assert_eq!(personal_ruta, Some(false));
    assert_eq!(tiene_acceso, Some(true));
    assert_eq!(texto, None);
}

#[test]
fn empresa_con_clave_valor_ignora_tildes_en_ambos_lados() {
    let mut s = ContratistasState::default();
    s.completar_empresas(Ok(vec![Empresa {
        id: 9,
        nombre: "Álvarez Ingeniería".into(),
        activo: true,
    }]));
    s.filtro = "empresa:alvarez".into();
    let AccionContratistas::Buscar { empresa_id, .. } = s.buscar(None) else {
        panic!("debía consultar")
    };
    assert_eq!(
        empresa_id,
        Some(crate::database::queries::Igualdad::Incluye(9))
    );
}

#[test]
fn negar_empresa_y_praind() {
    let mut s = ContratistasState::default();
    s.completar_empresas(Ok(vec![Empresa {
        id: 9,
        nombre: "Álvarez Ingeniería".into(),
        activo: true,
    }]));
    s.set_hoy(NaiveDate::from_ymd_opt(2026, 8, 17).unwrap());
    s.filtro = "-empresa:alvarez -praind:vencido".into();
    let AccionContratistas::Buscar {
        empresa_id,
        praind,
        praind_negado,
        texto,
        ..
    } = s.buscar(None)
    else {
        panic!("debía consultar")
    };
    assert_eq!(
        empresa_id,
        Some(crate::database::queries::Igualdad::Excluye(9))
    );
    assert_eq!(
        praind,
        Some(
            crate::database::queries::contratistas::FiltroPraind::Vencido {
                hoy: NaiveDate::from_ymd_opt(2026, 8, 17).unwrap()
            }
        )
    );
    assert!(praind_negado);
    assert_eq!(texto, None);
}

#[test]
fn clave_no_reconocida_o_lista_de_praind_cae_a_texto_libre() {
    let s = ContratistasState {
        filtro: "praind:vence,vencido tipo:invalido".into(),
        ..Default::default()
    };
    assert!(s.resumen_consulta().contains("sin interpretar"));
    let AccionContratistas::Buscar {
        texto,
        praind,
        tipos,
        ..
    } = s.buscar(None)
    else {
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
fn operador_no_puede_enfocar_ni_modificar_cedula() {
    let mut s = ContratistasState::default();
    cargar(&mut s);
    s.handle_key_con_rol(k(KeyCode::Enter), RolUsuario::Operador);
    // Nombre es el primer campo habilitado en modo Editar (Cédula queda
    // excluida); subir desde ahí da la vuelta al último campo (Acceso) en
    // vez de quedarse pegado, así que nunca aterriza en Cédula.
    s.handle_key(k(KeyCode::Up));
    let ModoContratistas::Formulario(f) = &s.modo else {
        panic!()
    };
    assert!(!f.cedula_editable);
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
fn administrador_y_root_pueden_editar_cedula() {
    for rol in [RolUsuario::Administrador, RolUsuario::Root] {
        let mut s = ContratistasState::default();
        cargar(&mut s);
        s.handle_key_con_rol(k(KeyCode::Enter), rol);

        let ModoContratistas::Formulario(f) = &s.modo else {
            panic!("debía abrir el formulario para {rol:?}")
        };
        assert!(f.cedula_editable);
        assert_eq!(CampoFormulario::TODOS[f.campo], CampoFormulario::Cedula);

        s.handle_key_con_rol(k(KeyCode::Backspace), rol);
        s.handle_key_con_rol(k(KeyCode::Char('9')), rol);
        let AccionContratistas::Actualizar { id, datos, .. } =
            s.handle_key_con_rol(k(KeyCode::Enter), rol)
        else {
            panic!("debía emitir la actualización para {rol:?}")
        };
        assert_eq!(id, 7);
        assert_eq!(datos.cedula, "001-9");
    }
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
    f.fecha_praind = TextInput::new("31/12/2026").with_max_chars(10);
    let Err(d) = construir(&f, Some(5)) else {
        panic!()
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

#[test]
fn exito_sobrevive_recarga_y_navegacion() {
    let mut s = ContratistasState::default();
    let recarga = s.completar_guardado(Ok(Some(7)), None, "José");
    assert!(matches!(recarga, AccionContratistas::Buscar { .. }));
    cargar(&mut s);
    s.handle_key(k(KeyCode::Down));

    assert_eq!(s.mensaje.as_deref(), Some("✓ Contratista creado — José"));
}
