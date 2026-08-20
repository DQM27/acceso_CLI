use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use control_acceso::database::backup::{
    ResultadoValidacion, TipoRespaldo, aplicar_retencion, crear_respaldo, listar_respaldos,
    restaurar_respaldo, validar_respaldo,
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

fn poblar(connection: &Connection, nombre_empresa: &str, cedula: &str) {
    let empresa_id = SqliteEmpresaRepository::new(connection)
        .crear(&Empresa {
            id: 0,
            nombre: nombre_empresa.into(),
            activo: true,
        })
        .unwrap();
    SqliteContratistaRepository::new(connection)
        .crear(&Contratista {
            id: 0,
            cedula: cedula.into(),
            nombre: "Ana Solano".into(),
            empresa_id,
            tipo_ingreso: TipoIngreso::Swat,
            fecha_vencimiento_praind: None,
            es_personal_ruta: false,
            tiene_acceso: true,
            empresa_activa: true,
        })
        .unwrap();
}

fn base_en_archivo(ruta: &std::path::Path, nombre_empresa: &str, cedula: &str) -> Connection {
    let connection = Connection::open(ruta).unwrap();
    initialize_database(&connection).unwrap();
    poblar(&connection, nombre_empresa, cedula);
    connection
}

fn base_con_datos() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    poblar(&connection, "Constructora Alfa", "101010101");
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
    assert!(
        !quedan_partials,
        "no debe quedar ningún .partial tras un respaldo exitoso"
    );

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
        std::fs::write(
            directorio.join(nombre),
            b"contenido de prueba, no es sqlite real",
        )
        .unwrap();
    }

    let respaldos = listar_respaldos(&directorio).unwrap();
    assert_eq!(respaldos.len(), 4, "el .partial no debe listarse");
    assert!(
        respaldos
            .windows(2)
            .all(|par| par[0].creado_en >= par[1].creado_en)
    );
    assert_eq!(respaldos[0].tipo, TipoRespaldo::Automatico);
    assert!(
        respaldos
            .iter()
            .any(|r| r.tipo == TipoRespaldo::PreRestauracion)
    );

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn listar_respaldos_de_un_directorio_inexistente_devuelve_vacio() {
    let directorio = directorio_temporal("inexistente");
    assert!(listar_respaldos(&directorio).unwrap().is_empty());
}

