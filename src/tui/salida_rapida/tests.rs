use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use super::*;
use crate::{
    domain::resultado_acceso::ResultadoAcceso,
    models::{medio_ingreso::MedioIngreso, tipo_ingreso::TipoIngreso},
};

fn k(c: KeyCode) -> KeyEvent {
    KeyEvent::new(c, KeyModifiers::NONE)
}

fn pagina(items: Vec<IngresoActivoResumen>) -> ListaIngresosActivosResumen {
    ListaIngresosActivosResumen {
        total: items.len(),
        items,
    }
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
        resultado_registrado:
            crate::models::registro_ingreso::ResultadoIngresoRegistrado::Permitido,
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
    assert_eq!(s.abrir(), AccionSalidaRapida::Buscar { texto: None });
    assert!(s.abierto());
}

#[test]
fn escribir_filtra_por_gafete_o_nombre_en_un_solo_campo_tras_el_debounce() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    let futuro = || Instant::now() + DURACION_DEBOUNCE + Duration::from_millis(1);

    assert_eq!(
        s.handle_key(k(KeyCode::Char('2'))),
        AccionSalidaRapida::Ninguna
    );
    assert_eq!(
        s.handle_key(k(KeyCode::Char('5'))),
        AccionSalidaRapida::Ninguna
    );
    assert_eq!(
        s.tick(futuro()),
        AccionSalidaRapida::Buscar {
            texto: Some("25".into())
        }
    );

    s.handle_key(k(KeyCode::Backspace));
    s.handle_key(k(KeyCode::Backspace));
    assert_eq!(s.tick(futuro()), AccionSalidaRapida::Buscar { texto: None });
}

/// Enter ya no registra la salida directo: primero pasa a
/// `Estado::ConfirmarSalida` (sin emitir ninguna acción); recién un segundo
/// Enter sobre esa pantalla de confirmación emite `Confirmar`. Antes un solo
/// Enter alcanzaba para registrar la salida sin mostrar a quién, lo que
/// volvía peligroso un Enter en blanco justo tras abrir el overlay (que
/// carga a todos los que están dentro con la fila 0 ya seleccionada).
#[test]
fn enter_pide_confirmar_antes_de_registrar_la_salida() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(pagina(vec![
        r(7, "José Peña", Some(12)),
        r(9, "Ana Solís", None),
    ])));

    assert_eq!(s.handle_key(k(KeyCode::Enter)), AccionSalidaRapida::Ninguna);
    assert_eq!(s.estado, Estado::ConfirmarSalida { registro_id: 7 });

    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionSalidaRapida::Confirmar {
            registro_id: 7,
            nombre: "José Peña".into(),
        }
    );
}

#[test]
fn enter_sobre_la_segunda_fila_confirma_el_registro_correcto() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(pagina(vec![
        r(7, "José Peña", Some(12)),
        r(9, "Ana Solís", None),
    ])));

    s.handle_key(k(KeyCode::Down));
    s.handle_key(k(KeyCode::Enter));
    assert_eq!(s.estado, Estado::ConfirmarSalida { registro_id: 9 });
    assert_eq!(
        s.handle_key(k(KeyCode::Enter)),
        AccionSalidaRapida::Confirmar {
            registro_id: 9,
            nombre: "Ana Solís".into(),
        }
    );
}

#[test]
fn esc_en_la_confirmacion_cancela_y_conserva_la_lista() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(pagina(vec![r(7, "José Peña", Some(12))])));
    s.handle_key(k(KeyCode::Enter));
    assert_eq!(s.estado, Estado::ConfirmarSalida { registro_id: 7 });

    assert_eq!(s.handle_key(k(KeyCode::Esc)), AccionSalidaRapida::Ninguna);
    assert_eq!(s.estado, Estado::Abierto);
    assert_eq!(s.registros.len(), 1);
}

#[test]
fn enter_sin_resultados_no_hace_nada() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(pagina(vec![])));
    assert_eq!(s.handle_key(k(KeyCode::Enter)), AccionSalidaRapida::Ninguna);
}

/// Regresión de "un solo Esc cierra todo el overlay y descarta lo escrito"
/// (`docs/hallazgos-buscador.md`): con filtro escrito, Esc sólo limpia,
/// igual que el resto de pantallas de búsqueda; recién con el filtro ya
/// vacío un segundo Esc cierra el overlay.
#[test]
fn esc_con_filtro_escrito_solo_limpia_sin_cerrar() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.handle_key(k(KeyCode::Char('2')));

    assert_eq!(
        s.handle_key(k(KeyCode::Esc)),
        AccionSalidaRapida::Buscar { texto: None }
    );
    assert!(s.abierto());
    assert_eq!(s.busqueda.value(), "");

    assert_eq!(s.handle_key(k(KeyCode::Esc)), AccionSalidaRapida::Ninguna);
    assert!(!s.abierto());
}

#[test]
fn esc_cierra_sin_confirmar_nada() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(pagina(vec![r(7, "José Peña", Some(12))])));
    s.handle_key(k(KeyCode::Esc));
    assert!(!s.abierto());
}

