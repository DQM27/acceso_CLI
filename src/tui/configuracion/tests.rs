use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

fn tecla(codigo: KeyCode) -> KeyEvent {
    KeyEvent::new(codigo, KeyModifiers::NONE)
}

fn sesion_prueba() -> crate::services::autenticacion_service::UsuarioSesion {
    crate::services::autenticacion_service::UsuarioSesion {
        id: 1,
        cedula: "1-1111-1111".into(),
        nombre: "Ana Quintana".into(),
        rol: crate::models::usuario::RolUsuario::Root,
    }
}

#[test]
fn reiniciar_respaldos_solicita_la_carga_inicial() {
    let mut estado = ConfiguracionState::default();

    let accion = estado.reiniciar();

    assert_eq!(accion, AccionAjustes::Respaldos(AccionRespaldos::Cargar));
}

/// A diferencia de `mensaje` (transitorio), el aviso de que el respaldo
/// automático falló debe seguir visible después de entrar y salir de la
/// pantalla — `reiniciar()` no debe borrarlo, sólo un intento nuevo lo
/// reemplaza (con éxito o con otro fallo).
#[test]
fn reiniciar_conserva_el_fallo_del_respaldo_automatico() {
    let mut estado = ConfiguracionState::default();
    estado.actualizar_fallo_respaldo_automatico(Some("Error de archivo: disco lleno".into()));

    estado.reiniciar();

    assert_eq!(
        estado.respaldos.fallo_automatico.as_deref(),
        Some("Error de archivo: disco lleno")
    );
}

#[test]
fn un_intento_nuevo_reemplaza_el_fallo_del_respaldo_automatico() {
    let mut estado = ConfiguracionState::default();
    estado.actualizar_fallo_respaldo_automatico(Some("Error de archivo: disco lleno".into()));

    estado.actualizar_fallo_respaldo_automatico(None);

    assert_eq!(estado.respaldos.fallo_automatico, None);
}

#[test]
fn esc_en_respaldos_vuelve_directamente_al_menu_principal() {
    let mut estado = ConfiguracionState::default();

    assert_eq!(
        estado.handle_key(tecla(KeyCode::Esc)),
        AccionAjustes::Volver
    );
}

/// `A` ya significa "Activar/Desactivar" en Empresas y Usuarios — Respaldos
/// usa `L` (Listar) para recargar en vez de reutilizar la misma letra con
/// un significado distinto entre pantallas.
#[test]
fn l_recarga_el_listado_de_respaldos() {
    let mut estado = ConfiguracionState::default();

    assert_eq!(
        estado.handle_key(tecla(KeyCode::Char('l'))),
        AccionAjustes::Respaldos(AccionRespaldos::Cargar)
    );
    assert_eq!(
        estado.handle_key(tecla(KeyCode::Char('a'))),
        AccionAjustes::Respaldos(AccionRespaldos::Ninguna)
    );
}

#[test]
fn crear_y_revalidar_disparan_las_acciones_correctas_solo_con_seleccion() {
    let mut estado = ConfiguracionState::default();

    assert_eq!(
        estado.handle_key(tecla(KeyCode::Char('c'))),
        AccionAjustes::Respaldos(AccionRespaldos::Crear)
    );
    // Sin filas cargadas todavía, revalidar no tiene nada que seleccionar.
    assert_eq!(
        estado.handle_key(tecla(KeyCode::Char('v'))),
        AccionAjustes::Respaldos(AccionRespaldos::Ninguna)
    );
}

#[test]
fn marcar_creando_respaldo_bloquea_c_hasta_que_completar_creacion_lo_libera() {
    let mut estado = ConfiguracionState::default();
    estado.marcar_creando_respaldo();
    assert!(estado.creando_respaldo());

    assert_eq!(
        estado.handle_key(tecla(KeyCode::Char('c'))),
        AccionAjustes::Respaldos(AccionRespaldos::Ninguna),
        "no debía disparar un segundo respaldo mientras el primero sigue en vuelo"
    );

    estado.completar_creacion(Err("fallo".into()));
    assert!(!estado.creando_respaldo());
    assert_eq!(
        estado.handle_key(tecla(KeyCode::Char('c'))),
        AccionAjustes::Respaldos(AccionRespaldos::Crear)
    );
}

