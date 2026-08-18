use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use std::time::{Duration, Instant};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn save() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
}
fn resumen(id: i64, rol: RolUsuario, activo: bool) -> UsuarioResumen {
    UsuarioResumen {
        id,
        cedula: format!("ID-{id}"),
        nombre: format!("Usuario {id}"),
        rol,
        activo,
    }
}
fn cargar(state: &mut UsuariosState) {
    state.completar_busqueda(
        Ok(vec![
            resumen(7, RolUsuario::Root, true),
            resumen(9, RolUsuario::Operador, false),
        ]),
        None,
    );
}

#[test]
fn inicia_vacio_y_carga_resumenes_reales() {
    let mut state = UsuariosState::default();
    assert!(state.usuarios.is_empty());
    cargar(&mut state);
    assert_eq!(state.seleccion, Some(0));
    assert_eq!(state.usuario(7).unwrap().rol, RolUsuario::Root);
}

#[test]
fn tab_en_el_formulario_da_la_vuelta_en_ambas_direcciones() {
    let mut state = UsuariosState::default();
    cargar(&mut state);
    state.handle_key(key(KeyCode::Enter));
    let ModoUsuarios::Formulario(f) = &state.modo else {
        panic!("debía abrir el formulario")
    };
    assert_eq!(f.campo, 0);

    // Editar tiene 4 campos (Cédula/Nombre/Rol/Activo); Shift+Tab desde el
    // primero debe dar la vuelta al último en vez de quedarse pegado.
    state.handle_key(key(KeyCode::BackTab));
    let ModoUsuarios::Formulario(f) = &state.modo else {
        panic!()
    };
    assert_eq!(f.campo, 3);

    state.handle_key(key(KeyCode::Tab));
    let ModoUsuarios::Formulario(f) = &state.modo else {
        panic!()
    };
    assert_eq!(f.campo, 0);
}

#[test]
fn busqueda_emite_consulta_sin_filtrar_memoria_tras_el_debounce() {
    let mut state = UsuariosState::default();
    cargar(&mut state);
    state.handle_key(key(KeyCode::Char('/')));
    let accion = state.handle_key(key(KeyCode::Char('j')));
    assert!(matches!(accion, AccionUsuarios::Ninguna));
    let accion = state.tick(Instant::now() + DURACION_DEBOUNCE + Duration::from_millis(1));
    assert!(matches!(accion, AccionUsuarios::Buscar { texto: Some(t), .. } if t == "j"));
    assert_eq!(state.usuarios.len(), 2);
}

#[test]
fn enter_edita_directamente_con_id_real_y_sin_password() {
    let mut state = UsuariosState::default();
    cargar(&mut state);
    state.handle_key(key(KeyCode::Enter));
    let ModoUsuarios::Formulario(f) = &state.modo else {
        panic!()
    };
    assert!(matches!(f.modo, ModoFormularioUsuario::Editar { id: 7 }));
    assert!(!f.campos().contains(&CampoUsuario::Password));
}

#[test]
fn panel_refleja_la_seleccion_resaltada_sin_pasos_extra() {
    let mut state = UsuariosState::default();
    cargar(&mut state);
    assert_eq!(state.seleccionado().map(|u| u.id), Some(7));
    state.handle_key(key(KeyCode::Down));
    assert_eq!(state.seleccionado().map(|u| u.id), Some(9));
}

#[test]
fn espacio_abre_el_selector_de_rol_y_alterna_activo() {
    let mut state = UsuariosState::default();
    cargar(&mut state);
    state.abrir_edicion(7);
    // campo 0 = Cédula, 1 = Nombre, 2 = Rol
    state.handle_key(key(KeyCode::Tab));
    state.handle_key(key(KeyCode::Tab));
    let ModoUsuarios::Formulario(f) = &state.modo else {
        panic!()
    };
    assert_eq!(f.campo_actual(), CampoUsuario::Rol);
    state.handle_key(key(KeyCode::Char(' ')));
    let ModoUsuarios::Formulario(f) = &state.modo else {
        panic!()
    };
    assert!(f.selector_rol.is_some());
    state.handle_key(key(KeyCode::Esc));

    state.handle_key(key(KeyCode::Tab));
    let ModoUsuarios::Formulario(f) = &state.modo else {
        panic!()
    };
    assert_eq!(f.campo_actual(), CampoUsuario::Activo);
    let activo_inicial = f.activo;
    state.handle_key(key(KeyCode::Char(' ')));
    let ModoUsuarios::Formulario(f) = &state.modo else {
        panic!()
    };
    assert_eq!(f.activo, !activo_inicial);
}

