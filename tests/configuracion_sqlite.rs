use rusqlite::Connection;

use control_acceso::database::connection::open_database;
use control_acceso::database::schema::{APPLICATION_ID, SchemaError, initialize_database};

fn preparar_base() -> Connection {
    open_database(":memory:").expect("No se pudo abrir la base de datos")
}

fn ruta_temporal(nombre: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "control_acceso_{nombre}_{}_{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn fija_el_perfil_de_durabilidad_esperado() {
    // `:memory:` fuerza journal_mode="memory" sin importar lo que se pida,
    // así que este perfil sólo se puede observar contra un archivo real.
    let ruta = ruta_temporal("perfil");
    let connection = open_database(&ruta).expect("No se pudo abrir la base de datos");

    let foreign_keys: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(foreign_keys, 1);

    let busy_timeout: i64 = connection
        .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
        .unwrap();
    assert_eq!(busy_timeout, 5000);

    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(journal_mode.to_lowercase(), "delete");

    let synchronous: i64 = connection
        .query_row("PRAGMA synchronous", [], |r| r.get(0))
        .unwrap();
    assert_eq!(synchronous, 3); // EXTRA = 3

    let trusted_schema: i64 = connection
        .query_row("PRAGMA trusted_schema", [], |r| r.get(0))
        .unwrap();
    assert_eq!(trusted_schema, 0);

    let secure_delete: i64 = connection
        .query_row("PRAGMA secure_delete", [], |r| r.get(0))
        .unwrap();
    assert_eq!(secure_delete, 2); // FAST = 2

    drop(connection);
    let _ = std::fs::remove_file(&ruta);
}

#[test]
fn adopta_el_application_id_en_una_base_nueva_o_preexistente_sin_marca() {
    let connection = preparar_base();

    let id: i64 = connection
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .unwrap();
    assert_eq!(id, i64::from(APPLICATION_ID));
}

#[test]
fn reabrir_una_base_ya_marcada_no_falla() {
    let connection = preparar_base();
    // Reabrir sobre la misma conexión ya inicializada simula el caso de una
    // base preexistente que ya tiene nuestro application_id: no debe
    // tratarse como una base ajena.
    initialize_database(&connection).expect("una base ya marcada debe poder reabrirse");
}

#[test]
fn rechaza_un_archivo_con_application_id_de_otra_aplicacion() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA application_id = 123456;")
        .unwrap();

    let resultado = initialize_database(&connection);
    assert!(matches!(resultado, Err(SchemaError::BaseAjena)));
}

#[test]
fn rechaza_un_archivo_truncado_en_vez_de_aceptarlo_en_silencio() {
    use std::fs::OpenOptions;
    use std::io::{Seek, SeekFrom, Write};

    let ruta = ruta_temporal("corrupcion");

    // Construye una base válida con varias páginas de datos.
    {
        let connection = Connection::open(&ruta).unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO empresas(nombre) VALUES ('Empresa de prueba de corrupción');",
            )
            .unwrap();
    }

    // Trunca el archivo a la mitad: dejar una página final incompleta hace
    // que `quick_check` (o, en el peor caso, apenas abrir la conexión)
    // detecte la corrupción en vez de aceptar el archivo tal cual.
    let longitud = std::fs::metadata(&ruta).unwrap().len();
    let mut archivo = OpenOptions::new().write(true).open(&ruta).unwrap();
    archivo.seek(SeekFrom::Start(longitud / 2)).unwrap();
    archivo.set_len(longitud / 2).unwrap();
    archivo.flush().unwrap();
    drop(archivo);

    let resultado = Connection::open(&ruta)
        .map_err(SchemaError::from)
        .and_then(|connection| initialize_database(&connection));
    assert!(
        resultado.is_err(),
        "un archivo truncado a la mitad nunca debe abrirse como si estuviera sano"
    );

    let _ = std::fs::remove_file(&ruta);
}

#[test]
fn drop_de_appcore_no_entra_en_panico_al_optimizar_al_cerrar() {
    use control_acceso::application::AppCore;

    let connection = preparar_base();
    let core = AppCore::new(connection);
    drop(core); // no debe entrar en pánico ni propagar el error de PRAGMA optimize
}
