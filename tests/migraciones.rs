use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use rusqlite::Connection;

use control_acceso::database::schema::{SCHEMA_VERSION, initialize_database};

static SECUENCIA: AtomicU64 = AtomicU64::new(0);

fn base_temporal(nombre: &str) -> PathBuf {
    let numero = SECUENCIA.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "control_acceso_migracion_{nombre}_{}_{numero}.db",
        std::process::id()
    ))
}

fn limpiar_base(ruta: &Path) {
    let _ = fs::remove_file(ruta);
    let _ = fs::remove_file(ruta.with_extension("db-journal"));
    let _ = fs::remove_file(ruta.with_extension("db-wal"));
    let _ = fs::remove_file(ruta.with_extension("db-shm"));
}

fn version(connection: &Connection) -> i64 {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap()
}

fn crear_trigger_cedula_inmutable(connection: &Connection) {
    connection
        .execute_batch(
            "CREATE TRIGGER contratistas_cedula_inmutable
             BEFORE UPDATE OF cedula ON contratistas
             WHEN NEW.cedula <> OLD.cedula
             BEGIN
                SELECT RAISE(ABORT, 'La cedula del contratista es inmutable');
             END;",
        )
        .unwrap();
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

/// Usada tanto contra el esquema crudo de la versión 1 (`crear_esquema_version_1`,
/// sin `empresas.activo`) como contra el esquema actual ya migrado — de ahí que
/// `empresas` liste sus columnas explícitamente en vez de depender del orden
/// posicional: así la columna `activo` (con `DEFAULT`) no rompe el INSERT en
/// ninguno de los dos casos.
fn insertar_referencias(connection: &Connection) {
    connection
        .execute_batch(
            "
            INSERT INTO empresas (id, nombre) VALUES (1, 'Empresa');
            INSERT INTO usuarios VALUES (1, '1001', 'Operador', 'hash', 'OPERADOR', 1);
            INSERT INTO contratistas VALUES (1, '2001', 'Persona', 1, 'PRAIND', '2030-01-01', 0, 1);
            ",
        )
        .unwrap();
}

fn insertar_movimiento_actual(
    connection: &Connection,
    id: i64,
    ingreso: &str,
    salida: Option<&str>,
    usuario_salida_id: Option<i64>,
    usuario_salida_nombre: Option<&str>,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO registro_ingresos(
            id,contratista_id,empresa_id,fecha_hora_ingreso,medio_ingreso,
            tipo_ingreso,gafete_numero,usuario_ingreso_id,fecha_hora_salida,
            usuario_salida_id,contratista_cedula,contratista_nombre,
            empresa_nombre,usuario_ingreso_nombre,usuario_salida_nombre,
            fecha_vencimiento_praind,es_personal_ruta,tiene_acceso,
            resultado_acceso,motivo_resultado,reglas_version
         ) VALUES (?1,1,1,?2,'CAMINANDO','PRAIND',NULL,1,?3,?4,
            '2001','Persona','Empresa','Operador',?5,'2030-01-01',0,1,
            'PERMITIDO',NULL,1)",
        rusqlite::params![
            id,
            ingreso,
            salida,
            usuario_salida_id,
            usuario_salida_nombre
        ],
    )?;
    Ok(())
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
fn migracion_10_procesa_auditoria_vieja_sin_perder_el_resto_del_esquema() {
    // MIGRACION_13 (más reciente que ésta) termina descartando
    // `auditoria_contratistas` por completo (reemplazada por
    // `auditoria_cambios`, ver el comentario junto a `MIGRACION_13` en
    // `schema.rs`) — así que ya no tiene sentido afirmar que esta fila
    // "se conserva" hasta el final de la cadena. Lo que sigue valiendo la
    // pena probar es que una base real congelada en v9, con filas ya
    // escritas en la forma vieja de la tabla, atraviesa el resto de la
    // cadena de migraciones (10 → 11 → 12 → 13) sin errores de SQL.
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    crear_trigger_cedula_inmutable(&connection);
    connection
        .execute_batch(
            "DROP INDEX idx_registro_ingresos_fecha_salida;
             DROP TABLE auditoria_cambios;
             CREATE TABLE auditoria_contratistas (
                id INTEGER PRIMARY KEY,
                fecha_hora TEXT NOT NULL,
                usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
                contratista_id INTEGER NOT NULL REFERENCES contratistas(id) ON DELETE RESTRICT,
                campo TEXT NOT NULL CHECK (
                    campo IN ('tipo_ingreso', 'fecha_vencimiento_praind')
                ),
                valor_anterior TEXT,
                valor_nuevo TEXT,
                CHECK (valor_anterior IS NOT valor_nuevo)
             );
             INSERT INTO auditoria_contratistas(
                fecha_hora,usuario_id,contratista_id,campo,valor_anterior,valor_nuevo
             ) VALUES(
                '2026-08-20T22:00:00Z',1,1,'tipo_ingreso','SWAT','PRAIND'
             );
             PRAGMA user_version = 9;",
        )
        .unwrap();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='auditoria_contratistas'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect_err("auditoria_contratistas debe haber desaparecido tras MIGRACION_13");
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM auditoria_cambios", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(total, 0);
}

#[test]
fn migracion_11_crea_indice_parcial_sin_perder_movimientos() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    crear_trigger_cedula_inmutable(&connection);
    insertar_movimiento_actual(
        &connection,
        1,
        "2026-08-21T10:00:00Z",
        Some("2026-08-21T11:00:00Z"),
        Some(1),
        Some("Operador"),
    )
    .unwrap();
    connection
        .execute_batch(
            "DROP INDEX idx_registro_ingresos_fecha_salida;
             -- MIGRACION_12 (que corre después de ésta al rebobinar a v10)
             -- necesita `auditoria_contratistas` en pie — el `initialize_database`
             -- de arriba ya la reemplazó por `auditoria_cambios` (MIGRACION_13),
             -- así que hay que recrearla en la forma que dejó MIGRACION_10 antes
             -- de simular que la base está congelada en v10.
             DROP TABLE auditoria_cambios;
             CREATE TABLE auditoria_contratistas (
                id INTEGER PRIMARY KEY,
                fecha_hora TEXT NOT NULL,
                usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
                contratista_id INTEGER NOT NULL REFERENCES contratistas(id) ON DELETE RESTRICT,
                campo TEXT NOT NULL CHECK (
                    campo IN ('tipo_ingreso', 'fecha_vencimiento_praind', 'tiene_acceso')
                ),
                valor_anterior TEXT,
                valor_nuevo TEXT,
                CHECK (valor_anterior IS NOT valor_nuevo)
             );
             PRAGMA user_version = 10;",
        )
        .unwrap();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    let definicion: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'index'
               AND name = 'idx_registro_ingresos_fecha_salida'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(definicion.contains("ON registro_ingresos(fecha_hora_salida)"));
    assert!(definicion.contains("WHERE fecha_hora_salida IS NOT NULL"));
    let movimiento: (String, String) = connection
        .query_row(
            "SELECT fecha_hora_ingreso, fecha_hora_salida
             FROM registro_ingresos WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        movimiento,
        (
            "2026-08-21T10:00:00Z".to_owned(),
            "2026-08-21T11:00:00Z".to_owned()
        )
    );
    initialize_database(&connection).unwrap();
}

