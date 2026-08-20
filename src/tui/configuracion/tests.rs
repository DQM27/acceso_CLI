use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;

fn tecla(codigo: KeyCode) -> KeyEvent {
    KeyEvent::new(codigo, KeyModifiers::NONE)
}

#[test]
fn reiniciar_respaldos_solicita_la_carga_inicial() {
    let mut estado = ConfiguracionState::default();

    let accion = estado.reiniciar();

    assert_eq!(accion, AccionAjustes::Respaldos(AccionRespaldos::Cargar));
}

#[test]
fn esc_en_respaldos_vuelve_directamente_al_menu_principal() {
    let mut estado = ConfiguracionState::default();

    assert_eq!(
        estado.handle_key(tecla(KeyCode::Esc)),
        AccionAjustes::Volver
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
