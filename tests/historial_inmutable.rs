use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rusqlite::Connection;

use control_acceso::database::queries::ingresos::{
    FiltroHistorial, FiltroIngresosActivos, IngresosQuery, SqliteIngresosQuery,
};
use control_acceso::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use control_acceso::database::repositories::registro_ingreso_repository::{
    RegistroIngresoRepository, SqliteRegistroIngresoRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::domain::resultado_acceso::{MotivoDenegacion, ResultadoAcceso};
use control_acceso::models::contratista::Contratista;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::registro_ingreso::{ResultadoIngresoRegistrado, VERSION_REGLAS_ACCESO};
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::services::registro_ingreso_service::{
    RegistroIngresoConsultaService, RegistroIngresoService,
};

fn fecha(valor: &str) -> DateTime<Utc> {
    control_acceso::tiempo::local_costa_rica_a_utc(
        NaiveDateTime::parse_from_str(valor, "%Y-%m-%d %H:%M:%S").unwrap(),
    )
    .unwrap()
}

fn preparar() -> (Connection, i64, i64, i64) {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas(nombre) VALUES ('Aldama'), ('Expenic')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO usuarios(cedula,nombre,password_hash,rol,activo)
             VALUES ('U1','Operador Entrada','hash','OPERADOR',1),
                    ('U2','Operador Salida','hash','OPERADOR',1)",
            [],
        )
        .unwrap();
    let contratista_id = SqliteContratistaRepository::new(&connection)
        .crear(&Contratista {
            id: 0,
            cedula: "1-1111-1111".into(),
            nombre: "Juan Original".into(),
            empresa_id: 1,
            tipo_ingreso: TipoIngreso::Praind,
            fecha_vencimiento_praind: NaiveDate::from_ymd_opt(2027, 12, 31),
            es_personal_ruta: false,
            tiene_acceso: true,
            empresa_activa: true,
        })
        .unwrap();
    (connection, contratista_id, 1, 2)
}

/// Regresión del hallazgo #7 de `docs/auditoria-dominio-2026-08-20.md`: la
/// decisión histórica debe poder reconstruirse sólo a partir del snapshot
/// guardado, sin volver a consultar el estado actual de la empresa. Se
/// registra la entrada con la empresa activa, se la desactiva *después*, y
/// se confirma que el historial sigue mostrando el estado que tenía al
/// momento del ingreso — ni la desactivación posterior lo cambia (sería
/// justamente el bug que este hallazgo describía: el dato quedaba implícito
/// en vez de guardado) ni queda ambiguo cuál era el estado real en ese
/// momento.
#[test]
fn historial_reconstruye_el_estado_de_la_empresa_al_momento_del_ingreso() {
    let (connection, contratista_id, usuario_entrada, _usuario_salida) = preparar();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros);
    servicio
        .registrar_entrada(
            contratista_id,
            MedioIngreso::Caminando,
            Some(18),
            usuario_entrada,
            fecha("2026-08-12 08:00:00"),
        )
        .unwrap();

    // La empresa se desactiva después de registrada la entrada.
    connection
        .execute("UPDATE empresas SET activo = 0 WHERE id = 1", [])
        .unwrap();

    let historial = SqliteIngresosQuery::new(&connection)
        .buscar_historial(&FiltroHistorial::nuevo(
            fecha("2026-08-01 00:00:00"),
            fecha("2026-09-01 00:00:00"),
        ))
        .unwrap();
    assert_eq!(historial.items.len(), 1);
    assert!(
        historial.items[0].empresa_activa_snapshot,
        "la empresa estaba activa al momento del ingreso, sin importar que se haya \
         desactivado después"
    );
}