#[test]
fn migracion_12_habilita_cambio_de_cedula() {
    // Igual comentario que en `migracion_10_...`: MIGRACION_13 termina
    // reemplazando `auditoria_contratistas` por `auditoria_cambios`, así que
    // ya no tiene sentido afirmar "se conserva la auditoría" al final de la
    // cadena — lo que sigue probando esta prueba es lo que le da nombre: que
    // el trigger que bloqueaba cambiar la cédula queda eliminado.
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);
    crear_trigger_cedula_inmutable(&connection);
    connection
        .execute_batch(
            "DROP TABLE auditoria_cambios;
             CREATE TABLE auditoria_contratistas (
                id INTEGER PRIMARY KEY,
                fecha_hora TEXT NOT NULL,
                usuario_id INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE RESTRICT,
                contratista_id INTEGER NOT NULL REFERENCES contratistas(id) ON DELETE RESTRICT,
                campo TEXT NOT NULL CHECK (
                    campo IN ('tipo_ingreso', 'fecha_vencimiento_praind', 'tiene_acceso')
                ),
                valor_anterior TEXT,
                valor_nuevo TEXT,
                CHECK (valor_anterior IS NOT valor_nuevo)
             );
             INSERT INTO auditoria_contratistas(
                fecha_hora,usuario_id,contratista_id,campo,valor_anterior,valor_nuevo
             ) VALUES(
                '2026-08-21T12:00:00Z',1,1,'tipo_ingreso','SWAT','PRAIND'
             );
             CREATE INDEX idx_auditoria_contratistas_fecha
             ON auditoria_contratistas(fecha_hora DESC, id DESC);
             CREATE INDEX idx_auditoria_contratistas_contratista
             ON auditoria_contratistas(contratista_id, id DESC);
             PRAGMA user_version = 11;",
        )
        .unwrap();

    initialize_database(&connection).unwrap();

    assert_eq!(version(&connection), SCHEMA_VERSION);
    connection
        .execute("UPDATE contratistas SET cedula = 'OTRA' WHERE id = 1", [])
        .unwrap();
    let cedula: String = connection
        .query_row("SELECT cedula FROM contratistas WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(cedula, "OTRA");
}

