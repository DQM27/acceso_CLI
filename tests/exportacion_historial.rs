use chrono::{NaiveDate, Utc};
use rusqlite::{Connection, params};

use control_acceso::{
    application::{AppCore, ExportarHistorialError},
    database::{queries::ingresos::FiltroHistorial, schema::initialize_database},
    exportacion_historial::ColumnaHistorial,
    tiempo::serializar_utc,
};

fn instante(dia: u32, hora: u32) -> chrono::DateTime<Utc> {
    chrono::DateTime::from_naive_utc_and_offset(
        NaiveDate::from_ymd_opt(2026, 8, dia)
            .unwrap()
            .and_hms_opt(hora, 0, 0)
            .unwrap(),
        Utc,
    )
}

fn core_con_movimientos(cantidad: usize) -> AppCore {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute("INSERT INTO empresas(nombre) VALUES ('Brisas')", [])
        .unwrap();
    connection
        .execute(
            "INSERT INTO usuarios(cedula,nombre,password_hash,rol,activo) \
             VALUES ('u1','Quintana','hash','OPERADOR',1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO contratistas(cedula,nombre,empresa_id,tipo_ingreso,\
             fecha_vencimiento_praind,es_personal_ruta,tiene_acceso) \
             VALUES ('00101','Ana Solano',1,'PRAIND','2027-01-01',0,1)",
            [],
        )
        .unwrap();

    for indice in 0..cantidad {
        let ingreso = serializar_utc(instante(20, 8));
        let salida = serializar_utc(instante(20, 9));
        connection
            .execute(
                "INSERT INTO registro_ingresos(contratista_id,empresa_id,\
                 fecha_hora_ingreso,medio_ingreso,tipo_ingreso,gafete_numero,\
                 usuario_ingreso_id,fecha_hora_salida,usuario_salida_id,\
                 contratista_cedula,contratista_nombre,empresa_nombre,\
                 usuario_ingreso_nombre,usuario_salida_nombre,fecha_vencimiento_praind,\
                 es_personal_ruta,tiene_acceso,resultado_acceso,motivo_resultado,reglas_version) \
                 VALUES (1,1,?1,'CAMINANDO','PRAIND',?2,1,?3,1,\
                 '00101','Ana Solano','Brisas','Quintana','Quintana','2027-01-01',\
                 0,1,'PERMITIDO',NULL,1)",
                params![ingreso, indice as i64 + 1, salida],
            )
            .unwrap();
    }
    AppCore::new(connection)
}

#[test]
fn exporta_todos_los_resultados_aunque_superen_una_pagina_sql() {
    let core = core_con_movimientos(205);
    let filtro = FiltroHistorial::nuevo(instante(1, 0), instante(31, 23));
    let directorio = tempfile::tempdir().unwrap();
    let destino = directorio.path().join("historial.xlsx");

    let exportados = core
        .exportar_historial(
            &filtro,
            &[ColumnaHistorial::Fecha, ColumnaHistorial::Nombre],
            &destino,
        )
        .unwrap();

    assert_eq!(exportados, 205);
    let contenido = std::fs::read(&destino).unwrap();
    assert!(contenido.starts_with(b"PK"));
    assert!(matches!(
        core.exportar_historial(&filtro, &[ColumnaHistorial::Nombre], &destino),
        Err(ExportarHistorialError::DestinoExiste(_))
    ));
}
