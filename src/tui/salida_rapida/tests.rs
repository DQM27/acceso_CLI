use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::{
    domain::resultado_acceso::ResultadoAcceso,
    models::{medio_ingreso::MedioIngreso, tipo_ingreso::TipoIngreso},
};

fn k(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}

fn r(id: i64, nombre: &str, gafete: Option<i64>) -> IngresoActivoResumen {
    IngresoActivoResumen {
        registro_id: id,
        contratista_id: id,
        cedula: format!("C-{id}"),
        contratista_nombre: nombre.into(),
        empresa_nombre: "Empresa".into(),
        tipo_ingreso: TipoIngreso::Praind,
        medio_ingreso: MedioIngreso::Caminando,
        fecha_hora_ingreso: crate::tiempo::local_costa_rica_a_utc(
            NaiveDate::from_ymd_opt(2026, 8, 12)
                .unwrap()
                .and_hms_opt(8, 0, 0)
                .unwrap(),
        )
        .unwrap(),
        gafete_numero: gafete,
        usuario_ingreso_nombre: "Ana".into(),
        resultado_acceso: ResultadoAcceso::Permitido,
    }
}

#[test]
fn cerrado_ignora_teclas() {
    let mut s = SalidaRapidaState::default();
    assert!(!s.abierto());
    assert_eq!(s.handle_key(k(KeyCode::Enter)), AccionSalidaRapida::Ninguna);
}

#[test]
fn abrir_dispara_una_busqueda_sin_texto() {
    let mut s = SalidaRapidaState::default();
    assert_eq!(
        s.abrir(),
        AccionSalidaRapida::Buscar { texto: None }
    );
    assert!(s.abierto());
}

#[test]
fn escribir_filtra_por_gafete_o_nombre_en_un_solo_campo() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    assert_eq!(
        s.handle_key(k(KeyCode::Char('2'))),
        AccionSalidaRapida::Buscar {
            texto: Some("2".into())
        }
    );
    assert_eq!(
        s.handle_key(k(KeyCode::Char('5'))),
        AccionSalidaRapida::Buscar {
            texto: Some("25".into())
        }
    );
    s.handle_key(k(KeyCode::Backspace));
    assert_eq!(
        s.handle_key(k(KeyCode::Backspace)),
        AccionSalidaRapida::Buscar { texto: None }
    );
}

#[test]
fn enter_sobre_el_resaltado_pide_confirmar_con_su_id_y_nombre() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(vec![r(7, "José Peña", Some(12)), r(9, "Ana Solís", None)]));

    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionSalidaRapida::Confirmar {
            registro_id: 7,
            nombre: "José Peña".into(),
        }
    );

    s.handle_key(k(KeyCode::Down));
    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionSalidaRapida::Confirmar {
            registro_id: 9,
            nombre: "Ana Solís".into(),
        }
    );
}

#[test]
fn enter_sin_resultados_no_hace_nada() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(vec![]));
    assert_eq!(s.handle_key(k(KeyCode::Enter)), AccionSalidaRapida::Ninguna);
}

#[test]
fn esc_cierra_sin_confirmar_nada() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(vec![r(7, "José Peña", Some(12))]));
    s.handle_key(k(KeyCode::Esc));
    assert!(!s.abierto());
}

#[test]
fn confirmacion_exitosa_muestra_mensaje_y_cualquier_tecla_cierra() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_confirmacion(Ok("✓ Salida registrada — José Peña".into()));
    assert!(s.abierto());

    s.handle_key(k(KeyCode::Char('x')));
    assert!(!s.abierto());
}

#[test]
fn error_de_confirmacion_deja_reintentar_sin_cerrar() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(vec![r(7, "José Peña", Some(12))]));
    s.completar_confirmacion(Err("El ingreso ya no está activo".into()));

    assert!(s.abierto());
    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionSalidaRapida::Confirmar {
            registro_id: 7,
            nombre: "José Peña".into(),
        }
    );
}

#[test]
fn renderiza_resultados_y_luego_el_mensaje_de_confirmacion() {
    use ratatui::{Terminal, backend::TestBackend};

    use crate::tui::ui_kit::ThemePreset;

    fn texto(backend: &TestBackend) -> String {
        backend
            .buffer()
            .content
            .iter()
            .map(|celda| celda.symbol())
            .collect()
    }

    let mut s = SalidaRapidaState::default();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("backend de prueba");
    let theme = ThemePreset::Brisas.theme();

    // Cerrado: no dibuja nada encima.
    terminal
        .draw(|frame| render::render(frame, frame.area(), &s, theme))
        .expect("debe renderizar");
    assert!(!texto(terminal.backend()).contains("SALIDA RÁPIDA"));

    s.abrir();
    s.completar_busqueda(Ok(vec![r(7, "José Peña", Some(12))]));
    terminal
        .draw(|frame| render::render(frame, frame.area(), &s, theme))
        .expect("debe renderizar");
    let abierto = texto(terminal.backend());
    assert!(abierto.contains("SALIDA RÁPIDA"));
    assert!(abierto.contains("José Peña"));
    assert!(abierto.contains("ENTER confirmar salida"));

    s.completar_confirmacion(Ok("✓ Salida registrada — José Peña".into()));
    terminal
        .draw(|frame| render::render(frame, frame.area(), &s, theme))
        .expect("debe renderizar");
    let confirmado = texto(terminal.backend());
    assert!(confirmado.contains("✓ Salida registrada — José Peña"));
}

#[test]
fn abrir_de_nuevo_reinicia_busqueda_y_seleccion_previas() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(vec![r(7, "José Peña", Some(12))]));
    s.handle_key(k(KeyCode::Char('1')));
    s.handle_key(k(KeyCode::Esc));

    s.abrir();
    assert_eq!(s.busqueda, "");
    assert!(s.registros.is_empty());
    assert_eq!(s.seleccion, None);
}
