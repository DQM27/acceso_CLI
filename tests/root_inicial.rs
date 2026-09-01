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
fn usuario(cedula: &str, rol: RolUsuario, activo: bool) -> CrearUsuarioInput {
    CrearUsuarioInput {
        cedula: cedula.to_string(),
        nombre: "Usuario".to_string(),
        password: "password2".to_string(),
        rol,
        activo,
    }
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
        s.desactivar(id),
        Err(UsuarioServiceError::UltimoRootActivo)
    ));
    assert!(matches!(
        s.actualizar_administracion(
            id,
            ActualizarUsuarioInput {
                cedula: "ROOT1".to_string(),
                nombre: "Root".to_string(),
                rol: RolUsuario::Administrador
            },
            true,
        ),
        Err(UsuarioServiceError::UltimoRootActivo)
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
        s.actualizar_administracion(
            id,
            ActualizarUsuarioInput {
                cedula: "ROOT1".to_string(),
                nombre: "Root".to_string(),
                rol: RolUsuario::Operador,
            },
            true,
        ),
        Err(UsuarioServiceError::UltimoRootActivo)
    ));
    assert_eq!(r.contar_roots_activos().unwrap(), 1);
}

#[test]
fn unico_root_puede_cambiar_identidad_y_password() {
    let c = base();
    let r = SqliteUsuarioRepository::new(&c);
    let s = UsuarioService::new(&r);
    let id = s.crear_root_inicial(root("ROOT1")).unwrap();

    s.actualizar_administracion(
        id,
        ActualizarUsuarioInput {
            cedula: "ROOT-NUEVO".to_string(),
            nombre: "Nombre Nuevo".to_string(),
            rol: RolUsuario::Root,
        },
        true,
    )
    .unwrap();
    s.cambiar_password(id, "password-nueva").unwrap();

    let actualizado = s.buscar_por_id(id).unwrap();
    assert_eq!(actualizado.cedula, "ROOT-NUEVO");
    assert_eq!(actualizado.nombre, "Nombre Nuevo");
    assert_eq!(actualizado.rol, RolUsuario::Root);
    assert!(actualizado.activo);
    assert!(
        control_acceso::services::autenticacion_service::AutenticacionService::new(&r)
            .autenticar("ROOT-NUEVO", "password-nueva")
            .is_ok()
    );
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
    s.actualizar_administracion(
        primero,
        ActualizarUsuarioInput {
            cedula: "ROOT1".to_string(),
            nombre: "Anterior Root".to_string(),
            rol: RolUsuario::Operador,
        },
        true,
    )
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
    s.crear(usuario("ROOT2", RolUsuario::Root, true)).unwrap();

    s.actualizar_administracion(
        primero,
        ActualizarUsuarioInput {
            cedula: "ROOT1".to_string(),
            nombre: "Administrador".to_string(),
            rol: RolUsuario::Administrador,
        },
        true,
    )
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
    let admin = s
        .crear(usuario("ADMIN1", RolUsuario::Administrador, true))
        .unwrap();

    s.actualizar_administracion(
        admin,
        ActualizarUsuarioInput {
            cedula: "ADMIN-EDITADO".to_string(),
            nombre: "Administrador Editado".to_string(),
            rol: RolUsuario::Administrador,
        },
        true,
    )
    .unwrap();
    assert_eq!(r.contar_roots_activos().unwrap(), 1);

    s.actualizar_administracion(
        admin,
        ActualizarUsuarioInput {
            cedula: "ADMIN-EDITADO".to_string(),
            nombre: "Nuevo Root".to_string(),
            rol: RolUsuario::Root,
        },
        true,
    )
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
        let segundo = servicio
            .crear(usuario("ROOT-B", RolUsuario::Root, true))
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
                let servicio = UsuarioService::new(&repositorio);
                barrera.wait();
                servicio.desactivar(id)
            })
        })
        .collect();

    let resultados: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(resultados.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        resultados
            .iter()
            .filter(|r| matches!(r, Err(UsuarioServiceError::UltimoRootActivo)))
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
    assert_eq!(u.nombre, "Usuario Desarrollo");
    assert_eq!(r.contar_usuarios().unwrap(), 0);
    assert!(matches!(
        control_acceso::services::dev_auth::actor_persistido(&u),
        Err(control_acceso::services::dev_auth::DevAuthError::ActorPersistidoRequerido)
    ));
}
