use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn tecla(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn escribir(state: &mut CambioPasswordState, texto: &str) {
    for caracter in texto.chars() {
        state.handle_key(tecla(KeyCode::Char(caracter)));
    }
}

#[test]
fn exige_actual_y_confirmacion_coincidente() {
    let mut state = CambioPasswordState::default();
    escribir(&mut state, "password-actual");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "password-nueva");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(&mut state, "password-nueva");
    assert_eq!(
        state.handle_key(tecla(KeyCode::Enter)),
        AccionCambioPassword::Cambiar {
            password_actual: "password-actual".into(),
            nueva_password: "password-nueva".into(),
        }
    );
}

#[test]
fn completar_o_salir_limpia_los_tres_secretos() {
    let mut state = CambioPasswordState::default();
    escribir(&mut state, "password-actual");
    state.completar(Ok(()));
    assert_eq!(state.mascara(Campo::Actual), "");
    assert_eq!(state.mascara(Campo::Nueva), "");
    assert_eq!(state.mascara(Campo::Confirmacion), "");
}
