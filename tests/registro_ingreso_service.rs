use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rusqlite::Connection;

use control_acceso::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use control_acceso::database::repositories::gafete_repository::SqliteGafeteRepository;
use control_acceso::database::repositories::registro_ingreso_repository::{
    RegistroIngresoRepository, SqliteRegistroIngresoRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::domain::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};
use control_acceso::models::contratista::Contratista;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::registro_ingreso::SalidaRegistroIngreso;
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::services::error::RegistroIngresoServiceError;
use control_acceso::services::registro_ingreso_service::RegistroIngresoService;

fn preparar_base() -> (Connection, i64, i64) {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();

    connection
        .execute(
            "INSERT INTO empresas (nombre) VALUES ('Empresa principal')",
            [],
        )
        .unwrap();
    let empresa_id = connection.last_insert_rowid();

    connection
        .execute(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
             VALUES ('1001', 'Operador', 'hash', 'OPERADOR', 1)",
            [],
        )
        .unwrap();
    let usuario_id = connection.last_insert_rowid();

    connection
        .execute_batch(
            "INSERT INTO gafetes (numero, estado) VALUES
                (10, 'DISPONIBLE'), (11, 'DISPONIBLE'), (15, 'DISPONIBLE'),
                (20, 'DISPONIBLE'), (30, 'DISPONIBLE'), (40, 'DISPONIBLE')",
        )
        .unwrap();

    (connection, empresa_id, usuario_id)
}

fn fecha_ingreso() -> DateTime<Utc> {
    control_acceso::tiempo::local_costa_rica_a_utc(
        NaiveDateTime::parse_from_str("2026-08-11 08:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
    )
    .unwrap()
}

fn fecha_salida() -> DateTime<Utc> {
    control_acceso::tiempo::local_costa_rica_a_utc(
        NaiveDateTime::parse_from_str("2026-08-11 17:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
    )
    .unwrap()
}

fn contratista(
    cedula: &str,
    empresa_id: i64,
    tipo_ingreso: TipoIngreso,
    fecha_vencimiento_praind: Option<NaiveDate>,
) -> Contratista {
    Contratista::reconstruir(
        0,
        cedula.to_string(),
        format!("Contratista {cedula}"),
        empresa_id,
        tipo_ingreso,
        fecha_vencimiento_praind,
        false,
        true,
        true,
    )
}

fn guardar_contratista(connection: &Connection, contratista: &Contratista) -> i64 {
    SqliteContratistaRepository::new(connection)
        .crear(contratista)
        .unwrap()
}

fn praind_vigente() -> NaiveDate {
    NaiveDate::from_ymd_opt(2027, 12, 31).unwrap()
}

fn resultado_praind_con_vencimiento(
    vencimiento: NaiveDate,
) -> Result<ResultadoAcceso, RegistroIngresoServiceError> {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista("2001", empresa_id, TipoIngreso::Praind, Some(vencimiento)),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    RegistroIngresoService::new(&contratistas, &registros, &gafetes)
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            Some(10),
            usuario_id,
            fecha_ingreso(),
        )
        .map(|resultado| resultado.resultado_acceso)
}

#[test]
fn empresa_inactiva_deniega_registrar_entrada() {
    use control_acceso::database::repositories::empresa_repository::{
        EmpresaRepository, SqliteEmpresaRepository,
    };

    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista("2001", empresa_id, TipoIngreso::Swat, None),
    );
    SqliteEmpresaRepository::new(&connection)
        .establecer_activo(empresa_id, false)
        .unwrap();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);

    let resultado = RegistroIngresoService::new(&contratistas, &registros, &gafetes)
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            None,
            usuario_id,
            fecha_ingreso(),
        );

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::AccesoDenegado(
            MotivoDenegacion::EmpresaInactiva
        ))
    ));
}

#[test]
fn praind_con_mas_de_30_dias_propaga_permitido() {
    assert_eq!(
        resultado_praind_con_vencimiento(NaiveDate::from_ymd_opt(2026, 9, 11).unwrap()).unwrap(),
        ResultadoAcceso::Permitido
    );
}

#[test]
fn praind_en_30_dias_propaga_advertencia() {
    assert_eq!(
        resultado_praind_con_vencimiento(NaiveDate::from_ymd_opt(2026, 9, 10).unwrap()).unwrap(),
        ResultadoAcceso::PermitidoConAdvertencia
    );
}

#[test]
fn praind_que_vence_hoy_propaga_advertencia() {
    assert_eq!(
        resultado_praind_con_vencimiento(NaiveDate::from_ymd_opt(2026, 8, 11).unwrap()).unwrap(),
        ResultadoAcceso::PermitidoConAdvertencia
    );
}

