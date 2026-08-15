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
fn app_core_compone_query_n1_y_preparacion_n3_sin_persistir() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let empresas = SqliteEmpresaRepository::new(&connection);
    let empresa_id = empresas
        .crear(&Empresa {
            id: 0,
            nombre: "Brisas".to_owned(),
        })
        .unwrap();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let contratista_id = contratistas
        .crear(&Contratista {
            id: 0,
            cedula: "1001".to_owned(),
            nombre: "Contratista Uno".to_owned(),
            empresa_id,
            tipo_ingreso: TipoIngreso::PorCorreo,
            fecha_vencimiento_praind: None,
            es_personal_ruta: false,
            tiene_acceso: true,
        })
        .unwrap();
    let core = AppCore::new(connection);

    let filas = core
        .buscar_contratistas(&FiltroContratistas::default())
        .unwrap();
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
