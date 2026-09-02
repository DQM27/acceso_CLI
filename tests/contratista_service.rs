use chrono::NaiveDate;
use rusqlite::Connection;

use control_acceso::database::repositories::contratista_repository::SqliteContratistaRepository;
use control_acceso::database::repositories::empresa_repository::{
    EmpresaRepository, SqliteEmpresaRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::empresa::Empresa;
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::services::contratista_service::{
    ContratistaService, DatosActualizacionContratista, DatosContratista,
};
use control_acceso::services::error::ContratistaServiceError;

fn preparar_base() -> (Connection, i64) {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let empresa_id = SqliteEmpresaRepository::new(&connection)
        .crear(&Empresa {
            id: 0,
            nombre: "Empresa principal".to_string(),
            activo: true,
        })
        .unwrap();
    (connection, empresa_id)
}

fn fecha_praind() -> NaiveDate {
    NaiveDate::from_ymd_opt(2027, 12, 31).unwrap()
}

fn datos(empresa_id: i64, tipo_ingreso: TipoIngreso) -> DatosContratista {
    DatosContratista {
        cedula: "2001".to_string(),
        nombre: "Persona Uno".to_string(),
        empresa_id,
        tipo_ingreso,
        fecha_vencimiento_praind: None,
        es_personal_ruta: false,
        tiene_acceso: true,
    }
}

fn actualizacion(empresa_id: i64, tipo_ingreso: TipoIngreso) -> DatosActualizacionContratista {
    DatosActualizacionContratista {
        cedula: "2001".to_string(),
        nombre: "Persona Uno".to_string(),
        empresa_id,
        tipo_ingreso,
        fecha_vencimiento_praind: None,
        es_personal_ruta: false,
        tiene_acceso: true,
    }
}

fn crear_y_recuperar(tipo: TipoIngreso, fecha: Option<NaiveDate>, ruta: bool) {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let mut entrada = datos(empresa_id, tipo);
    entrada.fecha_vencimiento_praind = fecha;
    entrada.es_personal_ruta = ruta;

    let id = servicio.crear(entrada).unwrap();
    let creado = servicio.buscar_por_id(id).unwrap();

    assert_eq!(creado.tipo_ingreso, tipo);
    assert_eq!(creado.es_personal_ruta, ruta);
}

fn crear_resultado(
    tipo: TipoIngreso,
    fecha: Option<NaiveDate>,
    ruta: bool,
) -> Result<i64, ContratistaServiceError> {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let mut entrada = datos(empresa_id, tipo);
    entrada.fecha_vencimiento_praind = fecha;
    entrada.es_personal_ruta = ruta;
    servicio.crear(entrada)
}

#[test]
fn debe_crear_praind_con_fecha() {
    crear_y_recuperar(TipoIngreso::Praind, Some(fecha_praind()), false);
}

#[test]
fn debe_crear_in_house_con_fecha() {
    crear_y_recuperar(TipoIngreso::InHouse, Some(fecha_praind()), false);
}

#[test]
fn debe_crear_por_correo_sin_fecha() {
    crear_y_recuperar(TipoIngreso::PorCorreo, None, false);
}

#[test]
fn debe_crear_swat_sin_fecha() {
    crear_y_recuperar(TipoIngreso::Swat, None, false);
}

#[test]
fn debe_crear_personal_de_ruta_con_fecha() {
    crear_y_recuperar(TipoIngreso::PorCorreo, Some(fecha_praind()), true);
}

#[test]
fn debe_rechazar_personal_de_ruta_sin_fecha() {
    assert!(matches!(
        crear_resultado(TipoIngreso::PorCorreo, None, true),
        Err(ContratistaServiceError::PraindRequerido)
    ));
}

#[test]
fn debe_rechazar_praind_normal_sin_fecha() {
    assert!(matches!(
        crear_resultado(TipoIngreso::Praind, None, false),
        Err(ContratistaServiceError::PraindRequerido)
    ));
}

#[test]
fn debe_rechazar_in_house_sin_fecha() {
    assert!(matches!(
        crear_resultado(TipoIngreso::InHouse, None, false),
        Err(ContratistaServiceError::PraindRequerido)
    ));
}

#[test]
fn debe_permitir_por_correo_sin_fecha() {
    assert!(crear_resultado(TipoIngreso::PorCorreo, None, false).is_ok());
}

#[test]
fn debe_permitir_swat_sin_fecha() {
    assert!(crear_resultado(TipoIngreso::Swat, None, false).is_ok());
}

#[test]
fn debe_rechazar_empresa_inexistente() {
    let (connection, _) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);

    assert!(matches!(
        servicio.crear(datos(999, TipoIngreso::Swat)),
        Err(ContratistaServiceError::EmpresaNoEncontrada)
    ));
}

