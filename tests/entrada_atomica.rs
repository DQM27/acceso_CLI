use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use rusqlite::{Connection, params};

use control_acceso::{
    application::AppCore,
    database::schema::initialize_database,
    domain::resultado_acceso::MotivoDenegacion,
    models::{medio_ingreso::MedioIngreso, usuario::RolUsuario},
    services::autenticacion_service::UsuarioSesion,
    services::error::RegistroIngresoServiceError,
};

static SECUENCIA: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, PartialEq, Eq)]
enum ResultadoConcurrente {
    Registrado,
    IngresoActivo,
    GafeteOcupado,
    AccesoRevocado,
    OtroError(String),
}

fn clasificar(
    resultado: Result<
        control_acceso::services::registro_ingreso_service::ResultadoRegistroEntrada,
        RegistroIngresoServiceError,
    >,
) -> ResultadoConcurrente {
    match resultado {
        Ok(_) => ResultadoConcurrente::Registrado,
        Err(RegistroIngresoServiceError::IngresoActivo) => ResultadoConcurrente::IngresoActivo,
        Err(RegistroIngresoServiceError::GafeteOcupado) => ResultadoConcurrente::GafeteOcupado,
        Err(RegistroIngresoServiceError::AccesoDenegado(MotivoDenegacion::SinAcceso)) => {
            ResultadoConcurrente::AccesoRevocado
        }
        Err(error) => ResultadoConcurrente::OtroError(error.to_string()),
    }
}

fn base_temporal(nombre: &str) -> PathBuf {
    let numero = SECUENCIA.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "control_acceso_entrada_atomica_{nombre}_{}_{numero}.db",
        std::process::id()
    ))
}

fn limpiar_base(ruta: &Path) {
    let _ = fs::remove_file(ruta);
    let _ = fs::remove_file(ruta.with_extension("db-journal"));
    let _ = fs::remove_file(ruta.with_extension("db-wal"));
    let _ = fs::remove_file(ruta.with_extension("db-shm"));
}

fn preparar_base(ruta: &Path) -> (i64, i64) {
    limpiar_base(ruta);
    let connection = Connection::open(ruta).unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute("INSERT INTO empresas (nombre) VALUES ('Empresa')", [])
        .unwrap();
    let empresa_id = connection.last_insert_rowid();
    connection
        .execute(
            "INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
             VALUES ('1001', 'Operador', 'hash', 'OPERADOR', 1)",
            [],
        )
        .unwrap();
    let usuario_id = connection.last_insert_rowid();
    connection
        .execute(
            "INSERT INTO gafetes (numero, estado) VALUES (25, 'DISPONIBLE')",
            [],
        )
        .unwrap();
    (empresa_id, usuario_id)
}

fn actor(usuario_id: i64) -> UsuarioSesion {
    UsuarioSesion {
        id: usuario_id,
        cedula: "1001".into(),
        nombre: "Operador".into(),
        rol: RolUsuario::Operador,
    }
}

fn crear_contratista(ruta: &Path, empresa_id: i64, cedula: &str, tipo_ingreso: &str) -> i64 {
    let connection = Connection::open(ruta).unwrap();
    connection
        .execute(
            "INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso,
                fecha_vencimiento_praind, es_personal_ruta, tiene_acceso
             ) VALUES (?1, ?2, ?3, ?4, NULL, 0, 1)",
            params![
                cedula,
                format!("Persona {cedula}"),
                empresa_id,
                tipo_ingreso
            ],
        )
        .unwrap();
    connection.last_insert_rowid()
}

fn ejecutar_en_paralelo(
    ruta: &Path,
    usuario_id: i64,
    solicitudes: [(i64, Option<i64>); 2],
) -> Vec<ResultadoConcurrente> {
    let barrera = Arc::new(Barrier::new(3));
    let conexiones = [AppCore::abrir(ruta).unwrap(), AppCore::abrir(ruta).unwrap()];
    let hilos: Vec<_> = solicitudes
        .into_iter()
        .zip(conexiones)
        .map(|((contratista_id, gafete), core)| {
            let barrera = Arc::clone(&barrera);
            thread::spawn(move || {
                barrera.wait();
                clasificar(core.registrar_ingreso(
                    &actor(usuario_id),
                    contratista_id,
                    MedioIngreso::Caminando,
                    gafete,
                ))
            })
        })
        .collect();

    barrera.wait();
    hilos.into_iter().map(|hilo| hilo.join().unwrap()).collect()
}

