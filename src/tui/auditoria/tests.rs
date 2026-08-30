use super::*;
use crate::database::queries::auditoria::EntidadAuditada;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn tecla(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn reinicia_carga_y_escape_vuelve() {
    let mut state = AuditoriaState::default();
    assert_eq!(state.reiniciar(), AccionAuditoria::Cargar { offset: 0 });
    assert_eq!(
        state.handle_key(tecla(KeyCode::Esc)),
        AccionAuditoria::Volver
    );
}

#[test]
fn pagina_siguiente_respeta_el_total_real() {
    let mut state = AuditoriaState {
        items: Vec::with_capacity(LIMITE_AUDITORIA_PREDETERMINADO),
        total: 51,
        ..Default::default()
    };
    state
        .items
        .resize_with(LIMITE_AUDITORIA_PREDETERMINADO, || CambioAuditado {
            id: 1,
            fecha_hora: chrono::Utc::now(),
            usuario_id: 1,
            usuario_nombre: "Root".into(),
            entidad: EntidadAuditada::Contratista,
            entidad_id: 1,
            entidad_nombre: "Persona".into(),
            campo: "tipo_ingreso".into(),
            valor_anterior: Some("SWAT".into()),
            valor_nuevo: Some("IN_HOUSE".into()),
        });
    assert_eq!(
        state.handle_key(tecla(KeyCode::PageDown)),
        AccionAuditoria::Cargar { offset: 50 }
    );
}

#[test]
fn render_usa_hora_de_costa_rica_y_el_orden_operativo_de_columnas() {
    use chrono::{TimeZone, Utc};
    use ratatui::{Terminal, backend::TestBackend};

    let mut state = AuditoriaState::default();
    state.completar(Ok(PaginaAuditoria {
        items: vec![CambioAuditado {
            id: 1,
            fecha_hora: Utc.with_ymd_and_hms(2026, 8, 20, 22, 13, 0).unwrap(),
            usuario_id: 1,
            usuario_nombre: "Daniel Quintana".into(),
            entidad: EntidadAuditada::Contratista,
            entidad_id: 1,
            entidad_nombre: "Omar Pasos Leon".into(),
            campo: "tiene_acceso".into(),
            valor_anterior: Some("HABILITADO".into()),
            valor_nuevo: Some("DESHABILITADO".into()),
        }],
        total: 1,
    }));
    let sesion = crate::services::autenticacion_service::UsuarioSesion {
        id: 1,
        cedula: "ROOT".into(),
        nombre: "Daniel Quintana".into(),
        rol: crate::models::usuario::RolUsuario::Root,
    };
    let mut terminal = Terminal::new(TestBackend::new(150, 24)).unwrap();
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

    assert!(texto.contains("FECHA Y HORA (CR)"));
    assert!(texto.contains("20/08/2026 16:13"));
    assert!(texto.contains("Omar Pasos Leon (contratista)"));
    assert!(texto.contains("Acceso: Habilitado → Deshabilitado"));
    let encabezado = texto
        .lines()
        .find(|linea| linea.contains("FECHA Y HORA (CR)"))
        .unwrap();
    let fecha = encabezado.find("FECHA Y HORA (CR)").unwrap();
    let entidad = encabezado.find("ENTIDAD").unwrap();
    let cambio = encabezado.find("CAMBIO REALIZADO").unwrap();
    let usuario = encabezado.find("MODIFICADO POR").unwrap();
    assert!(fecha < entidad && entidad < cambio && cambio < usuario);
}

#[test]
fn cambio_de_praind_usa_la_convencion_de_fecha_de_la_ui() {
    let cambio = CambioAuditado {
        id: 1,
        fecha_hora: chrono::Utc::now(),
        usuario_id: 1,
        usuario_nombre: "Root".into(),
        entidad: EntidadAuditada::Contratista,
        entidad_id: 1,
        entidad_nombre: "Persona".into(),
        campo: "fecha_vencimiento_praind".into(),
        valor_anterior: Some("2028-05-01".into()),
        valor_nuevo: Some("2026-05-01".into()),
    };

    assert_eq!(
        render::descripcion_cambio(&cambio),
        "Vencimiento PRAIND: 01/05/2028 → 01/05/2026"
    );
}

#[test]
fn cambio_de_cedula_usa_una_etiqueta_legible() {
    let cambio = CambioAuditado {
        id: 1,
        fecha_hora: chrono::Utc::now(),
        usuario_id: 1,
        usuario_nombre: "Administradora".into(),
        entidad: EntidadAuditada::Contratista,
        entidad_id: 1,
        entidad_nombre: "Persona".into(),
        campo: "cedula".into(),
        valor_anterior: Some("1-1111-1111".into()),
        valor_nuevo: Some("1-2222-2222".into()),
    };

    assert_eq!(
        render::descripcion_cambio(&cambio),
        "Cédula: 1-1111-1111 → 1-2222-2222"
    );
}

#[test]
fn cambio_de_password_no_muestra_valores() {
    let cambio = CambioAuditado {
        id: 1,
        fecha_hora: chrono::Utc::now(),
        usuario_id: 1,
        usuario_nombre: "Root".into(),
        entidad: EntidadAuditada::Usuario,
        entidad_id: 2,
        entidad_nombre: "Operador".into(),
        campo: "password".into(),
        valor_anterior: None,
        valor_nuevo: None,
    };

    assert_eq!(
        render::descripcion_cambio(&cambio),
        "Contraseña actualizada"
    );
}
