//! Medición manual (no corre con `cargo test` normal): cuánto cuesta abrir
//! una conexión nueva a la base, cifrada y sin cifrar.
//!
//! Sirve para decidir si el núcleo puede abrir una conexión por operación en
//! vez de compartir una sola detrás de un candado, o si hace falta un pool.
//! Hay que correrla en **release**: en depuración el `SQLite3MC` (C) y el resto
//! se compilan sin optimizar y los números no dicen nada de producción.
//!
//! ```text
//! cargo test --release --test medir_apertura_base -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rusqlite::Connection;

use control_acceso::database::connection::{
    abrir_conexion_secundaria_escritura, open_database, open_database_cifrada,
};

const CLAVE: [u8; 32] = [0x42; 32];
const REPETICIONES: usize = 60;
const CONTRATISTAS: i64 = 3_000;

fn ruta(nombre: &str) -> PathBuf {
    let ruta =
        std::env::temp_dir().join(format!("medir_apertura_{nombre}_{}.db", std::process::id()));
    limpiar(&ruta);
    ruta
}

fn limpiar(ruta: &Path) {
    for extension in ["db", "db-journal", "db-wal", "db-shm"] {
        let _ = std::fs::remove_file(ruta.with_extension(extension));
    }
}

/// Una base con el esquema real y `CONTRATISTAS` contratistas.
fn sembrar(connection: &Connection) {
    connection
        .execute_batch("INSERT INTO empresas(id,nombre) VALUES (1,'Empresa');")
        .unwrap();
    let transaccion = connection.unchecked_transaction().unwrap();
    {
        let mut insertar = transaccion
            .prepare(
                "INSERT INTO contratistas(
                    cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso
                 ) VALUES (?1, ?2, 1, 'SWAT', 0, 1)",
            )
            .unwrap();
        for n in 0..CONTRATISTAS {
            insertar
                .execute(rusqlite::params![
                    format!("C{n}"),
                    format!("Persona Numero {n}")
                ])
                .unwrap();
        }
    }
    transaccion.commit().unwrap();
}

/// Corre `trabajo` `REPETICIONES` veces y devuelve lo que tardó cada una,
/// ordenado.
fn tiempos(mut trabajo: impl FnMut()) -> Vec<Duration> {
    let mut medidas: Vec<Duration> = (0..REPETICIONES)
        .map(|_| {
            let inicio = Instant::now();
            trabajo();
            inicio.elapsed()
        })
        .collect();
    medidas.sort();
    medidas
}

fn ms(duracion: Duration) -> f64 {
    duracion.as_secs_f64() * 1_000.0
}

fn informar(titulo: &str, medidas: &[Duration]) {
    println!(
        "{titulo:<58} mediana {:>7.2} ms   p95 {:>7.2} ms   máx {:>7.2} ms",
        ms(medidas[medidas.len() / 2]),
        ms(medidas[medidas.len() * 95 / 100]),
        ms(*medidas.last().unwrap()),
    );
}

fn buscar_por_cedula(connection: &Connection) {
    let id: i64 = connection
        .query_row(
            "SELECT id FROM contratistas WHERE cedula = ?1",
            ["C1500"],
            |fila| fila.get(0),
        )
        .unwrap();
    assert!(id > 0);
}

#[allow(clippy::cast_precision_loss)] // Solo para mostrar los MB del archivo.
fn medir(etiqueta: &str, ruta: &Path, clave: Option<&[u8; 32]>) {
    println!("\n== {etiqueta} ==");
    let tamano = std::fs::metadata(ruta).unwrap().len();
    println!(
        "archivo: {:.1} MB, {CONTRATISTAS} contratistas",
        tamano as f64 / 1_048_576.0
    );

    informar(
        "abrir completa (clave + verificar + migrar el esquema)",
        &tiempos(|| {
            let conexion = clave.map_or_else(
                || open_database(ruta).unwrap(),
                |clave| open_database_cifrada(ruta, clave).unwrap(),
            );
            drop(conexion);
        }),
    );
    informar(
        "abrir secundaria (clave + pragmas, sin migrar)",
        &tiempos(|| {
            drop(abrir_conexion_secundaria_escritura(ruta, clave).unwrap());
        }),
    );
    informar(
        "abrir secundaria + 1ª lectura (busca un contratista)",
        &tiempos(|| {
            let conexion = abrir_conexion_secundaria_escritura(ruta, clave).unwrap();
            buscar_por_cedula(&conexion);
        }),
    );

    let abierta = abrir_conexion_secundaria_escritura(ruta, clave).unwrap();
    buscar_por_cedula(&abierta);
    informar(
        "la misma lectura en una conexión que ya estaba abierta",
        &tiempos(|| buscar_por_cedula(&abierta)),
    );
}

#[test]
#[ignore = "medición manual: ver el comentario del archivo"]
fn mide_el_costo_de_abrir_la_base() {
    let cifrada = ruta("cifrada");
    {
        let conexion = open_database_cifrada(&cifrada, &CLAVE).unwrap();
        sembrar(&conexion);
    }
    let plana = ruta("plana");
    {
        let conexion = open_database(&plana).unwrap();
        sembrar(&conexion);
    }

    println!("\nRepeticiones por medida: {REPETICIONES}");
    medir(
        "CIFRADA (SQLite3MC, ChaCha20, clave cruda)",
        &cifrada,
        Some(&CLAVE),
    );
    medir("SIN CIFRAR (para comparar)", &plana, None);

    limpiar(&cifrada);
    limpiar(&plana);
}
