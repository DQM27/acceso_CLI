use control_acceso::database::schema::initialize_database;
use rusqlite::Connection;

fn base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO empresas VALUES (1, 'Empresa', 1, 'uuid-empresa-1');
         INSERT INTO usuarios VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
         INSERT INTO contratistas VALUES (1, '2001', 'Persona', 1, 'PRAIND', '2030-01-01', 0, 1, 'uuid-contratista-1');",
        )
        .unwrap();
    connection
}

#[test]
fn ingreso_con_usuario_de_entrada_inexistente_es_rechazado() {
    let connection = base();
    assert!(connection.execute(
        "INSERT INTO registro_ingresos
         (contratista_id,empresa_id,fecha_hora_ingreso,medio_ingreso,tipo_ingreso,usuario_ingreso_id,
          contratista_cedula,contratista_nombre,empresa_nombre,usuario_ingreso_nombre,
          es_personal_ruta,tiene_acceso,resultado_acceso,reglas_version)
         VALUES (1,1,'2026-08-11T14:00:00Z','CAMINANDO','PRAIND',999,
                 '2001','Persona','Empresa','Operador',0,1,'PERMITIDO',1)", []
    ).is_err());
}

#[test]
fn salida_con_usuario_inexistente_es_rechazada() {
    let connection = base();
    connection.execute(
        "INSERT INTO registro_ingresos
         (id,contratista_id,empresa_id,fecha_hora_ingreso,medio_ingreso,tipo_ingreso,usuario_ingreso_id,
          contratista_cedula,contratista_nombre,empresa_nombre,usuario_ingreso_nombre,
          es_personal_ruta,tiene_acceso,resultado_acceso,reglas_version)
         VALUES (1,1,1,'2026-08-11T14:00:00Z','CAMINANDO','PRAIND',1,
                 '2001','Persona','Empresa','Operador',0,1,'PERMITIDO',1)", []
    ).unwrap();
    assert!(connection.execute(
        "UPDATE registro_ingresos SET fecha_hora_salida='2026-08-11T23:00:00Z', usuario_salida_id=999 WHERE id=1", []
    ).is_err());
}

#[test]
fn contratista_con_empresa_inexistente_es_rechazado() {
    let connection = base();
    assert!(
        connection
            .execute(
                "INSERT INTO contratistas
         (cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso)
         VALUES ('2002','Otra persona',999,'SWAT',0,1)",
                []
            )
            .is_err()
    );
}

#[test]
fn ingreso_con_contratista_inexistente_es_rechazado() {
    let connection = base();
    assert!(connection.execute(
        "INSERT INTO registro_ingresos
         (contratista_id,empresa_id,fecha_hora_ingreso,medio_ingreso,tipo_ingreso,usuario_ingreso_id,
          contratista_cedula,contratista_nombre,empresa_nombre,usuario_ingreso_nombre,
          es_personal_ruta,tiene_acceso,resultado_acceso,reglas_version)
         VALUES (999,1,'2026-08-11T14:00:00Z','CAMINANDO','PRAIND',1,
                 '2001','Persona','Empresa','Operador',0,1,'PERMITIDO',1)", []
    ).is_err());
}