#[test]
fn crear_emite_dto_real_y_debug_redacta_password() {
    let mut state = UsuariosState::default();
    let form = FormularioUsuario {
        modo: ModoFormularioUsuario::Crear,
        cedula: " 001-A ".into(),
        nombre: " Ana ".into(),
        rol: RolUsuario::Administrador,
        activo: true,
        password: Secreto("secreto-123".into()),
        confirmar_password: Secreto("secreto-123".into()),
        campo: 0,
        selector_rol: None,
        error: None,
    };
    state.modo = ModoUsuarios::Formulario(form);
    let accion = state.handle_key(save());
    let debug = format!("{accion:?}");
    assert!(!debug.contains("secreto-123"));
    assert!(debug.contains("[OCULTA]"));
    assert!(
        matches!(accion, AccionUsuarios::Crear { input: CrearUsuarioInput { cedula, rol: RolUsuario::Administrador, activo: true, .. }, .. } if cedula == "001-A")
    );
}

#[test]
fn password_valida_emite_y_limpia_secretos_en_callbacks() {
    let mut state = UsuariosState::default();
    cargar(&mut state);
    state.abrir_password(7);
    let ModoUsuarios::CambioPassword(f) = &mut state.modo else {
        panic!()
    };
    f.password = Secreto("password-B".into());
    f.confirmar = Secreto("password-B".into());
    let accion = state.handle_key(save());
    assert!(!format!("{accion:?}").contains("password-B"));
    state.completar_password(Ok(()), "Usuario 7");
    assert_eq!(state.modo, ModoUsuarios::Normal);
    assert_eq!(
        state.mensaje.as_deref(),
        Some("✓ Contraseña actualizada — Usuario 7")
    );
}

#[test]
fn activar_no_muta_hasta_respuesta_y_recarga() {
    let mut state = UsuariosState::default();
    cargar(&mut state);
    state.seleccion = Some(1);
    state.handle_key(key(KeyCode::Char('A')));
    let accion = state.handle_key(key(KeyCode::Enter));
    assert!(!state.usuario(9).unwrap().activo);
    assert!(matches!(
        accion,
        AccionUsuarios::EstablecerActivo {
            id: 9,
            activar: true,
            ..
        }
    ));
    assert!(matches!(
        state.completar_estado(Ok(()), 9, true, "Usuario 9"),
        AccionUsuarios::Buscar {
            seleccionar_id: Some(9),
            ..
        }
    ));
}

#[test]
fn errores_backend_permanecen_en_formulario_o_modal() {
    let mut state = UsuariosState {
        modo: ModoUsuarios::Formulario(FormularioUsuario::nuevo()),
        ..Default::default()
    };
    assert!(matches!(
        state.completar_guardado(Err("duplicado".into()), None, "Ana"),
        AccionUsuarios::Ninguna
    ));
    let ModoUsuarios::Formulario(f) = &state.modo else {
        panic!()
    };
    assert_eq!(f.error.as_deref(), Some("duplicado"));
}

#[test]
fn cancelar_formularios_no_emite_escritura() {
    let mut state = UsuariosState::default();
    cargar(&mut state);
    state.abrir_edicion(7);
    assert!(matches!(
        state.handle_key(key(KeyCode::Esc)),
        AccionUsuarios::Ninguna
    ));
    assert_eq!(state.usuario(7).unwrap().nombre, "Usuario 7");
}

#[test]
fn guardando_bloquea_cualquier_tecla_hasta_que_llega_el_resultado_real() {
    let mut state = UsuariosState::default();
    cargar(&mut state);
    state.handle_key(key(KeyCode::Enter));
    let ModoUsuarios::Formulario(antes) = state.modo.clone() else {
        panic!("debía abrir el formulario")
    };

    state.marcar_guardando();
    assert!(matches!(
        state.handle_key(key(KeyCode::Char('X'))),
        AccionUsuarios::Ninguna
    ));
    assert!(matches!(
        state.handle_key(key(KeyCode::Esc)),
        AccionUsuarios::Ninguna
    ));
    let ModoUsuarios::Formulario(despues) = &state.modo else {
        panic!("no debía salir del formulario mientras guarda")
    };
    assert_eq!(&antes, despues);

    // Al llegar el resultado (éxito o error) se libera y vuelve a aceptar teclas.
    state.completar_guardado(Err("duplicado".into()), None, "Ana");
    assert!(!state.guardando);
    let ModoUsuarios::Formulario(f) = &state.modo else {
        panic!()
    };
    assert_eq!(f.error.as_deref(), Some("duplicado"));
}