fn contar(ruta: &std::path::Path, tabla: &str) -> i64 {
    Connection::open(ruta)
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {tabla}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn restaurar_un_respaldo_valido_reemplaza_los_datos_activos() {
    let directorio = directorio_temporal("restaurar_ok");
    std::fs::create_dir_all(&directorio).unwrap();
    let ruta_activa = directorio.join("activa.db");
    let ruta_candidata_origen = directorio.join("candidata_origen.db");

    let activa = base_en_archivo(&ruta_activa, "Empresa Vieja", "1");
    let origen_candidata = base_en_archivo(&ruta_candidata_origen, "Empresa Nueva", "2");
    // Un segundo contratista en la candidata, para distinguirla claramente
    // de la activa (que sólo tiene uno) después de restaurar.
    poblar(&origen_candidata, "Otra Empresa", "3");
    let respaldo = crear_respaldo(
        &origen_candidata,
        &directorio.join("respaldos"),
        TipoRespaldo::Manual,
    )
    .unwrap();
    drop(activa);
    drop(origen_candidata);

    restaurar_respaldo(&respaldo.ruta, &ruta_activa).unwrap();

    assert_eq!(contar(&ruta_activa, "empresas"), 2);
    assert_eq!(contar(&ruta_activa, "contratistas"), 2);
    let nombre: String = Connection::open(&ruta_activa)
        .unwrap()
        .query_row("SELECT nombre FROM empresas ORDER BY id LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(nombre, "Empresa Nueva");

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn restaurar_rechaza_un_candidato_invalido_sin_tocar_la_base_activa() {
    let directorio = directorio_temporal("restaurar_invalido");
    std::fs::create_dir_all(&directorio).unwrap();
    let ruta_activa = directorio.join("activa.db");
    let activa = base_en_archivo(&ruta_activa, "Empresa Original", "1");
    drop(activa);

    let ruta_candidata = directorio.join("candidata_rota.db");
    std::fs::write(&ruta_candidata, b"esto no es un archivo sqlite").unwrap();

    let resultado = restaurar_respaldo(&ruta_candidata, &ruta_activa);
    assert!(resultado.is_err());
    assert_eq!(contar(&ruta_activa, "empresas"), 1);
    let nombre: String = Connection::open(&ruta_activa)
        .unwrap()
        .query_row("SELECT nombre FROM empresas", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        nombre, "Empresa Original",
        "la base activa no debe tocarse si el candidato no valida"
    );

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn restaurar_reinstala_la_base_anterior_si_falla_despues_del_intercambio() {
    let directorio = directorio_temporal("restaurar_rollback");
    std::fs::create_dir_all(&directorio).unwrap();
    let ruta_activa = directorio.join("activa.db");
    let activa = base_en_archivo(&ruta_activa, "Empresa Original", "1");
    drop(activa);

    // Candidato que pasa validar_respaldo (SQLite válido, integridad ok, sin
    // violaciones de FK, user_version dentro del rango conocido) pero que no
    // tiene el esquema real que esa versión supone tener: al intentar
    // migrarlo de verdad después del intercambio, tiene que fallar.
    let ruta_candidata = directorio.join("candidata_incompatible.db");
    let candidata = Connection::open(&ruta_candidata).unwrap();
    candidata.execute_batch("PRAGMA user_version = 1;").unwrap();
    drop(candidata);

    let resultado = restaurar_respaldo(&ruta_candidata, &ruta_activa);
    assert!(resultado.is_err());

    // La base activa debe seguir siendo la original, no la candidata rota,
    // y no debe quedar ningún archivo temporal de la operación.
    assert!(ruta_activa.exists());
    assert_eq!(contar(&ruta_activa, "empresas"), 1);
    let nombre: String = Connection::open(&ruta_activa)
        .unwrap()
        .query_row("SELECT nombre FROM empresas", [], |r| r.get(0))
        .unwrap();
    assert_eq!(nombre, "Empresa Original");
    assert!(
        !directorio
            .join(".control_acceso_restauracion.previa")
            .exists()
    );
    assert!(!directorio.join(".control_acceso_restauracion.tmp").exists());

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn restaurar_sobre_una_ruta_activa_inexistente_funciona_como_primera_carga() {
    let directorio = directorio_temporal("restaurar_sin_activa");
    std::fs::create_dir_all(&directorio).unwrap();
    let ruta_activa = directorio.join("no_existe_todavia.db");

    let ruta_candidata_origen = directorio.join("candidata_origen.db");
    let origen = base_en_archivo(&ruta_candidata_origen, "Empresa Semilla", "1");
    let respaldo =
        crear_respaldo(&origen, &directorio.join("respaldos"), TipoRespaldo::Manual).unwrap();
    drop(origen);

    restaurar_respaldo(&respaldo.ruta, &ruta_activa).unwrap();

    assert!(ruta_activa.exists());
    assert_eq!(contar(&ruta_activa, "empresas"), 1);

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn aplicar_retencion_conserva_solo_los_mas_recientes_del_tipo_indicado() {
    use chrono::Datelike;

    let directorio = directorio_temporal("retencion");
    std::fs::create_dir_all(&directorio).unwrap();

    // 5 automáticos en días distintos, más 2 manuales que la retención de
    // Automatico nunca debe tocar.
    for dia in 1..=5 {
        std::fs::write(
            directorio.join(format!(
                "control_acceso_2026-01-0{dia}_120000_automatico.db"
            )),
            b"",
        )
        .unwrap();
    }
    std::fs::write(
        directorio.join("control_acceso_2026-01-01_120000_manual.db"),
        b"",
    )
    .unwrap();
    std::fs::write(
        directorio.join("control_acceso_2026-01-02_120000_manual.db"),
        b"",
    )
    .unwrap();

    let eliminados = aplicar_retencion(&directorio, TipoRespaldo::Automatico, 3).unwrap();

    assert_eq!(eliminados.len(), 2);
    let restantes = listar_respaldos(&directorio).unwrap();
    let mut dias_automaticos_restantes: Vec<u32> = restantes
        .iter()
        .filter(|respaldo| respaldo.tipo == TipoRespaldo::Automatico)
        .map(|respaldo| respaldo.creado_en.day())
        .collect();
    dias_automaticos_restantes.sort_unstable();
    assert_eq!(dias_automaticos_restantes, vec![3, 4, 5]);
    assert_eq!(
        restantes
            .iter()
            .filter(|respaldo| respaldo.tipo == TipoRespaldo::Manual)
            .count(),
        2
    );

    let _ = std::fs::remove_dir_all(&directorio);
}

#[test]
fn aplicar_retencion_con_menos_respaldos_que_el_limite_no_borra_nada() {
    let directorio = directorio_temporal("retencion_sin_exceso");
    std::fs::create_dir_all(&directorio).unwrap();
    std::fs::write(
        directorio.join("control_acceso_2026-01-01_120000_pre_migracion.db"),
        b"",
    )
    .unwrap();

    let eliminados = aplicar_retencion(&directorio, TipoRespaldo::PreMigracion, 3).unwrap();

    assert!(eliminados.is_empty());
    assert_eq!(listar_respaldos(&directorio).unwrap().len(), 1);

    let _ = std::fs::remove_dir_all(&directorio);
}
