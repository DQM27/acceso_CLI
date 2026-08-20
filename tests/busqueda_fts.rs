use chrono::NaiveDate;
use control_acceso::database::queries::contratistas::{
    ContratistasQuery, FiltroContratistas, SqliteContratistasQuery,
};
use control_acceso::database::queries::ingresos::{
    FiltroHistorial, FiltroIngresosActivos, IngresosQuery, SqliteIngresosQuery,
};
use control_acceso::database::queries::usuarios::{
    FiltroUsuarios, SqliteUsuariosQuery, UsuariosQuery,
};
use control_acceso::database::schema::{SCHEMA_VERSION, initialize_database};
use rusqlite::{Connection, params};

fn base() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute(
            "INSERT INTO empresas(nombre) VALUES ('Álvarez Ingeniería')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO usuarios(cedula,nombre,password_hash,rol,activo)
             VALUES ('9001','José Hernández','hash','OPERADOR',1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO contratistas(
                cedula,nombre,empresa_id,tipo_ingreso,fecha_vencimiento_praind,
                es_personal_ruta,tiene_acceso
             ) VALUES ('1001','María Peña',1,'PRAIND','2030-01-01',0,1)",
            [],
        )
        .unwrap();
    connection
}

fn filtro_contratista(texto: &str) -> FiltroContratistas {
    FiltroContratistas {
        texto: Some(texto.to_owned()),
        ..FiltroContratistas::default()
    }
}

#[test]
fn contratistas_busca_subcadenas_sin_distinguir_tildes_o_mayusculas() {
    let connection = base();
    let query = SqliteContratistasQuery::new(&connection);

    for texto in ["pena", "PEÑA", "aria", "alvarez", "ÁLVA"] {
        assert_eq!(
            query
                .buscar(&filtro_contratista(texto))
                .unwrap()
                .items
                .len(),
            1
        );
    }
}

#[test]
fn usuarios_busca_subcadenas_sin_distinguir_tildes_o_mayusculas() {
    let connection = base();
    let query = SqliteUsuariosQuery::new(&connection);

    for texto in ["jose", "JOSÉ", "nandez", "HERNÁNDEZ"] {
        let filtro = FiltroUsuarios {
            texto: Some(texto.to_owned()),
            ..FiltroUsuarios::default()
        };
        assert_eq!(query.buscar(&filtro).unwrap().len(), 1);
    }
}

#[test]
fn activos_busca_persona_empresa_y_gafete_exacto() {
    let connection = base();
    connection
        .execute(
            "INSERT INTO registro_ingresos(
            contratista_id,empresa_id,fecha_hora_ingreso,medio_ingreso,tipo_ingreso,
            gafete_numero,usuario_ingreso_id,contratista_cedula,contratista_nombre,
            empresa_nombre,usuario_ingreso_nombre,fecha_vencimiento_praind,
            es_personal_ruta,tiene_acceso,resultado_acceso,motivo_resultado,reglas_version
         ) VALUES (1,1,'2026-08-12T14:00:00Z','CAMINANDO','PRAIND',12,1,
            '1001','María Peña','Álvarez Ingeniería','José Hernández','2030-01-01',0,1,
            'PERMITIDO',NULL,1)",
            [],
        )
        .unwrap();
    let query = SqliteIngresosQuery::new(&connection);

    for texto in ["pena", "alvarez", "12"] {
        let filtro = FiltroIngresosActivos {
            texto: Some(texto.to_owned()),
            ..Default::default()
        };
        assert_eq!(query.listar_activos(&filtro).unwrap().items.len(), 1);
    }
    let filtro = FiltroIngresosActivos {
        texto: Some("2".to_owned()),
        ..Default::default()
    };
    assert!(query.listar_activos(&filtro).unwrap().items.is_empty());
}

