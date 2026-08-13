use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use control_acceso::application::AppCore;
use control_acceso::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::error::UsuarioServiceError;
use control_acceso::services::usuario_service::CrearRootInicialInput;
use control_acceso::tui::configuracion_inicial::{AccionConfiguracion, ConfiguracionInicialState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusqlite::Connection;

fn tecla(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn escribir(state: &mut ConfiguracionInicialState, texto: &str) {
    for caracter in texto.chars() {
        state.handle_key(tecla(KeyCode::Char(caracter)));
    }
}

fn completar_formulario(state: &mut ConfiguracionInicialState) {
    escribir(state, "1-1111-1111");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(state, "Daniel Quintana");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(state, "password1");
    state.handle_key(tecla(KeyCode::Tab));
    escribir(state, "password1");
    state.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
}

fn archivo_temporal(nombre: &str) -> PathBuf {
    let unico = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "control_acceso_config_{nombre}_{}_{unico}.sqlite",
        std::process::id()
    ))
}

#[test]
fn flujo_completo_crea_root_real_y_permite_login() {
    let ruta = archivo_temporal("flujo");
    let core = AppCore::abrir(&ruta).unwrap();
    assert!(core.requiere_configuracion_inicial().unwrap());
    let mut state = ConfiguracionInicialState::default();
    completar_formulario(&mut state);
    let solicitud = state.tomar_solicitud().unwrap();

    core.crear_root_inicial(CrearRootInicialInput {
        cedula: solicitud.cedula,
        nombre: solicitud.nombre,
        password: solicitud.password,
    })
    .unwrap();
    state.limpiar_secretos();

    assert!(!core.requiere_configuracion_inicial().unwrap());
    let sesion = core.autenticar("1-1111-1111", "password1").unwrap();
    assert_eq!(sesion.rol, RolUsuario::Root);
    assert!(matches!(
        core.crear_root_inicial(CrearRootInicialInput {
            cedula: "ROOT2".to_owned(),
            nombre: "Segundo Root".to_owned(),
            password: "password2".to_owned(),
        }),
        Err(UsuarioServiceError::ConfiguracionInicialYaRealizada)
    ));
    drop(core);

    let connection = Connection::open(&ruta).unwrap();
    let usuarios = SqliteUsuarioRepository::new(&connection).listar().unwrap();
    assert_eq!(usuarios.len(), 1);
    assert_eq!(usuarios[0].rol, RolUsuario::Root);
    assert!(usuarios[0].activo);
    assert_ne!(usuarios[0].password_hash, "password1");
    drop(connection);
    std::fs::remove_file(ruta).unwrap();
}

#[test]
fn escape_antes_de_guardar_no_modifica_base() {
    let ruta = archivo_temporal("cancelar");
    let core = AppCore::abrir(&ruta).unwrap();
    let mut state = ConfiguracionInicialState::default();
    escribir(&mut state, "contenido-sin-guardar");

    assert_eq!(
        state.handle_key(tecla(KeyCode::Esc)),
        AccionConfiguracion::Salir
    );
    assert!(core.requiere_configuracion_inicial().unwrap());
    drop(core);
    std::fs::remove_file(ruta).unwrap();
}
