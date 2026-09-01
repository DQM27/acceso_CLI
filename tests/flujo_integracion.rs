use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rusqlite::Connection;

use control_acceso::database::repositories::contratista_repository::SqliteContratistaRepository;
use control_acceso::database::repositories::empresa_repository::SqliteEmpresaRepository;
use control_acceso::database::repositories::gafete_repository::SqliteGafeteRepository;
use control_acceso::database::repositories::registro_ingreso_repository::{
    RegistroIngresoRepository, SqliteRegistroIngresoRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::domain::resultado_acceso::MotivoDenegacion;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::registro_ingreso::{
    DatosHistoricosEntrada, NuevoRegistroIngreso, ResultadoIngresoRegistrado,
    SalidaRegistroIngreso, VERSION_REGLAS_ACCESO,
};
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::services::contratista_service::{ContratistaService, DatosContratista};
use control_acceso::services::empresa_service::EmpresaService;
use control_acceso::services::error::RegistroIngresoServiceError;
use control_acceso::services::registro_ingreso_service::RegistroIngresoService;

fn preparar_base() -> (Connection, i64) {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
             VALUES ('1001', 'Operador integración', 'hash', 'OPERADOR', 1)",
            [],
        )
        .unwrap();
    let usuario_id = connection.last_insert_rowid();
    connection
        .execute_batch(
            "INSERT INTO gafetes (numero, estado) VALUES
                (5, 'DISPONIBLE'), (6, 'DISPONIBLE'), (8, 'DISPONIBLE')",
        )
        .unwrap();
    (connection, usuario_id)
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

fn praind_vigente() -> NaiveDate {
    NaiveDate::from_ymd_opt(2027, 12, 31).unwrap()
}

fn praind_vencido() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, 10).unwrap()
}

fn crear_empresa(connection: &Connection) -> i64 {
    let repository = SqliteEmpresaRepository::new(connection);
    EmpresaService::new(&repository)
        .crear("Empresa Prueba")
        .unwrap()
}

fn crear_contratista(
    connection: &Connection,
    empresa_id: i64,
    cedula: &str,
    tipo_ingreso: TipoIngreso,
    fecha_vencimiento_praind: Option<NaiveDate>,
    es_personal_ruta: bool,
    tiene_acceso: bool,
) -> i64 {
    let contratistas = SqliteContratistaRepository::new(connection);
    let empresas = SqliteEmpresaRepository::new(connection);
    ContratistaService::new(&contratistas, &empresas)
        .crear(DatosContratista {
            cedula: cedula.to_string(),
            nombre: format!("Contratista {cedula}"),
            empresa_id,
            tipo_ingreso,
            fecha_vencimiento_praind,
            es_personal_ruta,
            tiene_acceso,
        })
        .unwrap()
}

