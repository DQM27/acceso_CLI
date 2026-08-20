use rusqlite::Connection;

use control_acceso::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::autenticacion_service::AutenticacionService;
use control_acceso::services::error::{AutenticacionError, UsuarioServiceError};
use control_acceso::services::password::generar_hash;
use control_acceso::services::usuario_service::{
    ActualizarUsuarioInput, CrearRootInicialInput, CrearUsuarioInput, UsuarioService,
};

fn base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
}

fn inicializar(connection: &Connection) -> i64 {
    let repository = SqliteUsuarioRepository::new(connection);
    UsuarioService::new(&repository)
        .crear_root_inicial(CrearRootInicialInput {
            cedula: "ROOT1".to_string(),
            nombre: "Usuario Root".to_string(),
            password: "password1".to_string(),
        })
        .unwrap()
}

fn input(cedula: &str, rol: RolUsuario, activo: bool) -> CrearUsuarioInput {
    CrearUsuarioInput {
        cedula: cedula.to_string(),
        nombre: "Usuario Nuevo".to_string(),
        password: "password2".to_string(),
        rol,
        activo,
    }
}

#[test]
fn crea_usuario_normalizado_y_nunca_guarda_password_plano() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let mut entrada = input("  2001  ", RolUsuario::Operador, true);
    entrada.nombre = "  Persona Dos  ".to_string();
    let id = servicio.crear(entrada).unwrap();
    let usuario = servicio.buscar_por_id(id).unwrap();
    assert_eq!(usuario.cedula, "2001");
    assert_eq!(usuario.nombre, "Persona Dos");
    assert_ne!(usuario.password_hash, "password2");
    assert_eq!(usuario.rol, RolUsuario::Operador);
    assert!(usuario.activo);
}

#[test]
fn valida_campos_obligatorios_y_password_corto() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let mut entrada = input(" ", RolUsuario::Operador, true);
    assert!(matches!(
        servicio.crear(entrada),
        Err(UsuarioServiceError::CedulaVacia)
    ));
    entrada = input("2001", RolUsuario::Operador, true);
    entrada.nombre = " ".to_string();
    assert!(matches!(
        servicio.crear(entrada),
        Err(UsuarioServiceError::NombreVacio)
    ));
    entrada = input("2001", RolUsuario::Operador, true);
    entrada.password = "corta".to_string();
    assert!(matches!(
        servicio.crear(entrada),
        Err(UsuarioServiceError::PasswordDemasiadoCorto)
    ));
}

#[test]
fn busca_por_id_y_cedula_normalizada_y_reporta_inexistentes() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let id = servicio
        .crear(input("2001", RolUsuario::Operador, true))
        .unwrap();
    assert_eq!(servicio.buscar_por_id(id).unwrap().id, id);
    assert_eq!(servicio.buscar_por_cedula(" 2001 ").unwrap().id, id);
    assert!(matches!(
        servicio.buscar_por_id(999),
        Err(UsuarioServiceError::UsuarioNoEncontrado)
    ));
}

#[test]
fn actualizar_administracion_preserva_hash_y_cambia_datos_rol_y_activo() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let id = servicio
        .crear(input("2001", RolUsuario::Operador, true))
        .unwrap();
    let anterior = servicio.buscar_por_id(id).unwrap();
    servicio
        .actualizar_administracion(
            id,
            ActualizarUsuarioInput {
                cedula: " 3001 ".to_string(),
                nombre: " Nombre Nuevo ".to_string(),
                rol: RolUsuario::Administrador,
            },
            false,
        )
        .unwrap();
    let nuevo = servicio.buscar_por_id(id).unwrap();
    assert_eq!(nuevo.cedula, "3001");
    assert_eq!(nuevo.nombre, "Nombre Nuevo");
    assert_eq!(nuevo.rol, RolUsuario::Administrador);
    assert_eq!(nuevo.password_hash, anterior.password_hash);
    assert!(!nuevo.activo);
}

