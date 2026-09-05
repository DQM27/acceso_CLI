//! Extremo a extremo del alta de contraseña para un operador global en un
//! dispositivo NUEVO (sin correo de por medio, decisión explícita dado que
//! el proyecto no tiene SMTP propio configurado todavía -- ver
//! docs/plan-panel-administrativo-web.md, sección de usuarios/operadores).
//!
//! El escenario real: `recibir_usuarios` (sync desde Supabase, todavía sin
//! implementar) inserta localmente un usuario que existe globalmente pero
//! nunca inició sesión EN ESTE dispositivo -- se simula acá insertando la
//! fila directo por SQL con el hash centinela (`SIN_PASSWORD_LOCAL`), tal
//! cual haría esa función.

use rusqlite::Connection;

use control_acceso::database::repositories::usuario_repository::SqliteUsuarioRepository;
use control_acceso::database::schema::initialize_database;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::autenticacion_service::AutenticacionService;
use control_acceso::services::error::AutenticacionError;
use control_acceso::services::password::SIN_PASSWORD_LOCAL;
use control_acceso::services::usuario_service::{CrearRootInicialInput, UsuarioService};

fn base_con_root() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let repository = SqliteUsuarioRepository::new(&connection);
    UsuarioService::new(&repository)
        .crear_root_inicial(CrearRootInicialInput {
            cedula: "ROOT1".to_string(),
            nombre: "Usuario Root".to_string(),
            password: "password-root".to_string(),
        })
        .unwrap();
    connection
}

/// Inserta directo por SQL -- simula lo que hará `recibir_usuarios`, que
/// todavía no existe. Ningún camino público de `UsuarioService` deja
/// escribir el hash centinela a propósito (`crear` siempre calcula un hash
/// real vía `generar_hash`).
fn insertar_usuario_global_sin_password(connection: &Connection, cedula: &str) -> i64 {
    connection
        .execute(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
             VALUES (?1, 'Operador Global', ?2, 'OPERADOR', 1)",
            rusqlite::params![cedula, SIN_PASSWORD_LOCAL],
        )
        .unwrap();
    connection.last_insert_rowid()
}

#[test]
fn manda_a_fijar_password_en_vez_de_credenciales_invalidas() {
    let connection = base_con_root();
    insertar_usuario_global_sin_password(&connection, "9-0001");
    let repository = SqliteUsuarioRepository::new(&connection);
    let autenticacion = AutenticacionService::new(&repository);

    // No importa qué contraseña se pruebe -- todavía no hay ninguna que
    // verificar, así que ni siquiera se llega a intentar Argon2.
    let resultado = autenticacion.autenticar("9-0001", "cualquier-cosa");

    assert!(matches!(resultado, Err(AutenticacionError::SinPasswordLocal)));
}

#[test]
fn fija_password_y_puede_iniciar_sesion_de_ahi_en_adelante() {
    let connection = base_con_root();
    let id = insertar_usuario_global_sin_password(&connection, "9-0002");
    let repository = SqliteUsuarioRepository::new(&connection);
    let usuarios = UsuarioService::new(&repository);
    let autenticacion = AutenticacionService::new(&repository);

    assert!(matches!(
        autenticacion.autenticar("9-0002", "lo-que-sea"),
        Err(AutenticacionError::SinPasswordLocal)
    ));

    // `cambiar_password` (no `cambiar_password_propio`) a propósito: el
    // alta de contraseña en un dispositivo nuevo no exige conocer una
    // anterior que nunca existió acá.
    usuarios.cambiar_password(id, "mi-password-nueva").unwrap();

    let sesion = autenticacion
        .autenticar("9-0002", "mi-password-nueva")
        .expect("ahora sí debería poder entrar");
    assert_eq!(sesion.cedula, "9-0002");
    assert_eq!(sesion.rol, RolUsuario::Operador);

    // Ya fijada, se comporta como cualquier password real -- una
    // incorrecta es un rechazo normal, no otra vez "sin password local".
    assert!(matches!(
        autenticacion.autenticar("9-0002", "password-incorrecta"),
        Err(AutenticacionError::CredencialesInvalidas)
    ));
}
