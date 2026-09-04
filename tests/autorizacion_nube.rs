//! `GestionarNube` (configurar el secreto del dispositivo) es exclusivo de
//! ROOT; `UsarNube` (sincronizar/leer/cerrar ingresos remotos) es de
//! cualquier rol activo -- ver el doc-comment de
//! `application::nube::GestionNubeError`. Antes ambos fallos de
//! autorización compartían la misma variante de error con un mensaje
//! redactado sólo para el caso ROOT; estas pruebas fijan el comportamiento
//! correcto: variantes distintas, cada una alcanzable desde el rol que le
//! corresponde.

#![cfg(feature = "nube")]

use rusqlite::Connection;

use control_acceso::{
    application::{AppCore, GestionNubeError},
    database::schema::initialize_database,
    models::usuario::RolUsuario,
    services::autenticacion_service::UsuarioSesion,
};

fn sesion(id: i64, rol: RolUsuario) -> UsuarioSesion {
    UsuarioSesion {
        id,
        cedula: format!("U{id}"),
        nombre: format!("Usuario {id}"),
        rol,
    }
}

fn base() -> AppCore {
    let connection = Connection::open_in_memory().unwrap();
    initialize_database(&connection).unwrap();
    connection
        .execute_batch(
            "INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo) VALUES
                (1,'U1','Root','hash','ROOT',1),
                (2,'U2','Administrador','hash','ADMINISTRADOR',1),
                (3,'U3','Operador','hash','OPERADOR',1),
                (4,'U4','Inactivo','hash','ROOT',0);",
        )
        .unwrap();
    AppCore::new(connection)
}

#[test]
fn solo_root_activo_puede_gestionar_la_nube() {
    let core = base();

    core.autorizar_gestion_nube(&sesion(1, RolUsuario::Root))
        .unwrap();

    assert!(matches!(
        core.autorizar_gestion_nube(&sesion(2, RolUsuario::Administrador)),
        Err(GestionNubeError::OperacionNoAutorizada)
    ));
    assert!(matches!(
        core.autorizar_gestion_nube(&sesion(3, RolUsuario::Operador)),
        Err(GestionNubeError::OperacionNoAutorizada)
    ));
    assert!(matches!(
        core.autorizar_gestion_nube(&sesion(4, RolUsuario::Root)),
        Err(GestionNubeError::OperacionNoAutorizada)
    ));
}

#[test]
fn cualquier_rol_activo_puede_usar_la_nube() {
    let core = base();

    core.autorizar_uso_nube(&sesion(1, RolUsuario::Root))
        .unwrap();
    core.autorizar_uso_nube(&sesion(2, RolUsuario::Administrador))
        .unwrap();
    core.autorizar_uso_nube(&sesion(3, RolUsuario::Operador))
        .unwrap();
}

/// Regresión del hallazgo de auditoría: una sesión inactiva/expirada que
/// intenta `UsarNube` (permitido para su rol) debe recibir
/// `UsoNoAutorizado`, no `OperacionNoAutorizada` -- ese mensaje está
/// redactado sólo para el caso exclusivo de ROOT (`GestionarNube`) y
/// confunde a quien depura un fallo de una operación que en realidad
/// cualquier rol puede hacer.
#[test]
fn una_sesion_inactiva_que_intenta_usar_la_nube_no_recibe_el_mensaje_de_root() {
    let core = base();

    assert!(matches!(
        core.autorizar_uso_nube(&sesion(4, RolUsuario::Root)),
        Err(GestionNubeError::UsoNoAutorizado)
    ));
}
