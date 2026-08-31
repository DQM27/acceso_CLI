use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use control_acceso::application::AppCore;
use control_acceso::database::queries::contratistas::FiltroContratistas;
use control_acceso::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use control_acceso::database::repositories::empresa_repository::{
    EmpresaRepository, SqliteEmpresaRepository,
};
use control_acceso::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use control_acceso::database::schema::{SCHEMA_VERSION, initialize_database};
use control_acceso::models::contratista::Contratista;
use control_acceso::models::empresa::Empresa;
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::models::usuario::{RolUsuario, Usuario};
use control_acceso::services::error::{AutenticacionError, UsuarioServiceError};
use control_acceso::services::password::generar_hash;
use control_acceso::services::usuario_service::CrearRootInicialInput;

fn root() -> CrearRootInicialInput {
    CrearRootInicialInput {
        cedula: "ROOT1".to_owned(),
        nombre: "Root Principal".to_owned(),
        password: "password1".to_owned(),
    }
}

fn core_memoria() -> AppCore {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    AppCore::new(connection)
}

fn archivo_temporal(nombre: &str) -> PathBuf {
    let unico = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "control_acceso_core_{nombre}_{}_{unico}.sqlite",
        std::process::id()
    ))
}

#[test]
fn app_core_se_crea_y_detecta_configuracion_inicial() {
    let core = core_memoria();
    assert!(core.requiere_configuracion_inicial().unwrap());
}

#[test]
fn app_core_crea_root_atomico_y_cierra_configuracion() {
    let core = core_memoria();
    let id = core.crear_root_inicial(root()).unwrap();

    assert!(id > 0);
    assert!(!core.requiere_configuracion_inicial().unwrap());
    assert!(matches!(
        core.crear_root_inicial(CrearRootInicialInput {
            cedula: "ROOT2".to_owned(),
            ..root()
        }),
        Err(UsuarioServiceError::ConfiguracionInicialYaRealizada)
    ));
}

#[test]
fn autenticacion_real_devuelve_contrato_seguro() {
    let core = core_memoria();
    let id = core.crear_root_inicial(root()).unwrap();
    let sesion = core.autenticar("ROOT1", "password1").unwrap();

    assert_eq!(sesion.id, id);
    assert_eq!(sesion.cedula, "ROOT1");
    assert_eq!(sesion.nombre, "Root Principal");
    assert_eq!(sesion.rol, RolUsuario::Root);
}

#[test]
fn autenticacion_conserva_errores_de_credenciales_e_inactivo() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let usuarios = SqliteUsuarioRepository::new(&connection);
    usuarios
        .crear(&Usuario {
            id: 0,
            cedula: "INACTIVO".to_owned(),
            nombre: "Usuario Inactivo".to_owned(),
            password_hash: generar_hash("password2").unwrap(),
            rol: RolUsuario::Operador,
            activo: false,
        })
        .unwrap();
    let core = AppCore::new(connection);

    assert!(matches!(
        core.autenticar("NO-EXISTE", "incorrecta"),
        Err(AutenticacionError::CredencialesInvalidas)
    ));
    assert!(matches!(
        core.autenticar("INACTIVO", "password2"),
        Err(AutenticacionError::UsuarioInactivo)
    ));
}

#[test]
fn buscar_candidato_resuelve_cedula_sin_verificar_password() {
    let core = core_memoria();
    core.crear_root_inicial(root()).unwrap();

    let candidato = core.buscar_candidato_autenticacion("ROOT1").unwrap();
    assert_eq!(candidato.sesion.cedula, "ROOT1");
    assert_eq!(candidato.sesion.rol, RolUsuario::Root);
    // El hash sigue siendo el mismo que produce generar_hash/verificar_password —
    // no se toca la contraseña real, sólo se difiere su verificación.
    assert!(
        control_acceso::services::password::verificar_password(
            "password1",
            &candidato.password_hash
        )
        .unwrap()
    );
    assert!(
        !control_acceso::services::password::verificar_password(
            "incorrecta",
            &candidato.password_hash
        )
        .unwrap()
    );

    assert!(matches!(
        core.buscar_candidato_autenticacion("NO-EXISTE"),
        Err(AutenticacionError::CredencialesInvalidas)
    ));
}

#[test]
fn autenticacion_en_hilo_aparte_no_bloquea_y_entrega_el_resultado_real_por_canal() {
    let core = core_memoria();
    core.crear_root_inicial(root()).unwrap();
    let candidato = core.buscar_candidato_autenticacion("ROOT1").unwrap();

    // Mismo patrón que usa App::iniciar_autenticacion: la parte lenta (Argon2)
    // corre en un hilo aparte y entrega el resultado por un canal.
    let (emisor, receptor) = std::sync::mpsc::channel();
    let candidato_correcto = candidato.clone();
    std::thread::spawn(move || {
        let resultado = match control_acceso::services::password::verificar_password(
            "password1",
            &candidato_correcto.password_hash,
        ) {
            Ok(true) => Ok(candidato_correcto.sesion),
            Ok(false) => Err(AutenticacionError::CredencialesInvalidas),
            Err(_) => Err(AutenticacionError::HashInvalido),
        };
        emisor.send(resultado).unwrap();
    });
    let sesion = receptor
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("el hilo debía responder")
        .expect("la contraseña correcta debía autenticar");
    assert_eq!(sesion.cedula, "ROOT1");

    let (emisor, receptor) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let resultado = match control_acceso::services::password::verificar_password(
            "incorrecta",
            &candidato.password_hash,
        ) {
            Ok(true) => Ok(candidato.sesion),
            Ok(false) => Err(AutenticacionError::CredencialesInvalidas),
            Err(_) => Err(AutenticacionError::HashInvalido),
        };
        emisor.send(resultado).unwrap();
    });
    let error = receptor
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("el hilo debía responder");
    assert!(matches!(
        error,
        Err(AutenticacionError::CredencialesInvalidas)
    ));
}