/// Regresión de "tras sacar a alguien, el overlay queda abierto pero no
/// muestra a los demás contratistas": la confirmación exitosa debe volver a
/// `Abierto` (no un estado aparte que sólo muestra el mensaje), limpiar la
/// búsqueda y devolver una acción `Buscar` para recargar la lista completa
/// de quienes siguen dentro — sin eso, el operador tenía que cerrar el
/// overlay con Esc y reabrirlo con F2 para ver al resto.
#[test]
fn confirmar_con_exito_vuelve_a_la_lista_completa_y_conserva_el_mensaje() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(pagina(vec![r(7, "José Peña", Some(12))])));
    s.handle_key(k(KeyCode::Char('1')));
    s.handle_key(k(KeyCode::Enter));

    let accion = s.completar_confirmacion(Ok("✓ Salida registrada — José Peña".into()));

    assert_eq!(accion, AccionSalidaRapida::Buscar { texto: None });
    assert!(s.abierto());
    assert_eq!(s.estado, Estado::Abierto);
    assert_eq!(
        s.mensaje.as_deref(),
        Some("✓ Salida registrada — José Peña")
    );
    assert_eq!(s.busqueda.value(), "");

    // El mensaje sobrevive al refresco de la lista que sigue a la
    // confirmación, igual que en Activos — así el operador ve la
    // confirmación Y a los demás contratistas al mismo tiempo.
    s.completar_busqueda(Ok(pagina(vec![
        r(9, "Ana Solís", None),
        r(11, "Beto Rojas", Some(4)),
    ])));
    assert_eq!(s.registros.len(), 2);
    assert_eq!(
        s.mensaje.as_deref(),
        Some("✓ Salida registrada — José Peña")
    );
}

#[test]
fn escribir_de_nuevo_tras_confirmar_limpia_el_mensaje() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(pagina(vec![r(7, "José Peña", Some(12))])));
    s.handle_key(k(KeyCode::Enter));
    s.completar_confirmacion(Ok("✓ Salida registrada — José Peña".into()));
    assert!(s.mensaje.is_some());

    s.handle_key(k(KeyCode::Char('a')));
    assert_eq!(s.mensaje, None);
}

#[test]
fn tras_confirmar_esc_cierra_el_overlay_como_de_costumbre() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(pagina(vec![r(7, "José Peña", Some(12))])));
    s.handle_key(k(KeyCode::Enter));
    s.completar_confirmacion(Ok("✓ Salida registrada — José Peña".into()));

    s.handle_key(k(KeyCode::Esc));
    assert!(!s.abierto());
}

#[test]
fn error_de_confirmacion_deja_reintentar_sin_salir_de_la_confirmacion() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(pagina(vec![r(7, "José Peña", Some(12))])));
    s.handle_key(k(KeyCode::Enter));

    let accion = s.completar_confirmacion(Err("El ingreso ya no está activo".into()));

    assert_eq!(accion, AccionSalidaRapida::Ninguna);
    assert!(s.abierto());
    assert_eq!(s.estado, Estado::ConfirmarSalida { registro_id: 7 });
    assert_eq!(s.error.as_deref(), Some("El ingreso ya no está activo"));

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
    s.completar_busqueda(Ok(pagina(vec![r(7, "José Peña", Some(12))])));
    terminal
        .draw(|frame| render::render(frame, frame.area(), &s, theme))
        .expect("debe renderizar");
    let abierto = texto(terminal.backend());
    assert!(abierto.contains("SALIDA RÁPIDA"));
    assert!(abierto.contains("José Peña"));
    assert!(abierto.contains("ENTER seleccionar"));

    s.handle_key(k(KeyCode::Enter));
    terminal
        .draw(|frame| render::render(frame, frame.area(), &s, theme))
        .expect("debe renderizar");
    let confirmar = texto(terminal.backend());
    assert!(confirmar.contains("¿Confirmar salida de José Peña?"));
    assert!(confirmar.contains("ENTER confirma"));

    s.completar_confirmacion(Ok("✓ Salida registrada — José Peña".into()));
    s.completar_busqueda(Ok(pagina(vec![])));
    terminal
        .draw(|frame| render::render(frame, frame.area(), &s, theme))
        .expect("debe renderizar");
    let confirmado = texto(terminal.backend());
    assert!(confirmado.contains("✓ Salida registrada — José Peña"));
    assert!(confirmado.contains("Sin ingresos activos con esa búsqueda"));
}

#[test]
fn abrir_de_nuevo_reinicia_busqueda_seleccion_y_mensaje_previos() {
    let mut s = SalidaRapidaState::default();
    s.abrir();
    s.completar_busqueda(Ok(pagina(vec![r(7, "José Peña", Some(12))])));
    s.handle_key(k(KeyCode::Enter));
    s.completar_confirmacion(Ok("✓ Salida registrada — José Peña".into()));
    s.handle_key(k(KeyCode::Esc));

    s.abrir();
    assert_eq!(s.busqueda.value(), "");
    assert!(s.registros.is_empty());
    assert_eq!(s.seleccion, None);
    assert_eq!(s.mensaje, None);
}