#[test]
fn migracion_13_reemplaza_auditoria_contratistas_por_auditoria_cambios() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);

    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='auditoria_contratistas'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect_err("auditoria_contratistas no debe existir en una base nueva");

    connection
        .execute(
            "INSERT INTO auditoria_cambios(
                fecha_hora,usuario_id,usuario_nombre,entidad,entidad_id,entidad_nombre,
                campo,valor_anterior,valor_nuevo
             ) VALUES(
                '2026-08-28T12:00:00Z',1,'Operador','contratista',1,'Persona',
                'nombre','Persona','Persona Nueva'
             )",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO auditoria_cambios(
                fecha_hora,usuario_id,usuario_nombre,entidad,entidad_id,entidad_nombre,
                campo,valor_anterior,valor_nuevo
             ) VALUES(
                '2026-08-28T12:01:00Z',1,'Operador','empresa',1,'Empresa',
                'activo','1','0'
             )",
            [],
        )
        .unwrap();
    // Cambio de contraseña: sólo la fecha importa, sin valores.
    connection
        .execute(
            "INSERT INTO auditoria_cambios(
                fecha_hora,usuario_id,usuario_nombre,entidad,entidad_id,entidad_nombre,campo
             ) VALUES(
                '2026-08-28T12:02:00Z',1,'Operador','usuario',1,'Operador','password'
             )",
            [],
        )
        .unwrap();

    let error = connection
        .execute(
            "INSERT INTO auditoria_cambios(
                fecha_hora,usuario_id,usuario_nombre,entidad,entidad_id,entidad_nombre,campo
             ) VALUES(
                '2026-08-28T12:03:00Z',1,'Operador','otra_cosa',1,'Lo que sea','campo'
             )",
            [],
        )
        .unwrap_err();
    assert!(
        error.to_string().to_lowercase().contains("check"),
        "{error}"
    );

    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM auditoria_cambios", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(total, 3);
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
    let snapshot: (String, String, String, i64, String) = connection
        .query_row(
            "SELECT contratista_cedula, contratista_nombre,
                    resultado_acceso, reglas_version, fecha_hora_ingreso
             FROM registro_ingresos WHERE id=1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        snapshot,
        (
            "2001".into(),
            "Persona".into(),
            "MIGRADO".into(),
            0,
            "2026-08-11T14:00:00Z".into()
        )
    );
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
fn esquema_actual_solo_admite_fechas_utc_normalizadas() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);

    assert!(
        insertar_movimiento_actual(&connection, 1, "2026-08-11 08:00:00", None, None, None)
            .is_err()
    );
    insertar_movimiento_actual(&connection, 1, "2026-08-11T14:00:00Z", None, None, None).unwrap();
}

#[test]
fn check_de_salida_acepta_pares_coherentes_y_rechaza_incoherentes() {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    insertar_referencias(&connection);

    insertar_movimiento_actual(&connection, 1, "2026-08-11T14:00:00Z", None, None, None).unwrap();
    insertar_movimiento_actual(
        &connection,
        2,
        "2026-08-10T14:00:00Z",
        Some("2026-08-10T23:00:00Z"),
        Some(1),
        Some("Operador"),
    )
    .unwrap();
    assert!(
        insertar_movimiento_actual(
            &connection,
            3,
            "2026-08-09T14:00:00Z",
            Some("2026-08-09T23:00:00Z"),
            None,
            None,
        )
        .is_err()
    );
    assert!(
        insertar_movimiento_actual(
            &connection,
            4,
            "2026-08-09T14:00:00Z",
            None,
            Some(1),
            Some("Operador"),
        )
        .is_err()
    );
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
fn fallo_tardio_revierte_todas_las_migraciones_pendientes() {
    let connection = Connection::open_in_memory().unwrap();
    crear_esquema_version_1(&connection);
    insertar_referencias(&connection);
    connection
        .execute_batch(
            "
            DROP INDEX idx_registro_ingresos_gafete;
            CREATE TABLE contratistas_fts (id INTEGER PRIMARY KEY);
            PRAGMA user_version = 0;
            ",
        )
        .unwrap();

    assert!(initialize_database(&connection).is_err());

    assert_eq!(version(&connection), 0);
    let indice_recreado: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name='idx_registro_ingresos_gafete'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(indice_recreado, 0);

    let definicion_registro: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type='table' AND name='registro_ingresos'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!definicion_registro.contains("fecha_hora_salida IS NULL"));

    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM contratistas", [], |row| row.get(0))
        .unwrap();
    assert_eq!(total, 1);
}

