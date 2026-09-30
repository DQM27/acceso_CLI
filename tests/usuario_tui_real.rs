use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use control_acceso::{
    application::AppCore, database::queries::usuarios::FiltroUsuarios, models::usuario::RolUsuario,
    services::usuario_service::CrearRootInicialInput,
};

fn ruta(nombre: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "brisas-{nombre}-{}-{}.db",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

/// La búsqueda de usuarios (FTS) ignora tildes y mayúsculas, no expone el
/// hash y sobrevive a reabrir la base. Los usuarios se dan de alta desde el
/// panel web; en un equipo sólo existe el ROOT del arranque inicial.
#[test]
fn appcore_busca_usuarios_con_fts_sin_exponer_hash() {
    let ruta = ruta("usuarios-busqueda");
    let core = AppCore::abrir(&ruta).unwrap();
    let id = core
        .crear_root_inicial(CrearRootInicialInput {
            cedula: "0-01".into(),
            nombre: "María José Hernández".into(),
            password: "password-A".into(),
        })
        .unwrap();
    let actor = core.autenticar("0-01", "password-A").unwrap();
    for texto in ["jose", "JOSÉ", "hernandez", "nandez"] {
        let items = core
            .buscar_usuarios(
                &actor,
                &FiltroUsuarios {
                    texto: Some(texto.into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let usuario = items.iter().find(|u| u.id == id).unwrap();
        assert_eq!(usuario.cedula, "0-01");
        assert_eq!(usuario.rol, RolUsuario::Root);
        assert!(!format!("{usuario:?}").contains("password_hash"));
    }
    drop(core);

    let core = AppCore::abrir(&ruta).unwrap();
    let actor = core.autenticar("0-01", "password-A").unwrap();
    assert!(
        core.buscar_usuarios(
            &actor,
            &FiltroUsuarios {
                texto: Some("hernandez".into()),
                ..Default::default()
            }
        )
        .unwrap()
        .iter()
        .any(|u| u.id == id)
    );
    drop(core);
    fs::remove_file(ruta).unwrap();
}