#[test]
fn historial_usa_fts_y_conserva_total_coherente() {
    let connection = base();
    connection
        .execute(
            "INSERT INTO registro_ingresos(
            contratista_id,empresa_id,fecha_hora_ingreso,medio_ingreso,tipo_ingreso,
            usuario_ingreso_id,contratista_cedula,contratista_nombre,empresa_nombre,
            usuario_ingreso_nombre,fecha_vencimiento_praind,es_personal_ruta,
            tiene_acceso,resultado_acceso,motivo_resultado,reglas_version
         ) VALUES (1,1,'2026-08-12T14:00:00Z','CAMINANDO','PRAIND',1,
            '1001','María Peña','Álvarez Ingeniería','José Hernández','2030-01-01',0,1,
            'PERMITIDO',NULL,1)",
            [],
        )
        .unwrap();
    let query = SqliteIngresosQuery::new(&connection);
    let mut filtro = FiltroHistorial::nuevo(
        control_acceso::tiempo::inicio_dia_costa_rica_utc(
            NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
        )
        .unwrap(),
        control_acceso::tiempo::inicio_dia_costa_rica_utc(
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
        )
        .unwrap(),
    );
    filtro.texto_persona = Some("PENA".to_owned());

    let pagina = query.buscar_historial(&filtro).unwrap();
    assert_eq!(pagina.items.len(), 1);
    assert_eq!(pagina.total, 1);
}

#[test]
fn texto_corto_y_operadores_fts_son_seguros() {
    let connection = base();
    let contratistas = SqliteContratistasQuery::new(&connection);
    assert_eq!(
        contratistas
            .buscar(&filtro_contratista("ía"))
            .unwrap()
            .items
            .len(),
        1
    );

    for texto in ["NEAR", "nombre:*", "a OR b", "\"", "-()"] {
        assert!(contratistas.buscar(&filtro_contratista(texto)).is_ok());
    }
}

#[test]
fn triggers_mantienen_indices_en_insert_update_delete() {
    let connection = base();
    connection
        .execute(
            "UPDATE contratistas SET nombre='Lucía Núñez' WHERE id=1",
            [],
        )
        .unwrap();
    connection
        .execute("UPDATE empresas SET nombre='Compañía Única' WHERE id=1", [])
        .unwrap();
    connection
        .execute("UPDATE usuarios SET nombre='Óscar Muñoz' WHERE id=1", [])
        .unwrap();

    let contratistas = SqliteContratistasQuery::new(&connection);
    let usuarios = SqliteUsuariosQuery::new(&connection);
    assert_eq!(
        contratistas
            .buscar(&filtro_contratista("nunez"))
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(
        contratistas
            .buscar(&filtro_contratista("pena"))
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        contratistas
            .buscar(&filtro_contratista("compania"))
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(
        contratistas
            .buscar(&filtro_contratista("alvarez"))
            .unwrap()
            .items
            .is_empty()
    );
    let filtro_usuario = FiltroUsuarios {
        texto: Some("munoz".to_owned()),
        ..Default::default()
    };
    assert_eq!(usuarios.buscar(&filtro_usuario).unwrap().len(), 1);
    let filtro_anterior = FiltroUsuarios {
        texto: Some("hernandez".to_owned()),
        ..Default::default()
    };
    assert!(usuarios.buscar(&filtro_anterior).unwrap().is_empty());

    connection
        .execute("DELETE FROM contratistas WHERE id=1", [])
        .unwrap();
    connection
        .execute("DELETE FROM usuarios WHERE id=1", [])
        .unwrap();
    connection
        .execute("DELETE FROM empresas WHERE id=1", [])
        .unwrap();
    for tabla in ["contratistas_fts", "usuarios_fts", "empresas_fts"] {
        let total: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {tabla}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(total, 0);
    }
}

#[test]
fn migracion_desde_v2_reconstruye_indices_con_datos_existentes() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(
        "CREATE TABLE empresas(id INTEGER PRIMARY KEY,nombre TEXT NOT NULL UNIQUE);
         CREATE TABLE usuarios(id INTEGER PRIMARY KEY,cedula TEXT NOT NULL UNIQUE,nombre TEXT NOT NULL,password_hash TEXT NOT NULL,rol TEXT NOT NULL,activo INTEGER NOT NULL);
         CREATE TABLE contratistas(id INTEGER PRIMARY KEY,cedula TEXT NOT NULL UNIQUE,nombre TEXT NOT NULL,empresa_id INTEGER NOT NULL,tipo_ingreso TEXT NOT NULL,fecha_vencimiento_praind TEXT,es_personal_ruta INTEGER NOT NULL,tiene_acceso INTEGER NOT NULL);
         CREATE TABLE registro_ingresos(id INTEGER PRIMARY KEY,contratista_id INTEGER NOT NULL,empresa_id INTEGER NOT NULL,fecha_hora_ingreso TEXT NOT NULL,medio_ingreso TEXT NOT NULL,tipo_ingreso TEXT NOT NULL,gafete_numero INTEGER,usuario_ingreso_id INTEGER NOT NULL,fecha_hora_salida TEXT,usuario_salida_id INTEGER);
         INSERT INTO empresas VALUES(1,'Construcción Álvarez');
         INSERT INTO usuarios VALUES(1,'9','José','hash','OPERADOR',1);
         INSERT INTO contratistas VALUES(1,'1','María Peña',1,'PRAIND',NULL,0,1);
         PRAGMA user_version=2;",
    ).unwrap();

    initialize_database(&connection).unwrap();
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    assert_eq!(
        SqliteContratistasQuery::new(&connection)
            .buscar(&filtro_contratista("alvarez"))
            .unwrap()
            .items
            .len(),
        1
    );
}

#[test]
fn plan_de_consulta_match_utiliza_el_indice_virtual() {
    let connection = base();
    for sql in [
        "SELECT rowid FROM contratistas_fts WHERE contratistas_fts MATCH ?1",
        "SELECT rowid FROM usuarios_fts WHERE usuarios_fts MATCH ?1",
        "SELECT c.id FROM empresas_fts INNER JOIN contratistas c
         ON c.empresa_id=empresas_fts.rowid WHERE empresas_fts MATCH ?1",
        "SELECT r.id FROM registro_ingresos r WHERE r.contratista_id IN (
         SELECT rowid FROM contratistas_fts WHERE contratistas_fts MATCH ?1)",
    ] {
        let mut statement = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap();
        let detalles = statement
            .query_map(params!["\"nandez\""], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            detalles
                .iter()
                .any(|detalle| detalle.contains("VIRTUAL TABLE INDEX")),
            "{detalles:?}"
        );
    }
}

