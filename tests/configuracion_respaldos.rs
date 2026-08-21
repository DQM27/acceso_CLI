use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{TimeZone, Utc};

use control_acceso::application::AppCore;
use control_acceso::database::backup::{ResultadoValidacion, TipoRespaldo};
use control_acceso::services::autenticacion_service::UsuarioSesion;
use control_acceso::services::usuario_service::CrearRootInicialInput;
use control_acceso::tiempo::RelojFijo;

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

fn root(core: &AppCore) -> UsuarioSesion {
    core.crear_root_inicial(CrearRootInicialInput {
        cedula: "ROOT".into(),
        nombre: "Root".into(),
        password: "password-root".into(),
    })
    .unwrap();
    core.autenticar("ROOT", "password-root").unwrap()
}

#[test]
fn crear_listar_y_validar_un_respaldo_a_traves_de_appcore() {
    let ruta = archivo_temporal("crear_listar_validar");
    let core = AppCore::abrir(&ruta).unwrap();
    let actor = root(&core);

    let creado = core.crear_respaldo(&actor, TipoRespaldo::Manual).unwrap();
    assert_eq!(creado.tipo, TipoRespaldo::Manual);
    assert!(creado.ruta.exists());

    let listado = core.listar_respaldos(&actor).unwrap();
    assert_eq!(listado.len(), 1);
    assert_eq!(listado[0].ruta, creado.ruta);

    let validacion = core.validar_respaldo(&actor, &creado.ruta).unwrap();
    assert!(matches!(validacion, ResultadoValidacion::Valido { .. }));
}

#[test]
fn exportar_copia_el_archivo_ya_validado_a_la_ruta_indicada() {
    let ruta = archivo_temporal("exportar");
    let core = AppCore::abrir(&ruta).unwrap();
    let actor = root(&core);
    let creado = core.crear_respaldo(&actor, TipoRespaldo::Manual).unwrap();

    let destino = archivo_temporal("exportar_destino");
    core.exportar_respaldo(&actor, &creado.ruta, &destino)
        .unwrap();

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
    let actor = root(&core);

    let creado = core.crear_respaldo(&actor, TipoRespaldo::Manual).unwrap();

    let directorio_esperado = ruta.parent().unwrap().join("backups");
    assert_eq!(creado.ruta.parent().unwrap(), directorio_esperado);
}

// `crear_respaldo` sella el nombre del archivo con la hora real de pared
// (`Utc::now()`), no con el reloj inyectado en `AppCore` — por diseño, el
// motor de respaldo (Fase 1) es independiente de `Reloj`. Para probar la
// lógica de "uno por día" de forma determinista sin depender de la hora real
// de la máquina, estas pruebas siembran a mano un respaldo `Automatico` con
// una fecha de archivo controlada (mismo patrón que ya usa
// `aplicar_retencion_conserva_solo_los_mas_recientes_del_tipo_indicado` en
// `tests/respaldo_backup.rs`).
fn sembrar_automatico(directorio_respaldos: &std::path::Path, fecha: &str) {
    std::fs::create_dir_all(directorio_respaldos).unwrap();
    std::fs::write(
        directorio_respaldos.join(format!("control_acceso_{fecha}_080000_automatico.db")),
        b"",
    )
    .unwrap();
}

#[test]
fn respaldo_automatico_diario_no_crea_uno_nuevo_si_ya_hay_uno_de_hoy() {
    let ruta = archivo_temporal("automatico_diario_mismo_dia");
    let hoy = Utc.with_ymd_and_hms(2026, 1, 16, 8, 0, 0).unwrap();
    sembrar_automatico(&ruta.parent().unwrap().join("backups"), "2026-01-16");
    let core = AppCore::abrir_con_reloj(&ruta, Arc::new(RelojFijo::new(hoy))).unwrap();
    let actor = root(&core);

    core.respaldo_automatico_diario_si_hace_falta();

    assert_eq!(core.listar_respaldos(&actor).unwrap().len(), 1);
}

/// Costa Rica es UTC-6 todo el año (sin horario de verano): 00:30 hora
/// local es 06:30 UTC. Corre de madrugada, no ni bien empieza el día
/// calendario, para no competir con el cierre de un turno nocturno.
#[test]
fn respaldo_automatico_diario_no_corre_antes_de_la_una_am_costa_rica() {
    let ruta = archivo_temporal("automatico_diario_antes_de_la_una");
    let antes_de_la_una = Utc.with_ymd_and_hms(2026, 1, 16, 6, 30, 0).unwrap();
    let core = AppCore::abrir_con_reloj(&ruta, Arc::new(RelojFijo::new(antes_de_la_una))).unwrap();
    let actor = root(&core);

    core.respaldo_automatico_diario_si_hace_falta();

    assert_eq!(core.listar_respaldos(&actor).unwrap().len(), 0);
}

#[test]
fn respaldo_automatico_diario_corre_a_partir_de_la_una_am_costa_rica() {
    let ruta = archivo_temporal("automatico_diario_desde_la_una");
    let la_una_en_punto = Utc.with_ymd_and_hms(2026, 1, 16, 7, 0, 0).unwrap();
    let core = AppCore::abrir_con_reloj(&ruta, Arc::new(RelojFijo::new(la_una_en_punto))).unwrap();
    let actor = root(&core);

    core.respaldo_automatico_diario_si_hace_falta();

    let listado = core.listar_respaldos(&actor).unwrap();
    assert_eq!(listado.len(), 1);
    assert_eq!(listado[0].tipo, TipoRespaldo::Automatico);
}

#[test]
fn respaldo_automatico_diario_crea_uno_nuevo_si_el_ultimo_es_de_otro_dia() {
    let ruta = archivo_temporal("automatico_diario_otro_dia");
    let hoy = Utc.with_ymd_and_hms(2026, 1, 16, 8, 0, 0).unwrap();
    sembrar_automatico(&ruta.parent().unwrap().join("backups"), "2026-01-15");
    let core = AppCore::abrir_con_reloj(&ruta, Arc::new(RelojFijo::new(hoy))).unwrap();
    let actor = root(&core);

    core.respaldo_automatico_diario_si_hace_falta();

    let listado = core.listar_respaldos(&actor).unwrap();
    assert_eq!(listado.len(), 2); // el sembrado de "ayer" + el nuevo de "hoy"
    assert!(listado.iter().all(|r| r.tipo == TipoRespaldo::Automatico));
}
