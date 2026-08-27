use super::*;
use crossterm::event::KeyModifiers;

fn tecla(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn login_completo() -> LoginState {
    let mut state = LoginState::default();
    for character in "1-1111-1111".chars() {
        state.handle_key(tecla(KeyCode::Char(character)));
    }
    state.handle_key(tecla(KeyCode::Tab));
    for character in "secreto".chars() {
        state.handle_key(tecla(KeyCode::Char(character)));
    }
    state
}

#[test]
fn presenta_contexto_y_dos_campos_desde_el_inicio() {
    use ratatui::{Terminal, backend::TestBackend};

    let ancho = 100;
    let mut terminal = Terminal::new(TestBackend::new(ancho, 30)).unwrap();
    terminal
        .draw(|frame| {
            render::render(
                frame,
                frame.area(),
                &LoginState::default(),
                crate::tui::ui_kit::ThemePreset::Classic.theme(),
            )
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let texto: String = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect::<Vec<String>>()
        .join("\n");

    assert!(texto.contains("CONTROL DE ACCESO"), "{texto}");
    assert!(texto.contains("Inicio de sesion local"), "{texto}");
    assert!(texto.contains("CEDULA"), "{texto}");
    assert!(texto.contains("CONTRASENA"), "{texto}");
    assert!(texto.contains("Enter continuar"), "{texto}");
    assert_eq!(texto.matches('┌').count(), 3, "{texto}");
}

#[test]
fn cambia_el_foco_en_ambas_direcciones() {
    let mut state = LoginState::default();
    state.handle_key(tecla(KeyCode::Tab));
    assert_eq!(state.campo_activo(), CampoLogin::Password);
    state.handle_key(tecla(KeyCode::BackTab));
    assert_eq!(state.campo_activo(), CampoLogin::Cedula);
}

#[test]
fn enter_desde_cedula_avanza_a_password() {
    let mut state = LoginState::default();
    state.handle_key(tecla(KeyCode::Enter));
    assert_eq!(state.campo_activo(), CampoLogin::Password);
}

#[test]
fn backspace_elimina_el_ultimo_caracter() {
    let mut state = LoginState::default();
    state.handle_key(tecla(KeyCode::Char('á')));
    state.handle_key(tecla(KeyCode::Char('b')));
    state.handle_key(tecla(KeyCode::Backspace));
    assert_eq!(state.cedula.value(), "á");
}

#[test]
fn campos_vacios_producen_error() {
    let mut state = LoginState::default();
    state.handle_key(tecla(KeyCode::Enter));
    state.handle_key(tecla(KeyCode::Enter));
    assert!(matches!(state.estado(), EstadoLogin::Error(_)));
}

#[test]
fn campos_completos_disparan_autenticacion_de_inmediato_y_quedan_validando() {
    let mut state = login_completo();
    assert_eq!(
        state.handle_key(tecla(KeyCode::Enter)),
        AccionLogin::Autenticar {
            cedula: "1-1111-1111".to_owned(),
            password: "secreto".to_owned(),
        }
    );
    assert!(matches!(state.estado(), EstadoLogin::Validando { .. }));
}

#[test]
fn completar_validacion_tras_el_resultado_real_limpia_la_contrasena() {
    let mut state = login_completo();
    state.handle_key(tecla(KeyCode::Enter));
    assert!(matches!(state.estado(), EstadoLogin::Validando { .. }));
    state.completar_validacion(None);
    assert_eq!(state.estado(), &EstadoLogin::Exito);
    assert!(state.password.value().is_empty());
}

#[test]
fn escribir_limpia_un_error_anterior() {
    let mut state = LoginState::default();
    state.handle_key(tecla(KeyCode::Enter));
    state.handle_key(tecla(KeyCode::Enter));
    state.handle_key(tecla(KeyCode::Char('x')));
    assert_eq!(state.estado(), &EstadoLogin::Normal);
}

#[test]
fn mascara_conserva_cantidad_de_caracteres() {
    let state = login_completo();
    assert_eq!(state.password_enmascarado(), "•••••••");
}

#[test]
fn representacion_para_render_no_expone_password() {
    let state = login_completo();
    let mascara = state.password_enmascarado();
    assert!(!mascara.contains("secreto"));
    assert!(!mascara.contains('s'));
}

#[test]
fn escape_limpia_la_contrasena_y_senala_salir() {
    let mut state = login_completo();
    assert_eq!(state.handle_key(tecla(KeyCode::Esc)), AccionLogin::Salir);
    assert!(state.password.value().is_empty());
}

#[test]
fn escape_funciona_incluso_mientras_valida() {
    let mut state = login_completo();
    state.handle_key(tecla(KeyCode::Enter));
    assert!(matches!(state.estado(), EstadoLogin::Validando { .. }));

    assert_eq!(state.handle_key(tecla(KeyCode::Esc)), AccionLogin::Salir);
    assert!(state.password.value().is_empty());
}

#[test]
fn cursor_parpadea_y_se_reinicia_al_escribir() {
    let inicio = Instant::now();
    let mut state = LoginState {
        cursor_iniciado: inicio,
        ..LoginState::default()
    };
    state.tick(inicio + DURACION_PARPADEO);
    assert!(!state.cursor_visible);

    state.handle_key(tecla(KeyCode::Char('1')));
    assert!(state.cursor_visible);
}