#[test]
fn praind_en_31_dias_propaga_permitido() {
    assert_eq!(
        resultado_praind_con_vencimiento(NaiveDate::from_ymd_opt(2026, 9, 11).unwrap()).unwrap(),
        ResultadoAcceso::Permitido
    );
}

#[test]
fn praind_vencido_sigue_siendo_denegado() {
    assert!(matches!(
        resultado_praind_con_vencimiento(NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()),
        Err(RegistroIngresoServiceError::AccesoDenegado(
            MotivoDenegacion::PraindVencido
        ))
    ));
}

#[test]
fn praind_normal_con_gafete_libre_crea_ingreso() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista(
            "2001",
            empresa_id,
            TipoIngreso::Praind,
            Some(praind_vigente()),
        ),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let registro_id = servicio
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            Some(10),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();

    assert_eq!(
        registros
            .buscar_por_id(registro_id.registro_id)
            .unwrap()
            .unwrap()
            .gafete_numero,
        Some(10)
    );
}

#[test]
fn praind_normal_sin_gafete_es_rechazado() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista(
            "2001",
            empresa_id,
            TipoIngreso::Praind,
            Some(praind_vigente()),
        ),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let resultado = servicio.registrar_entrada(
        id,
        MedioIngreso::Caminando,
        None,
        usuario_id,
        fecha_ingreso(),
    );

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::GafeteRequerido)
    ));
}

#[test]
fn por_correo_con_gafete_libre_crea_ingreso() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista("2001", empresa_id, TipoIngreso::PorCorreo, None),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let registro_id = servicio
        .registrar_entrada(
            id,
            MedioIngreso::Vehiculo,
            Some(11),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();

    assert_eq!(
        registros
            .buscar_por_id(registro_id.registro_id)
            .unwrap()
            .unwrap()
            .gafete_numero,
        Some(11)
    );
}

#[test]
fn in_house_ignora_gafete_informado() {
    probar_ingreso_sin_gafete(TipoIngreso::InHouse, Some(praind_vigente()));
}

#[test]
fn swat_ignora_gafete_informado() {
    probar_ingreso_sin_gafete(TipoIngreso::Swat, None);
}

fn probar_ingreso_sin_gafete(
    tipo_ingreso: TipoIngreso,
    fecha_vencimiento_praind: Option<NaiveDate>,
) {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista("2001", empresa_id, tipo_ingreso, fecha_vencimiento_praind),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let registro_id = servicio
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            Some(99),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();

    assert_eq!(
        registros
            .buscar_por_id(registro_id.registro_id)
            .unwrap()
            .unwrap()
            .gafete_numero,
        None
    );
}

#[test]
fn personal_de_ruta_con_praind_vigente_guarda_none() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let mut persona = contratista(
        "2001",
        empresa_id,
        TipoIngreso::Praind,
        Some(praind_vigente()),
    );
    persona.es_personal_ruta = true;
    let id = guardar_contratista(&connection, &persona);
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let registro_id = servicio
        .registrar_entrada(
            id,
            MedioIngreso::Vehiculo,
            Some(15),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();

    assert_eq!(
        registros
            .buscar_por_id(registro_id.registro_id)
            .unwrap()
            .unwrap()
            .gafete_numero,
        None
    );
}

#[test]
fn personal_de_ruta_con_praind_vencido_es_rechazado() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let mut persona = contratista(
        "2001",
        empresa_id,
        TipoIngreso::PorCorreo,
        Some(NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()),
    );
    persona.es_personal_ruta = true;
    let id = guardar_contratista(&connection, &persona);
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let resultado = servicio.registrar_entrada(
        id,
        MedioIngreso::Caminando,
        None,
        usuario_id,
        fecha_ingreso(),
    );

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::AccesoDenegado(
            MotivoDenegacion::PraindVencido
        ))
    ));
}

#[test]
fn contratista_sin_acceso_es_rechazado() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let mut persona = contratista("2001", empresa_id, TipoIngreso::Swat, None);
    persona.tiene_acceso = false;
    let id = guardar_contratista(&connection, &persona);
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let resultado = servicio.registrar_entrada(
        id,
        MedioIngreso::Caminando,
        None,
        usuario_id,
        fecha_ingreso(),
    );

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::AccesoDenegado(
            MotivoDenegacion::SinAcceso
        ))
    ));
}

#[test]
fn contratista_con_ingreso_activo_es_rechazado() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista("2001", empresa_id, TipoIngreso::Swat, None),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);
    servicio
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            None,
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();

    let resultado = servicio.registrar_entrada(
        id,
        MedioIngreso::Caminando,
        None,
        usuario_id,
        fecha_ingreso(),
    );

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::IngresoActivo)
    ));
}

