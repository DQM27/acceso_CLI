use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use control_acceso::database::backup::{
    ResultadoValidacion, TipoRespaldo, crear_respaldo, listar_respaldos, validar_respaldo,
};
use control_acceso::database::repositories::contratista_repository::{
    ContratistaRepository, SqliteContratistaRepository,
};
use control_acceso::database::repositories::empresa_repository::{
    EmpresaRepository, SqliteEmpresaRepository,
};
use control_acceso::database::schema::initialize_database;
use control_acceso::models::contratista::Contratista;
use control_acceso::models::empresa::Empresa;
use control_acceso::models::tipo_ingreso::TipoIngreso;

fn directorio_temporal(nombre: &str) -> PathBuf {
    let unico = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "control_acceso_respaldo_{nombre}_{}_{unico}",
        std::process::id()
    ))
}

fn base_con_datos() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    let empresa_id = SqliteEmpresaRepository::new(&connection)
        .crear(&Empresa {
            id: 0,
            nombre: "Constructora Alfa".into(),
        })
        .unwrap();
    SqliteContratistaRepository::new(&connection)
        .crear(&Contratista {
            id: 0,
            cedula: "101010101".into(),
            nombre: "Ana Solano".into(),
            empresa_id,
            tipo_ingreso: TipoIngreso::Swat,
            fecha_vencimiento_praind: None,
            es_personal_ruta: false,
            tiene_acceso: true,
        })
        .unwrap();
    connection
}

#[test]
fn crea_un_respaldo_valido_que_conserva_los_mismos_datos() {
    let connection = base_con_datos();
    let directorio = directorio_temporal("valido");

    let resumen = crear_respaldo(&connection, &directorio, TipoRespaldo::Manual).unwrap();

    assert_eq!(resumen.tipo, TipoRespaldo::Manual);
    assert!(resumen.tamano_bytes > 0);
    assert!(resumen.ruta.exists());
    assert_eq!(resumen.ruta.extension().unwrap(), "db");

    let copia = Connection::open(&resumen.ruta).unwrap();
    let contratistas: i64 = copia
        .query_row("SELECT COUNT(*) FROM contratistas", [], |r| r.get(0))
        .unwrap();
    let empresas: i64 = copia
        .query_row("SELECT COUNT(*) FROM empresas", [], |r| r.get(0))
        .unwrap();
    assert_eq!(contratistas, 1);
    assert_eq!(empresas, 1);

    assert!(matches!(
        validar_respaldo(&resumen.ruta).unwrap(),
        ResultadoValidacion::Valido { .. }
    ));

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn no_deja_archivos_partial_tras_una_creacion_exitosa() {
    let connection = base_con_datos();
    let directorio = directorio_temporal("sin_partial");

    crear_respaldo(&connection, &directorio, TipoRespaldo::Automatico).unwrap();

    let quedan_partials = std::fs::read_dir(&directorio)
        .unwrap()
        .filter_map(Result::ok)
        .any(|entrada| entrada.path().extension().and_then(|e| e.to_str()) == Some("partial"));
    assert!(!quedan_partials, "no debe quedar ningún .partial tras un respaldo exitoso");

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn dos_respaldos_seguidos_no_se_pisan_entre_si() {
    let connection = base_con_datos();
    let directorio = directorio_temporal("colision");

    let primero = crear_respaldo(&connection, &directorio, TipoRespaldo::Manual).unwrap();
    let segundo = crear_respaldo(&connection, &directorio, TipoRespaldo::Manual).unwrap();

    assert_ne!(primero.ruta, segundo.ruta);
    assert!(primero.ruta.exists());
    assert!(segundo.ruta.exists());

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn validar_respaldo_rechaza_un_archivo_truncado() {
    use std::fs::OpenOptions;
    use std::io::{Seek, SeekFrom};

    let connection = base_con_datos();
    let directorio = directorio_temporal("truncado");
    let resumen = crear_respaldo(&connection, &directorio, TipoRespaldo::Manual).unwrap();

    let longitud = std::fs::metadata(&resumen.ruta).unwrap().len();
    let mut archivo = OpenOptions::new().write(true).open(&resumen.ruta).unwrap();
    archivo.seek(SeekFrom::Start(longitud / 2)).unwrap();
    archivo.set_len(longitud / 2).unwrap();
    drop(archivo);

    // Un archivo truncado a la mitad no debe validar como sano: o falla al
    // abrirlo, o `integrity_check` lo detecta explícitamente.
    if let Ok(resultado) = validar_respaldo(&resumen.ruta) {
        assert!(!resultado.es_valido());
    }

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn validar_respaldo_rechaza_un_esquema_de_una_version_futura() {
    let directorio = directorio_temporal("futuro");
    std::fs::create_dir_all(&directorio).unwrap();
    let ruta = directorio.join("futuro.db");

    let connection = Connection::open(&ruta).unwrap();
    connection
        .execute_batch("PRAGMA user_version = 999999;")
        .unwrap();
    drop(connection);

    assert_eq!(
        validar_respaldo(&ruta).unwrap(),
        ResultadoValidacion::EsquemaIncompatible {
            version_encontrada: 999999
        }
    );

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn validar_respaldo_rechaza_claves_foraneas_invalidas() {
    let directorio = directorio_temporal("fk_invalida");
    std::fs::create_dir_all(&directorio).unwrap();
    let ruta = directorio.join("fk_invalida.db");

    let connection = Connection::open(&ruta).unwrap();
    initialize_database(&connection).unwrap();
    // Cuela una referencia inválida desactivando la protección normal de
    // claves foráneas, a propósito, para poder probar que la validación la
    // detecta igual (foreign_key_check no depende de que la conexión activa
    // haya tenido foreign_keys=ON al insertar).
    connection
        .execute_batch(
            "
            PRAGMA foreign_keys = OFF;
            INSERT INTO empresas(id, nombre) VALUES (1, 'Empresa real');
            INSERT INTO contratistas(
                cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso
            ) VALUES ('1', 'Fantasma', 999, 'SWAT', 0, 1);
            ",
        )
        .unwrap();
    drop(connection);

    let resultado = validar_respaldo(&ruta).unwrap();
    assert!(matches!(resultado, ResultadoValidacion::Invalido(_)));

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn listar_respaldos_ordena_del_mas_reciente_y_omite_partial() {
    let directorio = directorio_temporal("listado");
    std::fs::create_dir_all(&directorio).unwrap();

    for nombre in [
        "control_acceso_2026-08-10_080000_manual.db",
        "control_acceso_2026-08-15_120000_automatico.db",
        "control_acceso_2026-08-12_090000_pre_migracion.db",
        "control_acceso_2026-08-12_090000_pre_restauracion_2.db",
        "control_acceso_2026-08-16_000000_manual.partial",
    ] {
        std::fs::write(directorio.join(nombre), b"contenido de prueba, no es sqlite real").unwrap();
    }

    let respaldos = listar_respaldos(&directorio).unwrap();
    assert_eq!(respaldos.len(), 4, "el .partial no debe listarse");
    assert!(respaldos.windows(2).all(|par| par[0].creado_en >= par[1].creado_en));
    assert_eq!(respaldos[0].tipo, TipoRespaldo::Automatico);
    assert!(respaldos.iter().any(|r| r.tipo == TipoRespaldo::PreRestauracion));

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn listar_respaldos_de_un_directorio_inexistente_devuelve_vacio() {
    let directorio = directorio_temporal("inexistente");
    assert!(listar_respaldos(&directorio).unwrap().is_empty());
}
