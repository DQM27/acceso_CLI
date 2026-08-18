use super::*;
use crate::models::usuario::RolUsuario;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
fn k(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn la_lista_queda_centrada_en_vez_de_pegada_a_la_izquierda() {
    use crate::{models::usuario::RolUsuario, services::autenticacion_service::UsuarioSesion};
    use ratatui::{Terminal, backend::TestBackend};

    // Se selecciona la opción que se va a medir: su marcador ">" la
    // distingue del relleno en blanco, para no confundir uno con otro al
    // contar espacios iniciales.
    let state = MenuPrincipalState {
        seleccion: OpcionMenu::IngresosActivos,
        ..Default::default()
    };
    let sesion = UsuarioSesion {
        id: 1,
        cedula: "1-1111-1111".into(),
        nombre: "Daniel Quintana".into(),
        rol: RolUsuario::Root,
    };
    let ancho_terminal = 140;
    let backend = TestBackend::new(ancho_terminal, 30);
    let mut terminal = Terminal::new(backend).expect("backend de prueba");
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
        .expect("debe renderizar");

    let buffer = terminal.backend().buffer();
    // Cada celda del buffer es un carácter; se arma el texto fila por fila.
    // Se usa la fila de "Ingresos activos" porque es la opción con la
    // etiqueta más larga — el ancho del bloque se ajusta a ella, así que
    // esa fila queda al ras de ambos bordes del bloque y sus márgenes son
    // directamente los márgenes del bloque completo.
    let fila = (0..buffer.area.height)
        .find(|&y| {
            let texto: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            texto.contains("Ingresos activos")
        })
        .expect("debe encontrar la fila con 'Ingresos activos'");

    let texto: String = (0..buffer.area.width)
        .map(|x| buffer[(x, fila)].symbol())
        .collect();
    let margen_izquierdo = texto.chars().take_while(|c| *c == ' ').count();
    let margen_derecho = texto.chars().rev().take_while(|c| *c == ' ').count();

    // El bloque de opciones debe quedar centrado: los márgenes izquierdo y
    // derecho no deben diferir por más de un carácter de redondeo.
    assert!(
        margen_izquierdo.abs_diff(margen_derecho) <= 1,
        "margen izquierdo {margen_izquierdo} vs derecho {margen_derecho} — la lista no está centrada"
    );
}

#[test]
fn seleccion_inicial_movimiento_y_limites() {
    let mut s = MenuPrincipalState::default();
    assert_eq!(s.seleccion, OpcionMenu::NuevoIngreso);
    s.handle_key(k(KeyCode::Up), RolUsuario::Root);
    assert_eq!(s.seleccion, OpcionMenu::NuevoIngreso);
    s.handle_key(k(KeyCode::Down), RolUsuario::Root);
    assert_eq!(s.seleccion, OpcionMenu::IngresosActivos);
    s.handle_key(k(KeyCode::Up), RolUsuario::Root);
    assert_eq!(s.seleccion, OpcionMenu::NuevoIngreso);
    for _ in 0..20 {
        s.handle_key(k(KeyCode::Down), RolUsuario::Root);
    }
    assert_eq!(s.seleccion, OpcionMenu::Salir);
}

#[test]
fn enter_y_accesos_numericos_emiten_apertura_correcta() {
    let mut s = MenuPrincipalState::default();
    assert_eq!(
        s.handle_key(k(KeyCode::Enter), RolUsuario::Root),
        AccionMenu::Abrir(OpcionMenu::NuevoIngreso)
    );
    for (c, opcion) in [
        ('1', OpcionMenu::NuevoIngreso),
        ('2', OpcionMenu::IngresosActivos),
        ('3', OpcionMenu::Historial),
        ('4', OpcionMenu::Contratistas),
        ('5', OpcionMenu::Empresas),
        ('6', OpcionMenu::Usuarios),
        ('7', OpcionMenu::Configuracion),
    ] {
        assert_eq!(
            s.handle_key(k(KeyCode::Char(c)), RolUsuario::Root),
            AccionMenu::Abrir(opcion)
        );
    }
}

#[test]
fn logout_confirma_o_cancela() {
    let mut s = MenuPrincipalState::default();
    s.handle_key(k(KeyCode::Char('L')), RolUsuario::Root);
    assert_eq!(s.confirmacion, Some(ConfirmacionMenu::CerrarSesion));
    // Sólo ENTER confirma y sólo ESC cancela — Y/N ya no hacen nada.
    s.handle_key(k(KeyCode::Char('N')), RolUsuario::Root);
    assert_eq!(s.confirmacion, Some(ConfirmacionMenu::CerrarSesion));
    s.handle_key(k(KeyCode::Esc), RolUsuario::Root);
    assert_eq!(s.confirmacion, None);
    s.handle_key(k(KeyCode::Char('L')), RolUsuario::Root);
    assert_eq!(
        s.handle_key(k(KeyCode::Char('Y')), RolUsuario::Root),
        AccionMenu::Ninguna
    );
    assert_eq!(
        s.handle_key(k(KeyCode::Enter), RolUsuario::Root),
        AccionMenu::CerrarSesion
    );
}

#[test]
fn salida_confirma_o_cancela_y_escape_raiz_no_hace_nada() {
    let mut s = MenuPrincipalState::default();
    assert_eq!(
        s.handle_key(k(KeyCode::Esc), RolUsuario::Root),
        AccionMenu::Ninguna
    );
    s.handle_key(k(KeyCode::Char('Q')), RolUsuario::Root);
    s.handle_key(k(KeyCode::Esc), RolUsuario::Root);
    assert_eq!(s.confirmacion, None);
    s.handle_key(k(KeyCode::Char('Q')), RolUsuario::Root);
    s.handle_key(k(KeyCode::Char('N')), RolUsuario::Root);
    assert_eq!(s.confirmacion, Some(ConfirmacionMenu::Salir));
    s.handle_key(k(KeyCode::Esc), RolUsuario::Root);
    assert_eq!(s.confirmacion, None);
    s.handle_key(k(KeyCode::Char('Q')), RolUsuario::Root);
    assert_eq!(
        s.handle_key(k(KeyCode::Char('Y')), RolUsuario::Root),
        AccionMenu::Ninguna
    );
    assert_eq!(
        s.handle_key(k(KeyCode::Enter), RolUsuario::Root),
        AccionMenu::Salir
    );
}

#[test]
fn un_operador_no_ve_ni_puede_abrir_configuracion() {
    let visibles = OpcionMenu::visibles_para(RolUsuario::Operador);
    assert!(!visibles.contains(&OpcionMenu::Configuracion));

    let mut s = MenuPrincipalState::default();
    assert_eq!(
        s.handle_key(k(KeyCode::Char('7')), RolUsuario::Operador),
        AccionMenu::Ninguna
    );
}

#[test]
fn un_administrador_si_ve_y_puede_abrir_configuracion() {
    let visibles = OpcionMenu::visibles_para(RolUsuario::Administrador);
    assert!(visibles.contains(&OpcionMenu::Configuracion));

    let mut s = MenuPrincipalState::default();
    assert_eq!(
        s.handle_key(k(KeyCode::Char('7')), RolUsuario::Administrador),
        AccionMenu::Abrir(OpcionMenu::Configuracion)
    );
}