#[test]
fn completar_listado_puebla_la_tabla_y_permite_revalidar_la_fila_seleccionada() {
    use crate::database::backup::{RespaldoResumen, TipoRespaldo};

    let mut estado = ConfiguracionState::default();
    let resumen = RespaldoResumen {
        ruta: std::path::PathBuf::from("control_acceso_2026-08-18_120000_manual.db"),
        creado_en: chrono::Utc::now(),
        tipo: TipoRespaldo::Manual,
        tamano_bytes: 1024,
    };
    estado.completar_listado(Ok(vec![resumen.clone()]));

    let accion = estado.handle_key(tecla(KeyCode::Char('v')));

    assert_eq!(
        accion,
        AccionAjustes::Respaldos(AccionRespaldos::Revalidar { ruta: resumen.ruta })
    );
}

#[test]
fn restaurar_exige_confirmacion_y_esc_cancela_sin_disparar_nada() {
    use crate::database::backup::{RespaldoResumen, TipoRespaldo};

    let mut estado = ConfiguracionState::default();
    estado.completar_listado(Ok(vec![RespaldoResumen {
        ruta: std::path::PathBuf::from("respaldo.db"),
        creado_en: chrono::Utc::now(),
        tipo: TipoRespaldo::Manual,
        tamano_bytes: 10,
    }]));

    assert_eq!(
        estado.handle_key(tecla(KeyCode::Char('r'))),
        AccionAjustes::Respaldos(AccionRespaldos::Ninguna)
    );
    assert!(matches!(
        estado.respaldos.modo,
        ModoRespaldos::ConfirmandoRestauracion { .. }
    ));

    assert_eq!(
        estado.handle_key(tecla(KeyCode::Esc)),
        AccionAjustes::Respaldos(AccionRespaldos::Ninguna)
    );
    assert_eq!(estado.respaldos.modo, ModoRespaldos::Normal);
}

#[test]
fn restaurar_confirmado_con_enter_emite_la_accion_con_la_ruta_correcta() {
    use crate::database::backup::{RespaldoResumen, TipoRespaldo};

    let mut estado = ConfiguracionState::default();
    let resumen = RespaldoResumen {
        ruta: std::path::PathBuf::from("respaldo.db"),
        creado_en: chrono::Utc::now(),
        tipo: TipoRespaldo::Manual,
        tamano_bytes: 10,
    };
    estado.completar_listado(Ok(vec![resumen.clone()]));
    estado.handle_key(tecla(KeyCode::Char('r')));

    let accion = estado.handle_key(tecla(KeyCode::Enter));

    assert_eq!(
        accion,
        AccionAjustes::Respaldos(AccionRespaldos::Restaurar { ruta: resumen.ruta })
    );
}

#[test]
fn exportar_sin_texto_no_dispara_nada_y_esc_cancela() {
    use crate::database::backup::{RespaldoResumen, TipoRespaldo};

    let mut estado = ConfiguracionState::default();
    estado.completar_listado(Ok(vec![RespaldoResumen {
        ruta: std::path::PathBuf::from("respaldo.db"),
        creado_en: chrono::Utc::now(),
        tipo: TipoRespaldo::Manual,
        tamano_bytes: 10,
    }]));
    estado.handle_key(tecla(KeyCode::Char('e')));

    assert_eq!(
        estado.handle_key(tecla(KeyCode::Enter)),
        AccionAjustes::Respaldos(AccionRespaldos::Ninguna)
    );

    for c in "C:\\destino\\respaldo.db".chars() {
        estado.handle_key(tecla(KeyCode::Char(c)));
    }
    let accion = estado.handle_key(tecla(KeyCode::Enter));
    assert!(matches!(
        accion,
        AccionAjustes::Respaldos(AccionRespaldos::Exportar { .. })
    ));
}

/// El detalle técnico (a diferencia del aviso genérico del Menú Principal)
/// sólo lo ve quien ya está en Respaldos — alcanzable únicamente por Root.
#[test]
fn la_pantalla_respaldos_muestra_el_motivo_del_fallo_automatico() {
    use ratatui::{Terminal, backend::TestBackend};

    let mut estado = ConfiguracionState::default();
    estado.actualizar_fallo_respaldo_automatico(Some("Error de archivo: disco lleno".into()));

    let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
    terminal
        .draw(|frame| {
            render::render(
                frame,
                frame.area(),
                &estado,
                &sesion_prueba(),
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
    assert!(texto.contains("disco lleno"), "{texto}");
}