fn probar_datos_invalidos(cedula: &str, nombre: &str) -> Result<i64, ContratistaServiceError> {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let mut entrada = datos(empresa_id, TipoIngreso::Swat);
    entrada.cedula = cedula.to_string();
    entrada.nombre = nombre.to_string();
    servicio.crear(entrada)
}

#[test]
fn debe_rechazar_cedula_vacia() {
    assert!(matches!(
        probar_datos_invalidos("", "Persona"),
        Err(ContratistaServiceError::CedulaVacia)
    ));
}

#[test]
fn debe_rechazar_cedula_solo_espacios() {
    assert!(matches!(
        probar_datos_invalidos("   ", "Persona"),
        Err(ContratistaServiceError::CedulaVacia)
    ));
}

#[test]
fn debe_rechazar_nombre_vacio() {
    assert!(matches!(
        probar_datos_invalidos("2001", ""),
        Err(ContratistaServiceError::NombreVacio)
    ));
}

#[test]
fn debe_rechazar_nombre_solo_espacios() {
    assert!(matches!(
        probar_datos_invalidos("2001", "   "),
        Err(ContratistaServiceError::NombreVacio)
    ));
}

#[test]
fn debe_aplicar_trim_a_cedula_al_crear() {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let mut entrada = datos(empresa_id, TipoIngreso::Swat);
    entrada.cedula = "  2001  ".to_string();

    let id = servicio.crear(entrada).unwrap();

    assert_eq!(servicio.buscar_por_id(id).unwrap().cedula, "2001");
}

#[test]
fn debe_aplicar_trim_a_nombre_al_crear() {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let mut entrada = datos(empresa_id, TipoIngreso::Swat);
    entrada.nombre = "  Persona Uno  ".to_string();

    let id = servicio.crear(entrada).unwrap();

    assert_eq!(servicio.buscar_por_id(id).unwrap().nombre, "Persona Uno");
}

#[test]
fn debe_buscar_por_cedula() {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let id = servicio
        .crear(datos(empresa_id, TipoIngreso::Swat))
        .unwrap();

    assert_eq!(servicio.buscar_por_cedula("2001").unwrap().id, id);
}

#[test]
fn debe_aplicar_trim_al_buscar_por_cedula() {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let id = servicio
        .crear(datos(empresa_id, TipoIngreso::Swat))
        .unwrap();

    assert_eq!(servicio.buscar_por_cedula("  2001  ").unwrap().id, id);
}

#[test]
fn debe_buscar_por_id() {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let id = servicio
        .crear(datos(empresa_id, TipoIngreso::Swat))
        .unwrap();

    assert_eq!(servicio.buscar_por_id(id).unwrap().id, id);
}

#[test]
fn contratista_inexistente_debe_producir_error() {
    let (connection, _) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);

    assert!(matches!(
        servicio.buscar_por_id(999),
        Err(ContratistaServiceError::ContratistaNoEncontrado)
    ));
    assert!(matches!(
        servicio.buscar_por_cedula("999"),
        Err(ContratistaServiceError::ContratistaNoEncontrado)
    ));
}

