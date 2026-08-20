use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rusqlite::Connection;

use control_acceso::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use control_acceso::database::repositories::empresa_repository::SqliteEmpresaRepository;
use control_acceso::database::repositories::registro_ingreso_repository::{
    RegistroIngresoRepository, SqliteRegistroIngresoRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::domain::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};
use control_acceso::models::contratista::Contratista;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::services::error::RegistroIngresoServiceError;
use control_acceso::services::registro_ingreso_service::RegistroIngresoService;

struct Base {
    connection: Connection,
    empresa_id: i64,
    usuario_id: i64,
}

fn base() -> Base {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas(nombre) VALUES ('Empresa Principal')",
            [],
        )
        .unwrap();
    let empresa_id = connection.last_insert_rowid();
    connection.execute("INSERT INTO usuarios(cedula,nombre,password_hash,rol,activo) VALUES ('1','Operador','hash','OPERADOR',1)", []).unwrap();
    let usuario_id = connection.last_insert_rowid();
    Base {
        connection,
        empresa_id,
        usuario_id,
    }
}

fn crear_contratista(
    base: &Base,
    cedula: &str,
    tipo: TipoIngreso,
    fecha: Option<NaiveDate>,
    ruta: bool,
    acceso: bool,
) -> i64 {
    SqliteContratistaRepository::new(&base.connection)
        .crear(&Contratista {
            id: 0,
            cedula: cedula.into(),
            nombre: format!("Persona {cedula}"),
            empresa_id: base.empresa_id,
            tipo_ingreso: tipo,
            fecha_vencimiento_praind: fecha,
            es_personal_ruta: ruta,
            tiene_acceso: acceso,
            empresa_activa: true,
        })
        .unwrap()
}

fn hoy() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, 12).unwrap()
}

fn fecha_hora() -> DateTime<Utc> {
    control_acceso::tiempo::local_costa_rica_a_utc(
        NaiveDateTime::parse_from_str("2026-08-12 08:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
    )
    .unwrap()
}

#[test]
fn prepara_identidad_empresa_tipo_y_acceso_permitido() {
    let base = base();
    let id = crear_contratista(&base, "1001", TipoIngreso::Swat, None, false, true);
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    let preparacion = RegistroIngresoService::new(&contratistas, &registros)
        .preparar_ingreso(&empresas, id, hoy())
        .unwrap();

    assert_eq!(preparacion.contratista_id, id);
    assert_eq!(preparacion.cedula, "1001");
    assert_eq!(preparacion.nombre, "Persona 1001");
    assert_eq!(preparacion.empresa_nombre, "Empresa Principal");
    assert_eq!(preparacion.tipo_ingreso, TipoIngreso::Swat);
    assert_eq!(preparacion.resultado_acceso, ResultadoAcceso::Permitido);
    assert!(!preparacion.requiere_gafete);
    assert!(!preparacion.tiene_ingreso_activo);
}

#[test]
fn empresa_inactiva_deniega_el_acceso_aunque_el_contratista_lo_tenga() {
    use control_acceso::database::repositories::empresa_repository::EmpresaRepository;

    let base = base();
    let id = crear_contratista(&base, "1001", TipoIngreso::Swat, None, false, true);
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    empresas.establecer_activo(base.empresa_id, false).unwrap();

    let preparacion = RegistroIngresoService::new(&contratistas, &registros)
        .preparar_ingreso(&empresas, id, hoy())
        .unwrap();

    assert_eq!(
        preparacion.resultado_acceso,
        ResultadoAcceso::Denegado(MotivoDenegacion::EmpresaInactiva)
    );
}

#[test]
fn hoy_explicito_controla_advertencia_y_vencimiento() {
    let base = base();
    let id = crear_contratista(
        &base,
        "1001",
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2026, 8, 20).unwrap()),
        false,
        true,
    );
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    let service = RegistroIngresoService::new(&contratistas, &registros);

    assert_eq!(
        service
            .preparar_ingreso(&empresas, id, hoy())
            .unwrap()
            .resultado_acceso,
        ResultadoAcceso::PermitidoConAdvertencia
    );
    assert_eq!(
        service
            .preparar_ingreso(&empresas, id, NaiveDate::from_ymd_opt(2026, 8, 21).unwrap())
            .unwrap()
            .resultado_acceso,
        ResultadoAcceso::Denegado(MotivoDenegacion::PraindVencido)
    );
}

#[test]
fn acceso_administrativo_denegado_se_devuelve_como_resultado() {
    let base = base();
    let id = crear_contratista(&base, "1001", TipoIngreso::Swat, None, false, false);
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    let resultado = RegistroIngresoService::new(&contratistas, &registros)
        .preparar_ingreso(&empresas, id, hoy())
        .unwrap();
    assert_eq!(
        resultado.resultado_acceso,
        ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso)
    );
}