#[test]
fn dos_conexiones_migran_una_base_vacia_sin_reaplicar_pasos() {
    let ruta = base_temporal("concurrente");
    limpiar_base(&ruta);
    let barrera = Arc::new(Barrier::new(3));

    let hilos: Vec<_> = (0..2)
        .map(|_| {
            let ruta = ruta.clone();
            let barrera = Arc::clone(&barrera);
            thread::spawn(move || -> Result<(), String> {
                let connection = Connection::open(ruta).map_err(|error| error.to_string())?;
                connection
                    .busy_timeout(Duration::from_secs(5))
                    .map_err(|error| error.to_string())?;
                barrera.wait();
                initialize_database(&connection).map_err(|error| error.to_string())
            })
        })
        .collect();

    barrera.wait();
    for hilo in hilos {
        hilo.join().unwrap().unwrap();
    }

    let connection = Connection::open(&ruta).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
    let tablas_fts: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='table'
             AND name IN ('contratistas_fts', 'empresas_fts', 'usuarios_fts')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tablas_fts, 3);
    let integridad: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integridad, "ok");

    drop(connection);
    limpiar_base(&ruta);
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

/// A diferencia de `base_temporal` (un archivo suelto directamente en
/// `temp_dir()`), estas dos pruebas necesitan que `<directorio>/backups` sea
/// exclusivo de cada una — comparten `temp_dir()` con el resto de la suite,
/// así que si dos corridas en paralelo apuntaran al mismo `backups`
/// interferirían entre sí.
fn base_temporal_en_directorio_propio(nombre: &str) -> PathBuf {
    let ruta = base_temporal(nombre);
    let directorio = ruta.with_extension("");
    fs::create_dir_all(&directorio).unwrap();
    directorio.join("control_acceso.db")
}

#[test]
fn abrir_una_base_con_migracion_pendiente_deja_un_respaldo_pre_migracion_antes_de_migrar() {
    use control_acceso::database::backup::{TipoRespaldo, listar_respaldos};
    use control_acceso::database::connection::open_database;

    let ruta = base_temporal_en_directorio_propio("respaldo_previo");
    let previa = Connection::open(&ruta).unwrap();
    crear_esquema_version_1(&previa);
    insertar_referencias(&previa);
    drop(previa);

    let connection = open_database(&ruta).unwrap();
    assert_eq!(version(&connection), SCHEMA_VERSION);
    drop(connection);

    let directorio_respaldos = ruta.parent().unwrap().join("backups");
    let respaldos = listar_respaldos(&directorio_respaldos).unwrap();
    assert_eq!(respaldos.len(), 1);
    assert_eq!(respaldos[0].tipo, TipoRespaldo::PreMigracion);

    // El respaldo conserva el estado de ANTES de migrar (versión 1): abrirlo
    // con una conexión cruda, sin pasar por open_database, debe mostrar
    // todavía la referencia sembrada en el esquema viejo.
    let copia = Connection::open(&respaldos[0].ruta).unwrap();
    assert_eq!(version(&copia), 1);
    let referencias: i64 = copia
        .query_row("SELECT COUNT(*) FROM contratistas", [], |r| r.get(0))
        .unwrap();
    assert_eq!(referencias, 1);

    let _ = fs::remove_dir_all(ruta.parent().unwrap());
}

#[test]
fn una_base_ya_al_dia_no_genera_ningun_respaldo_pre_migracion() {
    use control_acceso::database::backup::listar_respaldos;
    use control_acceso::database::connection::open_database;

    let ruta = base_temporal_en_directorio_propio("sin_migracion_pendiente");
    let previa = Connection::open(&ruta).unwrap();
    initialize_database(&previa).unwrap();
    drop(previa);

    let connection = open_database(&ruta).unwrap();
    drop(connection);

    let directorio_respaldos = ruta.parent().unwrap().join("backups");
    assert!(listar_respaldos(&directorio_respaldos).unwrap().is_empty());

    let _ = fs::remove_dir_all(ruta.parent().unwrap());
}
