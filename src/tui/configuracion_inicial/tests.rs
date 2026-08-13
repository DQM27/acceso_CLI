use super::*;

fn tecla(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn escribir(state: &mut ConfiguracionInicialState, texto: &str) {
    for caracter in texto.chars() {
        state.handle_key(tecla(KeyCode::Char(caracter)));
    }
}

fn guardar(state: &mut ConfiguracionInicialState) {
    state.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
}

fn formulario_valido() -> ConfiguracionInicialState {
    let mut state = ConfiguracionInicialState::default();
    escribir(&mut state, "1-1111-1111");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "Daniel Quintana");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "password1");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "password1");
    state
}

#[test]
fn campo_inicial_y_navegacion_funcionan() {
    let mut state = ConfiguracionInicialState::default();
    assert_eq!(state.campo_activo(), CampoConfiguracion::Cedula);
    state.handle_key(tecla(KeyCode::Tab));
    assert_eq!(state.campo_activo(), CampoConfiguracion::Nombre);
    state.handle_key(tecla(KeyCode::Down));
    assert_eq!(state.campo_activo(), CampoConfiguracion::Password);
    state.handle_key(tecla(KeyCode::Up));
    assert_eq!(state.campo_activo(), CampoConfiguracion::Nombre);
    state.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(state.campo_activo(), CampoConfiguracion::Cedula);
}

#[test]
fn campos_aceptan_texto_conservan_contenido_y_backspace_funciona() {
    let mut state = ConfiguracionInicialState::default();
    escribir(&mut state, "ABC-1x");
    state.handle_key(tecla(KeyCode::Backspace));
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "Nombre");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "secreto1");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "secreto1");

    assert_eq!(state.cedula, "ABC-1");
    assert_eq!(state.nombre, "Nombre");
    assert_eq!(state.password, "secreto1");
    assert_eq!(state.confirmar_password, "secreto1");
    assert_eq!(state.password_enmascarado(), "••••••••");
}

#[test]
fn escape_solicita_salida() {
    let mut state = ConfiguracionInicialState::default();
    assert_eq!(
        state.handle_key(tecla(KeyCode::Esc)),
        AccionConfiguracion::Salir
    );
}

#[test]
fn cedula_vacia_es_rechazada() {
    let mut state = ConfiguracionInicialState::default();
    guardar(&mut state);
    assert!(matches!(state.estado(), EstadoConfiguracion::Error(_)));
    assert_eq!(state.campo_activo(), CampoConfiguracion::Cedula);
}

#[test]
fn nombre_vacio_es_rechazado() {
    let mut state = ConfiguracionInicialState::default();
    escribir(&mut state, "1001");
    guardar(&mut state);
    assert_eq!(state.campo_activo(), CampoConfiguracion::Nombre);
}

#[test]
fn password_vacio_es_rechazado() {
    let mut state = ConfiguracionInicialState::default();
    escribir(&mut state, "1001");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "Nombre");
    guardar(&mut state);
    assert_eq!(state.campo_activo(), CampoConfiguracion::Password);
}

#[test]
fn confirmacion_vacia_es_rechazada() {
    let mut state = ConfiguracionInicialState::default();
    escribir(&mut state, "1001");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "Nombre");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "password1");
    guardar(&mut state);
    assert_eq!(state.campo_activo(), CampoConfiguracion::ConfirmarPassword);
}

#[test]
fn passwords_distintos_son_rechazados() {
    let mut state = formulario_valido();
    state.handle_key(tecla(KeyCode::Backspace));
    state.handle_key(tecla(KeyCode::Char('2')));
    guardar(&mut state);
    assert!(matches!(
        state.estado(),
        EstadoConfiguracion::Error(error) if error == "Las contraseñas no coinciden"
    ));
}

#[test]
fn password_corto_es_rechazado() {
    let mut state = ConfiguracionInicialState::default();
    escribir(&mut state, "1001");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "Nombre");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "corta");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "corta");
    guardar(&mut state);
    assert_eq!(state.campo_activo(), CampoConfiguracion::Password);
}

#[test]
fn formulario_valido_produce_solicitud_normalizada() {
    let mut state = formulario_valido();
    guardar(&mut state);
    assert_eq!(state.estado(), &EstadoConfiguracion::Creando);
    let solicitud = state.tomar_solicitud().unwrap();
    assert_eq!(solicitud.cedula, "1-1111-1111");
    assert_eq!(solicitud.nombre, "Daniel Quintana");
    assert_eq!(solicitud.password, "password1");
}

#[test]
fn limpiar_secretos_no_los_expone_en_debug() {
    let mut state = formulario_valido();
    assert!(!format!("{state:?}").contains("password1"));
    state.limpiar_secretos();
    assert!(state.password.is_empty());
    assert!(state.confirmar_password.is_empty());
}
