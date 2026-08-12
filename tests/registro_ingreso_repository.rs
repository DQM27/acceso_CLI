use chrono::NaiveDateTime;
use control_acceso::database::error::DatabaseError;
use control_acceso::database::repositories::registro_ingreso_repository::{
    RegistroIngresoRepository, SqliteRegistroIngresoRepository,
};
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::registro_ingreso::RegistroIngreso;
use control_acceso::models::tipo_ingreso::TipoIngreso;

use rusqlite::Connection;

fn crear_base_de_prueba() -> Connection {
    let connection = Connection::open_in_memory().expect("No se pudo crear la base de datos");

    control_acceso::database::schema::initialize_database(&connection)
        .expect("No se pudo inicializar la base de datos");

    connection
}

fn crear_empresa(connection: &Connection) -> i64 {
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

fn crear_usuario(connection: &Connection) -> i64 {
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
            ["10000001", "Usuario Prueba", "hash", "OPERADOR", "1"],
        )
        .expect("No se pudo crear el usuario");

    connection.last_insert_rowid()
}

fn crear_contratista(connection: &Connection, empresa_id: i64) -> i64 {
    crear_contratista_con_cedula(connection, empresa_id, "20000001")
}

fn crear_contratista_con_cedula(connection: &Connection, empresa_id: i64, cedula: &str) -> i64 {
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
                cedula,
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

fn fecha_salida() -> NaiveDateTime {
    NaiveDateTime::parse_from_str("2026-08-11 17:30:00", "%Y-%m-%d %H:%M:%S").unwrap()
}

fn crear_registro(contratista_id: i64, empresa_id: i64, usuario_id: i64) -> RegistroIngreso {
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
        gafete_numero: None,
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
    let contratista_id = crear_contratista(&connection, empresa_id);

    let registro = crear_registro(contratista_id, empresa_id, usuario_id);

    let repository = SqliteRegistroIngresoRepository::new(&connection);

    let id = repository
        .crear(&registro)
        .expect("No se pudo crear el registro");

    let recuperado = repository
        .buscar_por_id(id)
        .expect("Error buscando el registro")
        .expect("El registro no fue encontrado");

    assert_eq!(recuperado.id, id);
    assert_eq!(recuperado.contratista_id, contratista_id);
    assert_eq!(recuperado.empresa_id, empresa_id);
    assert_eq!(recuperado.usuario_ingreso_id, usuario_id);
    assert_eq!(recuperado.medio_ingreso, MedioIngreso::Vehiculo);
    assert_eq!(recuperado.tipo_ingreso, TipoIngreso::Praind);
    assert!(recuperado.fecha_hora_salida.is_none());
    assert!(recuperado.usuario_salida_id.is_none());
}

#[test]
fn debe_detectar_ingreso_activo() {
    let connection = crear_base_de_prueba();

    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id = crear_contratista(&connection, empresa_id);

    let registro = crear_registro(contratista_id, empresa_id, usuario_id);

    let repository = SqliteRegistroIngresoRepository::new(&connection);

    repository
        .crear(&registro)
        .expect("No se pudo crear el registro");

    let activo = repository
        .buscar_ingreso_activo(contratista_id)
        .expect("Error buscando ingreso activo");

    assert!(activo.is_some());

    let activo = activo.unwrap();

    assert_eq!(activo.contratista_id, contratista_id);

    assert!(activo.fecha_hora_salida.is_none());
}

#[test]
fn debe_dejar_de_ser_activo_al_registrar_salida() {
    let connection = crear_base_de_prueba();

    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id = crear_contratista(&connection, empresa_id);

    let registro = crear_registro(contratista_id, empresa_id, usuario_id);

    let repository = SqliteRegistroIngresoRepository::new(&connection);

    let id = repository
        .crear(&registro)
        .expect("No se pudo crear el registro");

    let fecha_salida = fecha_salida();

    repository
        .registrar_salida(id, fecha_salida, usuario_id)
        .expect("No se pudo registrar la salida");

    let activo = repository
        .buscar_ingreso_activo(contratista_id)
        .expect("Error buscando ingreso activo");

    assert!(activo.is_none());

    let recuperado = repository
        .buscar_por_id(id)
        .expect("Error buscando registro")
        .expect("Registro no encontrado");

    assert_eq!(recuperado.fecha_hora_salida, Some(fecha_salida));

    assert_eq!(recuperado.usuario_salida_id, Some(usuario_id));
}

#[test]
fn debe_listar_los_registros() {
    let connection = crear_base_de_prueba();

    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id = crear_contratista(&connection, empresa_id);

    let repository = SqliteRegistroIngresoRepository::new(&connection);

    let registro = crear_registro(contratista_id, empresa_id, usuario_id);

    repository
        .crear(&registro)
        .expect("No se pudo crear el registro");

    let registros = repository
        .listar()
        .expect("No se pudieron listar los registros");

    assert_eq!(registros.len(), 1);
    assert_eq!(registros[0].contratista_id, contratista_id);
}

#[test]
fn debe_retornar_none_si_no_hay_ingreso_activo() {
    let connection = crear_base_de_prueba();

    let empresa_id = crear_empresa(&connection);
    let contratista_id = crear_contratista(&connection, empresa_id);

    let repository = SqliteRegistroIngresoRepository::new(&connection);

    let activo = repository
        .buscar_ingreso_activo(contratista_id)
        .expect("Error buscando ingreso activo");

    assert!(activo.is_none());
}

#[test]
fn debe_impedir_dos_ingresos_activos_del_mismo_contratista() {
    let connection = crear_base_de_prueba();

    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id = crear_contratista(&connection, empresa_id);

    let repository = SqliteRegistroIngresoRepository::new(&connection);

    let registro = crear_registro(contratista_id, empresa_id, usuario_id);

    repository
        .crear(&registro)
        .expect("No se pudo crear el primer ingreso");

    let segundo_resultado = repository.crear(&registro);

    assert!(segundo_resultado.is_err());
}

#[test]
fn debe_rechazar_una_segunda_salida() {
    let connection = crear_base_de_prueba();
    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id = crear_contratista(&connection, empresa_id);
    let repository = SqliteRegistroIngresoRepository::new(&connection);
    let registro = crear_registro(contratista_id, empresa_id, usuario_id);
    let id = repository.crear(&registro).unwrap();

    repository
        .registrar_salida(id, fecha_salida(), usuario_id)
        .unwrap();

    let resultado = repository.registrar_salida(id, fecha_salida(), usuario_id);

    assert!(matches!(resultado, Err(DatabaseError::RegistroNoActivo)));
}

#[test]
fn debe_rechazar_salida_de_id_inexistente() {
    let connection = crear_base_de_prueba();
    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    crear_contratista(&connection, empresa_id);
    let repository = SqliteRegistroIngresoRepository::new(&connection);

    let resultado = repository.registrar_salida(999, fecha_salida(), usuario_id);

    assert!(matches!(resultado, Err(DatabaseError::RegistroNoActivo)));
}

#[test]
fn debe_guardar_y_recuperar_gafete() {
    let connection = crear_base_de_prueba();
    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id = crear_contratista(&connection, empresa_id);
    let repository = SqliteRegistroIngresoRepository::new(&connection);
    let mut registro = crear_registro(contratista_id, empresa_id, usuario_id);
    registro.gafete_numero = Some(12);

    let id = repository.crear(&registro).unwrap();
    let recuperado = repository.buscar_por_id(id).unwrap().unwrap();

    assert_eq!(recuperado.gafete_numero, Some(12));
}

#[test]
fn debe_guardar_ingreso_sin_gafete_como_null() {
    let connection = crear_base_de_prueba();
    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id = crear_contratista(&connection, empresa_id);
    let repository = SqliteRegistroIngresoRepository::new(&connection);
    let registro = crear_registro(contratista_id, empresa_id, usuario_id);

    let id = repository.crear(&registro).unwrap();
    let es_null: bool = connection
        .query_row(
            "SELECT gafete_numero IS NULL FROM registro_ingresos WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .unwrap();

    assert!(es_null);
}

#[test]
fn debe_buscar_ingreso_activo_por_gafete() {
    let connection = crear_base_de_prueba();
    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id = crear_contratista(&connection, empresa_id);
    let repository = SqliteRegistroIngresoRepository::new(&connection);
    let mut registro = crear_registro(contratista_id, empresa_id, usuario_id);
    registro.gafete_numero = Some(23);
    let id = repository.crear(&registro).unwrap();

    let encontrado = repository
        .buscar_ingreso_activo_por_gafete(23)
        .unwrap()
        .unwrap();

    assert_eq!(encontrado.id, id);
    assert_eq!(encontrado.contratista_id, contratista_id);
}

#[test]
fn debe_retornar_none_al_buscar_gafete_despues_de_salida() {
    let connection = crear_base_de_prueba();
    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let contratista_id = crear_contratista(&connection, empresa_id);
    let repository = SqliteRegistroIngresoRepository::new(&connection);
    let mut registro = crear_registro(contratista_id, empresa_id, usuario_id);
    registro.gafete_numero = Some(24);
    let id = repository.crear(&registro).unwrap();

    repository
        .registrar_salida(id, fecha_salida(), usuario_id)
        .unwrap();

    assert!(
        repository
            .buscar_ingreso_activo_por_gafete(24)
            .unwrap()
            .is_none()
    );
}

#[test]
fn debe_impedir_mismo_gafete_en_dos_ingresos_activos() {
    let connection = crear_base_de_prueba();
    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let primer_contratista = crear_contratista(&connection, empresa_id);
    let segundo_contratista = crear_contratista_con_cedula(&connection, empresa_id, "20000002");
    let repository = SqliteRegistroIngresoRepository::new(&connection);
    let mut primero = crear_registro(primer_contratista, empresa_id, usuario_id);
    primero.gafete_numero = Some(25);
    let mut segundo = crear_registro(segundo_contratista, empresa_id, usuario_id);
    segundo.gafete_numero = Some(25);

    repository.crear(&primero).unwrap();

    assert!(repository.crear(&segundo).is_err());
}

#[test]
fn debe_permitir_reutilizar_gafete_despues_de_salida() {
    let connection = crear_base_de_prueba();
    let empresa_id = crear_empresa(&connection);
    let usuario_id = crear_usuario(&connection);
    let primer_contratista = crear_contratista(&connection, empresa_id);
    let segundo_contratista = crear_contratista_con_cedula(&connection, empresa_id, "20000002");
    let repository = SqliteRegistroIngresoRepository::new(&connection);
    let mut primero = crear_registro(primer_contratista, empresa_id, usuario_id);
    primero.gafete_numero = Some(26);
    let primer_id = repository.crear(&primero).unwrap();
    repository
        .registrar_salida(primer_id, fecha_salida(), usuario_id)
        .unwrap();

    let mut segundo = crear_registro(segundo_contratista, empresa_id, usuario_id);
    segundo.gafete_numero = Some(26);
    let segundo_id = repository.crear(&segundo).unwrap();

    assert_eq!(
        repository
            .buscar_ingreso_activo_por_gafete(26)
            .unwrap()
            .unwrap()
            .id,
        segundo_id
    );
}
