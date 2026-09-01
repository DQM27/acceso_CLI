fn main() {
    let ruta = std::env::args().nth(1).expect("uso: verificar_seed <ruta_db>");
    let conexion = control_acceso::database::connection::open_database(&ruta).expect("no se pudo abrir");

    let (cedula, nombre, rol): (String, String, String) = conexion
        .query_row(
            "SELECT cedula, nombre, rol FROM usuarios WHERE cedula = '123456789'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("usuario root no encontrado");
    println!("usuario: {cedula} / {nombre} / {rol}");

    let total: i64 = conexion
        .query_row("SELECT COUNT(*) FROM contratistas", [], |r| r.get(0))
        .expect("no se pudo contar contratistas");
    println!("contratistas: {total}");
}