/// Regresión del hallazgo #4 de `docs/auditoria-dominio-2026-08-20.md`: una
/// edición administrativa (nombre/rol/estado) no debe poder revertir una
/// contraseña cambiada por otra instancia mientras tanto. Reproduce el
/// camino exacto que tenía el bug: `UsuarioService::actualizar_administracion`
/// lee el `Usuario` completo (incluido `password_hash`) *antes* de la
/// transacción de escritura — si el hash cambia real en ese intervalo, la
/// versión vieja quedaba en memoria y se volvía a escribir al persistir.
#[test]
fn actualizar_administracion_no_revierte_una_contrasena_cambiada_concurrentemente() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let id = servicio
        .crear(input("2001", RolUsuario::Operador, true))
        .unwrap();

    // Instancia A lee el usuario para editar nombre/rol — igual que hace
    // `actualizar_administracion` internamente — y se queda con el hash
    // vigente en ese momento.
    let mut editado_por_instancia_a = servicio.buscar_por_id(id).unwrap();
    editado_por_instancia_a.cedula = "3001".to_string();
    editado_por_instancia_a.nombre = "Nombre Nuevo".to_string();
    editado_por_instancia_a.rol = RolUsuario::Administrador;

    // Mientras tanto, otra instancia cambia la contraseña real.
    let hash_nuevo = generar_hash("clave-cambiada-concurrentemente").unwrap();
    repository.actualizar_password(id, &hash_nuevo).unwrap();

    // Instancia A recién ahora persiste su edición, con el hash viejo todavía
    // en memoria.
    repository.actualizar(&editado_por_instancia_a).unwrap();

    let final_usuario = servicio.buscar_por_id(id).unwrap();
    assert_eq!(
        final_usuario.password_hash, hash_nuevo,
        "la edición administrativa no debe revertir la contraseña nueva"
    );
    assert_eq!(final_usuario.cedula, "3001");
    assert_eq!(final_usuario.rol, RolUsuario::Administrador);
}

#[test]
fn cambiar_password_invalida_anterior_y_habilita_nueva() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let id = servicio
        .crear(input("2001", RolUsuario::Operador, true))
        .unwrap();
    servicio.cambiar_password(id, "password3").unwrap();
    let auth = AutenticacionService::new(&repository);
    assert!(matches!(
        auth.autenticar("2001", "password2"),
        Err(AutenticacionError::CredencialesInvalidas)
    ));
    assert!(auth.autenticar("2001", "password3").is_ok());
}

#[test]
fn activar_y_desactivar_usuario_no_root() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let id = servicio
        .crear(input("2001", RolUsuario::Operador, true))
        .unwrap();
    servicio.desactivar(id).unwrap();
    assert!(!servicio.buscar_por_id(id).unwrap().activo);
    servicio.activar(id).unwrap();
    assert!(servicio.buscar_por_id(id).unwrap().activo);
}

#[test]
fn operaciones_sobre_id_inexistente_fallan() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    assert!(matches!(
        servicio.cambiar_password(999, "password3"),
        Err(UsuarioServiceError::UsuarioNoEncontrado)
    ));
    assert!(matches!(
        servicio.activar(999),
        Err(UsuarioServiceError::UsuarioNoEncontrado)
    ));
    assert!(matches!(
        servicio.desactivar(999),
        Err(UsuarioServiceError::UsuarioNoEncontrado)
    ));
}

#[test]
fn cedula_duplicada_devuelve_error_semantico_sin_crear_otro_usuario() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    servicio
        .crear(input("2001", RolUsuario::Operador, true))
        .unwrap();
    assert!(matches!(
        servicio.crear(input("2001", RolUsuario::Administrador, true)),
        Err(UsuarioServiceError::CedulaDuplicada)
    ));
    assert_eq!(servicio.listar().unwrap().len(), 2);
}

