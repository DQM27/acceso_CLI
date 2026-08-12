use control_acceso::database::connection::open_database;
use control_acceso::database::schema::initialize_database;

fn main() -> rusqlite::Result<()> {
    println!("Sistema de Control de Acceso");

    let connection = open_database("control_acceso.db")?;

    initialize_database(&connection)?;

    println!("Base de datos inicializada correctamente.");

    Ok(())
}