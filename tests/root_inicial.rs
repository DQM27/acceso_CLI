//! Arranque inicial (ROOT) y la regla del "último ROOT activo", que vive en
//! el repositorio (ver `usuario_repository.rs`). El alta y la edición de
//! usuarios se hacen desde el panel web; acá se prueban contra el
//! repositorio, que es lo que las aplica al recibir el catálogo.

use control_acceso::database::error::DatabaseError;
use control_acceso::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::usuario::{RolUsuario, Usuario};
use control_acceso::services::error::UsuarioServiceError;
use control_acceso::services::password::generar_hash;
use control_acceso::services::usuario_service::{CrearRootInicialInput, UsuarioService};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

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
fn usuario(cedula: &str, rol: RolUsuario, activo: bool) -> Usuario {
    Usuario {
        id: 0,
        cedula: cedula.to_string(),
        nombre: "Usuario".to_string(),
        password_hash: generar_hash("password2").unwrap(),
        rol,
        activo,
        password_hash_confirmado_en: None,
        password_temporal_cacheada: false,
    }
}

/// `id` con otro rol, leído del repositorio.
fn con_rol(r: &SqliteUsuarioRepository<'_>, id: i64, rol: RolUsuario) -> Usuario {
    let mut u = r.buscar_por_id(id).unwrap().unwrap();
    u.rol = rol;
    u
}

fn archivo_temporal(nombre: &str) -> PathBuf {
    let unico = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "control_acceso_{nombre}_{}_{unico}.sqlite",
        std::process::id()
    ))
}

#[test]
fn base_vacia_requiere_configuracion_inicial() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    assert!(
        UsuarioService::new(&r)
            .requiere_configuracion_inicial()
            .unwrap()
    );
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
fn fallo_de_insercion_del_root_inicial_hace_rollback() {
    let c = base();
    c.execute_batch(
        "CREATE TRIGGER impedir_root BEFORE INSERT ON usuarios
         BEGIN SELECT RAISE(ABORT, 'fallo inducido'); END;",
    )
    .unwrap();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);

    assert!(matches!(
        s.crear_root_inicial(root("ROOT1")),
        Err(UsuarioServiceError::Database(_))
    ));
    assert_eq!(r.contar_usuarios().unwrap(), 0);
    assert!(s.requiere_configuracion_inicial().unwrap());
}

#[test]
fn no_permite_desactivar_ni_degradar_ultimo_root_activo() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let id = s.crear_root_inicial(root("ROOT1")).unwrap();
    assert!(matches!(
        r.establecer_activo(id, false),
        Err(DatabaseError::UltimoRootActivo)
    ));
    assert!(matches!(
        r.actualizar_protegiendo_ultimo_root(&con_rol(&r, id, RolUsuario::Administrador)),
        Err(DatabaseError::UltimoRootActivo)
    ));
    assert_eq!(r.contar_roots_activos().unwrap(), 1);
}

#[test]
fn unico_root_no_puede_convertirse_en_operador() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let id = s.crear_root_inicial(root("ROOT1")).unwrap();

    assert!(matches!(
        r.actualizar_protegiendo_ultimo_root(&con_rol(&r, id, RolUsuario::Operador)),
        Err(DatabaseError::UltimoRootActivo)
    ));
    assert_eq!(r.contar_roots_activos().unwrap(), 1);
}

#[test]
fn con_dos_roots_puede_desactivar_uno_y_permanece_otro() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let primero = s.crear_root_inicial(root("ROOT1")).unwrap();
    r.crear(&usuario("ROOT2", RolUsuario::Root, true)).unwrap();
    r.establecer_activo(primero, false).unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 1);
}

#[test]
fn con_dos_roots_puede_degradar_uno_y_permanece_otro() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let primero = s.crear_root_inicial(root("ROOT1")).unwrap();
    r.crear(&usuario("ROOT2", RolUsuario::Root, true)).unwrap();
    r.actualizar_protegiendo_ultimo_root(&con_rol(&r, primero, RolUsuario::Operador))
        .unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 1);
    assert_eq!(s.buscar_por_id(primero).unwrap().rol, RolUsuario::Operador);
}

#[test]
fn con_dos_roots_uno_puede_convertirse_en_administrador() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let primero = s.crear_root_inicial(root("ROOT1")).unwrap();
    r.crear(&usuario("ROOT2", RolUsuario::Root, true)).unwrap();

    r.actualizar_protegiendo_ultimo_root(&con_rol(&r, primero, RolUsuario::Administrador))
        .unwrap();

    assert_eq!(r.contar_roots_activos().unwrap(), 1);
    assert_eq!(
        s.buscar_por_id(primero).unwrap().rol,
        RolUsuario::Administrador
    );
}

#[test]
fn promover_admin_activo_incrementa_roots_y_editar_no_root_no_activa_proteccion() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    s.crear_root_inicial(root("ROOT1")).unwrap();
    let admin = r
        .crear(&usuario("ADMIN1", RolUsuario::Administrador, true))
        .unwrap();

    let mut editado = con_rol(&r, admin, RolUsuario::Administrador);
    editado.nombre = "Administrador Editado".to_string();
    r.actualizar_protegiendo_ultimo_root(&editado).unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 1);

    r.actualizar_protegiendo_ultimo_root(&con_rol(&r, admin, RolUsuario::Root))
        .unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 2);
}