#[test]
fn cambios_maestros_no_reescriben_el_movimiento_historico() {
    let (connection, contratista_id, usuario_entrada, usuario_salida) = preparar();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros);
    let entrada = fecha("2026-08-12 08:00:00");
    let registro_id = servicio
        .registrar_entrada(
            contratista_id,
            MedioIngreso::Caminando,
            Some(18),
            usuario_entrada,
            entrada,
        )
        .unwrap()
        .registro_id;

    connection
        .execute(
            "UPDATE contratistas SET cedula='2-2222-2222', nombre='Juan Actual', empresa_id=2,
                    tipo_ingreso='SWAT', fecha_vencimiento_praind=NULL,
                    tiene_acceso=0 WHERE id=?1",
            [contratista_id],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE empresas SET nombre='Aldama Renombrada' WHERE id=1",
            [],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE usuarios SET nombre='Entrada Renombrada' WHERE id=?1",
            [usuario_entrada],
        )
        .unwrap();

    let consulta = SqliteIngresosQuery::new(&connection);
    let activos = RegistroIngresoConsultaService::new(&consulta)
        .listar_activos(
            &FiltroIngresosActivos::default(),
            NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
        )
        .unwrap();
    assert_eq!(activos.items[0].cedula, "1-1111-1111");
    assert_eq!(activos.items[0].contratista_nombre, "Juan Original");
    assert_eq!(activos.items[0].empresa_nombre, "Aldama");
    assert_eq!(activos.items[0].usuario_ingreso_nombre, "Operador Entrada");
    assert_eq!(
        activos.items[0].resultado_acceso,
        ResultadoAcceso::Denegado(MotivoDenegacion::SinAcceso)
    );

    servicio
        .registrar_salida(registro_id, fecha("2026-08-12 17:00:00"), usuario_salida)
        .unwrap();
    connection
        .execute(
            "UPDATE usuarios SET nombre='Salida Renombrada' WHERE id=?1",
            [usuario_salida],
        )
        .unwrap();

    let pagina = consulta
        .buscar_historial(&FiltroHistorial::nuevo(
            fecha("2026-08-01 00:00:00"),
            fecha("2026-09-01 00:00:00"),
        ))
        .unwrap();
    let movimiento = &pagina.items[0];
    assert_eq!(movimiento.cedula, "1-1111-1111");
    assert_eq!(movimiento.contratista_nombre, "Juan Original");
    assert_eq!(movimiento.empresa_nombre, "Aldama");
    assert_eq!(movimiento.tipo_ingreso, TipoIngreso::Praind);
    assert_eq!(movimiento.usuario_ingreso_nombre, "Operador Entrada");
    assert_eq!(
        movimiento.usuario_salida_nombre.as_deref(),
        Some("Operador Salida")
    );
    assert_eq!(
        movimiento.resultado_acceso,
        ResultadoIngresoRegistrado::Permitido
    );
    assert_eq!(movimiento.reglas_version, VERSION_REGLAS_ACCESO);

    let mut por_nombre_anterior =
        FiltroHistorial::nuevo(fecha("2026-08-01 00:00:00"), fecha("2026-09-01 00:00:00"));
    por_nombre_anterior.texto_persona = Some("Original".into());
    assert_eq!(
        consulta
            .buscar_historial(&por_nombre_anterior)
            .unwrap()
            .total,
        1
    );
    por_nombre_anterior.texto_persona = Some("Actual".into());
    assert_eq!(
        consulta
            .buscar_historial(&por_nombre_anterior)
            .unwrap()
            .total,
        0
    );
}

#[test]
fn sqlite_impide_reescribir_o_eliminar_un_movimiento() {
    let (connection, contratista_id, usuario_entrada, usuario_salida) = preparar();
    let contratistas = SqliteContratistaRepository::new(&connection);
    let registros = SqliteRegistroIngresoRepository::new(&connection);
    let servicio = RegistroIngresoService::new(&contratistas, &registros);
    let registro_id = servicio
        .registrar_entrada(
            contratista_id,
            MedioIngreso::Caminando,
            Some(18),
            usuario_entrada,
            fecha("2026-08-12 08:00:00"),
        )
        .unwrap()
        .registro_id;
    servicio
        .registrar_salida(registro_id, fecha("2026-08-12 17:00:00"), usuario_salida)
        .unwrap();

    assert!(
        connection
            .execute(
                "UPDATE registro_ingresos SET contratista_nombre='Reescrito' WHERE id=?1",
                [registro_id],
            )
            .is_err()
    );
    assert!(
        connection
            .execute("DELETE FROM registro_ingresos WHERE id=?1", [registro_id])
            .is_err()
    );
    assert!(
        connection
            .execute(
                "UPDATE registro_ingresos SET fecha_hora_salida='2026-08-12 18:00:00'
                 WHERE id=?1",
                [registro_id],
            )
            .is_err()
    );
    assert_eq!(registros.listar().unwrap().len(), 1);
}
