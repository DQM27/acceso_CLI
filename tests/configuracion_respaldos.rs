use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use control_acceso::application::AppCore;
use control_acceso::database::backup::{ResultadoValidacion, TipoRespaldo};

/// Cada base vive en su propio directorio temporal, no directamente en
/// `temp_dir()`: `AppCore` ubica el directorio de respaldos junto a la base
/// activa (`<directorio>/backups`), así que dos pruebas que compartieran
/// directorio compartirían también su carpeta de respaldos.
fn archivo_temporal(nombre: &str) -> PathBuf {
    let unico = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directorio = std::env::temp_dir().join(format!(
        "control_acceso_respaldos_tui_{nombre}_{}_{unico}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directorio).unwrap();
    directorio.join("control_acceso.sqlite")
}

#[test]
fn crear_listar_y_validar_un_respaldo_a_traves_de_appcore() {
    let ruta = archivo_temporal("crear_listar_validar");
    let core = AppCore::abrir(&ruta).unwrap();

    let creado = core.crear_respaldo(TipoRespaldo::Manual).unwrap();
    assert_eq!(creado.tipo, TipoRespaldo::Manual);
    assert!(creado.ruta.exists());

    let listado = core.listar_respaldos().unwrap();
    assert_eq!(listado.len(), 1);
    assert_eq!(listado[0].ruta, creado.ruta);

    let validacion = core.validar_respaldo(&creado.ruta).unwrap();
    assert!(matches!(validacion, ResultadoValidacion::Valido { .. }));
}

#[test]
fn exportar_copia_el_archivo_ya_validado_a_la_ruta_indicada() {
    let ruta = archivo_temporal("exportar");
    let core = AppCore::abrir(&ruta).unwrap();
    let creado = core.crear_respaldo(TipoRespaldo::Manual).unwrap();

    let destino = archivo_temporal("exportar_destino");
    core.exportar_respaldo(&creado.ruta, &destino).unwrap();

    assert!(destino.exists());
    assert_eq!(
        std::fs::metadata(&destino).unwrap().len(),
        std::fs::metadata(&creado.ruta).unwrap().len()
    );
}

#[test]
fn el_directorio_de_respaldos_vive_junto_a_la_base_activa() {
    let ruta = archivo_temporal("directorio");
    let core = AppCore::abrir(&ruta).unwrap();

    let creado = core.crear_respaldo(TipoRespaldo::Manual).unwrap();

    let directorio_esperado = ruta.parent().unwrap().join("backups");
    assert_eq!(creado.ruta.parent().unwrap(), directorio_esperado);
}
