use rusqlite::Connection;

use control_acceso::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::usuario::{RolUsuario, Usuario};
use control_acceso::services::autenticacion_service::AutenticacionService;
use control_acceso::services::error::AutenticacionError;
use control_acceso::services::password::generar_hash;

fn base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
}

fn guardar(connection: &Connection, activo: bool, hash: String) -> i64 {
    SqliteUsuarioRepository::new(connection)
        .crear(&Usuario {
            id: 0,
            cedula: "1001".to_string(),
            nombre: "Usuario Uno".to_string(),
            password_hash: hash,
            rol: RolUsuario::Administrador,
            activo,
        })
        .unwrap()
}

#[test]
fn credenciales_correctas_devuelven_usuario_y_rol() {
    let connection = base();
    let id = guardar(&connection, true, generar_hash("password1").unwrap());
    let repository = SqliteUsuarioRepository::new(&connection);
    let usuario = AutenticacionService::new(&repository)
        .autenticar("  1001  ", "password1")
        .unwrap();
    assert_eq!(usuario.id, id);
    assert_eq!(usuario.cedula, "1001");
    assert_eq!(usuario.rol, RolUsuario::Administrador);
}

#[test]
fn password_incorrecto_y_cedula_inexistente_no_revelan_diferencias() {
    let connection = base();
    guardar(&connection, true, generar_hash("password1").unwrap());
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = AutenticacionService::new(&repository);
    assert!(matches!(
        servicio.autenticar("1001", "incorrecta"),
        Err(AutenticacionError::CredencialesInvalidas)
    ));
    assert!(matches!(
        servicio.autenticar("9999", "password1"),
        Err(AutenticacionError::CredencialesInvalidas)
    ));
}

#[test]
fn usuario_inactivo_es_rechazado() {
    let connection = base();
    guardar(&connection, false, generar_hash("password1").unwrap());
    let repository = SqliteUsuarioRepository::new(&connection);
    assert!(matches!(
        AutenticacionService::new(&repository).autenticar("1001", "password1"),
        Err(AutenticacionError::UsuarioInactivo)
    ));
}

#[test]
fn hash_corrupto_es_error_tecnico_sin_panico() {
    let connection = base();
    guardar(&connection, true, "corrupto".to_string());
    let repository = SqliteUsuarioRepository::new(&connection);
    assert!(matches!(
        AutenticacionService::new(&repository).autenticar("1001", "password1"),
        Err(AutenticacionError::HashInvalido)
    ));
}
