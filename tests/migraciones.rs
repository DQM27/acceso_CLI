use rusqlite::Connection;

use control_acceso::database::schema::{SCHEMA_VERSION, initialize_database};

fn version(connection: &Connection) -> i64 {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap()
}

fn crear_esquema_version_1(connection: &Connection) {
    connection
        .execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE empresas (id INTEGER PRIMARY KEY, nombre TEXT NOT NULL UNIQUE);
            CREATE TABLE usuarios (
                id INTEGER PRIMARY KEY, cedula TEXT NOT NULL UNIQUE, nombre TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                rol TEXT NOT NULL CHECK (rol IN ('ROOT','ADMINISTRADOR','OPERADOR')),
                activo INTEGER NOT NULL CHECK (activo IN (0,1))
            );
            CREATE TABLE contratistas (
                id INTEGER PRIMARY KEY, cedula TEXT NOT NULL UNIQUE, nombre TEXT NOT NULL,
                empresa_id INTEGER NOT NULL,
                tipo_ingreso TEXT NOT NULL CHECK (tipo_ingreso IN ('PRAIND','IN_HOUSE','POR_CORREO','SWAT')),
                fecha_vencimiento_praind TEXT, es_personal_ruta INTEGER NOT NULL DEFAULT 0,
                tiene_acceso INTEGER NOT NULL,
                FOREIGN KEY (empresa_id) REFERENCES empresas(id)
            );
            CREATE TABLE registro_ingresos (
                id INTEGER PRIMARY KEY, contratista_id INTEGER NOT NULL, empresa_id INTEGER NOT NULL,
                fecha_hora_ingreso TEXT NOT NULL, medio_ingreso TEXT NOT NULL,
                tipo_ingreso TEXT NOT NULL, gafete_numero INTEGER,
                usuario_ingreso_id INTEGER NOT NULL, fecha_hora_salida TEXT, usuario_salida_id INTEGER,
                FOREIGN KEY (contratista_id) REFERENCES contratistas(id),
                FOREIGN KEY (empresa_id) REFERENCES empresas(id),
                FOREIGN KEY (usuario_ingreso_id) REFERENCES usuarios(id),
                FOREIGN KEY (usuario_salida_id) REFERENCES usuarios(id)
            );
            CREATE UNIQUE INDEX idx_registro_ingresos_contratista_activo
            ON registro_ingresos(contratista_id) WHERE fecha_hora_salida IS NULL;
            CREATE UNIQUE INDEX idx_registro_ingresos_gafete_activo
            ON registro_ingresos(gafete_numero)
            WHERE gafete_numero IS NOT NULL AND fecha_hora_salida IS NULL;
            CREATE INDEX idx_registro_ingresos_contratista ON registro_ingresos(contratista_id);
            CREATE INDEX idx_registro_ingresos_empresa ON registro_ingresos(empresa_id);
            CREATE INDEX idx_registro_ingresos_fecha_ingreso ON registro_ingresos(fecha_hora_ingreso);
            CREATE INDEX idx_registro_ingresos_gafete ON registro_ingresos(gafete_numero);
            PRAGMA user_version = 1;
            ",
        )
        .unwrap();
}

fn insertar_referencias(connection: &Connection) {
    connection
        .execute_batch(
            "
            INSERT INTO empresas VALUES (1, 'Empresa');
            INSERT INTO usuarios VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
            INSERT INTO contratistas VALUES (1, '2001', 'Persona', 1, 'PRAIND', '2030-01-01', 0, 1);
            ",
        )
        .unwrap();
}

#[test]
fn base_vacia_llega_a_version_actual_y_es_idempotente() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
    initialize_database(&connection).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
}

#[test]
fn esquema_existente_con_version_cero_migra_sin_perder_datos() {
    let connection = Connection::open_in_memory().unwrap();
    crear_esquema_version_1(&connection);
    insertar_referencias(&connection);
    connection
        .execute(
            "INSERT INTO registro_ingresos VALUES (1,1,1,'2026-08-11 08:00:00','CAMINANDO','PRAIND',5,1,NULL,NULL)",
            [],
        )
        .unwrap();
    connection.execute_batch("PRAGMA user_version = 0").unwrap();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    let gafete: i64 = connection
        .query_row(
            "SELECT gafete_numero FROM registro_ingresos WHERE id=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(gafete, 5);
}

#[test]
fn migra_version_1_y_conserva_datos_validos_e_indices() {
    let connection = Connection::open_in_memory().unwrap();
    crear_esquema_version_1(&connection);
    insertar_referencias(&connection);
    connection.execute(
        "INSERT INTO registro_ingresos VALUES (1,1,1,'2026-08-11 08:00:00','CAMINANDO','PRAIND',5,1,NULL,NULL)",
        [],
    ).unwrap();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM registro_ingresos", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total, 1);
    for indice in [
        "idx_registro_ingresos_contratista_activo",
        "idx_registro_ingresos_gafete_activo",
    ] {
        let existe: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                [indice],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(existe, 1);
    }
}

#[test]
fn check_de_salida_acepta_pares_coherentes_y_rechaza_incoherentes() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);

    connection.execute(
        "INSERT INTO registro_ingresos VALUES (1,1,1,'2026-08-11 08:00:00','CAMINANDO','PRAIND',NULL,1,NULL,NULL)", []
    ).unwrap();
    connection.execute(
        "INSERT INTO registro_ingresos VALUES (2,1,1,'2026-08-10 08:00:00','CAMINANDO','PRAIND',NULL,1,'2026-08-10 17:00:00',1)", []
    ).unwrap();
    assert!(connection.execute(
        "INSERT INTO registro_ingresos VALUES (3,1,1,'2026-08-09 08:00:00','CAMINANDO','PRAIND',NULL,1,'2026-08-09 17:00:00',NULL)", []
    ).is_err());
    assert!(connection.execute(
        "INSERT INTO registro_ingresos VALUES (4,1,1,'2026-08-09 08:00:00','CAMINANDO','PRAIND',NULL,1,NULL,1)", []
    ).is_err());
}

#[test]
fn migracion_fallida_revierte_tabla_y_version() {
    let connection = Connection::open_in_memory().unwrap();
    crear_esquema_version_1(&connection);
    insertar_referencias(&connection);
    connection.execute(
        "INSERT INTO registro_ingresos VALUES (1,1,1,'2026-08-11 08:00:00','CAMINANDO','PRAIND',NULL,1,'2026-08-11 17:00:00',NULL)", []
    ).unwrap();

    assert!(initialize_database(&connection).is_err());
    assert_eq!(version(&connection), 1);
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM registro_ingresos", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total, 1);
    let nueva: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='registro_ingresos_nueva'", [], |r| r.get(0)
    ).unwrap();
    assert_eq!(nueva, 0);
}

#[test]
fn claves_foraneas_permanecen_activas() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let activas: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(activas, 1);
    assert!(connection.execute(
        "INSERT INTO contratistas (cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso)
         VALUES ('2001','Persona',999,'SWAT',0,1)", []
    ).is_err());
}