#[test]
fn dos_conexiones_solo_pueden_crear_un_root_inicial() {
    let ruta = archivo_temporal("root_inicial");
    let inicial = Connection::open(&ruta).unwrap();
    initialize_database(&inicial).unwrap();
    drop(inicial);
    let barrera = Arc::new(Barrier::new(2));

    // El `collect()` es necesario, no "innecesario" (falso positivo del
    // lint): fuerza a lanzar los DOS hilos antes de unir ninguno — sin él,
    // un iterador perezoso encadenando spawn+join lanzaría el primer hilo y
    // lo esperaría antes de lanzar el segundo, y la carrera contra el
    // `Barrier` (el punto entero de este test) nunca ocurriría.
    #[allow(clippy::needless_collect)]
    let handles: Vec<_> = ["ROOT-A", "ROOT-B"]
        .into_iter()
        .map(|cedula| {
            let ruta = ruta.clone();
            let barrera = Arc::clone(&barrera);
            thread::spawn(move || {
                let conexion = Connection::open(ruta).unwrap();
                let repositorio = SqliteUsuarioRepository::new(&conexion);
                let servicio = UsuarioService::new(&repositorio);
                barrera.wait();
                servicio.crear_root_inicial(root(cedula))
            })
        })
        .collect();

    let resultados: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(resultados.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        resultados
            .iter()
            .filter(|r| matches!(r, Err(UsuarioServiceError::ConfiguracionInicialYaRealizada)))
            .count(),
        1
    );
    let verificacion = Connection::open(&ruta).unwrap();
    let repositorio = SqliteUsuarioRepository::new(&verificacion);
    assert_eq!(repositorio.contar_usuarios().unwrap(), 1);
    assert_eq!(repositorio.contar_roots_activos().unwrap(), 1);
    drop(verificacion);
    std::fs::remove_file(ruta).unwrap();
}

#[test]
fn dos_conexiones_no_pueden_desactivar_ambos_roots() {
    let ruta = archivo_temporal("ultimo_root");
    let (primero, segundo) = {
        let inicial = Connection::open(&ruta).unwrap();
        initialize_database(&inicial).unwrap();
        let repositorio = SqliteUsuarioRepository::new(&inicial);
        let servicio = UsuarioService::new(&repositorio);
        let primero = servicio.crear_root_inicial(root("ROOT-A")).unwrap();
        let segundo = repositorio
            .crear(&usuario("ROOT-B", RolUsuario::Root, true))
            .unwrap();
        (primero, segundo)
    };
    let barrera = Arc::new(Barrier::new(2));

    // Mismo motivo que en el test anterior: el `collect()` fuerza a lanzar
    // los dos hilos antes de unir ninguno, necesario para la carrera contra
    // el `Barrier`.
    #[allow(clippy::needless_collect)]
    // La conversión tuple→array sugerida es menos directa que enumerar los
    // dos IDs que deben participar en la carrera.
    #[allow(clippy::tuple_array_conversions)]
    let handles: Vec<_> = [primero, segundo]
        .into_iter()
        .map(|id| {
            let ruta = ruta.clone();
            let barrera = Arc::clone(&barrera);
            thread::spawn(move || {
                let conexion = Connection::open(ruta).unwrap();
                let repositorio = SqliteUsuarioRepository::new(&conexion);
                barrera.wait();
                repositorio.establecer_activo(id, false)
            })
        })
        .collect();

    let resultados: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(resultados.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        resultados
            .iter()
            .filter(|r| matches!(r, Err(DatabaseError::UltimoRootActivo)))
            .count(),
        1
    );
    let verificacion = Connection::open(&ruta).unwrap();
    let repositorio = SqliteUsuarioRepository::new(&verificacion);
    assert_eq!(repositorio.contar_roots_activos().unwrap(), 1);
    drop(verificacion);
    std::fs::remove_file(ruta).unwrap();
}

#[test]
fn root_inactivo_no_cuenta_como_activo() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    s.crear_root_inicial(root("ROOT1")).unwrap();
    r.crear(&usuario("ROOT2", RolUsuario::Root, false)).unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 1);
}

#[test]
fn activar_root_inactivo_lo_incluye_en_conteo() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    s.crear_root_inicial(root("ROOT1")).unwrap();
    let segundo = r.crear(&usuario("ROOT2", RolUsuario::Root, false)).unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 1);

    r.establecer_activo(segundo, true).unwrap();

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
    assert_eq!(u.nombre, "Usuario Desarrollo");
    assert_eq!(r.contar_usuarios().unwrap(), 0);
    assert!(matches!(
        control_acceso::services::dev_auth::actor_persistido(&u),
        Err(control_acceso::services::dev_auth::DevAuthError::ActorPersistidoRequerido)
    ));
}