#[test]
fn fts_funciona_despues_de_cerrar_y_reabrir() {
    let nombre = format!(
        "control_acceso_search_{}_{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let ruta = std::env::temp_dir().join(nombre);
    {
        let connection = Connection::open(&ruta).unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute("INSERT INTO empresas(nombre) VALUES('Álvarez')", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO contratistas(cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso)
                 VALUES('1','José Hernández',1,'SWAT',0,1)",
                [],
            )
            .unwrap();
    }
    {
        let connection = Connection::open(&ruta).unwrap();
        initialize_database(&connection).unwrap();
        assert_eq!(
            SqliteContratistasQuery::new(&connection)
                .buscar(&filtro_contratista("nandez"))
                .unwrap()
                .items
                .len(),
            1
        );
    }
    std::fs::remove_file(ruta).unwrap();
}

#[test]
fn version_desconocida_sigue_siendo_rechazada() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("PRAGMA user_version=99").unwrap();
    assert!(initialize_database(&connection).is_err());
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 99);
}

#[test]
fn falla_de_migracion_fts_revierte_objetos_y_conserva_version_2() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(
        "CREATE TABLE empresas(id INTEGER PRIMARY KEY,nombre TEXT NOT NULL);
         CREATE TABLE usuarios(id INTEGER PRIMARY KEY,cedula TEXT,nombre TEXT,password_hash TEXT,rol TEXT,activo INTEGER);
         CREATE TABLE contratistas(id INTEGER PRIMARY KEY,cedula TEXT,nombre TEXT,empresa_id INTEGER,tipo_ingreso TEXT,fecha_vencimiento_praind TEXT,es_personal_ruta INTEGER,tiene_acceso INTEGER);
         CREATE TABLE registro_ingresos(id INTEGER PRIMARY KEY);
         CREATE TABLE contratistas_fts(id INTEGER);
         PRAGMA user_version=2;",
    ).unwrap();

    assert!(initialize_database(&connection).is_err());
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    let empresas_fts: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='empresas_fts'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, 2);
    assert_eq!(empresas_fts, 0);
}