#[test]
fn dos_confirmaciones_del_mismo_contratista_producen_un_solo_ingreso() {
    let ruta = base_temporal("contratista");
    let (empresa_id, usuario_id) = preparar_base(&ruta);
    let contratista_id = crear_contratista(&ruta, empresa_id, "2001", "SWAT");

    let resultados = ejecutar_en_paralelo(
        &ruta,
        usuario_id,
        [(contratista_id, None), (contratista_id, None)],
    );

    assert_eq!(
        resultados
            .iter()
            .filter(|resultado| **resultado == ResultadoConcurrente::Registrado)
            .count(),
        1
    );
    assert_eq!(
        resultados
            .iter()
            .filter(|resultado| **resultado == ResultadoConcurrente::IngresoActivo)
            .count(),
        1
    );

    let connection = Connection::open(&ruta).unwrap();
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM registro_ingresos", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(total, 1);
    drop(connection);
    limpiar_base(&ruta);
}

#[test]
fn dos_contratistas_no_pueden_confirmar_el_mismo_gafete() {
    let ruta = base_temporal("gafete");
    let (empresa_id, usuario_id) = preparar_base(&ruta);
    let primero = crear_contratista(&ruta, empresa_id, "2001", "POR_CORREO");
    let segundo = crear_contratista(&ruta, empresa_id, "2002", "POR_CORREO");

    let resultados = ejecutar_en_paralelo(
        &ruta,
        usuario_id,
        [(primero, Some(25)), (segundo, Some(25))],
    );

    assert_eq!(
        resultados
            .iter()
            .filter(|resultado| **resultado == ResultadoConcurrente::Registrado)
            .count(),
        1
    );
    assert_eq!(
        resultados
            .iter()
            .filter(|resultado| **resultado == ResultadoConcurrente::GafeteOcupado)
            .count(),
        1
    );

    let connection = Connection::open(&ruta).unwrap();
    let total: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM registro_ingresos WHERE gafete_numero = 25",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(total, 1);
    drop(connection);
    limpiar_base(&ruta);
}

#[test]
fn una_revocacion_confirmada_antes_del_bloqueo_impide_el_ingreso() {
    let ruta = base_temporal("revocacion");
    let (empresa_id, usuario_id) = preparar_base(&ruta);
    let contratista_id = crear_contratista(&ruta, empresa_id, "2001", "SWAT");
    let core = AppCore::abrir(&ruta).unwrap();

    let bloqueadora = Connection::open(&ruta).unwrap();
    bloqueadora
        .execute_batch("BEGIN IMMEDIATE TRANSACTION")
        .unwrap();
    bloqueadora
        .execute(
            "UPDATE contratistas SET tiene_acceso = 0 WHERE id = ?1",
            [contratista_id],
        )
        .unwrap();

    let (inicio_tx, inicio_rx) = mpsc::channel();
    let (resultado_tx, resultado_rx) = mpsc::channel();
    let hilo = thread::spawn(move || {
        inicio_tx.send(()).unwrap();
        let resultado = clasificar(core.registrar_ingreso(
            &actor(usuario_id),
            contratista_id,
            MedioIngreso::Caminando,
            None,
        ));
        resultado_tx.send(resultado).unwrap();
    });

    inicio_rx.recv().unwrap();
    thread::sleep(Duration::from_millis(100));
    assert!(matches!(
        resultado_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));

    bloqueadora.execute_batch("COMMIT").unwrap();
    let resultado = resultado_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    hilo.join().unwrap();

    assert_eq!(resultado, ResultadoConcurrente::AccesoRevocado);
    let connection = Connection::open(&ruta).unwrap();
    let total: i64 = connection
        .query_row("SELECT COUNT(*) FROM registro_ingresos", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(total, 0);
    connection
        .execute(
            "UPDATE contratistas SET tiene_acceso = 1 WHERE id = ?1",
            [contratista_id],
        )
        .unwrap();
    drop(connection);

    AppCore::abrir(&ruta)
        .unwrap()
        .registrar_ingreso(
            &actor(usuario_id),
            contratista_id,
            MedioIngreso::Caminando,
            None,
        )
        .unwrap();

    limpiar_base(&ruta);
}