#[test]
fn cambio_password_respeta_limite_de_ocho_caracteres() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let id = servicio
        .crear(input("2001", RolUsuario::Operador, true))
        .unwrap();

    assert!(matches!(
        servicio.cambiar_password(id, "1234567"),
        Err(UsuarioServiceError::PasswordDemasiadoCorto)
    ));
    servicio.cambiar_password(id, "12345678").unwrap();
    assert!(
        AutenticacionService::new(&repository)
            .autenticar("2001", "12345678")
            .is_ok()
    );
}

#[test]
fn actualizar_administracion_rechaza_cedula_y_nombre_vacios() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let id = servicio
        .crear(input("2001", RolUsuario::Operador, true))
        .unwrap();

    assert!(matches!(
        servicio.actualizar_administracion(
            id,
            ActualizarUsuarioInput {
                cedula: " ".to_string(),
                nombre: "Nombre".to_string(),
                rol: RolUsuario::Operador,
            },
            true,
        ),
        Err(UsuarioServiceError::CedulaVacia)
    ));
    assert!(matches!(
        servicio.actualizar_administracion(
            id,
            ActualizarUsuarioInput {
                cedula: "2001".to_string(),
                nombre: " ".to_string(),
                rol: RolUsuario::Operador,
            },
            true,
        ),
        Err(UsuarioServiceError::NombreVacio)
    ));
}

#[test]
fn actualizar_administracion_con_cedula_duplicada_conserva_registro_original() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    servicio
        .crear(input("2001", RolUsuario::Operador, true))
        .unwrap();
    let segundo = servicio
        .crear(input("2002", RolUsuario::Administrador, false))
        .unwrap();
    let original = servicio.buscar_por_id(segundo).unwrap();

    assert!(matches!(
        servicio.actualizar_administracion(
            segundo,
            ActualizarUsuarioInput {
                cedula: "2001".to_string(),
                nombre: "Modificado".to_string(),
                rol: RolUsuario::Root,
            },
            false,
        ),
        Err(UsuarioServiceError::CedulaDuplicada)
    ));
    assert_eq!(servicio.buscar_por_id(segundo).unwrap(), original);
}

#[test]
fn validar_datos_para_crear_rechaza_password_corto_sin_tocar_el_repositorio() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let mut entrada = input("3001", RolUsuario::Operador, true);
    entrada.password = "corta".to_string();

    assert!(matches!(
        servicio.validar_datos_para_crear(&entrada),
        Err(UsuarioServiceError::PasswordDemasiadoCorto)
    ));
    assert!(matches!(
        servicio.buscar_por_cedula("3001"),
        Err(UsuarioServiceError::UsuarioNoEncontrado)
    ));
}

#[test]
fn crear_con_hash_guarda_el_hash_tal_cual_sin_volver_a_calcularlo() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);

    let hash = generar_hash("password-ya-calculado").unwrap();

    let id = servicio
        .crear_con_hash(
            "3002",
            "Persona Hash",
            RolUsuario::Operador,
            true,
            hash.clone(),
        )
        .unwrap();

    let usuario = servicio.buscar_por_id(id).unwrap();
    assert_eq!(usuario.password_hash, hash);
}

#[test]
fn crear_con_hash_rechaza_un_hash_con_formato_invalido() {
    let connection = base();
    inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);

    let resultado = servicio.crear_con_hash(
        "3003",
        "Persona Hash Invalido",
        RolUsuario::Operador,
        true,
        "no-es-un-hash-argon2".to_string(),
    );

    assert!(matches!(resultado, Err(UsuarioServiceError::Password(_))));
}

#[test]
fn validar_password_para_cambio_rechaza_password_corto_sin_tocar_el_repositorio() {
    let connection = base();
    let id = inicializar(&connection);
    let repository = SqliteUsuarioRepository::new(&connection);
    let servicio = UsuarioService::new(&repository);
    let hash_original = servicio.buscar_por_id(id).unwrap().password_hash;

    assert!(matches!(
        servicio.validar_password_para_cambio(id, "corta"),
        Err(UsuarioServiceError::PasswordDemasiadoCorto)
    ));
    assert_eq!(
        servicio.buscar_por_id(id).unwrap().password_hash,
        hash_original
    );
}
