use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
fn k(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn seleccion_inicial_movimiento_y_limites() {
    let mut s = MenuPrincipalState::default();
    assert_eq!(s.seleccion, OpcionMenu::NuevoIngreso);
    s.handle_key(k(KeyCode::Up));
    assert_eq!(s.seleccion, OpcionMenu::NuevoIngreso);
    s.handle_key(k(KeyCode::Down));
    assert_eq!(s.seleccion, OpcionMenu::IngresosActivos);
    s.handle_key(k(KeyCode::Up));
    assert_eq!(s.seleccion, OpcionMenu::NuevoIngreso);
    for _ in 0..20 {
        s.handle_key(k(KeyCode::Down));
    }
    assert_eq!(s.seleccion, OpcionMenu::Salir);
}

#[test]
fn enter_y_accesos_numericos_emiten_apertura_correcta() {
    let mut s = MenuPrincipalState::default();
    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionMenu::Abrir(OpcionMenu::NuevoIngreso)
    );
    for (c, opcion) in [
        ('1', OpcionMenu::NuevoIngreso),
        ('2', OpcionMenu::IngresosActivos),
        ('3', OpcionMenu::Historial),
        ('4', OpcionMenu::Contratistas),
        ('5', OpcionMenu::Empresas),
        ('6', OpcionMenu::Usuarios),
    ] {
        assert_eq!(s.handle_key(k(KeyCode::Char(c))), AccionMenu::Abrir(opcion));
    }
}

#[test]
fn logout_confirma_o_cancela() {
    let mut s = MenuPrincipalState::default();
    s.handle_key(k(KeyCode::Char('L')));
    assert_eq!(s.confirmacion, Some(ConfirmacionMenu::CerrarSesion));
    s.handle_key(k(KeyCode::Char('N')));
    assert_eq!(s.confirmacion, None);
    s.handle_key(k(KeyCode::Char('L')));
    s.handle_key(k(KeyCode::Esc));
    assert_eq!(s.confirmacion, None);
    s.handle_key(k(KeyCode::Char('L')));
    assert_eq!(
        s.handle_key(k(KeyCode::Char('Y'))),
        AccionMenu::CerrarSesion
    );
    s.handle_key(k(KeyCode::Char('L')));
    assert_eq!(s.handle_key(k(KeyCode::Enter)), AccionMenu::CerrarSesion);
}

#[test]
fn salida_confirma_o_cancela_y_escape_raiz_no_hace_nada() {
    let mut s = MenuPrincipalState::default();
    assert_eq!(s.handle_key(k(KeyCode::Esc)), AccionMenu::Ninguna);
    s.handle_key(k(KeyCode::Char('Q')));
    s.handle_key(k(KeyCode::Esc));
    assert_eq!(s.confirmacion, None);
    s.handle_key(k(KeyCode::Char('Q')));
    s.handle_key(k(KeyCode::Char('N')));
    assert_eq!(s.confirmacion, None);
    s.handle_key(k(KeyCode::Char('Q')));
    assert_eq!(s.handle_key(k(KeyCode::Char('Y'))), AccionMenu::Salir);
    s.handle_key(k(KeyCode::Char('Q')));
    assert_eq!(s.handle_key(k(KeyCode::Enter)), AccionMenu::Salir);
}
