//! Prueba de campo manual: hace exactamente lo que hace el botón
//! "Sincronizar ahora" de la pantalla Nube, contra la base real y el
//! receptor real. Uso:
//!
//! ```text
//! cargo run --example probar_sincronizacion --features nube
//! ```

use control_acceso::database::connection::{open_database, ruta_base_datos};
use control_acceso::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use control_acceso::instancia::InstanciaGuard;
use control_acceso::nube;

fn main() {
    let db_path = ruta_base_datos().expect("no se pudo resolver la ruta de la base de datos");
    println!("Base de datos: {}", db_path.display());

    let _instancia = InstanciaGuard::adquirir(&db_path)
        .expect("no se pudo adquirir el bloqueo de instancia (¿la app está abierta?)");
    let connection = open_database(&db_path).expect("no se pudo abrir/migrar la base de datos");

    // Toca un contratista real (sin cambiar sus datos) para que quede
    // encolado un 'actualizar' -- mismo camino que tocaría un cambio real
    // hecho desde la GUI.
    let repo = SqliteContratistaRepository::new(&connection);
    let contratista = repo
        .listar()
        .expect("listar contratistas")
        .into_iter()
        .next()
        .expect("la base necesita al menos un contratista para esta prueba");
    println!(
        "Tocando contratista #{}: {}",
        contratista.id, contratista.nombre
    );
    repo.actualizar(&contratista).expect("actualizar contratista");

    let pendientes: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM cola_salida WHERE estado = 'pendiente'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    println!("Pendientes en cola_salida antes de sincronizar: {pendientes}");

    let secreto =
        nube::credenciales::cargar_secreto().expect("no hay secreto de dispositivo guardado");
    let token = nube::autenticar_dispositivo(nube::BASE_URL, &secreto)
        .expect("no se pudo autenticar el dispositivo");
    println!(
        "Autenticado: sitio={} dispositivo={} tipo={}",
        token.sitio_id, token.dispositivo_id, token.tipo
    );

    let contexto = nube::ContextoSincronizacion {
        base_url: nube::BASE_URL,
        apikey: nube::APIKEY,
        token: &token.access_token,
        dispositivo_id: &token.dispositivo_id,
        sitio_id: &token.sitio_id,
    };
    let resumen = nube::drenar_cola(&connection, &contexto, 200).expect("fallo al drenar la cola");
    println!(
        "Resultado: {} enviados, {} fallidos",
        resumen.enviados, resumen.fallidos
    );
}
