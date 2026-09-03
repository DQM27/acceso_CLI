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
use control_acceso::database::repositories::empresa_repository::{
    EmpresaRepository, SqliteEmpresaRepository,
};
use control_acceso::database::repositories::gafete_repository::{
    GafeteRepository, SqliteGafeteRepository,
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
    // hecho desde la GUI. Toca también su empresa: si es una fila vieja
    // (previa al espejo de empresas), nunca tuvo su propio 'crear'
    // encolado, y el contratista fallaría por la FK real del lado de la
    // nube (`contratistas.empresa_id references empresas`) hasta que
    // alguien la toque una vez.
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

    let empresas = SqliteEmpresaRepository::new(&connection);
    let empresa = empresas
        .buscar_por_id(contratista.empresa_id)
        .expect("buscar empresa")
        .expect("la empresa del contratista debe existir");
    println!("Tocando empresa #{}: {}", empresa.id, empresa.nombre);
    empresas.actualizar(&empresa).expect("actualizar empresa");

    repo.actualizar(&contratista)
        .expect("actualizar contratista");

    // Toca (o crea, si no hay ninguno) un gafete para probar el espejo
    // nuevo -- mismo camino que tocaría un cambio real hecho desde la GUI.
    let gafetes = SqliteGafeteRepository::new(&connection);
    let gafete_numero: Option<i64> = connection
        .query_row("SELECT numero FROM gafetes LIMIT 1", [], |row| row.get(0))
        .ok();
    let gafete_id = gafete_numero.map_or_else(
        || {
            println!("No hay gafetes locales, creando uno de prueba (#999999)");
            gafetes.crear(999_999).expect("crear gafete de prueba")
        },
        |numero| {
            println!("Tocando gafete #{numero}");
            gafetes
                .buscar_por_numero(numero)
                .expect("buscar gafete")
                .expect("el gafete debe existir")
                .id
        },
    );
    gafetes
        .dar_de_baja(gafete_id)
        .expect("tocar gafete (dar de baja)");
    gafetes
        .resolver(gafete_id)
        .expect("tocar gafete (resolver)");

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

    if resumen.fallidos > 0 {
        let mut statement = connection
            .prepare("SELECT entidad, entidad_uuid, ultimo_error FROM cola_salida WHERE estado != 'enviado' ORDER BY id DESC LIMIT 5")
            .unwrap();
        let filas = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .unwrap();
        for fila in filas {
            let (entidad, uuid, error) = fila.unwrap();
            println!("  fallo: {entidad} {uuid} -> {error:?}");
        }
    }
}
