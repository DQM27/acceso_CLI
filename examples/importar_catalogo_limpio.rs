//! Importa un catalogo limpio de empresas y contratistas contra la base real.
//! Uso:
//!
//! ```text
//! cargo run --example importar_catalogo_limpio -- [--recrear] <archivo.sql> [ruta_db]
//! ```
//!
//! Sin `ruta_db`, usa la misma ruta que la app.

use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use control_acceso::{
    database::{
        backup::{TipoRespaldo, crear_respaldo},
        connection::{open_database, ruta_base_datos},
    },
    instancia::InstanciaGuard,
};
use rusqlite::{Connection, Transaction, TransactionBehavior};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let recrear = args.first().is_some_and(|arg| arg == "--recrear");
    if recrear {
        args.remove(0);
    }
    let mut args = args.into_iter();
    let sql_path = PathBuf::from(
        args.next()
            .expect("uso: importar_catalogo_limpio [--recrear] <archivo.sql> [ruta_db]"),
    );
    let db_path = args
        .next()
        .map_or_else(ruta_base_datos, |ruta| Ok(PathBuf::from(ruta)))?;

    println!("Base de datos: {}", db_path.display());
    println!("Script SQL:    {}", sql_path.display());
    println!(
        "Modo:          {}",
        if recrear {
            "recrear base"
        } else {
            "actualizar catalogo"
        }
    );

    preparar_directorio(&db_path)?;
    let _instancia = InstanciaGuard::adquirir(&db_path)?;
    if recrear {
        respaldar_y_retirar_base(&db_path)?;
    }
    let connection = open_database(&db_path)?;
    if !recrear {
        let respaldo = crear_respaldo(
            &connection,
            &directorio_respaldos(&db_path),
            TipoRespaldo::PorFlag,
        )?;
        println!("Respaldo:      {}", respaldo.ruta.display());
    }

    let sql = fs::read_to_string(&sql_path)?;
    println!(
        "Fuente:        {} empresas, {} contratistas",
        sql.matches("INSERT INTO empresas").count(),
        sql.matches("INSERT INTO contratistas").count()
    );

    let antes = Conteo::leer(&connection)?;
    let resultado = importar(&connection, &sql, recrear)?;
    let despues = Conteo::leer(&connection)?;
    verificar_integridad(&connection)?;

    println!(
        "Empresas:      {} -> {} ({} activas)",
        antes.empresas, despues.empresas, despues.empresas_activas
    );
    println!(
        "Contratistas:  {} -> {} ({} con acceso)",
        antes.contratistas, despues.contratistas, despues.contratistas_con_acceso
    );
    println!(
        "UUID nuevos:   {} contratistas, {} empresas",
        resultado.contratistas_uuid, resultado.empresas_uuid
    );
    println!(
        "Cola:          {} contratistas, {} empresas",
        resultado.contratistas_encolados, resultado.empresas_encoladas
    );

    Ok(())
}

struct Conteo {
    empresas: i64,
    empresas_activas: i64,
    contratistas: i64,
    contratistas_con_acceso: i64,
}

impl Conteo {
    fn leer(connection: &Connection) -> rusqlite::Result<Self> {
        Ok(Self {
            empresas: contar(connection, "SELECT COUNT(*) FROM empresas")?,
            empresas_activas: contar(connection, "SELECT COUNT(*) FROM empresas WHERE activo = 1")?,
            contratistas: contar(connection, "SELECT COUNT(*) FROM contratistas")?,
            contratistas_con_acceso: contar(
                connection,
                "SELECT COUNT(*) FROM contratistas WHERE tiene_acceso = 1",
            )?,
        })
    }
}

struct ResultadoImport {
    contratistas_uuid: usize,
    empresas_uuid: usize,
    contratistas_encolados: usize,
    empresas_encoladas: usize,
}

