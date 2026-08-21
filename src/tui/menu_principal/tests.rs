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
        seleccion: OpcionMenu::CambiarPassword,
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
    // Se usa la fila de "Cambiar mi contraseña" porque es la opción con la
    // etiqueta más larga — el ancho del bloque se ajusta a ella, así que
    // esa fila queda al ras de ambos bordes del bloque y sus márgenes son
    // directamente los márgenes del bloque completo.
    let fila = (0..buffer.area.height)
        .find(|&y| {
            let texto: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            texto.contains("Cambiar mi contraseña")
        })
        .expect("debe encontrar la fila con 'Cambiar mi contraseña'");

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
        ('7', OpcionMenu::Auditoria),
        ('8', OpcionMenu::Respaldos),
        ('9', OpcionMenu::CambiarPassword),
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
fn un_operador_no_ve_ni_puede_abrir_respaldos() {
    let visibles = OpcionMenu::visibles_para(RolUsuario::Operador);
    assert!(!visibles.contains(&OpcionMenu::Respaldos));

    let mut s = MenuPrincipalState::default();
    assert_eq!(
        s.handle_key(k(KeyCode::Char('8')), RolUsuario::Operador),
        AccionMenu::Ninguna
    );
}

#[test]
fn un_administrador_no_ve_ni_puede_abrir_respaldos() {
    let visibles = OpcionMenu::visibles_para(RolUsuario::Administrador);
    assert!(!visibles.contains(&OpcionMenu::Respaldos));

    let mut s = MenuPrincipalState::default();
    assert_eq!(
        s.handle_key(k(KeyCode::Char('8')), RolUsuario::Administrador),
        AccionMenu::Ninguna
    );
}

#[test]
fn todos_los_roles_pueden_abrir_cambio_de_password() {
    for rol in [
        RolUsuario::Root,
        RolUsuario::Administrador,
        RolUsuario::Operador,
    ] {
        assert!(OpcionMenu::visibles_para(rol).contains(&OpcionMenu::CambiarPassword));
        let mut state = MenuPrincipalState::default();
        assert_eq!(
            state.handle_key(k(KeyCode::Char('9')), rol),
            AccionMenu::Abrir(OpcionMenu::CambiarPassword)
        );
    }
}

#[test]
fn auditoria_es_visible_para_administrador_y_root_pero_no_operador() {
    for rol in [RolUsuario::Root, RolUsuario::Administrador] {
        assert!(OpcionMenu::visibles_para(rol).contains(&OpcionMenu::Auditoria));
        let mut state = MenuPrincipalState::default();
        assert_eq!(
            state.handle_key(k(KeyCode::Char('7')), rol),
            AccionMenu::Abrir(OpcionMenu::Auditoria)
        );
    }
    assert!(!OpcionMenu::visibles_para(RolUsuario::Operador).contains(&OpcionMenu::Auditoria));
}

#[test]
fn el_menu_root_muestra_todas_las_opciones_en_orden_sin_recortarlas() {
    use crate::services::autenticacion_service::UsuarioSesion;
    use ratatui::{Terminal, backend::TestBackend};

    let state = MenuPrincipalState::default();
    let sesion = UsuarioSesion {
        id: 1,
        cedula: "1".into(),
        nombre: "Root".into(),
        rol: RolUsuario::Root,
    };
    let mut terminal = Terminal::new(TestBackend::new(60, 22)).unwrap();
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
    let texto = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let opciones = [
        "1   Nuevo ingreso",
        "2   Ingresos activos",
        "3   Historial",
        "4   Contratistas",
        "5   Empresas",
        "6   Usuarios",
        "7   Auditoría",
        "8   Respaldos",
        "9   Cambiar mi contraseña",
        "L   Cerrar sesión",
        "Q   Salir",
    ];
    let mut posicion_anterior = 0;
    for opcion in opciones {
        let posicion = texto
            .find(opcion)
            .unwrap_or_else(|| panic!("falta {opcion}"));
        assert!(
            posicion >= posicion_anterior,
            "{opcion} aparece fuera de orden"
        );
        posicion_anterior = posicion;
    }
}

#[test]
fn un_operador_no_ve_ni_puede_abrir_usuarios() {
    let visibles = OpcionMenu::visibles_para(RolUsuario::Operador);
    assert!(!visibles.contains(&OpcionMenu::Usuarios));

    let mut s = MenuPrincipalState::default();
    assert_eq!(
        s.handle_key(k(KeyCode::Char('6')), RolUsuario::Operador),
        AccionMenu::Ninguna
    );
}

/// El aviso es genérico a propósito (sin el detalle técnico, que vive en
/// Respaldos) y visible para cualquier rol, incluido Operador, que ni
/// siquiera puede abrir la pantalla Respaldos — cualquiera puede ser quien
/// note el problema y avise al administrador.
#[test]
fn el_menu_avisa_si_el_respaldo_automatico_fallo_sin_importar_el_rol() {
    use crate::services::autenticacion_service::UsuarioSesion;
    use ratatui::{Terminal, backend::TestBackend};

    let state = MenuPrincipalState {
        fallo_respaldo_automatico: Some("Error de archivo: disco lleno".into()),
        ..Default::default()
    };
    let sesion = UsuarioSesion {
        id: 1,
        cedula: "1".into(),
        nombre: "Operador".into(),
        rol: RolUsuario::Operador,
    };
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
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
    let texto = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        texto.contains("Fallo en el sistema de respaldo de la base de datos"),
        "{texto}"
    );
    // El mensaje es a propósito genérico: el detalle técnico no debe
    // filtrarse a una pantalla que ve cualquier rol.
    assert!(!texto.contains("disco lleno"), "{texto}");
}

#[test]
fn un_administrador_si_ve_y_puede_abrir_usuarios() {
    let visibles = OpcionMenu::visibles_para(RolUsuario::Administrador);
    assert!(visibles.contains(&OpcionMenu::Usuarios));

    let mut s = MenuPrincipalState::default();
    assert_eq!(
        s.handle_key(k(KeyCode::Char('6')), RolUsuario::Administrador),
        AccionMenu::Abrir(OpcionMenu::Usuarios)
    );
}