#[test]
// Un único escenario de extremo a extremo; partirlo perdería la comprobación
// de que el mismo gafete se libera y reutiliza sobre el estado persistido.
#[allow(clippy::too_many_lines)]
fn flujo_completo_praind_libera_y_reutiliza_gafete() {
    let (connection, usuario_id) = preparar_base();
    let empresa_id = crear_empresa(&connection);
    let primer_contratista_id = crear_contratista(
        &connection,
        empresa_id,
        "2001",
        TipoIngreso::Praind,
        Some(praind_vigente()),
        false,
        true,
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let contratista_service = ContratistaService::new(&contratistas, &empresas);
    let ingreso_service = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let contratista = contratista_service
        .buscar_por_id(primer_contratista_id)
        .unwrap();
    assert_eq!(contratista.empresa_id, empresa_id);
    assert_eq!(contratista.tipo_ingreso, TipoIngreso::Praind);
    assert_eq!(contratista.fecha_vencimiento_praind, Some(praind_vigente()));
    assert!(contratista.tiene_acceso);
    assert!(!contratista.es_personal_ruta);

    let primer_ingreso_id = ingreso_service
        .registrar_entrada(
            primer_contratista_id,
            MedioIngreso::Vehiculo,
            Some(5),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();
    let primer_ingreso = registros
        .buscar_por_id(primer_ingreso_id.registro_id)
        .unwrap()
        .unwrap();
    assert_eq!(primer_ingreso.contratista_id, primer_contratista_id);
    assert_eq!(primer_ingreso.empresa_id, empresa_id);
    assert_eq!(primer_ingreso.tipo_ingreso, TipoIngreso::Praind);
    assert_eq!(primer_ingreso.gafete_numero, Some(5));
    assert!(primer_ingreso.salida.is_none());
    assert_eq!(
        ingreso_service
            .buscar_ingreso_activo_por_gafete(5)
            .unwrap()
            .id,
        primer_ingreso_id.registro_id
    );

    assert!(matches!(
        ingreso_service.registrar_entrada(
            primer_contratista_id,
            MedioIngreso::Caminando,
            Some(6),
            usuario_id,
            fecha_ingreso(),
        ),
        Err(RegistroIngresoServiceError::IngresoActivo)
    ));

    let segundo_contratista_id = crear_contratista(
        &connection,
        empresa_id,
        "2002",
        TipoIngreso::Praind,
        Some(praind_vigente()),
        false,
        true,
    );
    assert!(matches!(
        ingreso_service.registrar_entrada(
            segundo_contratista_id,
            MedioIngreso::Caminando,
            Some(5),
            usuario_id,
            fecha_ingreso(),
        ),
        Err(RegistroIngresoServiceError::GafeteOcupado)
    ));

    ingreso_service
        .registrar_salida_por_gafete(5, fecha_salida(), usuario_id)
        .unwrap();
    let cerrado = registros
        .buscar_por_id(primer_ingreso_id.registro_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        cerrado.salida,
        Some(SalidaRegistroIngreso {
            fecha_hora: fecha_salida(),
            usuario_id,
        })
    );
    assert!(
        registros
            .buscar_ingreso_activo(primer_contratista_id)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        ingreso_service.buscar_ingreso_activo_por_gafete(5),
        Err(RegistroIngresoServiceError::GafeteNoAsignado)
    ));

    let segundo_ingreso_id = ingreso_service
        .registrar_entrada(
            segundo_contratista_id,
            MedioIngreso::Caminando,
            Some(5),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();
    assert_eq!(
        ingreso_service
            .buscar_ingreso_activo_por_gafete(5)
            .unwrap()
            .id,
        segundo_ingreso_id.registro_id
    );
}

fn comprobar_flujo_sin_gafete(
    tipo_ingreso: TipoIngreso,
    fecha_praind: Option<NaiveDate>,
    es_personal_ruta: bool,
) {
    let (connection, usuario_id) = preparar_base();
    let empresa_id = crear_empresa(&connection);
    let contratista_id = crear_contratista(
        &connection,
        empresa_id,
        "2001",
        tipo_ingreso,
        fecha_praind,
        es_personal_ruta,
        true,
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    let ingreso_id = servicio
        .registrar_entrada(
            contratista_id,
            MedioIngreso::Caminando,
            Some(99),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();

    assert_eq!(
        registros
            .buscar_por_id(ingreso_id.registro_id)
            .unwrap()
            .unwrap()
            .gafete_numero,
        None
    );
}

#[test]
fn flujo_in_house_ignora_gafete_y_guarda_none() {
    comprobar_flujo_sin_gafete(TipoIngreso::InHouse, Some(praind_vigente()), false);
}

#[test]
fn flujo_swat_ignora_gafete_y_guarda_none() {
    comprobar_flujo_sin_gafete(TipoIngreso::Swat, None, false);
}

#[test]
fn flujo_personal_de_ruta_vigente_ignora_gafete_y_guarda_none() {
    comprobar_flujo_sin_gafete(TipoIngreso::PorCorreo, Some(praind_vigente()), true);
}

#[test]
fn flujo_por_correo_exige_y_persiste_gafete() {
    let (connection, usuario_id) = preparar_base();
    let empresa_id = crear_empresa(&connection);
    let contratista_id = crear_contratista(
        &connection,
        empresa_id,
        "2001",
        TipoIngreso::PorCorreo,
        None,
        false,
        true,
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros, &gafetes);

    assert!(matches!(
        servicio.registrar_entrada(
            contratista_id,
            MedioIngreso::Vehiculo,
            None,
            usuario_id,
            fecha_ingreso(),
        ),
        Err(RegistroIngresoServiceError::GafeteRequerido)
    ));

    let ingreso_id = servicio
        .registrar_entrada(
            contratista_id,
            MedioIngreso::Vehiculo,
            Some(8),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap();
    let ingreso = registros
        .buscar_por_id(ingreso_id.registro_id)
        .unwrap()
        .unwrap();
    assert_eq!(ingreso.empresa_id, empresa_id);
    assert_eq!(ingreso.contratista_id, contratista_id);
    assert_eq!(ingreso.tipo_ingreso, TipoIngreso::PorCorreo);
    assert_eq!(ingreso.medio_ingreso, MedioIngreso::Vehiculo);
    assert_eq!(ingreso.gafete_numero, Some(8));
}

fn intentar_ingreso_restringido(
    tipo_ingreso: TipoIngreso,
    fecha_praind: Option<NaiveDate>,
    es_personal_ruta: bool,
    tiene_acceso: bool,
) -> RegistroIngresoServiceError {
    let (connection, usuario_id) = preparar_base();
    let empresa_id = crear_empresa(&connection);
    let contratista_id = crear_contratista(
        &connection,
        empresa_id,
        "2001",
        tipo_ingreso,
        fecha_praind,
        es_personal_ruta,
        tiene_acceso,
    );
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let gafetes = SqliteGafeteRepository::new(&connection);
    RegistroIngresoService::new(&contratistas, &registros, &gafetes)
        .registrar_entrada(
            contratista_id,
            MedioIngreso::Caminando,
            Some(10),
            usuario_id,
            fecha_ingreso(),
        )
        .unwrap_err()
}

#[test]
fn flujo_rechaza_contratista_sin_acceso() {
    assert!(matches!(
        intentar_ingreso_restringido(TipoIngreso::Swat, None, false, false),
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::SinAcceso)
    ));
}

#[test]
fn flujo_rechaza_praind_vencido() {
    assert!(matches!(
        intentar_ingreso_restringido(TipoIngreso::Praind, Some(praind_vencido()), false, true,),
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::PraindVencido)
    ));
}

#[test]
fn flujo_rechaza_in_house_con_praind_vencido() {
    assert!(matches!(
        intentar_ingreso_restringido(TipoIngreso::InHouse, Some(praind_vencido()), false, true,),
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::PraindVencido)
    ));
}

#[test]
fn flujo_rechaza_personal_de_ruta_con_praind_vencido() {
    assert!(matches!(
        intentar_ingreso_restringido(TipoIngreso::PorCorreo, Some(praind_vencido()), true, true,),
        RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::PraindVencido)
    ));
}

#[test]
fn sqlite_impide_ingreso_activo_y_gafete_activo_duplicados() {
    let (connection, usuario_id) = preparar_base();
    let empresa_id = crear_empresa(&connection);
    let primer_contratista = crear_contratista(
        &connection,
        empresa_id,
        "2001",
        TipoIngreso::Praind,
        Some(praind_vigente()),
        false,
        true,
    );
    let segundo_contratista = crear_contratista(
        &connection,
        empresa_id,
        "2002",
        TipoIngreso::Praind,
        Some(praind_vigente()),
        false,
        true,
    );
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let registro = |contratista_id, gafete_numero| NuevoRegistroIngreso {
        contratista_id,
        empresa_id,
        fecha_hora_ingreso: fecha_ingreso(),
        medio_ingreso: MedioIngreso::Caminando,
        tipo_ingreso: TipoIngreso::Praind,
        gafete_numero: Some(gafete_numero),
        usuario_ingreso_id: usuario_id,
        datos_historicos: DatosHistoricosEntrada {
            contratista_cedula: contratista_id.to_string(),
            contratista_nombre: format!("Contratista {contratista_id}"),
            fecha_vencimiento_praind: Some(praind_vigente()),
            es_personal_ruta: false,
            tiene_acceso: true,
            empresa_activa: true,
            resultado_acceso: ResultadoIngresoRegistrado::Permitido,
            reglas_version: VERSION_REGLAS_ACCESO,
        },
    };

    registros.crear(&registro(primer_contratista, 5)).unwrap();

    assert!(registros.crear(&registro(primer_contratista, 6)).is_err());
    assert!(registros.crear(&registro(segundo_contratista, 5)).is_err());
    assert_eq!(registros.listar().unwrap().len(), 1);
}
