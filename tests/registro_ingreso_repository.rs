use chrono::NaiveDateTime;
use control_acceso::database::repositories::registro_ingreso_repository::{
    RegistroIngresoRepository,
    SqliteRegistroIngresoRepository,
};
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::registro_ingreso::RegistroIngreso;
use control_acceso::models::tipo_ingreso::TipoIngreso;

use rusqlite::Connection;

fn crear_base_de_prueba() -> Connection {
    let connection = Connection::open_in_memory()
        .expect("No se pudo crear la base de datos");

    control_acceso::database::schema::initialize_database(
        &connection,
    )
    .expect("No se pudo inicializar la base de datos");

    connection
}

fn crear_empresa(
    connection: &Connection,
) -> i64 {
    connection
        .execute(
            "
            INSERT INTO empresas (nombre)
            VALUES (?1)
            ",
            ["Empresa de Prueba"],
        )
        .expect("No se pudo crear la empresa");

    connection.last_insert_rowid()
}

fn crear_usuario(
    connection: &Connection,
) -> i64 {
    connection
        .execute(
            "
            INSERT INTO usuarios (
                cedula,
                nombre,
                password_hash,
                rol,
                activo
            )
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            [
                "10000001",
                "Usuario Prueba",
                "hash",
                "OPERADOR",
                "1",
            ],
        )
        .expect("No se pudo crear el usuario");

    connection.last_insert_rowid()
}

fn crear_contratista(
    connection: &Connection,
    empresa_id: i64,
) -> i64 {
    connection
        .execute(
            "
            INSERT INTO contratistas (
                cedula,
                nombre,
                empresa_id,
                tipo_ingreso,
                fecha_vencimiento_praind,
                tiene_acceso
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            [
                "20000001",
                "Contratista Prueba",
                &empresa_id.to_string(),
                "PRAIND",
                "2030-12-31",
                "1",
            ],
        )
        .expect("No se pudo crear el contratista");

    connection.last_insert_rowid()
}

fn crear_registro(
    contratista_id: i64,
    empresa_id: i64,
    usuario_id: i64,
) -> RegistroIngreso {
    RegistroIngreso {
        id: 0,
        contratista_id,
        empresa_id,
        fecha_hora_ingreso: NaiveDateTime::parse_from_str(
            "2026-08-11 08:30:00",
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap(),
        medio_ingreso: MedioIngreso::Vehiculo,
        tipo_ingreso: TipoIngreso::Praind,
        usuario_ingreso_id: usuario_id,
        fecha_hora_salida: None,
        usuario_salida_id: None,
    }
}

#[test]
fn debe_crear_y_recuperar_registro() {
    let connection = crear_base_de_prueba();

    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id =
        crear_contratista(&connection, empresa_id);

    let registro =
        crear_registro(
            contratista_id,
            empresa_id,
            usuario_id,
        );

    let repository =
        SqliteRegistroIngresoRepository::new(
            &connection,
        );

    let id = repository
        .crear(&registro)
        .expect("No se pudo crear el registro");

    let recuperado = repository
        .buscar_por_id(id)
        .expect("Error buscando el registro")
        .expect("El registro no fue encontrado");

    assert_eq!(recuperado.id, id);
    assert_eq!(
        recuperado.contratista_id,
        contratista_id
    );
    assert_eq!(
        recuperado.empresa_id,
        empresa_id
    );
    assert_eq!(
        recuperado.usuario_ingreso_id,
        usuario_id
    );
    assert_eq!(
        recuperado.medio_ingreso,
        MedioIngreso::Vehiculo
    );
    assert_eq!(
        recuperado.tipo_ingreso,
        TipoIngreso::Praind
    );
    assert!(recuperado.fecha_hora_salida.is_none());
    assert!(recuperado.usuario_salida_id.is_none());
}

#[test]
fn debe_detectar_ingreso_activo() {
    let connection = crear_base_de_prueba();

    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id =
        crear_contratista(&connection, empresa_id);

    let registro =
        crear_registro(
            contratista_id,
            empresa_id,
            usuario_id,
        );

    let repository =
        SqliteRegistroIngresoRepository::new(
            &connection,
        );

    repository
        .crear(&registro)
        .expect("No se pudo crear el registro");

    let activo = repository
        .buscar_ingreso_activo(contratista_id)
        .expect("Error buscando ingreso activo");

    assert!(activo.is_some());

    let activo = activo.unwrap();

    assert_eq!(
        activo.contratista_id,
        contratista_id
    );

    assert!(
        activo.fecha_hora_salida.is_none()
    );
}

#[test]
fn debe_dejar_de_ser_activo_al_registrar_salida() {
    let connection = crear_base_de_prueba();

    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id =
        crear_contratista(&connection, empresa_id);

    let registro =
        crear_registro(
            contratista_id,
            empresa_id,
            usuario_id,
        );

    let repository =
        SqliteRegistroIngresoRepository::new(
            &connection,
        );

    let id = repository
        .crear(&registro)
        .expect("No se pudo crear el registro");

    let fecha_salida =
        NaiveDateTime::parse_from_str(
            "2026-08-11 17:30:00",
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap();

    repository
        .registrar_salida(
            id,
            fecha_salida,
            usuario_id,
        )
        .expect("No se pudo registrar la salida");

    let activo = repository
        .buscar_ingreso_activo(contratista_id)
        .expect("Error buscando ingreso activo");

    assert!(activo.is_none());

    let recuperado = repository
        .buscar_por_id(id)
        .expect("Error buscando registro")
        .expect("Registro no encontrado");

    assert_eq!(
        recuperado.fecha_hora_salida,
        Some(fecha_salida)
    );

    assert_eq!(
        recuperado.usuario_salida_id,
        Some(usuario_id)
    );
}

#[test]
fn debe_listar_los_registros() {
    let connection = crear_base_de_prueba();

    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id =
        crear_contratista(&connection, empresa_id);

    let repository =
        SqliteRegistroIngresoRepository::new(
            &connection,
        );

    let registro =
        crear_registro(
            contratista_id,
            empresa_id,
            usuario_id,
        );

    repository
        .crear(&registro)
        .expect("No se pudo crear el registro");

    let registros = repository
        .listar()
        .expect("No se pudieron listar los registros");

    assert_eq!(registros.len(), 1);
    assert_eq!(
        registros[0].contratista_id,
        contratista_id
    );
}

#[test]
fn debe_retornar_none_si_no_hay_ingreso_activo() {
    let connection = crear_base_de_prueba();

    let empresa_id = crear_empresa(&connection);
    let contratista_id =
        crear_contratista(&connection, empresa_id);

    let repository =
        SqliteRegistroIngresoRepository::new(
            &connection,
        );

    let activo = repository
        .buscar_ingreso_activo(contratista_id)
        .expect("Error buscando ingreso activo");

    assert!(activo.is_none());
}