#[test]
fn app_core_compone_query_n1_y_preparacion_n3_sin_persistir() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let empresas = SqliteEmpresaRepository::new(&connection);
    let empresa_id = empresas
        .crear(&Empresa {
            id: 0,
            nombre: "Brisas".to_owned(),
            activo: true,
        })
        .unwrap();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let contratista_id = contratistas
        .crear(&Contratista::reconstruir(
            0,
            "1001".to_owned(),
            "Contratista Uno".to_owned(),
            empresa_id,
            TipoIngreso::PorCorreo,
            None,
            false,
            true,
            true,
        ))
        .unwrap();
    let core = AppCore::new(connection);

    let filas = core
        .buscar_contratistas(&FiltroContratistas::default())
        .unwrap()
        .items;
    assert_eq!(filas.len(), 1);
    assert_eq!(filas[0].empresa_nombre, "Brisas");

    let preparacion = core.preparar_ingreso(contratista_id).unwrap();
    assert_eq!(preparacion.empresa_nombre, "Brisas");
    assert!(!preparacion.tiene_ingreso_activo);
}

#[test]
fn archivo_persistente_se_reabre_con_root_y_autenticacion() {
    let ruta = archivo_temporal("persistencia");
    {
        let core = AppCore::abrir(&ruta).unwrap();
        core.crear_root_inicial(root()).unwrap();
    }
    {
        let core = AppCore::abrir(&ruta).unwrap();
        assert!(!core.requiere_configuracion_inicial().unwrap());
        assert!(core.autenticar("ROOT1", "password1").is_ok());
    }
    std::fs::remove_file(ruta).unwrap();
}

#[test]
fn listar_roots_activos_excluye_otros_roles_e_inactivos() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let usuarios = SqliteUsuarioRepository::new(&connection);
    let root_id = usuarios
        .crear(&Usuario {
            id: 0,
            cedula: "ROOT1".to_owned(),
            nombre: "Root Principal".to_owned(),
            password_hash: generar_hash("password1").unwrap(),
            rol: RolUsuario::Root,
            activo: true,
        })
        .unwrap();
    usuarios
        .crear(&Usuario {
            id: 0,
            cedula: "OP1".to_owned(),
            nombre: "Operador".to_owned(),
            password_hash: generar_hash("password2").unwrap(),
            rol: RolUsuario::Operador,
            activo: true,
        })
        .unwrap();
    usuarios
        .crear(&Usuario {
            id: 0,
            cedula: "ROOT-INACTIVO".to_owned(),
            nombre: "Root Inactivo".to_owned(),
            password_hash: generar_hash("password3").unwrap(),
            rol: RolUsuario::Root,
            activo: false,
        })
        .unwrap();
    let core = AppCore::new(connection);

    let roots = core.listar_roots_activos().unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].id, root_id);
    assert_eq!(roots[0].rol, RolUsuario::Root);
}

#[test]
fn resetear_password_root_actualiza_hash_sin_necesitar_actor() {
    let core = core_memoria();
    let root_id = core.crear_root_inicial(root()).unwrap();

    core.resetear_password_root(root_id, "password-nueva")
        .unwrap();

    assert!(core.autenticar("ROOT1", "password-nueva").is_ok());
    assert!(matches!(
        core.autenticar("ROOT1", "password1"),
        Err(AutenticacionError::CredencialesInvalidas)
    ));
}

#[test]
fn resetear_password_root_rechaza_usuario_que_no_es_root_activo() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let usuarios = SqliteUsuarioRepository::new(&connection);
    let operador_id = usuarios
        .crear(&Usuario {
            id: 0,
            cedula: "OP1".to_owned(),
            nombre: "Operador".to_owned(),
            password_hash: generar_hash("password2").unwrap(),
            rol: RolUsuario::Operador,
            activo: true,
        })
        .unwrap();
    let core = AppCore::new(connection);

    assert!(matches!(
        core.resetear_password_root(operador_id, "otra-nueva"),
        Err(UsuarioServiceError::UsuarioNoEncontrado)
    ));
}

#[test]
fn crear_respaldo_por_flag_no_requiere_actor_y_queda_marcado_como_tal() {
    // Directorio propio (no sólo un archivo en temp_dir): "backups" se crea
    // junto a la base, y un directorio compartido entre tests colisionaría.
    let unico = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directorio = std::env::temp_dir().join(format!(
        "control_acceso_core_respaldo_por_flag_{}_{unico}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directorio).unwrap();
    let ruta = directorio.join("control_acceso.sqlite");
    let core = AppCore::abrir(&ruta).unwrap();
    core.crear_root_inicial(root()).unwrap();

    let respaldo = core.crear_respaldo_por_flag().unwrap();

    assert_eq!(
        respaldo.tipo,
        control_acceso::database::backup::TipoRespaldo::PorFlag
    );
    assert!(respaldo.ruta.exists());

    drop(core);
    std::fs::remove_dir_all(&directorio).unwrap();
}

#[test]
fn apertura_productiva_lleva_base_nueva_a_version_actual() {
    let ruta = archivo_temporal("version");
    let core = AppCore::abrir(&ruta).unwrap();
    drop(core);
    let connection = Connection::open(&ruta).unwrap();
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    drop(connection);
    std::fs::remove_file(ruta).unwrap();
}