fn preparar_actualizacion() -> (Connection, i64, i64) {
    let (connection, empresa_id) = preparar_base();
    let id = {
        let contratistas = SqliteContratistaRepository::new(&connection);
        let empresas = SqliteEmpresaRepository::new(&connection);
        ContratistaService::new(&contratistas, &empresas)
            .crear(datos(empresa_id, TipoIngreso::Swat))
            .unwrap()
    };
    (connection, empresa_id, id)
}

#[test]
fn debe_actualizar_contratista() {
    let (connection, empresa_id, id) = preparar_actualizacion();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let mut entrada = actualizacion(empresa_id, TipoIngreso::PorCorreo);
    entrada.cedula = "3001".to_string();
    entrada.nombre = "Nombre actualizado".to_string();

    servicio.actualizar(id, entrada).unwrap();
    let actualizado = servicio.buscar_por_id(id).unwrap();

    assert_eq!(actualizado.cedula, "3001");
    assert_eq!(actualizado.nombre, "Nombre actualizado");
    assert!(matches!(
        servicio.buscar_por_cedula("2001"),
        Err(ContratistaServiceError::ContratistaNoEncontrado)
    ));
}

#[test]
fn debe_aplicar_trim_a_cedula_al_actualizar() {
    let (connection, empresa_id, id) = preparar_actualizacion();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let mut entrada = actualizacion(empresa_id, TipoIngreso::Swat);
    entrada.cedula = "  3001  ".to_string();

    servicio.actualizar(id, entrada).unwrap();

    assert_eq!(servicio.buscar_por_id(id).unwrap().cedula, "3001");
}

#[test]
fn debe_rechazar_cedula_vacia_al_actualizar() {
    let (connection, empresa_id, id) = preparar_actualizacion();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);

    for cedula in ["", "   "] {
        let mut entrada = actualizacion(empresa_id, TipoIngreso::Swat);
        entrada.cedula = cedula.to_string();
        assert!(matches!(
            servicio.actualizar(id, entrada),
            Err(ContratistaServiceError::CedulaVacia)
        ));
    }
    assert_eq!(servicio.buscar_por_id(id).unwrap().cedula, "2001");
}

#[test]
fn actualizar_conserva_tipo_ingreso_solicitado() {
    let (connection, empresa_id, id) = preparar_actualizacion();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let entrada = actualizacion(empresa_id, TipoIngreso::PorCorreo);

    servicio.actualizar(id, entrada).unwrap();

    assert_eq!(
        servicio.buscar_por_id(id).unwrap().tipo_ingreso,
        TipoIngreso::PorCorreo
    );
}

#[test]
fn actualizar_conserva_personal_de_ruta_solicitado() {
    let (connection, empresa_id, id) = preparar_actualizacion();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let mut entrada = actualizacion(empresa_id, TipoIngreso::Swat);
    entrada.es_personal_ruta = true;
    entrada.fecha_vencimiento_praind = Some(fecha_praind());

    servicio.actualizar(id, entrada).unwrap();

    assert!(servicio.buscar_por_id(id).unwrap().es_personal_ruta);
}

#[test]
fn actualizar_conserva_tiene_acceso_solicitado() {
    let (connection, empresa_id, id) = preparar_actualizacion();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let mut entrada = actualizacion(empresa_id, TipoIngreso::Swat);
    entrada.tiene_acceso = false;

    servicio.actualizar(id, entrada).unwrap();

    assert!(!servicio.buscar_por_id(id).unwrap().tiene_acceso);
}

#[test]
fn debe_rechazar_actualizacion_de_id_inexistente_primero() {
    let (connection, _) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let entrada = actualizacion(999, TipoIngreso::Praind);

    assert!(matches!(
        servicio.actualizar(999, entrada),
        Err(ContratistaServiceError::ContratistaNoEncontrado)
    ));
}

#[test]
fn debe_rechazar_actualizacion_con_empresa_inexistente() {
    let (connection, _, id) = preparar_actualizacion();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);

    assert!(matches!(
        servicio.actualizar(id, actualizacion(999, TipoIngreso::Swat)),
        Err(ContratistaServiceError::EmpresaNoEncontrada)
    ));
}

