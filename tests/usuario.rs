use control_acceso::database::repositories::usuario_repository::{
    SqliteUsuarioRepository,
    UsuarioRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::usuario::{
    RolUsuario,
    Usuario,
};
use rusqlite::Connection;

fn crear_base_datos() -> Connection {
    let connection = Connection::open_in_memory()
        .expect("No se pudo crear la base de datos");

    initialize_database(&connection)
        .expect("No se pudo inicializar la base de datos");

    connection
}

fn crear_usuario(
    cedula: &str,
    nombre: &str,
    rol: RolUsuario,
    activo: bool,
) -> Usuario {
    Usuario {
        id: 0,
        cedula: cedula.to_string(),
        nombre: nombre.to_string(),
        password_hash: "hash_de_prueba".to_string(),
        rol,
        activo,
    }
}

#[test]
fn debe_crear_y_recuperar_usuario() {
    let connection = crear_base_datos();

    let repository =
        SqliteUsuarioRepository::new(&connection);

    let usuario = crear_usuario(
        "101010101",
        "Juan Pérez",
        RolUsuario::Operador,
        true,
    );

    let id = repository
        .crear(&usuario)
        .expect("No se pudo crear el usuario");

    let encontrado = repository
        .buscar_por_cedula("101010101")
        .expect("Error buscando usuario")
        .expect("El usuario no fue encontrado");

    assert_eq!(encontrado.id, id);
    assert_eq!(encontrado.cedula, "101010101");
    assert_eq!(encontrado.nombre, "Juan Pérez");
    assert_eq!(
        encontrado.rol,
        RolUsuario::Operador
    );
    assert!(encontrado.activo);
}

#[test]
fn debe_buscar_usuario_por_id() {
    let connection = crear_base_datos();

    let repository =
        SqliteUsuarioRepository::new(&connection);

    let usuario = crear_usuario(
        "202020202",
        "María López",
        RolUsuario::Administrador,
        true,
    );

    let id = repository
        .crear(&usuario)
        .expect("No se pudo crear el usuario");

    let encontrado = repository
        .buscar_por_id(id)
        .expect("Error buscando usuario")
        .expect("El usuario no fue encontrado");

    assert_eq!(encontrado.id, id);
    assert_eq!(
        encontrado.rol,
        RolUsuario::Administrador
    );
}

#[test]
fn debe_retornar_none_si_la_cedula_no_existe() {
    let connection = crear_base_datos();

    let repository =
        SqliteUsuarioRepository::new(&connection);

    let resultado = repository
        .buscar_por_cedula("999999999")
        .expect("La búsqueda produjo un error");

    assert!(resultado.is_none());
}

#[test]
fn debe_actualizar_usuario() {
    let connection = crear_base_datos();

    let repository =
        SqliteUsuarioRepository::new(&connection);

    let mut usuario = crear_usuario(
        "303030303",
        "Carlos Rodríguez",
        RolUsuario::Operador,
        true,
    );

    let id = repository
        .crear(&usuario)
        .expect("No se pudo crear el usuario");

    usuario.id = id;
    usuario.nombre = "Carlos Rodríguez Actualizado".to_string();
    usuario.rol = RolUsuario::Administrador;
    usuario.activo = false;

    repository
        .actualizar(&usuario)
        .expect("No se pudo actualizar el usuario");

    let actualizado = repository
        .buscar_por_id(id)
        .expect("Error buscando usuario")
        .expect("El usuario no fue encontrado");

    assert_eq!(
        actualizado.nombre,
        "Carlos Rodríguez Actualizado"
    );

    assert_eq!(
        actualizado.rol,
        RolUsuario::Administrador
    );

    assert!(!actualizado.activo);
}

#[test]
fn debe_listar_usuarios() {
    let connection = crear_base_datos();

    let repository =
        SqliteUsuarioRepository::new(&connection);

    let usuario1 = crear_usuario(
        "111111111",
        "Usuario Root",
        RolUsuario::Root,
        true,
    );

    let usuario2 = crear_usuario(
        "222222222",
        "Usuario Administrador",
        RolUsuario::Administrador,
        true,
    );

    let usuario3 = crear_usuario(
        "333333333",
        "Usuario Operador",
        RolUsuario::Operador,
        true,
    );

    repository
        .crear(&usuario1)
        .expect("No se pudo crear usuario 1");

    repository
        .crear(&usuario2)
        .expect("No se pudo crear usuario 2");

    repository
        .crear(&usuario3)
        .expect("No se pudo crear usuario 3");

    let usuarios = repository
        .listar()
        .expect("No se pudieron listar los usuarios");

    assert_eq!(usuarios.len(), 3);

    assert!(
        usuarios
            .iter()
            .any(|u| u.rol == RolUsuario::Root)
    );

    assert!(
        usuarios
            .iter()
            .any(|u| u.rol == RolUsuario::Administrador)
    );

    assert!(
        usuarios
            .iter()
            .any(|u| u.rol == RolUsuario::Operador)
    );
}

#[test]
fn debe_guardar_los_tres_roles() {
    let connection = crear_base_datos();

    let repository =
        SqliteUsuarioRepository::new(&connection);

    let usuarios = [
        crear_usuario(
            "444444444",
            "Root",
            RolUsuario::Root,
            true,
        ),
        crear_usuario(
            "555555555",
            "Administrador",
            RolUsuario::Administrador,
            true,
        ),
        crear_usuario(
            "666666666",
            "Operador",
            RolUsuario::Operador,
            true,
        ),
    ];

    for usuario in &usuarios {
        repository
            .crear(usuario)
            .expect("No se pudo crear el usuario");
    }

    let root = repository
        .buscar_por_cedula("444444444")
        .unwrap()
        .unwrap();

    let administrador = repository
        .buscar_por_cedula("555555555")
        .unwrap()
        .unwrap();

    let operador = repository
        .buscar_por_cedula("666666666")
        .unwrap()
        .unwrap();

    assert_eq!(root.rol, RolUsuario::Root);
    assert_eq!(
        administrador.rol,
        RolUsuario::Administrador
    );
    assert_eq!(
        operador.rol,
        RolUsuario::Operador
    );
}