#[test]
fn gafete_respeta_tipo_y_personal_de_ruta() {
    let base = base();
    let casos = [
        (TipoIngreso::Praind, false, true),
        (TipoIngreso::PorCorreo, false, true),
        (TipoIngreso::InHouse, false, false),
        (TipoIngreso::Swat, false, false),
        (TipoIngreso::Praind, true, false),
    ];
    for (indice, (tipo, ruta, esperado)) in casos.into_iter().enumerate() {
        let id = crear_contratista(
            &base,
            &format!("{indice}"),
            tipo,
            Some(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
            ruta,
            true,
        );
        let contratistas = SqliteContratistaRepository::new(&base.connection);
        let empresas = SqliteEmpresaRepository::new(&base.connection);
        let registros = SqliteRegistroIngresoRepository::new(&base.connection);
        let preparacion = RegistroIngresoService::new(&contratistas, &registros)
            .preparar_ingreso(&empresas, id, hoy())
            .unwrap();
        assert_eq!(preparacion.requiere_gafete, esperado);
    }
}

#[test]
fn personal_de_ruta_sigue_requiriendo_praind_para_acceso() {
    let base = base();
    let id = crear_contratista(&base, "1001", TipoIngreso::Swat, None, true, true);
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    let resultado = RegistroIngresoService::new(&contratistas, &registros)
        .preparar_ingreso(&empresas, id, hoy())
        .unwrap();
    assert_eq!(
        resultado.resultado_acceso,
        ResultadoAcceso::Denegado(MotivoDenegacion::PraindNoRegistrado)
    );
    assert!(!resultado.requiere_gafete);
}

#[test]
fn detecta_ingreso_activo_sin_cargar_listado() {
    let base = base();
    let id = crear_contratista(
        &base,
        "1001",
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
        false,
        true,
    );
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    let service = RegistroIngresoService::new(&contratistas, &registros);
    assert!(
        !service
            .preparar_ingreso(&empresas, id, hoy())
            .unwrap()
            .tiene_ingreso_activo
    );
    service
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            Some(10),
            base.usuario_id,
            fecha_hora(),
        )
        .unwrap();
    assert!(
        service
            .preparar_ingreso(&empresas, id, hoy())
            .unwrap()
            .tiene_ingreso_activo
    );
}

#[test]
fn preparar_no_persiste_ni_reserva_gafete() {
    let base = base();
    let id = crear_contratista(
        &base,
        "1001",
        TipoIngreso::Praind,
        Some(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
        false,
        true,
    );
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    let service = RegistroIngresoService::new(&contratistas, &registros);
    let antes: i64 = base
        .connection
        .query_row("SELECT COUNT(*) FROM registro_ingresos", [], |r| r.get(0))
        .unwrap();
    service.preparar_ingreso(&empresas, id, hoy()).unwrap();
    let despues: i64 = base
        .connection
        .query_row("SELECT COUNT(*) FROM registro_ingresos", [], |r| r.get(0))
        .unwrap();
    assert_eq!(antes, despues);
    assert!(
        registros
            .buscar_ingreso_activo_por_gafete(25)
            .unwrap()
            .is_none()
    );
}

#[test]
fn contratista_inexistente_es_error_esperado() {
    let base = base();
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    assert!(matches!(
        RegistroIngresoService::new(&contratistas, &registros).preparar_ingreso(
            &empresas,
            999,
            hoy()
        ),
        Err(RegistroIngresoServiceError::ContratistaNoEncontrado)
    ));
}

#[test]
fn registrar_entrada_revalida_cambio_de_acceso_despues_de_preparar() {
    let base = base();
    let id = crear_contratista(&base, "1001", TipoIngreso::Swat, None, false, true);
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    let service = RegistroIngresoService::new(&contratistas, &registros);
    assert_eq!(
        service
            .preparar_ingreso(&empresas, id, hoy())
            .unwrap()
            .resultado_acceso,
        ResultadoAcceso::Permitido
    );
    let mut actualizado = contratistas.buscar_por_id(id).unwrap().unwrap();
    actualizado.tiene_acceso = false;
    contratistas.actualizar(&actualizado).unwrap();
    assert!(matches!(
        service.registrar_entrada(
            id,
            MedioIngreso::Caminando,
            None,
            base.usuario_id,
            fecha_hora()
        ),
        Err(RegistroIngresoServiceError::AccesoDenegado(
            MotivoDenegacion::SinAcceso
        ))
    ));
}

#[test]
fn registrar_entrada_revalida_ingreso_creado_despues_de_preparar() {
    let base = base();
    let id = crear_contratista(&base, "1001", TipoIngreso::Swat, None, false, true);
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    let service = RegistroIngresoService::new(&contratistas, &registros);
    assert!(
        !service
            .preparar_ingreso(&empresas, id, hoy())
            .unwrap()
            .tiene_ingreso_activo
    );
    service
        .registrar_entrada(
            id,
            MedioIngreso::Caminando,
            None,
            base.usuario_id,
            fecha_hora(),
        )
        .unwrap();
    assert!(matches!(
        service.registrar_entrada(
            id,
            MedioIngreso::Caminando,
            None,
            base.usuario_id,
            fecha_hora()
        ),
        Err(RegistroIngresoServiceError::IngresoActivo)
    ));
}

#[test]
fn error_de_datos_persistidos_se_propaga_como_database() {
    let base = base();
    base.connection
        .execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    base.connection.execute("INSERT INTO contratistas(cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,es_personal_ruta,tiene_acceso) VALUES ('x','Inválido',1,'TIPO_INVALIDO',NULL,0,1)", []).unwrap();
    let id = base.connection.last_insert_rowid();
    let contratistas = SqliteContratistaRepository::new(&base.connection);
    let empresas = SqliteEmpresaRepository::new(&base.connection);
    let registros = SqliteRegistroIngresoRepository::new(&base.connection);
    assert!(matches!(
        RegistroIngresoService::new(&contratistas, &registros).preparar_ingreso(
            &empresas,
            id,
            hoy()
        ),
        Err(RegistroIngresoServiceError::Database(_))
    ));
}
