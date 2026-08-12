use rusqlite::Connection;

use control_acceso::database::repositories::empresa_repository::SqliteEmpresaRepository;
use control_acceso::database::schema::initialize_database;
use control_acceso::services::empresa_service::EmpresaService;
use control_acceso::services::error::EmpresaServiceError;

fn preparar_base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
}

#[test]
fn debe_crear_empresa_valida() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);

    let id = servicio.crear("Empresa Uno").unwrap();

    assert_eq!(servicio.buscar_por_id(id).unwrap().nombre, "Empresa Uno");
}

#[test]
fn debe_rechazar_nombre_vacio() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);

    assert!(matches!(
        servicio.crear(""),
        Err(EmpresaServiceError::NombreEmpresaVacio)
    ));
}

#[test]
fn debe_rechazar_nombre_compuesto_solo_por_espacios() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);

    assert!(matches!(
        servicio.crear("   "),
        Err(EmpresaServiceError::NombreEmpresaVacio)
    ));
}

#[test]
fn debe_aplicar_trim_al_crear() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);

    let id = servicio.crear("  Empresa Uno  ").unwrap();

    assert_eq!(servicio.buscar_por_id(id).unwrap().nombre, "Empresa Uno");
}

#[test]
fn debe_buscar_empresa_por_id() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);
    let id = servicio.crear("Empresa Uno").unwrap();

    let empresa = servicio.buscar_por_id(id).unwrap();

    assert_eq!(empresa.id, id);
    assert_eq!(empresa.nombre, "Empresa Uno");
}

#[test]
fn debe_buscar_empresa_por_nombre() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);
    let id = servicio.crear("Empresa Uno").unwrap();

    let empresa = servicio.buscar_por_nombre("Empresa Uno").unwrap();

    assert_eq!(empresa.id, id);
}

#[test]
fn debe_aplicar_trim_al_buscar_por_nombre() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);
    let id = servicio.crear("Empresa Uno").unwrap();

    let empresa = servicio.buscar_por_nombre("  Empresa Uno  ").unwrap();

    assert_eq!(empresa.id, id);
}

#[test]
fn debe_actualizar_empresa() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);
    let id = servicio.crear("Nombre anterior").unwrap();

    servicio.actualizar(id, "Nombre nuevo").unwrap();

    assert_eq!(servicio.buscar_por_id(id).unwrap().nombre, "Nombre nuevo");
}

#[test]
fn debe_aplicar_trim_al_actualizar() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);
    let id = servicio.crear("Nombre anterior").unwrap();

    servicio.actualizar(id, "  Nombre nuevo  ").unwrap();

    assert_eq!(servicio.buscar_por_id(id).unwrap().nombre, "Nombre nuevo");
}

#[test]
fn debe_listar_empresas() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);
    servicio.crear("Empresa B").unwrap();
    servicio.crear("Empresa A").unwrap();

    let empresas = servicio.listar().unwrap();

    assert_eq!(empresas.len(), 2);
    assert_eq!(empresas[0].nombre, "Empresa A");
    assert_eq!(empresas[1].nombre, "Empresa B");
}

#[test]
fn id_inexistente_debe_producir_error() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);

    assert!(matches!(
        servicio.buscar_por_id(999),
        Err(EmpresaServiceError::EmpresaNoEncontrada)
    ));
}

#[test]
fn nombre_inexistente_debe_producir_error() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);

    assert!(matches!(
        servicio.buscar_por_nombre("No existe"),
        Err(EmpresaServiceError::EmpresaNoEncontrada)
    ));
}

#[test]
fn actualizar_id_inexistente_debe_producir_error() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);

    assert!(matches!(
        servicio.actualizar(999, "Empresa"),
        Err(EmpresaServiceError::EmpresaNoEncontrada)
    ));
}

#[test]
fn debe_rechazar_nombre_vacio_al_actualizar() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);
    let id = servicio.crear("Nombre original").unwrap();

    assert!(matches!(
        servicio.actualizar(id, "   "),
        Err(EmpresaServiceError::NombreEmpresaVacio)
    ));
    assert_eq!(
        servicio.buscar_por_id(id).unwrap().nombre,
        "Nombre original"
    );
}

#[test]
fn nombre_duplicado_produce_error_de_database_y_conserva_integridad() {
    let connection = preparar_base();
    let repository = SqliteEmpresaRepository::new(&connection);
    let servicio = EmpresaService::new(&repository);
    servicio.crear("Empresa Uno").unwrap();
    let segundo_id = servicio.crear("Empresa Dos").unwrap();

    assert!(matches!(
        servicio.crear("Empresa Uno"),
        Err(EmpresaServiceError::Database(_))
    ));
    assert_eq!(servicio.listar().unwrap().len(), 2);

    assert!(matches!(
        servicio.actualizar(segundo_id, "Empresa Uno"),
        Err(EmpresaServiceError::Database(_))
    ));
    assert_eq!(
        servicio.buscar_por_id(segundo_id).unwrap().nombre,
        "Empresa Dos"
    );
}
