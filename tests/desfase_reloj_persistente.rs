//! La hora de internet sobrevive al cierre de la app: el desfase medido
//! contra el servidor se guarda en la base y el próximo arranque lo aplica
//! antes de hablar con la nube (o aunque no haya internet). Antes vivía
//! sólo en memoria y cada arranque volvía a sellar con el reloj del equipo.
#![cfg(feature = "nube")]

use std::sync::Arc;

use chrono::{Duration, Utc};

use control_acceso::application::AppCore;
use control_acceso::tiempo::{Reloj, RelojCorregido};

const DIEZ_MINUTOS_MS: i64 = 10 * 60 * 1000;

#[test]
fn el_proximo_arranque_aplica_el_desfase_medido_antes() {
    let directorio = tempfile::tempdir().unwrap();
    let ruta = directorio.path().join("control_acceso.db");

    // Primer arranque: la nube mide el equipo 10 minutos adelantado.
    let core = AppCore::abrir_con_reloj(&ruta, Arc::new(RelojCorregido::nuevo())).unwrap();
    core.actualizar_desfase_reloj(DIEZ_MINUTOS_MS);
    drop(core);

    // Segundo arranque, sin haber hablado todavía con la nube.
    let reloj = Arc::new(RelojCorregido::nuevo());
    let _core = AppCore::abrir_con_reloj(&ruta, reloj.clone()).unwrap();

    let atraso = Utc::now() - reloj.ahora_utc();
    assert!(
        (atraso - Duration::milliseconds(DIEZ_MINUTOS_MS))
            .num_seconds()
            .abs()
            < 2,
        "debería sellar ~10 minutos detrás del reloj del equipo: {atraso}"
    );
}

#[test]
fn sin_medicion_guardada_el_reloj_queda_como_el_del_equipo() {
    let directorio = tempfile::tempdir().unwrap();
    let ruta = directorio.path().join("control_acceso.db");

    let reloj = Arc::new(RelojCorregido::nuevo());
    let _core = AppCore::abrir_con_reloj(&ruta, reloj.clone()).unwrap();

    assert!((Utc::now() - reloj.ahora_utc()).num_seconds().abs() < 2);
}