#[test]
fn gafete_ocupado_es_rechazado() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let primero = guardar_contratista(
        &connection,
        &contratista(
            "2001",
            empresa_id,
            TipoIngreso::Praind,
            Some(praind_vigente()),
        ),
    );
    let segundo = guardar_contratista(
        &connection,
        &contratista("2002", empresa_id, TipoIngreso::PorCorreo, None),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);
    servicio
        .registrar_entrada(
            primero,
            MedioIngreso::Caminando,
            Some(20),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();

    let resultado = servicio.registrar_entrada(
        segundo,
        MedioIngreso::Caminando,
        Some(20),
        usuario_id,
        fecha_ingreso(),
    );

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::GafeteOcupado)
    ));
}

#[test]
fn empresa_y_tipo_salen_del_contratista() {
    let (connection, _, usuario_id) = preparar_base();
    connection
        .execute(
            "INSERT INTO empresas (nombre) VALUES ('Empresa histórica')",
            [],
        )
        .unwrap();
    let empresa_id = connection.last_insert_rowid();
    let id = guardar_contratista(
        &connection,
        &contratista("2001", empresa_id, TipoIngreso::PorCorreo, None),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let registro_id = servicio
        .registrar_entrada(
            id,
            MedioIngreso::Vehiculo,
            Some(30),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();
    let registro = registros
        .buscar_por_id(registro_id.registro_id)
        .unwrap()
        .unwrap();

    assert_eq!(registro.empresa_id, empresa_id);
    assert_eq!(registro.tipo_ingreso, TipoIngreso::PorCorreo);
}

#[test]
fn salida_por_id_funciona() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista("2001", empresa_id, TipoIngreso::Swat, None),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);
    let registro_id = servicio
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            None,
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();

    servicio
        .registrar_salida(registro_id.registro_id, fecha_salida(), usuario_id)
        .unwrap();

    assert!(registros.buscar_ingreso_activo(id).unwrap().is_none());
}

#[test]
fn salida_igual_al_ingreso_es_permitida_y_conserva_usuario() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista("2001", empresa_id, TipoIngreso::Swat, None),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);
    let entrada = servicio
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            None,
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();

    servicio
        .registrar_salida(entrada.registro_id, fecha_ingreso(), usuario_id)
        .unwrap();

    let cerrado = registros
        .buscar_por_id(entrada.registro_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        cerrado.salida,
        Some(SalidaRegistroIngreso {
            fecha_hora: fecha_ingreso(),
            usuario_id,
        })
    );
}

#[test]
fn salida_anterior_es_rechazada_y_no_modifica_sqlite() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista("2001", empresa_id, TipoIngreso::Swat, None),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);
    let entrada = servicio
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            None,
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();
    let salida_anterior = control_acceso::tiempo::local_costa_rica_a_utc(
        NaiveDateTime::parse_from_str("2026-08-11 07:59:59", "%Y-%m-%d %H:%M:%S").unwrap(),
    )
    .unwrap();

    assert!(matches!(
        servicio.registrar_salida(entrada.registro_id, salida_anterior, usuario_id),
        Err(RegistroIngresoServiceError::SalidaAnteriorAIngreso)
    ));
    let registro = registros
        .buscar_por_id(entrada.registro_id)
        .unwrap()
        .unwrap();
    assert!(registro.salida.is_none());
}

#[test]
fn salida_por_gafete_funciona() {
    let (connection, empresa_id, usuario_id) = preparar_base();
    let id = guardar_contratista(
        &connection,
        &contratista(
            "2001",
            empresa_id,
            TipoIngreso::Praind,
            Some(praind_vigente()),
        ),
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);
    servicio
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            Some(40),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();

    servicio
        .registrar_salida_por_gafete(40, fecha_salida(), usuario_id)
        .unwrap();

    assert!(
        registros
            .buscar_ingreso_activo_por_gafete(40)
            .unwrap()
            .is_none()
    );
}

#[test]
fn gafete_no_asignado_produce_error() {
    let (connection, _, usuario_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    assert!(matches!(
        servicio.buscar_ingreso_activo_por_gafete(99),
        Err(RegistroIngresoServiceError::GafeteNoAsignado)
    ));
    assert!(matches!(
        servicio.registrar_salida_por_gafete(99, fecha_salida(), usuario_id),
        Err(RegistroIngresoServiceError::GafeteNoAsignado)
    ));
}

#[test]
fn contratista_inexistente_es_rechazado() {
    let (connection, _, usuario_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let resultado = servicio.registrar_entrada(
        999,
        MedioIngreso::Caminando,
        None,
        usuario_id,
        fecha_ingreso(),
    );

    assert!(matches!(
        resultado,
        Err(RegistroIngresoServiceError::ContratistaNoEncontrado)
    ));
}
