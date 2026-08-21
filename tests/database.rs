use chrono::NaiveDate;

use control_acceso::database::connection::open_database;
use control_acceso::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use control_acceso::database::repositories::empresa_repository::{
    EmpresaRepository, SqliteEmpresaRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::contratista::Contratista;
use control_acceso::models::empresa::Empresa;
use control_acceso::models::tipo_ingreso::TipoIngreso;

fn preparar_base() -> rusqlite::Connection {
    let connection = open_database(":memory:").expect("No se pudo abrir la base de datos");

    initialize_database(&connection).expect("No se pudo inicializar la base de datos");

    connection
}

#[test]
fn debe_crear_y_recuperar_empresa() {
    let connection = preparar_base();

    let repository = SqliteEmpresaRepository::new(&connection);

    let empresa = Empresa {
        id: 0,
        nombre: "Empresa de Prueba".to_string(),
        activo: true,
    };

    let id = repository
        .crear(&empresa)
        .expect("No se pudo crear la empresa");

    let recuperada = repository
        .buscar_por_id(id)
        .expect("Error buscando empresa")
        .expect("La empresa no fue encontrada");

    assert_eq!(recuperada.id, id);
    assert_eq!(recuperada.nombre, "Empresa de Prueba");
}

#[test]
fn debe_retornar_none_si_la_cedula_no_existe() {
    let connection = preparar_base();

    let repository = SqliteContratistaRepository::new(&connection);

    let resultado = repository
        .buscar_por_cedula("999999999")
        .expect("La consulta produjo un error");

    assert!(resultado.is_none());
}

#[test]
fn debe_actualizar_un_contratista() {
    let connection = preparar_base();

    let empresa_repository = SqliteEmpresaRepository::new(&connection);

    let contratista_repository = SqliteContratistaRepository::new(&connection);

    let empresa = Empresa {
        id: 0,
        nombre: "Empresa Original".to_string(),
        activo: true,
    };

    let empresa_id = empresa_repository
        .crear(&empresa)
        .expect("No se pudo crear la empresa");

    let contratista = Contratista {
        id: 0,
        cedula: "109870123".to_string(),
        nombre: "Juan Pérez".to_string(),
        empresa_id,
        tipo_ingreso: TipoIngreso::Praind,
        fecha_vencimiento_praind: Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        es_personal_ruta: false,
        tiene_acceso: true,
        empresa_activa: true,
    };

    let id = contratista_repository
        .crear(&contratista)
        .expect("No se pudo crear el contratista");

    let actualizado = Contratista {
        id,
        cedula: "209870123".to_string(),
        nombre: "Juan Pérez Actualizado".to_string(),
        empresa_id,
        tipo_ingreso: TipoIngreso::InHouse,
        fecha_vencimiento_praind: Some(NaiveDate::from_ymd_opt(2027, 12, 31).unwrap()),
        es_personal_ruta: false,
        tiene_acceso: false,
        empresa_activa: true,
    };

    contratista_repository
        .actualizar(&actualizado)
        .expect("No se pudo actualizar el contratista");

    let recuperado = contratista_repository
        .buscar_por_cedula("209870123")
        .expect("Error buscando contratista")
        .expect("El contratista no fue encontrado");

    assert_eq!(recuperado.nombre, "Juan Pérez Actualizado");
    assert_eq!(recuperado.cedula, "209870123");
    assert!(
        contratista_repository
            .buscar_por_cedula("109870123")
            .unwrap()
            .is_none()
    );

    assert_eq!(recuperado.tipo_ingreso, TipoIngreso::InHouse);

    assert_eq!(
        recuperado.fecha_vencimiento_praind,
        Some(NaiveDate::from_ymd_opt(2027, 12, 31).unwrap())
    );

    assert!(!recuperado.tiene_acceso);
}

#[test]
fn debe_guardar_los_cuatro_tipos_de_ingreso() {
    let connection = preparar_base();

    let empresa_repository = SqliteEmpresaRepository::new(&connection);

    let contratista_repository = SqliteContratistaRepository::new(&connection);

    let empresa = Empresa {
        id: 0,
        nombre: "Empresa de Prueba".to_string(),
        activo: true,
    };

    let empresa_id = empresa_repository
        .crear(&empresa)
        .expect("No se pudo crear la empresa");

    let casos = [
        (
            "100000001",
            TipoIngreso::Praind,
            Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        ),
        (
            "100000002",
            TipoIngreso::InHouse,
            Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        ),
        ("100000003", TipoIngreso::PorCorreo, None),
        ("100000004", TipoIngreso::Swat, None),
    ];

    for (cedula, tipo_ingreso, fecha) in casos {
        let contratista = Contratista {
            id: 0,
            cedula: cedula.to_string(),
            nombre: "Contratista de Prueba".to_string(),
            empresa_id,
            tipo_ingreso,
            fecha_vencimiento_praind: fecha,
            es_personal_ruta: false,
            tiene_acceso: true,
            empresa_activa: true,
        };

        contratista_repository
            .crear(&contratista)
            .expect("No se pudo crear el contratista");
    }

    let praind = contratista_repository
        .buscar_por_cedula("100000001")
        .unwrap()
        .unwrap();

    assert_eq!(praind.tipo_ingreso, TipoIngreso::Praind);

    assert!(praind.fecha_vencimiento_praind.is_some());

    let in_house = contratista_repository
        .buscar_por_cedula("100000002")
        .unwrap()
        .unwrap();

    assert_eq!(in_house.tipo_ingreso, TipoIngreso::InHouse);

    assert!(in_house.fecha_vencimiento_praind.is_some());

    let por_correo = contratista_repository
        .buscar_por_cedula("100000003")
        .unwrap()
        .unwrap();

    assert_eq!(por_correo.tipo_ingreso, TipoIngreso::PorCorreo);

    assert!(por_correo.fecha_vencimiento_praind.is_none());

    let swat = contratista_repository
        .buscar_por_cedula("100000004")
        .unwrap()
        .unwrap();

    assert_eq!(swat.tipo_ingreso, TipoIngreso::Swat);

    assert!(swat.fecha_vencimiento_praind.is_none());
}

#[test]
fn debe_identificar_si_un_tipo_requiere_praind() {
    let empresa_id = 1;

    for (tipo_ingreso, esperado) in [
        (TipoIngreso::Praind, true),
        (TipoIngreso::InHouse, true),
        (TipoIngreso::PorCorreo, false),
        (TipoIngreso::Swat, false),
    ] {
        let contratista = Contratista {
            id: 0,
            cedula: String::new(),
            nombre: String::new(),
            empresa_id,
            tipo_ingreso,
            fecha_vencimiento_praind: None,
            es_personal_ruta: false,
            tiene_acceso: true,
            empresa_activa: true,
        };

        assert_eq!(contratista.requiere_praind(), esperado);
    }
}