#[test]
fn debe_validar_praind_al_actualizar() {
    let (connection, empresa_id, id) = preparar_actualizacion();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);

    assert!(matches!(
        servicio.actualizar(id, actualizacion(empresa_id, TipoIngreso::Praind)),
        Err(ContratistaServiceError::PraindRequerido)
    ));
}

#[test]
fn debe_listar_contratistas() {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    servicio
        .crear(datos(empresa_id, TipoIngreso::Swat))
        .unwrap();
    let mut segundo = datos(empresa_id, TipoIngreso::PorCorreo);
    segundo.cedula = "2002".to_string();
    segundo.nombre = "Persona Dos".to_string();
    servicio.crear(segundo).unwrap();

    assert_eq!(servicio.listar().unwrap().len(), 2);
}

#[test]
fn cedula_duplicada_devuelve_error_semantico_y_no_crea_otro_registro() {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    servicio
        .crear(datos(empresa_id, TipoIngreso::Swat))
        .unwrap();

    let resultado = servicio.crear(datos(empresa_id, TipoIngreso::PorCorreo));

    assert!(matches!(
        resultado,
        Err(ContratistaServiceError::CedulaDuplicada)
    ));
    assert_eq!(servicio.listar().unwrap().len(), 1);
}

#[test]
fn actualizar_con_cedula_duplicada_devuelve_error_semantico_y_conserva_registro() {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    servicio
        .crear(datos(empresa_id, TipoIngreso::Swat))
        .unwrap();
    let mut segundo = datos(empresa_id, TipoIngreso::Swat);
    segundo.cedula = "2002".to_owned();
    segundo.nombre = "Persona Dos".to_owned();
    let segundo_id = servicio.crear(segundo).unwrap();
    let mut actualizacion = actualizacion(empresa_id, TipoIngreso::Swat);
    actualizacion.nombre = "Nombre Modificado".to_owned();
    assert!(matches!(
        servicio.actualizar(segundo_id, actualizacion),
        Err(ContratistaServiceError::CedulaDuplicada)
    ));
    let conservado = servicio.buscar_por_id(segundo_id).unwrap();
    assert_eq!(conservado.cedula, "2002");
    assert_eq!(conservado.nombre, "Persona Dos");
}

// Bandeja de salida hacia la nube (`docs/plan-persistencia-nube.md`): crear
// o actualizar un contratista debe dejar siempre su aviso correspondiente
// en `cola_salida`, listo para sincronizar más adelante.

fn contar_cola_salida(connection: &Connection, entidad_uuid: &str, operacion: &str) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM cola_salida
             WHERE entidad = 'contratista' AND entidad_uuid = ?1 AND operacion = ?2
               AND estado = 'pendiente'",
            [entidad_uuid, operacion],
            |row| row.get(0),
        )
        .unwrap()
}

fn uuid_de_contratista(connection: &Connection, id: i64) -> String {
    connection
        .query_row(
            "SELECT uuid FROM contratistas WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn debe_encolar_hacia_la_nube_al_crear() {
    let (connection, empresa_id) = preparar_base();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);

    let id = servicio
        .crear(datos(empresa_id, TipoIngreso::Swat))
        .unwrap();
    let uuid = uuid_de_contratista(&connection, id);

    assert_eq!(contar_cola_salida(&connection, &uuid, "crear"), 1);
}

#[test]
fn debe_encolar_hacia_la_nube_al_actualizar() {
    let (connection, empresa_id, id) = preparar_actualizacion();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let empresas = SqliteEmpresaRepository::new(&connection);
    let servicio = ContratistaService::new(&contratistas, &empresas);
    let uuid = uuid_de_contratista(&connection, id);

    servicio
        .actualizar(id, actualizacion(empresa_id, TipoIngreso::PorCorreo))
        .unwrap();

    assert_eq!(contar_cola_salida(&connection, &uuid, "actualizar"), 1);
}