fn importar(
    connection: &Connection,
    sql: &str,
    recrear: bool,
) -> rusqlite::Result<ResultadoImport> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;

    if !recrear {
        transaction.execute("UPDATE contratistas SET tiene_acceso = 0", [])?;
        transaction.execute("UPDATE empresas SET activo = 0", [])?;
    }
    transaction.execute_batch(sql)?;

    let contratistas_uuid = rellenar_uuid(&transaction, "contratistas")?;
    let empresas_uuid = rellenar_uuid(&transaction, "empresas")?;

    reconstruir_fts(&transaction)?;
    let empresas_encoladas = encolar_estado(&transaction, "empresa", "empresas")?;
    let contratistas_encolados = encolar_estado(&transaction, "contratista", "contratistas")?;
    transaction.commit()?;

    Ok(ResultadoImport {
        contratistas_uuid,
        empresas_uuid,
        contratistas_encolados,
        empresas_encoladas,
    })
}

fn contar(connection: &Connection, sql: &str) -> rusqlite::Result<i64> {
    connection.query_row(sql, [], |row| row.get(0))
}

fn directorio_respaldos(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups")
}

fn preparar_directorio(db_path: &Path) -> std::io::Result<()> {
    if let Some(directorio) = db_path.parent() {
        fs::create_dir_all(directorio)?;
    }
    Ok(())
}

fn respaldar_y_retirar_base(db_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !db_path.exists() {
        return Ok(());
    }

    let connection = open_database(db_path)?;
    let respaldo = crear_respaldo(
        &connection,
        &directorio_respaldos(db_path),
        TipoRespaldo::PorFlag,
    )?;
    println!("Respaldo:      {}", respaldo.ruta.display());
    drop(connection);

    for ruta in rutas_sqlite(db_path) {
        if ruta.exists() {
            fs::remove_file(&ruta)?;
        }
    }
    println!("Base anterior: retirada");
    Ok(())
}

fn rutas_sqlite(db_path: &Path) -> [PathBuf; 4] {
    [
        db_path.to_path_buf(),
        ruta_con_sufijo(db_path, "-journal"),
        ruta_con_sufijo(db_path, "-wal"),
        ruta_con_sufijo(db_path, "-shm"),
    ]
}

fn ruta_con_sufijo(db_path: &Path, sufijo: &str) -> PathBuf {
    let mut ruta = OsString::from(db_path.as_os_str());
    ruta.push(sufijo);
    PathBuf::from(ruta)
}

fn rellenar_uuid(transaction: &Transaction<'_>, tabla: &str) -> rusqlite::Result<usize> {
    transaction.execute(
        &format!("UPDATE {tabla} SET uuid = ({UUID_SQL}) WHERE uuid IS NULL"),
        [],
    )
}

fn reconstruir_fts(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO empresas_fts(empresas_fts) VALUES ('rebuild')",
        [],
    )?;
    transaction.execute(
        "INSERT INTO contratistas_fts(contratistas_fts) VALUES ('rebuild')",
        [],
    )?;
    Ok(())
}

fn encolar_estado(
    transaction: &Transaction<'_>,
    entidad: &str,
    tabla: &str,
) -> rusqlite::Result<usize> {
    transaction.execute(
        &format!(
            "
            INSERT INTO cola_salida (
                entidad, entidad_uuid, operacion, creado_en, actualizado_en
            )
            SELECT
                '{entidad}', uuid, 'actualizar',
                strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            FROM {tabla}
            WHERE uuid IS NOT NULL
            "
        ),
        [],
    )
}

fn verificar_integridad(connection: &Connection) -> rusqlite::Result<()> {
    let integridad: String =
        connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integridad != "ok" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if connection.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

const UUID_SQL: &str = "
lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4'
|| substr(lower(hex(randomblob(2))), 2) || '-'
|| substr('89ab', abs(random()) % 4 + 1, 1)
|| substr(lower(hex(randomblob(2))), 2) || '-'
|| lower(hex(randomblob(6)))
";
