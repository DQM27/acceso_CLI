use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use control_acceso::application::AppCore;
use control_acceso::database::queries::ingresos::FiltroIngresosActivos;
use control_acceso::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use control_acceso::database::repositories::empresa_repository::{
    EmpresaRepository, SqliteEmpresaRepository,
};
use control_acceso::database::repositories::usuario_repository::{
    SqliteUsuarioRepository, UsuarioRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::contratista::Contratista;
use control_acceso::models::empresa::Empresa;
use control_acceso::models::medio_ingreso::MedioIngreso;
use control_acceso::models::tipo_ingreso::TipoIngreso;
use control_acceso::models::usuario::{RolUsuario, Usuario};
use control_acceso::services::password::generar_hash;
use rusqlite::Connection;

const CLAVE: &str = "brisas-e2e-sqlcipher-2026-09-10";
const CLAVE_MALA: &str = "clave-incorrecta-intencional";
const CABECERA_SQLITE: &[u8] = b"SQLite format 3\0";
const ITERACIONES: usize = 1_000;
const RONDAS: usize = 5;

fn limpiar(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(PathBuf::from(format!("{}-wal", path.display())));
    let _ = fs::remove_file(PathBuf::from(format!("{}-shm", path.display())));
}

fn configurar(conn: &Connection, clave: &str) -> rusqlite::Result<()> {
    conn.pragma_update(None, "key", clave)
}

fn sembrar(conn: &Connection) -> Result<(i64, i64), Box<dyn Error>> {
    let empresas = SqliteEmpresaRepository::new(conn);
    let empresa_id = empresas.crear(&Empresa {
        id: 0,
        nombre: "E2E CIPHER".to_owned(),
        activo: true,
    })?;

    let usuarios = SqliteUsuarioRepository::new(conn);
    let usuario_id = usuarios.crear(&Usuario {
        id: 0,
        cedula: "E2EROOT".to_owned(),
        nombre: "Root E2E".to_owned(),
        password_hash: generar_hash("password-e2e")?,
        rol: RolUsuario::Root,
        activo: true,
    })?;

    let contratistas = SqliteContratistaRepository::new(conn);
    let contratista_id = contratistas.crear(&Contratista::reconstruir(
        0,
        "E2E1001".to_owned(),
        "Contratista E2E".to_owned(),
        empresa_id,
        TipoIngreso::PorCorreo,
        None,
        false,
        true,
        true,
    ))?;

    Ok((usuario_id, contratista_id))
}

fn ejecutar_ronda(indice: usize) -> Result<f64, Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "brisas-e2e-sqlcipher-{}-{indice}.db",
        std::process::id()
    ));
    limpiar(&path);

    let conn = Connection::open(&path)?;
    configurar(&conn, CLAVE)?;
    let version: String = conn.pragma_query_value(None, "cipher_version", |row| row.get(0))?;
    initialize_database(&conn)?;

    let journal: String = conn.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    if !journal.eq_ignore_ascii_case("wal") {
        return Err(format!("journal_mode inesperado: {journal}").into());
    }

    let (_usuario_id, contratista_id) = sembrar(&conn)?;
    let core = AppCore::new(conn);
    let actor = core.autenticar("E2EROOT", "password-e2e")?;

    // Warm-up: ejercita exactamente el camino de negocio antes de medir.
    let warm = core.registrar_ingreso(&actor, contratista_id, MedioIngreso::Caminando, None)?;
    core.registrar_salida(&actor, warm.registro_id)?;

    let inicio = Instant::now();
    for _ in 0..ITERACIONES {
        let entrada = core.registrar_ingreso(
            &actor,
            contratista_id,
            MedioIngreso::Caminando,
            None,
        )?;
        core.registrar_salida(&actor, entrada.registro_id)?;
    }
    let elapsed = inicio.elapsed();

    let activos = core.listar_ingresos_activos(&FiltroIngresosActivos::default())?;
    if activos.total != 0 || !activos.items.is_empty() {
        return Err("quedó un ingreso activo después del ciclo E2E".into());
    }
    drop(core);

    let bytes = fs::read(&path)?;
    if bytes.starts_with(CABECERA_SQLITE) {
        return Err("la base final conserva la cabecera SQLite en claro".into());
    }

    let conn = Connection::open(&path)?;
    configurar(&conn, CLAVE)?;
    let movimientos: i64 = conn.query_row("SELECT COUNT(*) FROM registro_ingresos", [], |r| r.get(0))?;
    if movimientos != (ITERACIONES as i64 + 1) {
        return Err(format!("conteo inesperado de movimientos: {movimientos}").into());
    }
    drop(conn);

    let conn_mala = Connection::open(&path)?;
    configurar(&conn_mala, CLAVE_MALA)?;
    if conn_mala
        .query_row("SELECT COUNT(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))
        .is_ok()
    {
        return Err("una clave incorrecta pudo leer sqlite_master".into());
    }
    drop(conn_mala);

    let ms = elapsed.as_secs_f64() * 1_000.0;
    println!(
        "ROUND engine=sqlcipher cipher_version={} round={} cycles={} total_ms={:.3} us_per_cycle={:.3} db_bytes={}",
        version.trim(),
        indice + 1,
        ITERACIONES,
        ms,
        elapsed.as_secs_f64() * 1_000_000.0 / ITERACIONES as f64,
        bytes.len()
    );

    limpiar(&path);
    Ok(ms)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut muestras = Vec::with_capacity(RONDAS);
    for ronda in 0..RONDAS {
        muestras.push(ejecutar_ronda(ronda)?);
    }
    muestras.sort_by(f64::total_cmp);
    let mediana = muestras[RONDAS / 2];
    println!(
        "RESULT engine=sqlcipher rounds={} cycles_per_round={} median_ms={:.3} median_us_per_cycle={:.3}",
        RONDAS,
        ITERACIONES,
        mediana,
        mediana * 1_000.0 / ITERACIONES as f64
    );
    Ok(())
}
