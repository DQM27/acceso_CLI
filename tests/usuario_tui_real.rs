use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use control_acceso::{
    application::AppCore,
    database::queries::usuarios::FiltroUsuarios,
    models::usuario::RolUsuario,
    services::{
        autenticacion_service::UsuarioSesion,
        error::{AutenticacionError, UsuarioServiceError},
        usuario_service::{ActualizarUsuarioInput, CrearRootInicialInput, CrearUsuarioInput},
    },
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
fn root(core: &AppCore) -> i64 {
    core.crear_root_inicial(CrearRootInicialInput {
        cedula: "ROOT-1".into(),
        nombre: "Root Inicial".into(),
        password: "password-A".into(),
    })
    .unwrap()
}
fn sesion_root(core: &AppCore) -> UsuarioSesion {
    core.autenticar("ROOT-1", "password-A").unwrap()
}
fn crear(
    core: &AppCore,
    actor: &UsuarioSesion,
    cedula: &str,
    nombre: &str,
    rol: RolUsuario,
    activo: bool,
) -> i64 {
    core.crear_usuario(
        actor,
        CrearUsuarioInput {
            cedula: cedula.into(),
            nombre: nombre.into(),
            password: "password-A".into(),
            rol,
            activo,
        },
    )
    .unwrap()
}

#[test]
fn appcore_busca_crea_edita_activa_y_fts_sin_exponer_hash() {
    let ruta = ruta("usuarios-crud");
    let core = AppCore::abrir(&ruta).unwrap();
    root(&core);
    let actor = sesion_root(&core);
    let id = crear(
        &core,
        &actor,
        "0-01",
        "María José Hernández",
        RolUsuario::Operador,
        false,
    );
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
        assert_eq!(items.iter().find(|u| u.id == id).unwrap().cedula, "0-01");
    }
    core.actualizar_usuario(
        &actor,
        id,
        ActualizarUsuarioInput {
            cedula: "0-02".into(),
            nombre: "José Peña".into(),
            rol: RolUsuario::Administrador,
        },
        false,
    )
    .unwrap();
    core.activar_usuario(&actor, id).unwrap();
    let items = core
        .buscar_usuarios(
            &actor,
            &FiltroUsuarios {
                texto: Some("pena".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let u = items.iter().find(|u| u.id == id).unwrap();
    assert!(u.activo);
    assert_eq!(u.rol, RolUsuario::Administrador);
    assert!(!format!("{u:?}").contains("password_hash"));
    drop(core);
    let core = AppCore::abrir(&ruta).unwrap();
    let actor = sesion_root(&core);
    assert!(
        core.buscar_usuarios(
            &actor,
            &FiltroUsuarios {
                texto: Some("pena".into()),
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

#[test]
fn password_real_cambia_autenticacion_y_persiste() {
    let ruta = ruta("usuarios-password");
    let core = AppCore::abrir(&ruta).unwrap();
    root(&core);
    let actor = sesion_root(&core);
    let id = crear(
        &core,
        &actor,
        "USR-1",
        "Usuario",
        RolUsuario::Operador,
        true,
    );
    core.cambiar_password_usuario(&actor, id, "password-B")
        .unwrap();
    assert!(matches!(
        core.autenticar("USR-1", "password-A"),
        Err(AutenticacionError::CredencialesInvalidas)
    ));
    assert_eq!(core.autenticar("USR-1", "password-B").unwrap().id, id);
    drop(core);
    let core = AppCore::abrir(&ruta).unwrap();
    assert_eq!(core.autenticar("USR-1", "password-B").unwrap().id, id);
    drop(core);
    fs::remove_file(ruta).unwrap();
}

#[test]
fn n5_rechaza_edicion_y_desactivacion_del_ultimo_root_sin_cambios_parciales() {
    let ruta = ruta("usuarios-root");
    let core = AppCore::abrir(&ruta).unwrap();
    let id = root(&core);
    let actor = sesion_root(&core);
    let error = core.actualizar_usuario(
        &actor,
        id,
        ActualizarUsuarioInput {
            cedula: "ROOT-NUEVO".into(),
            nombre: "Nombre Nuevo".into(),
            rol: RolUsuario::Administrador,
        },
        false,
    );
    assert!(matches!(error, Err(UsuarioServiceError::UltimoRootActivo)));
    assert!(matches!(
        core.desactivar_usuario(&actor, id),
        Err(UsuarioServiceError::UltimoRootActivo)
    ));
    let u = core
        .buscar_usuarios(&actor, &FiltroUsuarios::default())
        .unwrap()
        .into_iter()
        .find(|u| u.id == id)
        .unwrap();
    assert_eq!(u.cedula, "ROOT-1");
    assert_eq!(u.nombre, "Root Inicial");
    assert_eq!(u.rol, RolUsuario::Root);
    assert!(u.activo);
    drop(core);
    fs::remove_file(ruta).unwrap();
}
