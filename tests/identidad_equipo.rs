//! La unidad y la etiqueta del equipo llegan en cada token y quedan
//! guardadas, para que el login y la barra de estado las muestren aunque
//! el próximo arranque sea sin internet.
#![cfg(feature = "nube")]

use std::sync::Arc;

use control_acceso::application::AppCore;
use control_acceso::nube::TokenDispositivo;
use control_acceso::tiempo::RelojCorregido;

fn token(sitio_nombre: Option<&str>, etiqueta: Option<&str>) -> TokenDispositivo {
    TokenDispositivo {
        access_token: "jwt".to_string(),
        expires_in: 3600,
        sitio_id: "s1".to_string(),
        dispositivo_id: "d1".to_string(),
        tipo: "pc".to_string(),
        desfase_reloj_ms: None,
        sitio_nombre: sitio_nombre.map(str::to_string),
        etiqueta: etiqueta.map(str::to_string),
    }
}

#[test]
fn la_unidad_del_token_sobrevive_al_reinicio_y_un_token_sin_ella_no_la_borra() {
    let directorio = tempfile::tempdir().unwrap();
    let ruta = directorio.path().join("control_acceso.db");

    let core = AppCore::abrir_con_reloj(&ruta, Arc::new(RelojCorregido::nuevo())).unwrap();
    assert!(
        core.identidad_equipo().esta_vacia(),
        "antes de cualquier token"
    );
    core.aplicar_token(&token(Some("Planta Cartago"), Some("PC portería norte")));
    // Un servidor anterior no manda los nombres: no se pierde lo sabido.
    core.aplicar_token(&token(None, None));
    drop(core);

    let core = AppCore::abrir_con_reloj(&ruta, Arc::new(RelojCorregido::nuevo())).unwrap();
    let identidad = core.identidad_equipo();
    assert_eq!(identidad.unidad.as_deref(), Some("Planta Cartago"));
    assert_eq!(identidad.etiqueta.as_deref(), Some("PC portería norte"));

    // Si el panel cambia la etiqueta, el próximo token la actualiza.
    core.aplicar_token(&token(Some("Planta Cartago"), Some("PC portería sur")));
    assert_eq!(
        core.identidad_equipo().etiqueta.as_deref(),
        Some("PC portería sur")
    );
}
