//! `UsuarioService` en un equipo: el arranque inicial (ROOT) y las
//! búsquedas. El alta y la edición de usuarios se hacen sólo desde el panel
//! web, y la contraseña se cambia sólo en escritorio, en Supabase Auth.

use rusqlite::Connection;

use control_acceso::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::error::UsuarioServiceError;
use control_acceso::services::password::generar_hash;
use control_acceso::services::usuario_service::{CrearRootInicialInput, UsuarioService};

fn base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
}

fn root_inicial(cedula: &str, nombre: &str, password: &str) -> CrearRootInicialInput {
    CrearRootInicialInput {
        cedula: cedula.to_string(),
        nombre: nombre.to_string(),
        password: password.to_string(),
    }
}

fn inicializar(connection: &Connection) -> i64 {
    let repository = SqliteUsuarioRepository::new(connection);
    UsuarioService::new(&repository)
        .crear_root_inicial(root_inicial("ROOT1", "Usuario Root", "password1"))
        .unwrap()
}

#[test]
fn root_inicial_normalizado_y_nunca_guarda_password_plano() {
    let connection = base();
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let id = servicio
        .crear_root_inicial(root_inicial("  2001  ", "  Persona Uno  ", "password1"))
        .unwrap();
    let usuario = servicio.buscar_por_id(id).unwrap();
    assert_eq!(usuario.cedula, "2001");
    assert_eq!(usuario.nombre, "Persona Uno");
    assert_ne!(usuario.password_hash, "password1");
    assert_eq!(usuario.rol, RolUsuario::Root);
    assert!(usuario.activo);
}

#[test]
fn root_inicial_valida_campos_obligatorios_y_password_corto() {
    let connection = base();
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    assert!(matches!(
        servicio.validar_datos_para_root_inicial(&root_inicial(" ", "Nombre", "password1")),
        Err(UsuarioServiceError::CedulaVacia)
    ));
    assert!(matches!(
        servicio.validar_datos_para_root_inicial(&root_inicial("2001", " ", "password1")),
        Err(UsuarioServiceError::NombreVacio)
    ));
    assert!(matches!(
        servicio.validar_datos_para_root_inicial(&root_inicial("2001", "Nombre", "corta")),
        Err(UsuarioServiceError::PasswordDemasiadoCorto)
    ));
    assert!(matches!(
        servicio.buscar_por_cedula("2001"),
        Err(UsuarioServiceError::UsuarioNoEncontrado)
    ));
}

#[test]
fn root_inicial_con_hash_guarda_el_hash_tal_cual_y_rechaza_formato_invalido() {
    let connection = base();
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);

    assert!(matches!(
        servicio.crear_root_inicial_con_hash(
            root_inicial("3003", "Persona", "password1"),
            "no-es-un-hash-argon2".to_string(),
        ),
        Err(UsuarioServiceError::Password(_))
    ));

    let hash = generar_hash("password-ya-calculado").unwrap();
    let id = servicio
        .crear_root_inicial_con_hash(
            root_inicial("3002", "Persona Hash", "password1"),
            hash.clone(),
        )
        .unwrap();
    assert_eq!(servicio.buscar_por_id(id).unwrap().password_hash, hash);
}

#[test]
fn busca_por_id_y_cedula_normalizada_y_reporta_inexistentes() {
    let connection = base();
    let id = inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    assert_eq!(servicio.buscar_por_id(id).unwrap().id, id);
    assert_eq!(servicio.buscar_por_cedula(" ROOT1 ").unwrap().id, id);
    assert!(matches!(
        servicio.buscar_por_id(999),
        Err(UsuarioServiceError::UsuarioNoEncontrado)
    ));
}

/// Regresión del hallazgo #4 de `docs/auditoria-dominio-2026-08-20.md`: una
/// escritura de los datos del usuario hecha con una copia leída ANTES de un
/// cambio de contraseña no debe revertir esa contraseña (el repositorio no
/// reescribe `password_hash` al actualizar).
#[test]
fn actualizar_datos_no_revierte_una_contrasena_cambiada_concurrentemente() {
    let connection = base();
    let id = inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);

    let mut copia_vieja = servicio.buscar_por_id(id).unwrap();
    copia_vieja.nombre = "Nombre Nuevo".to_string();

    let hash_nuevo = generar_hash("clave-cambiada-concurrentemente").unwrap();
    repository.actualizar_password(id, &hash_nuevo).unwrap();
    repository.actualizar(&copia_vieja).unwrap();

    let final_usuario = servicio.buscar_por_id(id).unwrap();
    assert_eq!(final_usuario.password_hash, hash_nuevo);
    assert_eq!(final_usuario.nombre, "Nombre Nuevo");
}
