use control_acceso::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::error::UsuarioServiceError;
use control_acceso::services::usuario_service::{
    ActualizarUsuarioInput, CrearRootInicialInput, CrearUsuarioInput, UsuarioService,
};
use rusqlite::Connection;

fn base() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    initialize_database(&c).unwrap();
    c
}
fn root(cedula: &str) -> CrearRootInicialInput {
    CrearRootInicialInput {
        cedula: cedula.to_string(),
        nombre: "Root Inicial".to_string(),
        password: "password1".to_string(),
    }
}
fn usuario(cedula: &str, rol: RolUsuario, activo: bool) -> CrearUsuarioInput {
    CrearUsuarioInput {
        cedula: cedula.to_string(),
        nombre: "Usuario".to_string(),
        password: "password2".to_string(),
        rol,
        activo,
    }
}

#[test]
fn base_vacia_requiere_configuracion_y_crear_normal_es_rechazado() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    assert!(s.requiere_configuracion_inicial().unwrap());
    assert!(matches!(
        s.crear(usuario("2001", RolUsuario::Operador, true)),
        Err(UsuarioServiceError::ConfiguracionInicialRequerida)
    ));
}

#[test]
fn root_inicial_es_root_activo_normalizado_y_cierra_configuracion() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let mut entrada = root("  ROOT1  ");
    entrada.nombre = "  Root Principal  ".to_string();
    let id = s.crear_root_inicial(entrada).unwrap();
    let u = s.buscar_por_id(id).unwrap();
    assert_eq!(u.cedula, "ROOT1");
    assert_eq!(u.nombre, "Root Principal");
    assert_eq!(u.rol, RolUsuario::Root);
    assert!(u.activo);
    assert!(!s.requiere_configuracion_inicial().unwrap());
    assert!(matches!(
        s.crear_root_inicial(root("ROOT2")),
        Err(UsuarioServiceError::ConfiguracionInicialYaRealizada)
    ));
}

#[test]
fn root_inicial_valida_password() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let mut entrada = root("ROOT1");
    entrada.password = "corta".to_string();
    assert!(matches!(
        s.crear_root_inicial(entrada),
        Err(UsuarioServiceError::PasswordDemasiadoCorto)
    ));
    assert!(s.requiere_configuracion_inicial().unwrap());
}

#[test]
fn no_permite_desactivar_ni_degradar_ultimo_root_activo() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let id = s.crear_root_inicial(root("ROOT1")).unwrap();
    assert!(matches!(
        s.desactivar(id),
        Err(UsuarioServiceError::UltimoRootActivo)
    ));
    assert!(matches!(
        s.actualizar(
            id,
            ActualizarUsuarioInput {
                cedula: "ROOT1".to_string(),
                nombre: "Root".to_string(),
                rol: RolUsuario::Administrador
            }
        ),
        Err(UsuarioServiceError::UltimoRootActivo)
    ));
    assert_eq!(r.contar_roots_activos().unwrap(), 1);
}

#[test]
fn con_dos_roots_puede_desactivar_uno_y_permanece_otro() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let primero = s.crear_root_inicial(root("ROOT1")).unwrap();
    s.crear(usuario("ROOT2", RolUsuario::Root, true)).unwrap();
    s.desactivar(primero).unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 1);
}

#[test]
fn con_dos_roots_puede_degradar_uno_y_permanece_otro() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let primero = s.crear_root_inicial(root("ROOT1")).unwrap();
    s.crear(usuario("ROOT2", RolUsuario::Root, true)).unwrap();
    s.actualizar(
        primero,
        ActualizarUsuarioInput {
            cedula: "ROOT1".to_string(),
            nombre: "Anterior Root".to_string(),
            rol: RolUsuario::Operador,
        },
    )
    .unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 1);
    assert_eq!(s.buscar_por_id(primero).unwrap().rol, RolUsuario::Operador);
}

#[test]
fn root_inactivo_no_cuenta_como_activo() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    s.crear_root_inicial(root("ROOT1")).unwrap();
    s.crear(usuario("ROOT2", RolUsuario::Root, false)).unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 1);
}

#[test]
fn activar_root_inactivo_lo_incluye_en_conteo() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    s.crear_root_inicial(root("ROOT1")).unwrap();
    let segundo = s.crear(usuario("ROOT2", RolUsuario::Root, false)).unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 1);

    s.activar(segundo).unwrap();

    assert_eq!(r.contar_roots_activos().unwrap(), 2);
    assert!(s.buscar_por_id(segundo).unwrap().activo);
}

#[cfg(feature = "dev-auth")]
#[test]
fn dev_auth_es_solo_memoria_y_no_modifica_sqlite() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let u = control_acceso::services::dev_auth::usuario_desarrollo();
    assert_eq!(u.id, 0);
    assert_eq!(u.rol, RolUsuario::Root);
    assert!(u.activo);
    assert!(u.password_hash.is_empty());
    assert_eq!(r.contar_usuarios().unwrap(), 0);
    assert!(matches!(
        control_acceso::services::dev_auth::actor_persistido(&u),
        Err(control_acceso::services::dev_auth::DevAuthError::ActorPersistidoRequerido)
    ));
}
