use super::*;
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
        .resize_with(LIMITE_AUDITORIA_PREDETERMINADO, || {
            CambioContratistaAuditado {
                id: 1,
                fecha_hora: chrono::Utc::now(),
                usuario_id: 1,
                usuario_nombre: "Root".into(),
                contratista_id: 1,
                contratista_nombre: "Persona".into(),
                campo: "tipo_ingreso".into(),
                valor_anterior: Some("SWAT".into()),
                valor_nuevo: Some("IN_HOUSE".into()),
            }
        });
    assert_eq!(
        state.handle_key(tecla(KeyCode::PageDown)),
        AccionAuditoria::Cargar { offset: 50 }
    );
}
